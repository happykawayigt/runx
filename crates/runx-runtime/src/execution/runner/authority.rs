use std::collections::BTreeMap;
use std::path::Path;

use runx_contracts::{
    AuthoritySubsetProof, AuthorityTerm, AuthorityVerb, Decision, JsonObject, ProofKind,
};
use runx_core::policy::{
    PaymentSpendCapabilityBinding, StepAuthorityAdmission, admit_step_authority,
    authority_term_has_verb,
};
use runx_core::state_machine::AuthorityAdmissionWitness;
use runx_parser::GraphStep;

use super::inputs::{
    optional_typed_input, optional_typed_vec_input, require_non_empty_string_field,
    require_object_input, require_reference_input, required_typed_input,
};
use crate::RuntimeError;
use crate::adapter::SkillOutput;
use crate::payment_state::{
    PaymentIdempotencyKey, PaymentRecoveryState, RailMutationStatus,
    consumed_spend_capability_recorded, escalate_payment_rail_mutation,
    lookup_payment_idempotency_entry, lookup_payment_rail_mutation,
};

pub(super) fn enforce_step_authority_receipt_before_success(
    step: &GraphStep,
    authority: Option<&StepAuthorityContext>,
    output: &SkillOutput,
    receipt: &runx_contracts::HarnessReceipt,
) -> Result<(), RuntimeError> {
    let Some(authority) = authority else {
        return Ok(());
    };
    if !output.succeeded() || authority.verb != AuthorityVerb::Spend {
        return Ok(());
    }
    let proof_present = receipt
        .harness
        .acts
        .iter()
        .any(|act| act.verification_refs.iter().any(is_payment_rail_proof_ref));
    if proof_present {
        return Ok(());
    }
    Err(authority_denied(
        step,
        AuthorityVerb::Spend,
        "spend success requires a sealed rail proof reference".to_owned(),
    ))
}

fn is_payment_rail_proof_ref(reference: &runx_contracts::Reference) -> bool {
    reference.reference_type == runx_contracts::ReferenceType::Verification
        && reference.proof_kind.as_ref() == Some(&ProofKind::PaymentRail)
}

pub(super) fn enforce_step_authority_admission(
    step: &GraphStep,
    inputs: &JsonObject,
    env: &BTreeMap<String, String>,
    graph_dir: &Path,
) -> Result<Option<StepAuthorityContext>, RuntimeError> {
    let Some(input) = step_authority_submission(step, inputs)? else {
        return Ok(None);
    };
    let consumed_spend_capability_refs =
        consumed_spend_capability_refs_for_admission(&input, env, graph_dir)?;
    let act_id = format!("act_{}", step.id);
    let decision = admit_step_authority(StepAuthorityAdmission {
        parent_authority: &input.parent_authority,
        child_authority: &input.child_authority,
        reservation_decision: input.reservation_decision.as_ref(),
        subset_proof: input.subset_proof.as_ref(),
        child_harness_ref: &input.child_harness_ref,
        act_id: &act_id,
        idempotency_key: Some(&input.idempotency_key),
        spend_capability_binding: input.spend_capability_binding.clone(),
        consumed_spend_capability_refs: &consumed_spend_capability_refs,
        spend_capability_ref: Some(&input.spend_capability_ref),
    })
    .map_err(|source| authority_denied(step, AuthorityVerb::Spend, source.to_string()))?;
    let payment = payment_context(&input);
    Ok(decision.verb.map(|verb| {
        let admission_witness = AuthorityAdmissionWitness {
            verb: verb.clone(),
            parent_term_id: decision.parent_term_id.to_owned(),
            child_term_id: decision.child_term_id.to_owned(),
            idempotency_key: decision.idempotency_key.map(str::to_owned),
            spend_capability_ref: decision.spend_capability_ref.cloned(),
        };
        let payment = if verb == AuthorityVerb::Spend {
            payment
        } else {
            None
        };
        StepAuthorityContext {
            verb,
            payment,
            admission_witness,
        }
    }))
}

