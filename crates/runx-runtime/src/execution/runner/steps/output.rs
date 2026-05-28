//! Step output projection helpers. Translate the skill's stdout claim and
//! declared run-outputs / artifact-emits into the typed step projection that
//! downstream graph state machines and receipt sealers consume.

use runx_contracts::{JsonObject, JsonValue};
use runx_parser::GraphStep;

use crate::RuntimeError;
use crate::adapter::SkillOutput;
use crate::execution::output_projection::{StepOutputProjection, project_step_output};

pub(super) fn step_output_projection(
    step: &GraphStep,
    output: &SkillOutput,
) -> Result<StepOutputProjection, RuntimeError> {
    build_step_output_projection(step, output, ClaimContextExposure::DeclaredAndContext)
}

pub(super) fn build_step_output_projection(
    step: &GraphStep,
    output: &SkillOutput,
    exposure: ClaimContextExposure,
) -> Result<StepOutputProjection, RuntimeError> {
    let mut projection = project_step_output(output);
    expose_declared_run_outputs(step, &projection.claim, &mut projection.outputs)?;
    expose_declared_artifacts(step, &projection.claim, &mut projection.outputs)?;
    if matches!(exposure, ClaimContextExposure::DeclaredAndContext) {
        expose_skill_claim_context_fields(&projection.claim, &mut projection.outputs);
    }
    Ok(projection)
}

pub(super) enum ClaimContextExposure {
    DeclaredOnly,
    DeclaredAndContext,
}

fn expose_declared_run_outputs(
    step: &GraphStep,
    claim: &JsonObject,
    outputs: &mut JsonObject,
) -> Result<(), RuntimeError> {
    let Some(run) = &step.run else {
        return Ok(());
    };
    let Some(JsonValue::Object(declared_outputs)) = run.get("outputs") else {
        return Ok(());
    };
    if claim.is_empty() {
        return Ok(());
    }

    for name in declared_outputs.keys() {
        reject_reserved_step_output_name(step, name, "declared run output")?;
        let Some(value) = declared_claim_value(claim, name) else {
            continue;
        };
        outputs.insert(name.clone(), value);
    }
    Ok(())
}

fn expose_declared_artifacts(
    step: &GraphStep,
    claim: &JsonObject,
    outputs: &mut JsonObject,
) -> Result<(), RuntimeError> {
    let Some(artifacts) = &step.artifacts else {
        return Ok(());
    };
    if claim.is_empty() {
        return Ok(());
    }

    if let Some(wrap_as) = artifacts.get("wrap_as").and_then(JsonValue::as_str) {
        reject_reserved_step_output_name(step, wrap_as, "artifact output")?;
        let value = declared_claim_value(claim, wrap_as).unwrap_or_else(|| {
            let mut wrapper = JsonObject::new();
            wrapper.insert("data".to_owned(), JsonValue::Object(claim.clone()));
            JsonValue::Object(wrapper)
        });
        outputs.insert(wrap_as.to_owned(), value);
    }

    if let Some(JsonValue::Object(named_emits)) = artifacts.get("named_emits") {
        for name in named_emits.keys() {
            reject_reserved_step_output_name(step, name, "artifact output")?;
            let Some(value) = declared_claim_value(claim, name) else {
                continue;
            };
            outputs.insert(name.clone(), artifact_data_wrapper(value));
        }
    }
    Ok(())
}

fn declared_claim_value(claim: &JsonObject, name: &str) -> Option<JsonValue> {
    claim.get(name).cloned().or_else(|| {
        ["output", "outputs", "payload"]
            .iter()
            .find_map(|envelope| {
                let JsonValue::Object(object) = claim.get(*envelope)? else {
                    return None;
                };
                object.get(name).cloned()
            })
    })
}

fn expose_skill_claim_context_fields(claim: &JsonObject, outputs: &mut JsonObject) {
    const RESERVED_OUTPUT_FIELDS: &[&str] = &["raw", "skill_claim", "stdout", "stderr", "status"];
    for (name, value) in claim {
        if RESERVED_OUTPUT_FIELDS.contains(&name.as_str()) || outputs.contains_key(name) {
            continue;
        }
        outputs.insert(name.clone(), value.clone());
    }
}

fn reject_reserved_step_output_name(
    step: &GraphStep,
    name: &str,
    output_kind: &str,
) -> Result<(), RuntimeError> {
    const RESERVED_OUTPUT_FIELDS: &[&str] = &["raw", "skill_claim", "stdout", "stderr", "status"];
    if RESERVED_OUTPUT_FIELDS.contains(&name) {
        return Err(RuntimeError::InvalidRunStep {
            step_id: step.id.clone(),
            reason: format!("{output_kind} name {name:?} is reserved"),
        });
    }
    Ok(())
}

fn artifact_data_wrapper(value: JsonValue) -> JsonValue {
    match value {
        JsonValue::Object(object) if object.contains_key("data") => JsonValue::Object(object),
        other => {
            let mut wrapper = JsonObject::new();
            wrapper.insert("data".to_owned(), other);
            JsonValue::Object(wrapper)
        }
    }
}
