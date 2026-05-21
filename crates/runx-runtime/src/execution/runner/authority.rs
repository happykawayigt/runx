use runx_contracts::{AuthorityTerm, AuthorityVerb, Decision, JsonObject, ProofKind};
use runx_core::policy::{
    PaymentSpendCapabilityBinding, StepAuthorityAdmission, admit_step_authority,
    authority_term_has_verb,
};
use runx_parser::GraphStep;

use super::inputs::{
    optional_bool_field, optional_typed_input, optional_typed_vec_input,
    require_non_empty_string_field, require_object_input, require_reference_input,
    required_typed_input,
};
use crate::RuntimeError;
use crate::adapter::SkillOutput;

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
) -> Result<Option<StepAuthorityContext>, RuntimeError> {
    let Some(input) = step_authority_submission(step, inputs)? else {
        return Ok(None);
    };
    let act_id = format!("act_{}", step.id);
    let decision = admit_step_authority(StepAuthorityAdmission {
        parent_authority: &input.parent_authority,
        child_authority: &input.child_authority,
        reservation_decision: input.reservation_decision.as_ref(),
        subset_proof_present: input.subset_proof_present,
        child_harness_ref: &input.child_harness_ref,
        act_id: &act_id,
        idempotency_key: Some(&input.idempotency_key),
        spend_capability_binding: input.spend_capability_binding,
        consumed_spend_capability_refs: &input.consumed_spend_capability_refs,
        spend_capability_ref: Some(&input.spend_capability_ref),
    })
    .map_err(|source| authority_denied(step, AuthorityVerb::Spend, source.to_string()))?;
    Ok(decision.verb.map(|verb| StepAuthorityContext { verb }))
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
        subset_proof_present: reserved.subset_proof_present,
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
        subset_proof_present: optional_bool_field(step, object, "subset_proof_present")?
            .unwrap_or(false),
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
    verb: AuthorityVerb,
}

#[derive(Clone, Debug)]
struct OwnedStepAuthoritySubmission {
    parent_authority: AuthorityTerm,
    child_authority: AuthorityTerm,
    reservation_decision: Option<Decision>,
    subset_proof_present: bool,
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
    subset_proof_present: bool,
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