pub(super) fn sealed_payment_replay(
    step: &GraphStep,
    inputs: &JsonObject,
    env: &BTreeMap<String, String>,
    graph_dir: &Path,
) -> Result<Option<StepPaymentReplay>, RuntimeError> {
    let Some(input) = step_authority_submission(step, inputs)? else {
        return Ok(None);
    };
    let Some(payment) = payment_context(&input) else {
        return Ok(None);
    };
    let Some(entry) = lookup_payment_idempotency_entry(env, graph_dir, &payment.idempotency_key)
        .map_err(|source| {
            RuntimeError::payment_state("reading payment state for replay lookup", source)
        })?
    else {
        return Ok(None);
    };

    let act_id = format!("act_{}", step.id);
    let decision = admit_step_authority(StepAuthorityAdmission {
        parent_authority: &input.parent_authority,
        child_authority: &input.child_authority,
        reservation_decision: input.reservation_decision.as_ref(),
        subset_proof: input.subset_proof.as_ref(),
        child_harness_ref: &input.child_harness_ref,
        act_id: &act_id,
        idempotency_key: Some(&input.idempotency_key),
        spend_capability_binding: input.spend_capability_binding.clone(),
        consumed_spend_capability_refs: &input.consumed_spend_capability_refs,
        spend_capability_ref: Some(&input.spend_capability_ref),
    })
    .map_err(|source| authority_denied(step, AuthorityVerb::Spend, source.to_string()))?;
    if decision.verb != Some(AuthorityVerb::Spend) {
        return Ok(None);
    }
    if entry.amount_minor != payment.amount_minor || entry.currency != payment.currency {
        return Err(authority_denied(
            step,
            AuthorityVerb::Spend,
            format!(
                "payment idempotency key {} was sealed for {} {}, but this spend requested {} {}",
                payment.idempotency_key.key,
                entry.amount_minor,
                entry.currency,
                payment.amount_minor,
                payment.currency
            ),
        ));
    }

    Ok(Some(StepPaymentReplay {
        receipt_ref: entry.receipt_ref,
        receipt_created_at: entry.receipt_created_at,
        receipt_digest: entry.receipt_digest,
        rail_proof_ref: entry.rail_proof_ref,
        outputs: entry.outputs,
    }))
}

pub(super) fn escalate_in_flight_payment_recovery(
    step: &GraphStep,
    inputs: &JsonObject,
    env: &BTreeMap<String, String>,
    graph_dir: &Path,
) -> Result<(), RuntimeError> {
    let Some(input) = step_authority_submission(step, inputs)? else {
        return Ok(());
    };
    let Some(payment) = payment_context(&input) else {
        return Ok(());
    };
    let Some(mutation) = lookup_payment_rail_mutation(env, graph_dir, &payment.idempotency_key)
        .map_err(|source| {
            RuntimeError::payment_state("reading payment state for rail recovery", source)
        })?
    else {
        return Ok(());
    };
    if mutation.recovery_state != PaymentRecoveryState::InFlight
        && mutation.status != RailMutationStatus::Partial
    {
        return Ok(());
    }

    let act_id = format!("act_{}", step.id);
    admit_step_authority(StepAuthorityAdmission {
        parent_authority: &input.parent_authority,
        child_authority: &input.child_authority,
        reservation_decision: input.reservation_decision.as_ref(),
        subset_proof: input.subset_proof.as_ref(),
        child_harness_ref: &input.child_harness_ref,
        act_id: &act_id,
        idempotency_key: Some(&input.idempotency_key),
        spend_capability_binding: input.spend_capability_binding.clone(),
        consumed_spend_capability_refs: &input.consumed_spend_capability_refs,
        spend_capability_ref: Some(&input.spend_capability_ref),
    })
    .map_err(|source| authority_denied(step, AuthorityVerb::Spend, source.to_string()))?;
    if mutation.amount_minor != payment.amount_minor
        || mutation.currency != payment.currency
        || mutation.rail != payment.rail
        || mutation.counterparty != payment.counterparty
    {
        return Err(authority_denied(
            step,
            AuthorityVerb::Spend,
            format!(
                "payment idempotency key {} has in-flight rail mutation for {} {} on {} {}, but this spend requested {} {} on {} {}",
                payment.idempotency_key.key,
                mutation.amount_minor,
                mutation.currency,
                mutation.rail,
                mutation.counterparty,
                payment.amount_minor,
                payment.currency,
                payment.rail,
                payment.counterparty
            ),
        ));
    }

    let _ = escalate_payment_rail_mutation(env, graph_dir, &payment.idempotency_key).map_err(
        |source| RuntimeError::payment_state("escalating payment rail recovery", source),
    )?;
    Err(authority_denied(
        step,
        AuthorityVerb::Spend,
        format!(
            "payment idempotency key {} has an in-flight rail mutation; recovery escalated without issuing a second rail mutation",
            payment.idempotency_key.key
        ),
    ))
}

