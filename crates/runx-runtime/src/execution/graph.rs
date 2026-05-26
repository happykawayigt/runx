use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use runx_contracts::{JsonObject, JsonValue};
use runx_core::state_machine::{RetryPolicy, SequentialGraphStepDefinition};
use runx_parser::{
    ExecutionGraph, GraphStep, SkillRunnerDefinition, SkillRunnerManifest, SkillSource,
    ValidatedSkill, parse_graph_yaml, parse_runner_manifest_yaml, validate_graph,
    validate_runner_manifest,
};

use crate::adapter::SkillOutput;
use crate::{RuntimeError, StepRun};

pub(crate) struct LoadedStepSkill {
    pub(crate) name: String,
    pub(crate) source: SkillSource,
    pub(crate) directory: PathBuf,
}

pub(crate) fn load_graph(graph_path: &Path) -> Result<ExecutionGraph, RuntimeError> {
    let source = fs::read_to_string(graph_path)
        .map_err(|source| RuntimeError::io("reading graph file", source))?;
    let raw = parse_graph_yaml(&source)?;
    validate_graph(raw).map_err(RuntimeError::from)
}

pub(crate) fn load_skill(skill_dir: &Path) -> Result<ValidatedSkill, RuntimeError> {
    let skill_path = skill_dir.join("SKILL.md");
    if !skill_path.exists() {
        return Err(RuntimeError::SkillFileMissing { path: skill_path });
    }
    let source = fs::read_to_string(&skill_path)
        .map_err(|source| RuntimeError::io("reading skill markdown", source))?;
    let raw = runx_parser::parse_skill_markdown(&source)?;
    runx_parser::validate_skill(raw).map_err(RuntimeError::from)
}

pub(crate) fn load_step_skill(
    graph_dir: &Path,
    step: &GraphStep,
) -> Result<LoadedStepSkill, RuntimeError> {
    let directory = skill_dir(graph_dir, step)?;
    if let Some(runner) = load_step_runner(&directory, step.runner.as_deref())? {
        return Ok(LoadedStepSkill {
            name: runner.name,
            source: runner.source,
            directory,
        });
    }
    let skill = load_skill(&directory)?;
    Ok(LoadedStepSkill {
        name: skill.name,
        source: skill.source,
        directory,
    })
}

fn load_step_runner(
    skill_dir: &Path,
    requested_runner: Option<&str>,
) -> Result<Option<SkillRunnerDefinition>, RuntimeError> {
    let manifest_path = skill_dir.join("X.yaml");
    if !manifest_path.exists() {
        if let Some(runner) = requested_runner {
            return Err(RuntimeError::UnsupportedRunnerSelection {
                runner: runner.to_owned(),
            });
        }
        return Ok(None);
    }
    let source = fs::read_to_string(&manifest_path).map_err(|source| {
        RuntimeError::io(format!("reading {}", manifest_path.display()), source)
    })?;
    let parsed = parse_runner_manifest_yaml(&source).map_err(RuntimeError::from)?;
    let manifest = validate_runner_manifest(parsed).map_err(RuntimeError::from)?;
    select_step_runner(&manifest, requested_runner)
        .cloned()
        .map(Some)
}

fn select_step_runner<'a>(
    manifest: &'a SkillRunnerManifest,
    requested_runner: Option<&str>,
) -> Result<&'a SkillRunnerDefinition, RuntimeError> {
    if let Some(runner) = requested_runner {
        return manifest.runners.get(runner).ok_or_else(|| {
            RuntimeError::UnsupportedRunnerSelection {
                runner: runner.to_owned(),
            }
        });
    }
    let defaults = manifest
        .runners
        .values()
        .filter(|runner| runner.default)
        .collect::<Vec<_>>();
    match defaults.as_slice() {
        [runner] => Ok(*runner),
        [] if manifest.runners.len() == 1 => manifest.runners.values().next().ok_or_else(|| {
            RuntimeError::UnsupportedRunnerSelection {
                runner: "default".to_owned(),
            }
        }),
        [] => Err(RuntimeError::UnsupportedRunnerSelection {
            runner: "default".to_owned(),
        }),
        _ => Err(RuntimeError::UnsupportedRunnerSelection {
            runner: "default".to_owned(),
        }),
    }
}

pub(crate) fn step_definitions(graph: &ExecutionGraph) -> Vec<SequentialGraphStepDefinition> {
    graph
        .steps
        .iter()
        .map(|step| SequentialGraphStepDefinition {
            id: step.id.clone(),
            context_from: context_from(step),
            retry: step.retry.as_ref().map(|retry| RetryPolicy {
                max_attempts: retry_attempts(retry.max_attempts),
            }),
            fanout_group: step.fanout_group.clone(),
        })
        .collect()
}

