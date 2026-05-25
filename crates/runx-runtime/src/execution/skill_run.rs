// rust-style-allow: large-file - native skill execution keeps request parsing,
// continuation hydration, and sealed receipt assembly together for parity review.
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use runx_contracts::{
    ClosureDisposition, JsonNumber, JsonObject, JsonValue, ResolutionRequest, ResolutionResponse,
    ResolutionResponseActor, sha256_hex,
};
use runx_core::state_machine::GraphStatus;
use runx_parser::{
    ExecutionGraph, SkillRunnerDefinition, SkillRunnerManifest, parse_runner_manifest_yaml,
    validate_runner_manifest,
};
use serde::{Deserialize, Serialize};
#[cfg(feature = "cli-tool")]
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::RuntimeError;
#[cfg(feature = "cli-tool")]
use crate::adapter::SkillAdapter;
use crate::adapter::{InvocationStatus, SkillInvocation, SkillOutput};
#[cfg(feature = "cli-tool")]
use crate::adapters::cli_tool::CliToolAdapter;
use crate::agent_invocation::{
    AgentActInvocationSourceType, agent_act_invocation_id, agent_act_resolution_request,
};
use crate::execution::orchestrator::SkillRunRequest;
use crate::execution::runner::{GraphCheckpoint, GraphRun, Runtime, RuntimeOptions};
use crate::host::Host;
use crate::receipts::paths::{
    RUNX_CWD_ENV, RUNX_PROJECT_DIR_ENV, ReceiptPathInputs, resolve_receipt_path,
};
use crate::receipts::store::{LocalReceiptStore, ReceiptStoreError};
use crate::receipts::{
    RuntimeReceiptSignatureConfig, StepReceiptWithDisposition,
    step_receipt_with_disposition_and_policy,
};

const SKILL_RUN_SCHEMA: &str = "runx.skill_run.v1";
const GRAPH_SKILL_STATE_SCHEMA: &str = "runx.graph_skill_state.v1";