fn consumed_spend_capability_refs_for_admission(
    input: &OwnedStepAuthoritySubmission,
    env: &BTreeMap<String, String>,
    graph_dir: &Path,
) -> Result<Vec<runx_contracts::Reference>, RuntimeError> {
    let mut refs = input.consumed_spend_capability_refs.clone();
    if consumed_spend_capability_recorded(env, graph_dir, &input.spend_capability_ref.uri).map_err(
        |source| RuntimeError::payment_state("reading payment state for admission", source),
    )? && !refs
        .iter()
        .any(|reference| same_reference(reference, &input.spend_capability_ref))
    {
        refs.push(input.spend_capability_ref.clone());
    }
    Ok(refs)
}

fn payment_context(input: &OwnedStepAuthoritySubmission) -> Option<StepPaymentAuthorityContext> {
    let binding = input.spend_capability_binding.as_ref()?;
    Some(StepPaymentAuthorityContext {
        idempotency_key: PaymentIdempotencyKey::new(
            binding.rail.clone(),
            binding.counterparty.clone(),
            input.idempotency_key.clone(),
        ),
        spend_capability_ref: input.spend_capability_ref.clone(),
        rail: binding.rail.clone(),
        counterparty: binding.counterparty.clone(),
        amount_minor: binding.amount_minor,
        currency: binding.currency.clone(),
    })
}

fn same_reference(left: &runx_contracts::Reference, right: &runx_contracts::Reference) -> bool {
    left.reference_type == right.reference_type && left.uri == right.uri
}

fn step_authority_submission(
    step: &GraphStep,
    inputs: &JsonObject,
) -> Result<Option<OwnedStepAuthoritySubmission>, RuntimeError> {
    let Some(reserved) = optional_payment_authority_object(step, inputs)? else {
        return Ok(None);
    };
    let idempotency = require_object_input(step, inputs, "idempotency")?;
    let reserved = parse_reserved_payment_authority(step, reserved)?;
    if !authority_term_has_verb(&reserved.child_authority, AuthorityVerb::Spend) {
        return Ok(None);
    }
    Ok(Some(OwnedStepAuthoritySubmission {
        spend_capability_ref: require_reference_input(step, inputs, "spend_capability_ref")?,
        idempotency_key: require_non_empty_string_field(step, idempotency, "idempotency.key")?,
        parent_authority: reserved.parent_authority,
        child_authority: reserved.child_authority,
        reservation_decision: reserved.reservation_decision,
        subset_proof: reserved.subset_proof,
        child_harness_ref: reserved.child_harness_ref,
        spend_capability_binding: reserved.spend_capability_binding,
        consumed_spend_capability_refs: reserved.consumed_spend_capability_refs,
    }))
}

fn optional_payment_authority_object<'a>(
    step: &GraphStep,
    inputs: &'a JsonObject,
) -> Result<Option<&'a JsonObject>, RuntimeError> {
    if inputs.contains_key("reserved_payment_authority") {
        return require_object_input(step, inputs, "reserved_payment_authority").map(Some);
    }
    if payment_admission_field_present(inputs) {
        return Err(authority_denied(
            step,
            AuthorityVerb::Spend,
            "reserved_payment_authority is required before payment rail execution".to_owned(),
        ));
    }
    Ok(None)
}

fn payment_admission_field_present(inputs: &JsonObject) -> bool {
    inputs.contains_key("spend_capability_ref") || inputs.contains_key("payment_challenge")
}

