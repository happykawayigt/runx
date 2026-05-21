use std::path::Path;

use runx_contracts::{ApprovalGate, JsonObject, JsonValue, ResolutionResponseActor};
use runx_parser::GraphStep;

use super::super::graph::{load_skill, output_object, resolve_inputs, skill_dir};
use super::authority::{
    enforce_step_authority_admission, enforce_step_authority_receipt_before_success,
};
use super::inputs::{optional_input_string, required_input_string, string_value, string_value_ref};
use super::{Runtime, StepRun};
use crate::RuntimeError;
use crate::adapter::{InvocationStatus, SkillAdapter, SkillInvocation, SkillOutput};
use crate::approval::{ApprovalResolution, request_approval};
use crate::caller::Caller;
use crate::receipts::step_receipt;

pub(super) fn output_error(run: &StepRun) -> String {
    if run.output.stderr.is_empty() {
        "cli-tool failed without stderr".to_owned()
    } else {
        run.output.stderr.clone()
    }
}

pub(super) fn run_step<A>(
    runtime: &Runtime<A>,
    graph_dir: &Path,
    graph_name: &str,
    step: &GraphStep,
    attempt: u32,
    prior_runs: &[StepRun],
    caller: &mut dyn Caller,
) -> Result<StepRun, RuntimeError>
where
    A: SkillAdapter,
{
    let inputs = resolve_inputs(step, prior_runs)?;
    let authority = enforce_step_authority_admission(step, &inputs)?;
    if step.run.is_some() {
        return run_native_step(runtime, graph_name, step, attempt, inputs, caller);
    }

    let skill_dir = skill_dir(graph_dir, step)?;
    let skill = load_skill(&skill_dir)?;
    let skill_name = skill.name.clone();
    let output = runtime.adapter.invoke(SkillInvocation {
        skill_name: skill.name,
        source: skill.source,
        inputs,
        resolved_inputs: JsonObject::new(),
        skill_directory: skill_dir,
        env: runtime.options.env.clone(),
    })?;
    let outputs = output_object(&output);
    let receipt = step_receipt(
        graph_name,
        &step.id,
        attempt,
        &output,
        &runtime.options.created_at,
    )?;
    enforce_step_authority_receipt_before_success(step, authority.as_ref(), &output, &receipt)?;
    Ok(StepRun {
        step_id: step.id.clone(),
        attempt,
        skill: skill_name,
        runner: step.runner.clone(),
        fanout_group: step.fanout_group.clone(),
        output,
        outputs,
        receipt,
    })
}

pub(super) fn run_native_step<A>(
    runtime: &Runtime<A>,
    graph_name: &str,
    step: &GraphStep,
    attempt: u32,
    inputs: JsonObject,
    caller: &mut dyn Caller,
) -> Result<StepRun, RuntimeError>
where
    A: SkillAdapter,
{
    let run_type = run_type(step)?;
    match run_type.as_str() {
        "approval" => run_approval_step(runtime, graph_name, step, attempt, inputs, caller),
        other => Err(RuntimeError::UnsupportedRunStep {
            step_id: step.id.clone(),
            run_type: other.to_owned(),
        }),
    }
}

pub(super) fn run_approval_step<A>(
    runtime: &Runtime<A>,
    graph_name: &str,
    step: &GraphStep,
    attempt: u32,
    inputs: JsonObject,
    caller: &mut dyn Caller,
) -> Result<StepRun, RuntimeError>
where
    A: SkillAdapter,
{
    let gate = approval_gate(step, &inputs)?;
    let request_id = format!("{}_approval", step.id);
    let resolution = request_approval(caller, request_id, gate.clone()).map_err(|source| {
        RuntimeError::InvalidRunStep {
            step_id: step.id.clone(),
            reason: source.to_string(),
        }
    })?;
    let outputs = approval_outputs(step, &gate, &resolution)?;
    let stdout = serde_json::to_string(&outputs)
        .map_err(|source| RuntimeError::json("serializing approval run output", source))?;
    let output = SkillOutput {
        status: InvocationStatus::Success,
        stdout,
        stderr: String::new(),
        exit_code: Some(0),
        duration_ms: 0,
        metadata: JsonObject::new(),
    };
    let receipt = step_receipt(
        graph_name,
        &step.id,
        attempt,
        &output,
        &runtime.options.created_at,
    )?;
    Ok(StepRun {
        step_id: step.id.clone(),
        attempt,
        skill: "run:approval".to_owned(),
        runner: step.runner.clone(),
        fanout_group: step.fanout_group.clone(),
        output,
        outputs,
        receipt,
    })
}