#[derive(Debug, Error)]
pub enum SkillRunError {
    #[error("skill run failed: {0}")]
    Invalid(String),
    #[error(transparent)]
    Runtime(#[from] RuntimeError),
    #[error(transparent)]
    ReceiptStore(#[from] ReceiptStoreError),
}

pub(crate) fn execute_skill_run(request: &SkillRunRequest) -> Result<JsonValue, SkillRunError> {
    let signature_config = RuntimeReceiptSignatureConfig::from_env(&request.env)
        .map_err(|error| SkillRunError::Invalid(error.to_string()))?;
    let skill_dir = resolve_skill_dir(&request.skill_path)?;
    let manifest = load_runner_manifest(&skill_dir)?;
    let runner = selected_runner(&manifest)?;
    if runner.source.source_type == runx_parser::SourceKind::CliTool
        && request.local_credential.is_some()
    {
        return Err(invalid(
            "local credential process-env delivery is not supported for cli-tool runners",
        ));
    }
    let invocation = runner_invocation(
        &skill_dir,
        runner,
        &request.inputs,
        &request.env,
        request.local_credential.as_ref(),
    )?;
    if runner.source.source_type == runx_parser::SourceKind::CliTool {
        return execute_cli_tool_skill_run(
            request,
            &signature_config,
            &manifest,
            runner,
            invocation,
        );
    }
    if runner.source.source_type == runx_parser::SourceKind::Graph {
        return execute_graph_skill_run(request, &signature_config, &manifest, runner);
    }

    execute_agent_skill_run(request, &signature_config, &manifest, runner, invocation)
}

fn execute_agent_skill_run(
    request: &SkillRunRequest,
    signature_config: &RuntimeReceiptSignatureConfig,
    manifest: &SkillRunnerManifest,
    runner: &SkillRunnerDefinition,
    invocation: SkillInvocation,
) -> Result<JsonValue, SkillRunError> {
    let source_type = agent_invocation_source_type(runner.source.source_type.as_str())?;
    let request_id = agent_act_invocation_id(&invocation, source_type);
    let run_id = agent_run_id(request, &request_id)?;
    let resolution_request = agent_request(&invocation, source_type)?;

    let Some(answers_path) = &request.answers_path else {
        return Ok(JsonValue::Object(needs_agent_output(
            &run_id,
            &request_id,
            resolution_request,
        )));
    };

    let answer = read_answer(answers_path, &request_id)?;
    let stdout = serde_json::to_string(&answer)
        .map_err(|error| SkillRunError::Invalid(format!("failed to serialize answer: {error}")))?;
    let disposition = answer_disposition(&answer);
    let receipt = seal_skill_answer(&run_id, runner, &stdout, disposition, signature_config)?;
    write_skill_receipt(request, &receipt, signature_config)?;

    Ok(JsonValue::Object(sealed_output(
        manifest,
        &run_id,
        &agent_skill_output(stdout, &receipt),
        &answer,
        &receipt,
        contract_json_value(&receipt)?,
    )))
}

// rust-style-allow: long-function because graph-backed skill execution keeps
// checkpoint hydration, host resolution, and final receipt sealing in one path.
fn execute_graph_skill_run(
    request: &SkillRunRequest,
    signature_config: &RuntimeReceiptSignatureConfig,
    manifest: &SkillRunnerManifest,
    runner: &SkillRunnerDefinition,
) -> Result<JsonValue, SkillRunError> {
    if request.local_credential.is_some() {
        return Err(invalid(
            "local credential process-env delivery is not supported for graph runners",
        ));
    }
    let graph = runner
        .source
        .graph
        .clone()
        .ok_or_else(|| invalid("graph runner is missing source.graph"))?;
    let graph = materialize_graph_inputs(graph, &request.inputs);
    let run_id = graph_run_id(request, runner)?;
    let skill_dir = resolve_skill_dir(&request.skill_path)?;
    let env = graph_runtime_env(request, &skill_dir);
    let runtime = Runtime::new(
        SkillRunGraphAdapter,
        RuntimeOptions {
            created_at: crate::time::now_iso8601(),
            env,
            receipt_signature: signature_config.clone(),
            payment_supervisor: Default::default(),
        },
    );
    let answers = match &request.answers_path {
        Some(path) => read_answers(path)?,
        None => JsonObject::new(),
    };
    let mut host = SkillRunGraphHost::new(answers);
    let mut checkpoint = if request.answers_path.is_some() {
        read_graph_state(request, &run_id, &runner.name)?.checkpoint
    } else {
        runtime.run_graph_until_steps_with_host(&skill_dir, &graph, 0, &mut host)?
    };

    loop {
        let previous_checkpoint = checkpoint.clone();
        match runtime
            .resume_graph_until_steps_with_host(&skill_dir, &graph, checkpoint, 1, &mut host)
        {
            Ok(next_checkpoint) => {
                if next_checkpoint.state.status == GraphStatus::Succeeded {
                    let mut final_host = SkillRunGraphHost::new(JsonObject::new());
                    let run = runtime.resume_graph_with_host(
                        &skill_dir,
                        graph.clone(),
                        previous_checkpoint,
                        &mut final_host,
                    )?;
                    write_skill_receipt(request, &run.receipt, signature_config)?;
                    let payload = graph_payload(&run)?;
                    let output = graph_skill_output(&payload, &run)?;
                    return Ok(JsonValue::Object(sealed_output(
                        manifest,
                        &run_id,
                        &output,
                        &payload,
                        &run.receipt,
                        contract_json_value(&run.receipt)?,
                    )));
                }
                write_graph_state(
                    request,
                    &run_id,
                    &GraphSkillRunState {
                        schema: GRAPH_SKILL_STATE_SCHEMA.to_owned(),
                        run_id: run_id.clone(),
                        runner_name: runner.name.clone(),
                        checkpoint: next_checkpoint.clone(),
                    },
                )?;
                checkpoint = next_checkpoint;
            }
            Err(RuntimeError::GraphBlocked { .. }) if host.pending_request().is_some() => {
                write_graph_state(
                    request,
                    &run_id,
                    &GraphSkillRunState {
                        schema: GRAPH_SKILL_STATE_SCHEMA.to_owned(),
                        run_id: run_id.clone(),
                        runner_name: runner.name.clone(),
                        checkpoint: previous_checkpoint,
                    },
                )?;
                let (request_id, request_value) = host
                    .pending_request()
                    .ok_or_else(|| invalid("graph blocked without pending request"))?;
                return Ok(JsonValue::Object(needs_agent_output(
                    &run_id,
                    request_id,
                    request_value.clone(),
                )));
            }
            Err(error) => return Err(error.into()),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct GraphSkillRunState {
    schema: String,
    run_id: String,
    runner_name: String,
    checkpoint: GraphCheckpoint,
}

#[derive(Clone, Copy, Debug, Default)]
struct SkillRunGraphAdapter;

impl crate::adapter::SkillAdapter for SkillRunGraphAdapter {
    fn adapter_type(&self) -> &'static str {
        "skill-run-graph"
    }

    fn invoke(&self, request: SkillInvocation) -> Result<SkillOutput, RuntimeError> {
        match request.source.source_type.as_str() {
            "cli-tool" => invoke_graph_cli_tool(request),
            "catalog" => invoke_graph_catalog_tool(request),
            other => Err(RuntimeError::UnsupportedAdapter {
                adapter_type: other.to_owned(),
            }),
        }
    }
}

#[cfg(feature = "cli-tool")]
fn invoke_graph_cli_tool(request: SkillInvocation) -> Result<SkillOutput, RuntimeError> {
    CliToolAdapter.invoke(request)
}

#[cfg(not(feature = "cli-tool"))]
fn invoke_graph_cli_tool(request: SkillInvocation) -> Result<SkillOutput, RuntimeError> {
    Err(RuntimeError::UnsupportedAdapter {
        adapter_type: request.source.source_type.as_str().to_owned(),
    })
}

#[cfg(feature = "catalog")]
fn invoke_graph_catalog_tool(request: SkillInvocation) -> Result<SkillOutput, RuntimeError> {
    crate::adapters::catalog::CatalogAdapter::default().invoke(request)
}

#[cfg(not(feature = "catalog"))]
fn invoke_graph_catalog_tool(request: SkillInvocation) -> Result<SkillOutput, RuntimeError> {
    Err(RuntimeError::UnsupportedAdapter {
        adapter_type: request.source.source_type.as_str().to_owned(),
    })
}

#[derive(Default)]
struct SkillRunGraphHost {
    answers: JsonObject,
    pending: Vec<(String, JsonValue)>,
}

impl SkillRunGraphHost {
    fn new(answers: JsonObject) -> Self {
        Self {
            answers,
            pending: Vec::new(),
        }
    }

    fn pending_request(&self) -> Option<(&str, &JsonValue)> {
        self.pending
            .first()
            .map(|(request_id, request)| (request_id.as_str(), request))
    }
}

impl Host for SkillRunGraphHost {
    fn report(&mut self, _event: runx_contracts::ExecutionEvent) -> Result<(), RuntimeError> {
        Ok(())
    }

    fn resolve(
        &mut self,
        request: ResolutionRequest,
    ) -> Result<Option<ResolutionResponse>, RuntimeError> {
        let request_id = resolution_request_id(&request).to_owned();
        if let Some(answer) = self.answers.get(&request_id) {
            return Ok(Some(ResolutionResponse {
                actor: ResolutionResponseActor::Agent,
                payload: answer.clone(),
            }));
        }
        let request_value = serde_json::to_value(&request)
            .and_then(serde_json::from_value)
            .map_err(|source| RuntimeError::json("serializing graph resolution request", source))?;
        self.pending.push((request_id, request_value));
        Ok(None)
    }
}

fn resolution_request_id(request: &ResolutionRequest) -> &str {
    match request {
        ResolutionRequest::Input { id, .. }
        | ResolutionRequest::Approval { id, .. }
        | ResolutionRequest::AgentAct { id, .. } => id.as_str(),
    }
}

fn graph_run_id(
    request: &SkillRunRequest,
    runner: &SkillRunnerDefinition,
) -> Result<String, SkillRunError> {
    match (&request.run_id, &request.answers_path) {
        (Some(run_id), Some(_)) => Ok(run_id.clone()),
        (Some(_), None) => Err(invalid("runx skill --run-id requires --answers")),
        (None, Some(_)) => Err(invalid("runx skill --answers requires --run-id")),
        (None, None) => {
            let input_bytes = serde_json::to_vec(&request.inputs).unwrap_or_default();
            let digest = sha256_hex(&input_bytes);
            Ok(format!(
                "run_{}_{}",
                identifier_segment(&runner.name),
                digest.chars().take(12).collect::<String>()
            ))
        }
    }
}

fn graph_runtime_env(request: &SkillRunRequest, skill_dir: &Path) -> BTreeMap<String, String> {
    let mut env = request.env.clone();
    for key in ["PATH", "SystemRoot", "PATHEXT"] {
        if !env.contains_key(key) {
            if let Ok(value) = std::env::var(key) {
                env.insert(key.to_owned(), value);
            }
        }
    }
    let cwd = request.cwd.to_string_lossy().into_owned();
    env.entry(RUNX_CWD_ENV.to_owned())
        .or_insert_with(|| cwd.clone());
    env.entry(RUNX_PROJECT_DIR_ENV.to_owned()).or_insert(cwd);
    if !env.contains_key("RUNX_TOOL_ROOTS") {
        if let Some(joined) = inferred_tool_roots(skill_dir) {
            env.insert("RUNX_TOOL_ROOTS".to_owned(), joined);
        }
    }
    env
}

fn inferred_tool_roots(skill_dir: &Path) -> Option<String> {
    let root = skill_dir
        .parent()
        .filter(|parent| parent.file_name().and_then(|name| name.to_str()) == Some("skills"))
        .and_then(Path::parent)?;
    let roots = [root.join("packages/cli/tools"), root.join("tools")]
        .into_iter()
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();
    if roots.is_empty() {
        return None;
    }
    std::env::join_paths(roots)
        .ok()
        .map(|value| value.to_string_lossy().into_owned())
}

fn read_answers(path: &Path) -> Result<JsonObject, SkillRunError> {
    let raw = fs::read_to_string(path)
        .map_err(|source| RuntimeError::io(format!("reading {}", path.display()), source))?;
    let value = serde_json::from_str::<JsonValue>(&raw).map_err(|source| {
        RuntimeError::json(format!("parsing answers file {}", path.display()), source)
    })?;
    let answers = match value {
        JsonValue::Object(mut object) => match object.remove("answers") {
            Some(JsonValue::Object(nested)) => nested,
            Some(_) => return Err(invalid("answers field must be a JSON object")),
            None => object,
        },
        _ => return Err(invalid("answers file must be a JSON object")),
    };
    Ok(answers)
}

fn graph_state_path(request: &SkillRunRequest, run_id: &str) -> PathBuf {
    let receipt_path = resolve_receipt_path(ReceiptPathInputs {
        explicit_dir: request.receipt_dir.as_deref(),
        runtime_config: None,
        env: &request.env,
        cwd: &request.cwd,
    });
    receipt_path
        .path
        .join("runs")
        .join(format!("{}.graph-state.json", identifier_segment(run_id)))
}

fn write_graph_state(
    request: &SkillRunRequest,
    run_id: &str,
    state: &GraphSkillRunState,
) -> Result<(), SkillRunError> {
    let path = graph_state_path(request, run_id);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|source| RuntimeError::io(format!("creating {}", parent.display()), source))?;
    }
    let bytes = serde_json::to_vec_pretty(state)
        .map_err(|source| RuntimeError::json("serializing graph state", source))?;
    fs::write(&path, bytes)
        .map_err(|source| RuntimeError::io(format!("writing {}", path.display()), source))?;
    Ok(())
}

fn read_graph_state(
    request: &SkillRunRequest,
    run_id: &str,
    runner_name: &str,
) -> Result<GraphSkillRunState, SkillRunError> {
    let path = graph_state_path(request, run_id);
    let raw = fs::read_to_string(&path)
        .map_err(|source| RuntimeError::io(format!("reading {}", path.display()), source))?;
    let state: GraphSkillRunState = serde_json::from_str(&raw)
        .map_err(|source| RuntimeError::json(format!("parsing {}", path.display()), source))?;
    if state.schema != GRAPH_SKILL_STATE_SCHEMA {
        return Err(invalid(format!(
            "graph state schema mismatch for run {run_id}: expected {GRAPH_SKILL_STATE_SCHEMA}, got {}",
            state.schema
        )));
    }
    if state.run_id != run_id {
        return Err(invalid(format!(
            "graph state run_id mismatch: expected {run_id}, got {}",
            state.run_id
        )));
    }
    if state.runner_name != runner_name {
        return Err(invalid(format!(
            "graph state runner_name mismatch for run {run_id}: expected {runner_name}, got {}",
            state.runner_name
        )));
    }
    Ok(state)
}

fn materialize_graph_inputs(
    mut graph: ExecutionGraph,
    graph_inputs: &BTreeMap<String, JsonValue>,
) -> ExecutionGraph {
    let graph_inputs = graph_inputs
        .iter()
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect::<JsonObject>();
    for step in &mut graph.steps {
        let mut inputs = graph_inputs.clone();
        for (key, value) in &step.inputs {
            if let Some(value) = materialize_graph_input_value(value, &graph_inputs) {
                inputs.insert(key.clone(), value);
            } else {
                inputs.remove(key);
            }
        }
        step.inputs = inputs;
    }
    graph
}

fn materialize_graph_input_value(
    value: &JsonValue,
    graph_inputs: &JsonObject,
) -> Option<JsonValue> {
    match value {
        JsonValue::String(value) => {
            if let Some(path) = value.strip_prefix("$input.") {
                return resolve_json_path(graph_inputs, path).cloned();
            }
            Some(JsonValue::String(value.clone()))
        }
        JsonValue::Array(values) => Some(JsonValue::Array(
            values
                .iter()
                .filter_map(|value| materialize_graph_input_value(value, graph_inputs))
                .collect(),
        )),
        JsonValue::Object(object) => Some(JsonValue::Object(
            object
                .iter()
                .filter_map(|(key, value)| {
                    materialize_graph_input_value(value, graph_inputs)
                        .map(|value| (key.clone(), value))
                })
                .collect(),
        )),
        other => Some(other.clone()),
    }
}

fn resolve_json_path<'a>(object: &'a JsonObject, path: &str) -> Option<&'a JsonValue> {
    let mut segments = path.split('.');
    let mut value = object.get(segments.next()?)?;
    for segment in segments {
        let JsonValue::Object(nested) = value else {
            return None;
        };
        value = nested.get(segment)?;
    }
    Some(value)
}

