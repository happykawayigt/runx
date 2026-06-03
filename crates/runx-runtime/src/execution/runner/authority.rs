use std::collections::BTreeMap;
use std::path::Path;

use runx_contracts::{AuthorityVerb, JsonObject, Receipt};
use runx_parser::GraphStep;

use crate::RuntimeError;
use crate::adapter::SkillOutput;
use crate::effects::{
    EffectAdmission, EffectOutputRequest, EffectReceiptRequest, EffectReplay,
    EffectReplayOutputRequest, EffectReplayReceiptRequest, EffectStepRequest, RuntimeEffectError,
    RuntimeEffectRegistry,
};

pub(super) fn find_effect_replay(
    step: &GraphStep,
    inputs: &JsonObject,
    env: &BTreeMap<String, String>,
    graph_dir: &Path,
    effects: &RuntimeEffectRegistry,
) -> Result<Option<EffectReplay>, RuntimeError> {
    effects
        .find_replay(EffectStepRequest {
            step,
            inputs,
            env,
            graph_dir,
        })
        .map_err(|source| runtime_effect_error(step, source))
}

pub(super) fn recover_pending_effects(
    step: &GraphStep,
    inputs: &JsonObject,
    env: &BTreeMap<String, String>,
    graph_dir: &Path,
    effects: &RuntimeEffectRegistry,
) -> Result<(), RuntimeError> {
    effects
        .recover_pending(EffectStepRequest {
            step,
            inputs,
            env,
            graph_dir,
        })
        .map_err(|source| runtime_effect_error(step, source))
}

pub(super) fn enforce_step_authority_admission(
    step: &GraphStep,
    inputs: &JsonObject,
    env: &BTreeMap<String, String>,
    graph_dir: &Path,
    effects: &RuntimeEffectRegistry,
) -> Result<Option<StepAuthorityContext>, RuntimeError> {
    effects
        .admit(EffectStepRequest {
            step,
            inputs,
            env,
            graph_dir,
        })
        .map(|admission| admission.map(StepAuthorityContext::new))
        .map_err(|source| runtime_effect_error(step, source))
}

pub(super) fn prepare_effect_output_before_gate(
    step: &GraphStep,
    authority: Option<&StepAuthorityContext>,
    claim: &JsonObject,
    output: &mut SkillOutput,
    effects: &RuntimeEffectRegistry,
) -> Result<(), RuntimeError> {
    let Some(authority) = authority else {
        return Ok(());
    };
    effects
        .prepare_output(EffectOutputRequest {
            step,
            admission: &authority.admission,
            claim,
            output,
        })
        .map_err(|source| runtime_effect_error(step, source))
}

pub(super) fn finalize_effect_output_before_success(
    context: EffectReceiptContext<'_>,
) -> Result<(), RuntimeError> {
    let Some(authority) = context.authority else {
        return Ok(());
    };
    context
        .effects
        .finalize_output(EffectReceiptRequest {
            step: context.step,
            graph_dir: context.graph_dir,
            admission: &authority.admission,
            claim: context.claim,
            output: context.output,
            receipt: context.receipt,
            env: context.env,
        })
        .map_err(|source| runtime_effect_error(context.step, source))
}

pub(super) fn persist_effect_state_for_step(
    context: EffectReceiptContext<'_>,
) -> Result<(), RuntimeError> {
    let Some(authority) = context.authority else {
        return Ok(());
    };
    context
        .effects
        .persist(EffectReceiptRequest {
            step: context.step,
            graph_dir: context.graph_dir,
            admission: &authority.admission,
            claim: context.claim,
            output: context.output,
            receipt: context.receipt,
            env: context.env,
        })
        .map_err(|source| runtime_effect_error(context.step, source))
}

pub(super) fn prepare_replay_output(
    step: &GraphStep,
    replay: &EffectReplay,
    output: &mut SkillOutput,
    effects: &RuntimeEffectRegistry,
) -> Result<(), RuntimeError> {
    effects
        .prepare_replay_output(EffectReplayOutputRequest {
            step,
            replay,
            output,
        })
        .map_err(|source| runtime_effect_error(step, source))
}

pub(super) fn validate_replayed_effect(
    step: &GraphStep,
    replay: &EffectReplay,
    receipt: &runx_contracts::Receipt,
    output: &SkillOutput,
    claim: &JsonObject,
    effects: &RuntimeEffectRegistry,
) -> Result<(), RuntimeError> {
    effects
        .validate_replay(EffectReplayReceiptRequest {
            step,
            replay,
            receipt,
            output,
            claim,
        })
        .map_err(|source| runtime_effect_error(step, source))
}

pub(super) fn authority_denied(
    step: &GraphStep,
    verb: AuthorityVerb,
    reason: String,
) -> RuntimeError {
    RuntimeError::AuthorityDenied {
        verb,
        step_id: step.id.clone(),
        reason,
    }
}

pub(super) struct EffectReceiptContext<'a> {
    pub(super) step: &'a GraphStep,
    pub(super) graph_dir: &'a Path,
    pub(super) authority: Option<&'a StepAuthorityContext>,
    pub(super) claim: &'a JsonObject,
    pub(super) output: &'a mut SkillOutput,
    pub(super) receipt: &'a Receipt,
    pub(super) env: &'a BTreeMap<String, String>,
    pub(super) effects: &'a RuntimeEffectRegistry,
}

fn runtime_effect_error(step: &GraphStep, source: RuntimeEffectError) -> RuntimeError {
    match source {
        RuntimeEffectError::Denied { verb, message, .. } => authority_denied(step, verb, message),
        RuntimeEffectError::Failed {
            operation, message, ..
        } if operation.contains("state") => RuntimeError::effect_state(operation, message),
        other => RuntimeError::ReceiptInvalid {
            message: other.to_string(),
        },
    }
}

#[derive(Clone, Debug)]
pub(super) struct StepAuthorityContext {
    admission: EffectAdmission,
}

impl StepAuthorityContext {
    fn new(admission: EffectAdmission) -> Self {
        Self { admission }
    }

    pub(super) fn admission_witness(&self) -> &runx_core::state_machine::AuthorityAdmissionWitness {
        self.admission.witness()
    }
}