pub(super) fn run_type(step: &GraphStep) -> Result<String, RuntimeError> {
    let Some(run) = &step.run else {
        return Err(RuntimeError::InvalidRunStep {
            step_id: step.id.clone(),
            reason: "missing run configuration".to_owned(),
        });
    };
    let Some(value) = run.get("type") else {
        return Err(RuntimeError::InvalidRunStep {
            step_id: step.id.clone(),
            reason: "run.type is required".to_owned(),
        });
    };
    string_value(step, "run.type", value)
}

pub(super) fn approval_gate(
    step: &GraphStep,
    inputs: &JsonObject,
) -> Result<ApprovalGate, RuntimeError> {
    let gate_id = required_input_string(step, inputs, "gate_id")?;
    let reason = required_input_string(step, inputs, "reason")?;
    let gate_type = optional_input_string(step, inputs, "gate_type")?;
    let summary = approval_summary(inputs);
    Ok(ApprovalGate {
        id: gate_id,
        reason,
        gate_type,
        summary,
    })
}

pub(super) fn approval_summary(inputs: &JsonObject) -> Option<JsonObject> {
    let mut summary = JsonObject::new();
    for (key, value) in inputs {
        if matches!(key.as_str(), "gate_id" | "reason" | "gate_type") {
            continue;
        }
        summary.insert(key.clone(), value.clone());
    }
    (!summary.is_empty()).then_some(summary)
}

pub(super) fn approval_outputs(
    step: &GraphStep,
    gate: &ApprovalGate,
    resolution: &ApprovalResolution,
) -> Result<JsonObject, RuntimeError> {
    let mut data = JsonObject::new();
    data.insert("approved".to_owned(), approved_value(resolution));
    data.insert("gate_id".to_owned(), JsonValue::String(gate.id.clone()));
    data.insert(
        "idempotency_key".to_owned(),
        JsonValue::String(resolution.idempotency_key().to_owned()),
    );
    data.insert(
        "status".to_owned(),
        JsonValue::String(approval_status(resolution).to_owned()),
    );
    if let Some(actor) = resolution.actor() {
        data.insert("actor".to_owned(), JsonValue::String(actor_name(actor)));
    }

    let mut packet = JsonObject::new();
    if let Some(packet_id) = artifact_packet(step)? {
        packet.insert("packet".to_owned(), JsonValue::String(packet_id));
    }
    packet.insert("data".to_owned(), JsonValue::Object(data));

    let mut outputs = JsonObject::new();
    outputs.insert(
        artifact_wrap_as(step)?.to_owned(),
        JsonValue::Object(packet),
    );
    Ok(outputs)
}

pub(super) fn approved_value(resolution: &ApprovalResolution) -> JsonValue {
    resolution
        .approved()
        .map_or(JsonValue::Null, JsonValue::Bool)
}

pub(super) fn approval_status(resolution: &ApprovalResolution) -> &'static str {
    match resolution {
        ApprovalResolution::Approved { .. } => "approved",
        ApprovalResolution::Denied { .. } => "denied",
        ApprovalResolution::Pending { .. } => "pending",
    }
}

pub(super) fn actor_name(actor: &ResolutionResponseActor) -> String {
    match actor {
        ResolutionResponseActor::Human => "human".to_owned(),
        ResolutionResponseActor::Agent => "agent".to_owned(),
    }
}

pub(super) fn artifact_wrap_as(step: &GraphStep) -> Result<&str, RuntimeError> {
    let Some(artifacts) = &step.artifacts else {
        return Ok("approval");
    };
    let Some(value) = artifacts.get("wrap_as") else {
        return Ok("approval");
    };
    string_value_ref(step, "artifacts.wrap_as", value)
}

pub(super) fn artifact_packet(step: &GraphStep) -> Result<Option<String>, RuntimeError> {
    let Some(artifacts) = &step.artifacts else {
        return Ok(None);
    };
    let Some(value) = artifacts.get("packet") else {
        return Ok(None);
    };
    Ok(Some(string_value(step, "artifacts.packet", value)?))
}

pub(super) fn runtime_error_step_run<A>(
    runtime: &Runtime<A>,
    graph_name: &str,
    step: &GraphStep,
    attempt: u32,
    error: RuntimeError,
) -> Result<StepRun, RuntimeError>
where
    A: SkillAdapter,
{
    let output = SkillOutput {
        status: InvocationStatus::Failure,
        stdout: String::new(),
        stderr: error.to_string(),
        exit_code: None,
        duration_ms: 0,
        metadata: JsonObject::new(),
    };
    let outputs = output_object(&output);
    let receipt = step_receipt(
        graph_name,
        &step.id,
        attempt,
        &output,
        &runtime.options.created_at,
    )?;
    Ok(StepRun {
        step_id: step.id.clone(),
        attempt,
        skill: step.skill.as_deref().unwrap_or(step.id.as_str()).to_owned(),
        runner: step.runner.clone(),
        fanout_group: step.fanout_group.clone(),
        output,
        outputs,
        receipt,
    })
}