fn graph_payload(run: &GraphRun) -> Result<JsonValue, SkillRunError> {
    let mut payload = JsonObject::new();
    payload.insert(
        "graph".to_owned(),
        JsonValue::String(run.graph.name.clone()),
    );
    payload.insert(
        "graph_status".to_owned(),
        JsonValue::String(format!("{:?}", run.state.status)),
    );
    let mut step_outputs = JsonObject::new();
    let mut step_summaries = Vec::new();
    for step in &run.steps {
        let mut summary = JsonObject::new();
        summary.insert(
            "step_id".to_owned(),
            JsonValue::String(step.step_id.clone()),
        );
        summary.insert("skill".to_owned(), JsonValue::String(step.skill.clone()));
        summary.insert(
            "status".to_owned(),
            JsonValue::String(if step.output.succeeded() {
                "success".to_owned()
            } else {
                "failure".to_owned()
            }),
        );
        summary.insert(
            "receipt_id".to_owned(),
            JsonValue::String(step.receipt.id.to_string()),
        );
        step_summaries.push(JsonValue::Object(summary));
        step_outputs.insert(
            step.step_id.clone(),
            JsonValue::Object(step.outputs.clone()),
        );
        for (key, value) in &step.outputs {
            payload.entry(key.clone()).or_insert_with(|| value.clone());
        }
    }
    payload.insert("steps".to_owned(), JsonValue::Array(step_summaries));
    payload.insert("step_outputs".to_owned(), JsonValue::Object(step_outputs));
    Ok(JsonValue::Object(payload))
}