fn parse_reserved_payment_authority(
    step: &GraphStep,
    object: &JsonObject,
) -> Result<ReservedAuthorityInput, RuntimeError> {
    Ok(ReservedAuthorityInput {
        parent_authority: required_typed_input(
            step,
            object,
            "reserved_payment_authority.parent_authority",
            "parent_authority",
        )?,
        child_authority: required_typed_input(
            step,
            object,
            "reserved_payment_authority.child_authority",
            "child_authority",
        )?,
        reservation_decision: optional_typed_input(
            step,
            object,
            "reserved_payment_authority.reservation_decision",
            "reservation_decision",
        )?,
        subset_proof: optional_typed_input(
            step,
            object,
            "reserved_payment_authority.subset_proof",
            "subset_proof",
        )?,
        child_harness_ref: required_typed_input(
            step,
            object,
            "reserved_payment_authority.child_harness_ref",
            "child_harness_ref",
        )?,
        spend_capability_binding: optional_typed_input(
            step,
            object,
            "reserved_payment_authority.spend_capability_binding",
            "spend_capability_binding",
        )?,
        consumed_spend_capability_refs: optional_typed_vec_input(
            step,
            object,
            "reserved_payment_authority.consumed_spend_capability_refs",
            "consumed_spend_capability_refs",
        )?
        .unwrap_or_default(),
    })
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

#[derive(Clone, Debug)]
pub(super) struct StepAuthorityContext {
    pub(super) verb: AuthorityVerb,
    pub(super) payment: Option<StepPaymentAuthorityContext>,
    pub(super) admission_witness: AuthorityAdmissionWitness,
}

#[derive(Clone, Debug)]
pub(super) struct StepPaymentAuthorityContext {
    pub(super) idempotency_key: PaymentIdempotencyKey,
    pub(super) spend_capability_ref: runx_contracts::Reference,
    pub(super) rail: String,
    pub(super) counterparty: String,
    pub(super) amount_minor: u64,
    pub(super) currency: String,
}

#[derive(Clone, Debug)]
pub(super) struct StepPaymentReplay {
    pub(super) receipt_ref: String,
    pub(super) receipt_created_at: String,
    pub(super) receipt_digest: String,
    pub(super) rail_proof_ref: String,
    pub(super) outputs: JsonObject,
}

#[derive(Clone, Debug)]
struct OwnedStepAuthoritySubmission {
    parent_authority: AuthorityTerm,
    child_authority: AuthorityTerm,
    reservation_decision: Option<Decision>,
    subset_proof: Option<AuthoritySubsetProof>,
    child_harness_ref: runx_contracts::Reference,
    spend_capability_binding: Option<PaymentSpendCapabilityBinding>,
    consumed_spend_capability_refs: Vec<runx_contracts::Reference>,
    spend_capability_ref: runx_contracts::Reference,
    idempotency_key: String,
}

#[derive(Clone, Debug)]
struct ReservedAuthorityInput {
    parent_authority: AuthorityTerm,
    child_authority: AuthorityTerm,
    reservation_decision: Option<Decision>,
    subset_proof: Option<AuthoritySubsetProof>,
    child_harness_ref: runx_contracts::Reference,
    spend_capability_binding: Option<PaymentSpendCapabilityBinding>,
    consumed_spend_capability_refs: Vec<runx_contracts::Reference>,
}

#[cfg(test)]
mod tests {
    use runx_contracts::{ProofKind, Reference, ReferenceType};

    use super::is_payment_rail_proof_ref;

    #[test]
    fn payment_rail_proof_matching_uses_typed_kind_not_label() {
        let typed_ref = Reference {
            reference_type: ReferenceType::Verification,
            uri: "receipt-proof:mock:typed".to_owned(),
            provider: None,
            locator: None,
            label: Some("human display text".to_owned()),
            observed_at: None,
            proof_kind: Some(ProofKind::PaymentRail),
        };
        let label_only_ref = Reference {
            reference_type: ReferenceType::Verification,
            uri: "receipt-proof:mock:label-only".to_owned(),
            provider: None,
            locator: None,
            label: Some("payment rail proof".to_owned()),
            observed_at: None,
            proof_kind: None,
        };

        assert!(is_payment_rail_proof_ref(&typed_ref));
        assert!(!is_payment_rail_proof_ref(&label_only_ref));
    }
}