pub(crate) fn find_step<'a>(
    graph: &'a ExecutionGraph,
    step_id: &str,
) -> Result<&'a GraphStep, RuntimeError> {
    graph
        .steps
        .iter()
        .find(|step| step.id == step_id)
        .ok_or_else(|| RuntimeError::StepMissing {
            step_id: step_id.to_owned(),
        })
}

pub(crate) fn skill_dir(graph_dir: &Path, step: &GraphStep) -> Result<PathBuf, RuntimeError> {
    let Some(skill) = &step.skill else {
        return Err(RuntimeError::StepMissingSkill {
            step_id: step.id.clone(),
        });
    };
    Ok(graph_dir.join(skill))
}

pub(crate) fn resolve_inputs(
    step: &GraphStep,
    prior_runs: &[StepRun],
) -> Result<JsonObject, RuntimeError> {
    let mut inputs = step.inputs.clone();
    if step.context_edges.is_empty() {
        return Ok(inputs);
    }
    if step.context_edges.len() == 1 {
        let edge = &step.context_edges[0];
        let value = context_output(prior_runs, &edge.from_step, &edge.output)?;
        inputs.insert(edge.input.clone(), value);
        return Ok(inputs);
    }
    let prior_run_index = prior_run_index(prior_runs);
    for edge in &step.context_edges {
        let value = context_output_from_index(&prior_run_index, &edge.from_step, &edge.output)?;
        inputs.insert(edge.input.clone(), value);
    }
    Ok(inputs)
}

pub(crate) fn output_object(output: &SkillOutput) -> JsonObject {
    output_object_with_claim(output).0
}

pub(crate) fn output_object_with_claim(output: &SkillOutput) -> (JsonObject, JsonObject) {
    let mut object = JsonObject::new();
    let parsed_stdout = serde_json::from_str::<JsonValue>(&output.stdout).ok();
    if let Some(parsed) = parsed_stdout.as_ref() {
        object.insert("raw".to_owned(), JsonValue::String(output.stdout.clone()));
        object.insert("skill_claim".to_owned(), parsed.clone());
    }
    object.insert(
        "stdout".to_owned(),
        JsonValue::String(output.stdout.clone()),
    );
    object.insert(
        "stderr".to_owned(),
        JsonValue::String(output.stderr.clone()),
    );
    object.insert(
        "status".to_owned(),
        JsonValue::String(if output.succeeded() {
            "success".to_owned()
        } else {
            "failure".to_owned()
        }),
    );
    let claim = match parsed_stdout {
        Some(JsonValue::Object(object)) => object,
        _ => JsonObject::new(),
    };
    (object, claim)
}

fn context_from(step: &GraphStep) -> Option<Vec<String>> {
    let refs = step
        .context_edges
        .iter()
        .map(|edge| edge.from_step.clone())
        .collect::<Vec<_>>();
    (!refs.is_empty()).then_some(refs)
}

fn retry_attempts(max_attempts: u64) -> u32 {
    u32::try_from(max_attempts).unwrap_or(u32::MAX)
}

fn context_output(
    prior_runs: &[StepRun],
    from_step: &str,
    output: &str,
) -> Result<JsonValue, RuntimeError> {
    let Some(run) = prior_runs.iter().find(|run| run.step_id == from_step) else {
        return Err(RuntimeError::GraphBlocked {
            step_id: from_step.to_owned(),
            reason: "context source step has not run".to_owned(),
        });
    };
    Ok(resolve_output_path(&run.outputs, output).unwrap_or(JsonValue::Null))
}

fn prior_run_index<'a>(prior_runs: &'a [StepRun]) -> BTreeMap<&'a str, &'a StepRun> {
    let mut index = BTreeMap::new();
    for run in prior_runs {
        index.entry(run.step_id.as_str()).or_insert(run);
    }
    index
}

fn context_output_from_index(
    prior_runs: &BTreeMap<&str, &StepRun>,
    from_step: &str,
    output: &str,
) -> Result<JsonValue, RuntimeError> {
    let Some(run) = prior_runs.get(from_step) else {
        return Err(RuntimeError::GraphBlocked {
            step_id: from_step.to_owned(),
            reason: "context source step has not run".to_owned(),
        });
    };
    Ok(resolve_output_path(&run.outputs, output).unwrap_or(JsonValue::Null))
}

fn resolve_output_path(outputs: &JsonObject, output: &str) -> Option<JsonValue> {
    let mut segments = output.split('.');
    let first = segments.next()?;
    let mut value = outputs.get(first)?;
    for segment in segments {
        let JsonValue::Object(object) = value else {
            return None;
        };
        value = object.get(segment)?;
    }
    Some(value.clone())
}