fn graph_skill_output(payload: &JsonValue, run: &GraphRun) -> Result<SkillOutput, SkillRunError> {
    let stdout = serde_json::to_string(payload)
        .map_err(|source| RuntimeError::json("serializing graph payload", source))?;
    Ok(SkillOutput {
        status: if run.state.status == GraphStatus::Succeeded {
            InvocationStatus::Success
        } else {
            InvocationStatus::Failure
        },
        stdout,
        stderr: String::new(),
        exit_code: Some(0),
        duration_ms: 0,
        metadata: JsonObject::new(),
    })
}

fn agent_run_id(request: &SkillRunRequest, request_id: &str) -> Result<String, SkillRunError> {
    match (&request.run_id, &request.answers_path) {
        (Some(run_id), Some(_)) => Ok(run_id.clone()),
        (Some(_), None) => Err(invalid("runx skill --run-id requires --answers")),
        (None, Some(_)) => Err(invalid("runx skill --answers requires --run-id")),
        (None, None) => Ok(format!("run_{}", identifier_segment(request_id))),
    }
}

fn agent_skill_output(stdout: String, receipt: &runx_contracts::Receipt) -> SkillOutput {
    let succeeded = receipt.seal.disposition == ClosureDisposition::Closed;
    SkillOutput {
        status: if succeeded {
            InvocationStatus::Success
        } else {
            InvocationStatus::Failure
        },
        stdout,
        stderr: if succeeded {
            String::new()
        } else {
            format!(
                "agent act closed with {}",
                closure_disposition_label(&receipt.seal.disposition)
            )
        },
        exit_code: succeeded.then_some(0),
        duration_ms: 0,
        metadata: JsonObject::new(),
    }
}

fn resolve_skill_dir(path: &Path) -> Result<PathBuf, SkillRunError> {
    if path.is_dir() {
        return Ok(path.to_path_buf());
    }
    if path.file_name().and_then(|name| name.to_str()) == Some("SKILL.md") {
        return path
            .parent()
            .map(Path::to_path_buf)
            .ok_or_else(|| invalid(format!("skill path has no parent: {}", path.display())));
    }
    Err(invalid(format!(
        "runx skill requires a skill package directory or SKILL.md: {}",
        path.display()
    )))
}

fn load_runner_manifest(skill_dir: &Path) -> Result<SkillRunnerManifest, SkillRunError> {
    let manifest_path = skill_dir.join("X.yaml");
    let raw = fs::read_to_string(&manifest_path).map_err(|source| {
        RuntimeError::io(format!("reading {}", manifest_path.display()), source)
    })?;
    let parsed = parse_runner_manifest_yaml(&raw).map_err(RuntimeError::from)?;
    validate_runner_manifest(parsed)
        .map_err(RuntimeError::from)
        .map_err(Into::into)
}

fn selected_runner(
    manifest: &SkillRunnerManifest,
) -> Result<&SkillRunnerDefinition, SkillRunError> {
    let defaults = manifest
        .runners
        .values()
        .filter(|runner| runner.default)
        .collect::<Vec<_>>();
    match defaults.as_slice() {
        [runner] => Ok(*runner),
        [] if manifest.runners.len() == 1 => manifest
            .runners
            .values()
            .next()
            .ok_or_else(|| invalid("runner manifest declares no runners")),
        [] => Err(invalid("runner manifest has no default runner")),
        _ => Err(invalid("runner manifest declares multiple default runners")),
    }
}

fn runner_invocation(
    skill_dir: &Path,
    runner: &SkillRunnerDefinition,
    inputs: &BTreeMap<String, JsonValue>,
    env: &BTreeMap<String, String>,
    local_credential: Option<&crate::execution::orchestrator::LocalCredentialDescriptor>,
) -> Result<SkillInvocation, SkillRunError> {
    if !matches!(
        runner.source.source_type.as_str(),
        "agent" | "agent-step" | "cli-tool" | "graph"
    ) {
        return Err(invalid(format!(
            "runx skill native execution only supports agent, agent-step, graph, and cli-tool runners, got {}",
            runner.source.source_type
        )));
    }
    let credential_delivery = match local_credential {
        Some(descriptor) => crate::credentials::CredentialDelivery::from_local_descriptor(
            descriptor.provider.clone(),
            descriptor.auth_mode.clone(),
            descriptor.env_var.clone(),
            descriptor.material_ref.clone(),
            descriptor.scopes.clone(),
            descriptor.secret.clone(),
        )
        .map_err(|error| invalid(format!("local credential provision failed: {error}")))?,
        None => crate::credentials::CredentialDelivery::none(),
    };
    Ok(SkillInvocation {
        skill_name: runner.name.clone(),
        source: runner.source.clone(),
        inputs: inputs.clone().into_iter().collect(),
        resolved_inputs: JsonObject::new(),
        skill_directory: skill_dir.to_path_buf(),
        env: env.clone(),
        credential_delivery,
    })
}

#[cfg(feature = "cli-tool")]
fn execute_cli_tool_skill_run(
    request: &SkillRunRequest,
    signature_config: &RuntimeReceiptSignatureConfig,
    manifest: &SkillRunnerManifest,
    runner: &SkillRunnerDefinition,
    invocation: SkillInvocation,
) -> Result<JsonValue, SkillRunError> {
    if request.answers_path.is_some() {
        return Err(invalid(
            "runx skill cli-tool runners do not support --answers",
        ));
    }
    let run_id = request
        .run_id
        .clone()
        .unwrap_or_else(|| cli_tool_run_id(runner, &request.inputs));
    let credential_observation = invocation.credential_delivery.public_observation().cloned();
    let mut output = CliToolAdapter.invoke(invocation)?;
    if let Some(observation) = &credential_observation {
        record_credential_observation(&mut output, observation)?;
    }
    let disposition = if output.succeeded() {
        ClosureDisposition::Closed
    } else {
        ClosureDisposition::Failed
    };
    let receipt = seal_skill_output(
        &run_id,
        runner,
        &output,
        disposition.clone(),
        format!("process_{}", closure_disposition_label(&disposition)),
        format!("cli-tool {} completed", runner.name),
        signature_config,
    )?;
    write_skill_receipt(request, &receipt, signature_config)?;
    Ok(JsonValue::Object(sealed_output(
        manifest,
        &run_id,
        &output,
        &parse_output_payload(&output.stdout),
        &receipt,
        contract_json_value(&receipt)?,
    )))
}

#[cfg(not(feature = "cli-tool"))]
fn execute_cli_tool_skill_run(
    _request: &SkillRunRequest,
    _signature_config: &RuntimeReceiptSignatureConfig,
    _manifest: &SkillRunnerManifest,
    _runner: &SkillRunnerDefinition,
    _invocation: SkillInvocation,
) -> Result<JsonValue, SkillRunError> {
    Err(invalid(
        "runx skill cli-tool execution is unavailable because runx-runtime was built without the cli-tool feature",
    ))
}

fn write_skill_receipt(
    request: &SkillRunRequest,
    receipt: &runx_contracts::Receipt,
    signature_config: &RuntimeReceiptSignatureConfig,
) -> Result<(), SkillRunError> {
    let receipt_path = resolve_receipt_path(ReceiptPathInputs {
        explicit_dir: request.receipt_dir.as_deref(),
        runtime_config: None,
        env: &request.env,
        cwd: &request.cwd,
    });
    LocalReceiptStore::new(&receipt_path.path)
        .write_receipt_with_policy(receipt, signature_config.signature_policy())
        .map_err(Into::into)
}

fn agent_invocation_source_type(
    value: &str,
) -> Result<AgentActInvocationSourceType, SkillRunError> {
    AgentActInvocationSourceType::from_contract_value(value)
        .ok_or_else(|| invalid(format!("unsupported agent source type {value}")))
}

fn agent_request(
    invocation: &SkillInvocation,
    source_type: AgentActInvocationSourceType,
) -> Result<JsonValue, SkillRunError> {
    contract_json_value(&agent_act_resolution_request(invocation, source_type)?)
}

fn needs_agent_output(run_id: &str, request_id: &str, request: JsonValue) -> JsonObject {
    let mut output = JsonObject::new();
    output.insert(
        "schema".to_owned(),
        JsonValue::String(SKILL_RUN_SCHEMA.to_owned()),
    );
    output.insert(
        "status".to_owned(),
        JsonValue::String("needs_agent".to_owned()),
    );
    output.insert("run_id".to_owned(), JsonValue::String(run_id.to_owned()));
    output.insert(
        "requests".to_owned(),
        JsonValue::Array(vec![request_for_public_loop(request_id, request)]),
    );
    output
}

fn request_for_public_loop(request_id: &str, request: JsonValue) -> JsonValue {
    let mut object = match request {
        JsonValue::Object(object) => object,
        _ => JsonObject::new(),
    };
    object.insert("id".to_owned(), JsonValue::String(request_id.to_owned()));
    object
        .entry("kind".to_owned())
        .or_insert_with(|| JsonValue::String("agent_act".to_owned()));
    JsonValue::Object(object)
}

fn read_answer(path: &Path, request_id: &str) -> Result<JsonValue, SkillRunError> {
    let raw = fs::read_to_string(path)
        .map_err(|source| RuntimeError::io(format!("reading {}", path.display()), source))?;
    let value = serde_json::from_str::<JsonValue>(&raw).map_err(|source| {
        RuntimeError::json(format!("parsing answers file {}", path.display()), source)
    })?;
    let answers = match &value {
        JsonValue::Object(object) => match object.get("answers") {
            Some(JsonValue::Object(nested)) => nested,
            _ => object,
        },
        _ => return Err(invalid("answers file must be a JSON object")),
    };
    answers
        .get(request_id)
        .cloned()
        .ok_or_else(|| invalid(format!("answers file did not include {request_id}")))
}

fn seal_skill_answer(
    run_id: &str,
    runner: &SkillRunnerDefinition,
    stdout: &str,
    disposition: ClosureDisposition,
    signature_config: &RuntimeReceiptSignatureConfig,
) -> Result<runx_contracts::Receipt, SkillRunError> {
    let disposition_label = closure_disposition_label(&disposition);
    let succeeded = disposition == ClosureDisposition::Closed;
    let status = if succeeded {
        InvocationStatus::Success
    } else {
        InvocationStatus::Failure
    };
    let skill_output = SkillOutput {
        status,
        stdout: stdout.to_owned(),
        stderr: if succeeded {
            String::new()
        } else {
            format!("agent act closed with {disposition_label}")
        },
        exit_code: succeeded.then_some(0),
        duration_ms: 0,
        metadata: JsonObject::new(),
    };
    seal_skill_output(
        run_id,
        runner,
        &skill_output,
        disposition,
        format!("agent_act_{disposition_label}"),
        format!("agent act closed with {disposition_label}"),
        signature_config,
    )
}

/// Record the non-secret credential-delivery observation on the skill output so
/// the sealed receipt carries an auditable trace that a credential was
/// provisioned for the run. The observation contains no secret material.
#[cfg(feature = "cli-tool")]
fn record_credential_observation(
    output: &mut SkillOutput,
    observation: &runx_contracts::CredentialDeliveryObservation,
) -> Result<(), SkillRunError> {
    let value: JsonValue = serde_json::to_value(observation)
        .and_then(serde_json::from_value)
        .map_err(|error| {
            SkillRunError::Invalid(format!(
                "serializing credential delivery observation: {error}"
            ))
        })?;
    output.metadata.insert(
        crate::adapter::CREDENTIAL_DELIVERY_OBSERVATIONS_METADATA.to_owned(),
        JsonValue::Array(vec![value]),
    );
    Ok(())
}

fn seal_skill_output(
    run_id: &str,
    runner: &SkillRunnerDefinition,
    output: &SkillOutput,
    disposition: ClosureDisposition,
    reason_code: String,
    summary: String,
    signature_config: &RuntimeReceiptSignatureConfig,
) -> Result<runx_contracts::Receipt, SkillRunError> {
    let graph_name = identifier_segment(run_id);
    let step_id = identifier_segment(&runner.name);
    Ok(step_receipt_with_disposition_and_policy(
        StepReceiptWithDisposition {
            graph_name: &graph_name,
            step_id: &step_id,
            attempt: 1,
            output,
            created_at: &crate::time::now_iso8601(),
            disposition,
            reason_code,
            summary,
        },
        signature_config.signature_policy(),
    )?)
}

fn answer_disposition(answer: &JsonValue) -> ClosureDisposition {
    match json_object(answer)
        .and_then(|object| object.get("closure"))
        .and_then(json_object)
        .and_then(|closure| closure.get("disposition"))
        .and_then(json_string)
    {
        Some("deferred") => ClosureDisposition::Deferred,
        Some("superseded") => ClosureDisposition::Superseded,
        Some("declined") => ClosureDisposition::Declined,
        Some("blocked") => ClosureDisposition::Blocked,
        Some("failed") => ClosureDisposition::Failed,
        Some("killed") => ClosureDisposition::Killed,
        Some("timed_out") => ClosureDisposition::TimedOut,
        _ => ClosureDisposition::Closed,
    }
}

fn json_object(value: &JsonValue) -> Option<&JsonObject> {
    match value {
        JsonValue::Object(object) => Some(object),
        _ => None,
    }
}

fn json_string(value: &JsonValue) -> Option<&str> {
    match value {
        JsonValue::String(value) => Some(value),
        _ => None,
    }
}

fn sealed_output(
    manifest: &SkillRunnerManifest,
    run_id: &str,
    skill_output: &SkillOutput,
    payload: &JsonValue,
    receipt: &runx_contracts::Receipt,
    receipt_value: JsonValue,
) -> JsonObject {
    let mut execution = JsonObject::new();
    execution.insert(
        "stdout".to_owned(),
        JsonValue::String(skill_output.stdout.clone()),
    );
    execution.insert(
        "stderr".to_owned(),
        JsonValue::String(skill_output.stderr.clone()),
    );
    execution.insert(
        "exit_code".to_owned(),
        skill_output.exit_code.map_or(JsonValue::Null, |exit_code| {
            JsonValue::Number(JsonNumber::I64(i64::from(exit_code)))
        }),
    );
    execution.insert("structured_output".to_owned(), payload.clone());
    if let Some(observations) = skill_output
        .metadata
        .get(crate::adapter::CREDENTIAL_DELIVERY_OBSERVATIONS_METADATA)
    {
        execution.insert(
            crate::adapter::CREDENTIAL_DELIVERY_OBSERVATIONS_METADATA.to_owned(),
            observations.clone(),
        );
    }

    let mut output = JsonObject::new();
    output.insert(
        "schema".to_owned(),
        JsonValue::String(SKILL_RUN_SCHEMA.to_owned()),
    );
    output.insert("status".to_owned(), JsonValue::String("sealed".to_owned()));
    output.insert(
        "skill_name".to_owned(),
        JsonValue::String(manifest.skill.clone().unwrap_or_else(|| "skill".to_owned())),
    );
    output.insert("run_id".to_owned(), JsonValue::String(run_id.to_owned()));
    output.insert(
        "receipt_id".to_owned(),
        JsonValue::String(receipt.id.to_string()),
    );
    output.insert(
        "closure".to_owned(),
        JsonValue::Object(closure_output(&receipt.seal)),
    );
    output.insert("receipt".to_owned(), receipt_value);
    output.insert("execution".to_owned(), JsonValue::Object(execution));
    output.insert("payload".to_owned(), payload.clone());
    output
}

fn closure_output(seal: &runx_contracts::Seal) -> JsonObject {
    let mut closure = JsonObject::new();
    closure.insert(
        "disposition".to_owned(),
        JsonValue::String(closure_disposition_label(&seal.disposition).to_owned()),
    );
    closure.insert(
        "reason_code".to_owned(),
        JsonValue::String(seal.reason_code.to_string()),
    );
    closure.insert(
        "summary".to_owned(),
        JsonValue::String(seal.summary.to_string()),
    );
    closure.insert(
        "closed_at".to_owned(),
        JsonValue::String(seal.closed_at.to_string()),
    );
    closure
}

fn closure_disposition_label(disposition: &ClosureDisposition) -> &'static str {
    match disposition {
        ClosureDisposition::Closed => "closed",
        ClosureDisposition::Deferred => "deferred",
        ClosureDisposition::Superseded => "superseded",
        ClosureDisposition::Declined => "declined",
        ClosureDisposition::Blocked => "blocked",
        ClosureDisposition::Failed => "failed",
        ClosureDisposition::Killed => "killed",
        ClosureDisposition::TimedOut => "timed_out",
    }
}

fn normalize_request_id(value: &str) -> String {
    let mut normalized = String::new();
    let mut replaced = false;
    for character in value.chars() {
        if character.is_ascii_alphanumeric() || matches!(character, '_' | '.' | '-') {
            normalized.push(character);
            replaced = false;
        } else if !replaced {
            normalized.push('_');
            replaced = true;
        }
    }
    normalized
}

fn identifier_segment(value: &str) -> String {
    normalize_request_id(value)
        .trim_matches(['.', '_', '-'])
        .replace('.', "-")
}

#[cfg(feature = "cli-tool")]
fn cli_tool_run_id(runner: &SkillRunnerDefinition, inputs: &BTreeMap<String, JsonValue>) -> String {
    let input_bytes = serde_json::to_vec(inputs).unwrap_or_default();
    let digest = Sha256::digest(input_bytes);
    format!(
        "run_{}_{}",
        identifier_segment(&runner.name),
        hex_prefix(&digest, 12)
    )
}

#[cfg(feature = "cli-tool")]
fn hex_prefix(bytes: &[u8], chars: usize) -> String {
    let full = bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    full.chars().take(chars).collect()
}

#[cfg(feature = "cli-tool")]
fn parse_output_payload(stdout: &str) -> JsonValue {
    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        return JsonValue::String(String::new());
    }
    serde_json::from_str(trimmed).unwrap_or_else(|_| JsonValue::String(trimmed.to_owned()))
}

fn contract_json_value(value: &impl serde::Serialize) -> Result<JsonValue, SkillRunError> {
    let value = serde_json::to_value(value)
        .map_err(|source| RuntimeError::json("serializing native skill contract value", source))?;
    serde_json::from_value(value).map_err(|source| {
        RuntimeError::json("normalizing native skill contract value", source).into()
    })
}

fn invalid(message: impl Into<String>) -> SkillRunError {
    SkillRunError::Invalid(message.into())
}
