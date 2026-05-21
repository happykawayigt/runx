// rust-style-allow: large-file - payment authority comparison is one algebraic boundary; splitting it before the core term model settles would hide subset invariants.
use runx_contracts::{
    AuthorityCapability, AuthorityCondition, AuthorityResourceFamily, AuthorityTerm, AuthorityVerb,
    Decision, DecisionChoice, PaymentAuthorityBounds, PaymentCredentialForm, Reference,
};
use thiserror::Error;

#[derive(Clone, Debug, PartialEq)]
pub struct StepAuthorityAdmission<'a> {
    pub parent_authority: &'a AuthorityTerm,
    pub child_authority: &'a AuthorityTerm,
    pub reservation_decision: Option<&'a Decision>,
    pub subset_proof_present: bool,
    pub child_harness_ref: &'a Reference,
    pub act_id: &'a str,
    pub idempotency_key: Option<&'a str>,
    pub spend_capability_binding: Option<PaymentSpendCapabilityBinding>,
    pub consumed_spend_capability_refs: &'a [Reference],
    pub spend_capability_ref: Option<&'a Reference>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StepAuthorityAdmissionDecision<'a> {
    pub verb: Option<AuthorityVerb>,
    pub parent_term_id: &'a str,
    pub child_term_id: &'a str,
    pub idempotency_key: Option<&'a str>,
    pub spend_capability_ref: Option<&'a Reference>,
}

#[cfg(test)]
#[derive(Clone, Debug, PartialEq)]
struct PaymentRailAuthorization<'a> {
    pub parent_authority: &'a AuthorityTerm,
    pub child_authority: &'a AuthorityTerm,
    pub reservation_decision: Option<&'a Decision>,
    pub subset_proof_present: bool,
    pub child_harness_ref: &'a Reference,
    pub act_id: &'a str,
    pub idempotency_key: Option<&'a str>,
    pub spend_capability_binding: Option<PaymentSpendCapabilityBinding>,
    pub rail_proof_refs: &'a [Reference],
    pub consumed_spend_capability_refs: &'a [Reference],
    pub spend_capability_ref: Option<&'a Reference>,
}

#[derive(Clone, Debug, PartialEq)]
struct PaymentRailAdmission<'a> {
    pub parent_authority: &'a AuthorityTerm,
    pub child_authority: &'a AuthorityTerm,
    pub reservation_decision: Option<&'a Decision>,
    pub subset_proof_present: bool,
    pub child_harness_ref: &'a Reference,
    pub act_id: &'a str,
    pub idempotency_key: Option<&'a str>,
    pub spend_capability_binding: Option<PaymentSpendCapabilityBinding>,
    pub consumed_spend_capability_refs: &'a [Reference],
    pub spend_capability_ref: Option<&'a Reference>,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaymentSpendCapabilityBinding {
    pub child_harness_ref: Reference,
    pub act_id: String,
    pub reservation_decision_id: String,
    pub idempotency_key: String,
    pub amount_minor: u64,
    pub currency: String,
    pub counterparty: String,
    pub rail: String,
}

#[cfg(test)]
#[derive(Clone, Debug, PartialEq, Eq)]
struct PaymentRailAuthorizationDecision<'a> {
    pub parent_term_id: &'a str,
    pub child_term_id: &'a str,
    pub idempotency_key: Option<&'a str>,
    pub spend_capability_ref: Option<&'a Reference>,
    pub rail_proof_refs: &'a [Reference],
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PaymentRailAdmissionDecision<'a> {
    pub parent_term_id: &'a str,
    pub child_term_id: &'a str,
    pub idempotency_key: Option<&'a str>,
    pub spend_capability_ref: Option<&'a Reference>,
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum PaymentAuthorityError {
    #[error("payment spend requires a reservation decision")]
    MissingReservationDecision,
    #[error("payment reservation decision did not select an act or harness")]
    ReservationDecisionNotSelected,
    #[error("payment authority attenuation requires a subset proof")]
    MissingSubsetProof,
    #[error("child payment authority is not a subset of parent authority")]
    AuthorityNotSubset,
    #[error("payment spend requires a single-use spend capability")]
    SpendRequiresSingleUseCapability,
    #[error("payment spend capability binding does not match the child harness act")]
    SpendCapabilityBindingMismatch,
    #[error("single-use payment spend capability was already consumed")]
    SpendCapabilityAlreadyConsumed,
    #[error("payment spend requires a deterministic idempotency key")]
    MissingIdempotencyKey,
    #[error("payment spend requires a bounded non-wildcard counterparty")]
    WildcardCounterpartyDenied,
    #[cfg(test)]
    #[error("payment authority requires a rail receipt before success")]
    MissingReceiptBeforeSuccess,
    #[cfg(test)]
    #[error("payment authority requires rail proof")]
    MissingRailProof,
}

#[must_use]
pub fn authority_term_has_verb(term: &AuthorityTerm, verb: AuthorityVerb) -> bool {
    term.verbs.iter().any(|candidate| candidate == &verb)
}

#[cfg(test)]
#[must_use]
fn payment_authority_requires_receipt_before_success(term: &AuthorityTerm) -> bool {
    term.bounds
        .payment
        .as_ref()
        .is_some_and(|payment| payment.receipt_before_success)
}

#[must_use]
fn payment_authority_spends(term: &AuthorityTerm) -> bool {
    authority_term_has_verb(term, AuthorityVerb::Spend)
}

pub fn admit_step_authority(
    input: StepAuthorityAdmission<'_>,
) -> Result<StepAuthorityAdmissionDecision<'_>, PaymentAuthorityError> {
    if input.child_authority.resource_family == AuthorityResourceFamily::Payment
        && payment_authority_spends(input.child_authority)
    {
        let admission = admit_payment_rail(PaymentRailAdmission {
            parent_authority: input.parent_authority,
            child_authority: input.child_authority,
            reservation_decision: input.reservation_decision,
            subset_proof_present: input.subset_proof_present,
            child_harness_ref: input.child_harness_ref,
            act_id: input.act_id,
            idempotency_key: input.idempotency_key,
            spend_capability_binding: input.spend_capability_binding,
            consumed_spend_capability_refs: input.consumed_spend_capability_refs,
            spend_capability_ref: input.spend_capability_ref,
        })?;
        return Ok(StepAuthorityAdmissionDecision {
            verb: Some(AuthorityVerb::Spend),
            parent_term_id: admission.parent_term_id,
            child_term_id: admission.child_term_id,
            idempotency_key: admission.idempotency_key,
            spend_capability_ref: admission.spend_capability_ref,
        });
    }

    Ok(StepAuthorityAdmissionDecision {
        verb: None,
        parent_term_id: &input.parent_authority.term_id,
        child_term_id: &input.child_authority.term_id,
        idempotency_key: input.idempotency_key,
        spend_capability_ref: input.spend_capability_ref,
    })
}

#[cfg(test)]
fn authorize_payment_rail(
    input: PaymentRailAuthorization<'_>,
) -> Result<PaymentRailAuthorizationDecision<'_>, PaymentAuthorityError> {
    let spends = payment_authority_spends(input.child_authority);
    let admission = admit_payment_rail(PaymentRailAdmission {
        parent_authority: input.parent_authority,
        child_authority: input.child_authority,
        reservation_decision: input.reservation_decision,
        subset_proof_present: input.subset_proof_present,
        child_harness_ref: input.child_harness_ref,
        act_id: input.act_id,
        idempotency_key: input.idempotency_key,
        spend_capability_binding: input.spend_capability_binding,
        consumed_spend_capability_refs: input.consumed_spend_capability_refs,
        spend_capability_ref: input.spend_capability_ref,
    })?;

    if requires_receipt_before_success(input.parent_authority, input.child_authority)
        && input.rail_proof_refs.is_empty()
    {
        return Err(PaymentAuthorityError::MissingReceiptBeforeSuccess);
    }

    if spends && input.rail_proof_refs.is_empty() {
        return Err(PaymentAuthorityError::MissingRailProof);
    }

    Ok(PaymentRailAuthorizationDecision {
        parent_term_id: admission.parent_term_id,
        child_term_id: admission.child_term_id,
        idempotency_key: admission.idempotency_key,
        spend_capability_ref: admission.spend_capability_ref,
        rail_proof_refs: input.rail_proof_refs,
    })
}

fn admit_payment_rail(
    input: PaymentRailAdmission<'_>,
) -> Result<PaymentRailAdmissionDecision<'_>, PaymentAuthorityError> {
    let spends = payment_authority_spends(input.child_authority);

    if spends {
        let Some(decision) = input.reservation_decision else {
            return Err(PaymentAuthorityError::MissingReservationDecision);
        };
        if !decision_selects_payment_execution(decision) {
            return Err(PaymentAuthorityError::ReservationDecisionNotSelected);
        }
        ensure_idempotency_key(&input)?;
        ensure_bounded_spend_counterparty(input.child_authority)?;
        ensure_single_use_spend_capability(&input)?;
    }

    if !input.subset_proof_present {
        return Err(PaymentAuthorityError::MissingSubsetProof);
    }

    if !is_payment_authority_subset(input.child_authority, input.parent_authority) {
        return Err(PaymentAuthorityError::AuthorityNotSubset);
    }

    Ok(PaymentRailAdmissionDecision {
        parent_term_id: &input.parent_authority.term_id,
        child_term_id: &input.child_authority.term_id,
        idempotency_key: input.idempotency_key,
        spend_capability_ref: input.spend_capability_ref,
    })
}

fn decision_selects_payment_execution(decision: &Decision) -> bool {
    matches!(
        decision.choice,
        DecisionChoice::Open
            | DecisionChoice::Continue
            | DecisionChoice::SpawnChild
            | DecisionChoice::Close
    ) && (decision.selected_act_id.is_some() || decision.selected_harness_ref.is_some())
}

fn ensure_single_use_spend_capability(
    input: &PaymentRailAdmission<'_>,
) -> Result<(), PaymentAuthorityError> {
    let Some(payment) = input.child_authority.bounds.payment.as_ref() else {
        return Err(PaymentAuthorityError::SpendRequiresSingleUseCapability);
    };

    let has_single_use = payment.single_use_spend
        && payment.credential_form == Some(PaymentCredentialForm::SingleUseSpendCapability)
        && input
            .child_authority
            .capabilities
            .contains(&AuthorityCapability::PaymentSingleUseSpend);

    if !has_single_use || input.spend_capability_ref.is_none() {
        return Err(PaymentAuthorityError::SpendRequiresSingleUseCapability);
    }

    let Some(binding) = input.spend_capability_binding.as_ref() else {
        return Err(PaymentAuthorityError::SpendRequiresSingleUseCapability);
    };
    let Some(decision) = input.reservation_decision else {
        return Err(PaymentAuthorityError::MissingReservationDecision);
    };
    if !spend_capability_binding_matches(input, decision, payment, binding) {
        return Err(PaymentAuthorityError::SpendCapabilityBindingMismatch);
    }

    let Some(spend_capability_ref) = input.spend_capability_ref else {
        return Err(PaymentAuthorityError::SpendRequiresSingleUseCapability);
    };

    if input
        .consumed_spend_capability_refs
        .iter()
        .any(|consumed| same_reference(consumed, spend_capability_ref))
    {
        return Err(PaymentAuthorityError::SpendCapabilityAlreadyConsumed);
    }

    Ok(())
}

fn ensure_idempotency_key(input: &PaymentRailAdmission<'_>) -> Result<(), PaymentAuthorityError> {
    let idempotency_required = input
        .child_authority
        .bounds
        .payment
        .as_ref()
        .is_some_and(|payment| payment.idempotency_required);

    if idempotency_required && input.idempotency_key.is_none_or(str::is_empty) {
        return Err(PaymentAuthorityError::MissingIdempotencyKey);
    }

    Ok(())
}

fn ensure_bounded_spend_counterparty(term: &AuthorityTerm) -> Result<(), PaymentAuthorityError> {
    let Some(payment) = term.bounds.payment.as_ref() else {
        return Err(PaymentAuthorityError::WildcardCounterpartyDenied);
    };
    let Some(counterparty) = payment.counterparty.as_deref() else {
        return Err(PaymentAuthorityError::WildcardCounterpartyDenied);
    };
    if matches!(counterparty, "" | "*" | "any") {
        return Err(PaymentAuthorityError::WildcardCounterpartyDenied);
    }

    Ok(())
}

fn spend_capability_binding_matches(
    input: &PaymentRailAdmission<'_>,
    decision: &Decision,
    payment: &PaymentAuthorityBounds,
    binding: &PaymentSpendCapabilityBinding,
) -> bool {
    same_reference(&binding.child_harness_ref, input.child_harness_ref)
        && binding.act_id == input.act_id
        && decision
            .selected_act_id
            .as_deref()
            .is_none_or(|id| id == input.act_id)
        && decision
            .selected_harness_ref
            .as_ref()
            .is_none_or(|reference| same_reference(reference, input.child_harness_ref))
        && binding.reservation_decision_id == decision.decision_id
        && input
            .idempotency_key
            .is_some_and(|idempotency_key| idempotency_key == binding.idempotency_key)
        && payment
            .max_per_call_minor
            .is_some_and(|max| binding.amount_minor > 0 && binding.amount_minor <= max)
        && binding.currency == payment.currency
        && payment.rails.iter().any(|rail| rail == &binding.rail)
        && payment
            .counterparty
            .as_deref()
            .is_some_and(|counterparty| counterparty == binding.counterparty)
}

#[cfg(test)]
fn requires_receipt_before_success(parent: &AuthorityTerm, child: &AuthorityTerm) -> bool {
    payment_authority_requires_receipt_before_success(parent)
        || payment_authority_requires_receipt_before_success(child)
}

fn same_reference(left: &Reference, right: &Reference) -> bool {
    left.reference_type == right.reference_type && left.uri == right.uri
}

/// Returns true when `child` is no broader than `parent` under the pure payment
/// authority algebra.
///
/// The comparator is intentionally fail-closed: missing required payment
/// dimensions make the terms incomparable, and incomparable terms are denied by
/// returning false.
#[must_use]
pub fn is_payment_authority_subset(child: &AuthorityTerm, parent: &AuthorityTerm) -> bool {
    child.resource_family == AuthorityResourceFamily::Payment
        && parent.resource_family == AuthorityResourceFamily::Payment
        && same_authority_resource(&child.resource_ref, &parent.resource_ref)
        && verbs_subset(&child.verbs, &parent.verbs)
        && capabilities_subset(child, parent)
        && parent_conditions_preserved(child, parent)
        && parent_approvals_preserved(child, parent)
        && expiry_subset(child, parent)
        && payment_bounds_subset(child, parent)
}

fn same_authority_resource(child: &Reference, parent: &Reference) -> bool {
    child.reference_type == parent.reference_type && child.uri == parent.uri
}

fn payment_bounds_subset(child: &AuthorityTerm, parent: &AuthorityTerm) -> bool {
    let Some(child_payment) = child.bounds.payment.as_ref() else {
        return false;
    };
    let Some(parent_payment) = parent.bounds.payment.as_ref() else {
        return false;
    };

    required_currency_equal(child_payment, parent_payment)
        && minor_unit_caps_subset(child, child_payment, parent_payment)
        && rails_subset(child_payment, parent_payment)
        && optional_exact_or_narrower(&child_payment.realm, &parent_payment.realm)
        && optional_exact_or_narrower(&child_payment.counterparty, &parent_payment.counterparty)
        && optional_exact_or_narrower(&child_payment.operation, &parent_payment.operation)
        && optional_exact_or_narrower(&child_payment.period, &parent_payment.period)
        && required_booleans_subset(child_payment, parent_payment)
        && optional_u64_lte_when_parent_set(child_payment.quote_ttl_ms, parent_payment.quote_ttl_ms)
        && optional_u64_lte_when_parent_set(
            child_payment.approval_threshold_minor,
            parent_payment.approval_threshold_minor,
        )
        && optional_exact_or_narrower(
            &child_payment.credential_form,
            &parent_payment.credential_form,
        )
        && single_use_spend_capability_for_reserve_or_spend(child, parent)
}

fn verbs_subset(child: &[AuthorityVerb], parent: &[AuthorityVerb]) -> bool {
    child.iter().all(|verb| parent.contains(verb))
}

fn capabilities_subset(child: &AuthorityTerm, parent: &AuthorityTerm) -> bool {
    child
        .capabilities
        .iter()
        .all(|capability| parent.capabilities.contains(capability))
}

fn parent_conditions_preserved(child: &AuthorityTerm, parent: &AuthorityTerm) -> bool {
    parent
        .conditions
        .iter()
        .all(|condition| condition_is_preserved(condition, &child.conditions))
}

fn condition_is_preserved(
    parent: &AuthorityCondition,
    child_conditions: &[AuthorityCondition],
) -> bool {
    child_conditions.iter().any(|child| child == parent)
}

fn parent_approvals_preserved(child: &AuthorityTerm, parent: &AuthorityTerm) -> bool {
    parent
        .approvals
        .iter()
        .all(|approval| child.approvals.contains(approval))
}

fn expiry_subset(child: &AuthorityTerm, parent: &AuthorityTerm) -> bool {
    match (&child.expires_at, &parent.expires_at) {
        (_, None) => true,
        (Some(child), Some(parent)) => child <= parent,
        (None, Some(_)) => false,
    }
}

fn required_currency_equal(
    child: &PaymentAuthorityBounds,
    parent: &PaymentAuthorityBounds,
) -> bool {
    child.currency == parent.currency
}

fn minor_unit_caps_subset(
    child: &AuthorityTerm,
    child_payment: &PaymentAuthorityBounds,
    parent_payment: &PaymentAuthorityBounds,
) -> bool {
    if uses_minor_units(child) && child_payment.max_per_call_minor.is_none() {
        return false;
    }
    if uses_minor_units(child) && parent_payment.max_per_call_minor.is_none() {
        return false;
    }

    optional_cap_subset(
        child_payment.max_per_call_minor,
        parent_payment.max_per_call_minor,
    ) && optional_cap_subset(
        child_payment.max_per_run_minor,
        parent_payment.max_per_run_minor,
    ) && optional_cap_subset(
        child_payment.max_per_period_minor,
        parent_payment.max_per_period_minor,
    )
}

fn uses_minor_units(term: &AuthorityTerm) -> bool {
    term.verbs.iter().any(|verb| {
        matches!(
            verb,
            AuthorityVerb::Quote
                | AuthorityVerb::Reserve
                | AuthorityVerb::Spend
                | AuthorityVerb::Refund
        )
    })
}

fn optional_cap_subset(child: Option<u64>, parent: Option<u64>) -> bool {
    match (child, parent) {
        (Some(child), Some(parent)) => child <= parent,
        (None, Some(_)) => false,
        (Some(_), None) | (None, None) => true,
    }
}

fn rails_subset(child: &PaymentAuthorityBounds, parent: &PaymentAuthorityBounds) -> bool {
    !child.rails.is_empty()
        && !parent.rails.is_empty()
        && child.rails.iter().all(|rail| parent.rails.contains(rail))
}

fn optional_exact_or_narrower<T: Eq>(child: &Option<T>, parent: &Option<T>) -> bool {
    match (child, parent) {
        (_, None) => true,
        (Some(child), Some(parent)) => child == parent,
        (None, Some(_)) => false,
    }
}

fn required_booleans_subset(
    child: &PaymentAuthorityBounds,
    parent: &PaymentAuthorityBounds,
) -> bool {
    (!parent.quote_required || child.quote_required)
        && (!parent.reservation_required || child.reservation_required)
        && (!parent.idempotency_required || child.idempotency_required)
        && (!parent.recovery_required || child.recovery_required)
        && (!parent.receipt_before_success || child.receipt_before_success)
}

fn optional_u64_lte_when_parent_set(child: Option<u64>, parent: Option<u64>) -> bool {
    match parent {
        Some(parent) => child.is_some_and(|child| child <= parent),
        None => true,
    }
}

fn single_use_spend_capability_for_reserve_or_spend(
    child: &AuthorityTerm,
    parent: &AuthorityTerm,
) -> bool {
    if !requires_single_use_capability(child) {
        return true;
    }
    let child_payment = child.bounds.payment.as_ref();
    let parent_payment = parent.bounds.payment.as_ref();
    child_payment.is_some_and(|payment| payment.single_use_spend)
        && parent_payment.is_some_and(|payment| payment.single_use_spend)
        && child
            .capabilities
            .contains(&AuthorityCapability::PaymentSingleUseSpend)
        && parent
            .capabilities
            .contains(&AuthorityCapability::PaymentSingleUseSpend)
        && child
            .bounds
            .payment
            .as_ref()
            .and_then(|payment| payment.credential_form.as_ref())
            == Some(&PaymentCredentialForm::SingleUseSpendCapability)
}

fn requires_single_use_capability(term: &AuthorityTerm) -> bool {
    term.verbs
        .iter()
        .any(|verb| matches!(verb, AuthorityVerb::Spend))
}

#[cfg(test)]
mod tests {
    use super::{
        PaymentAuthorityError, PaymentRailAuthorization, PaymentRailAuthorizationDecision,
        PaymentSpendCapabilityBinding, authorize_payment_rail,
    };
    use runx_contracts::{
        AuthorityApproval, AuthorityBounds, AuthorityCapability, AuthorityCondition,
        AuthorityConditionPredicate, AuthorityResourceFamily, AuthorityTerm, AuthorityVerb,
        Decision, DecisionChoice, DecisionInputs, DecisionJustification, Intent,
        PaymentAuthorityBounds, PaymentCredentialForm, Reference, ReferenceType,
    };

    const ACT_ID: &str = "act_payment_spend";
    const IDEMPOTENCY_KEY: &str = "idem:decision_payment_reservation:harness-payment-rail";
    const COUNTERPARTY: &str = "merchant-123";

    #[test]
    fn admits_reserved_spend_with_subset_proof_and_rail_proof() {
        let scenario = PaymentScenario::standard();

        let result = scenario.authorize_decision();

        assert_eq!(
            result.map(|decision| (
                decision.parent_term_id,
                decision.child_term_id,
                decision.idempotency_key,
                decision.rail_proof_refs.len(),
            )),
            Ok(("parent", "child", Some(IDEMPOTENCY_KEY), 1))
        );
    }

    #[test]
    fn denies_amount_widening_before_rail() {
        let mut scenario = PaymentScenario::with_child(payment_term(
            "child",
            vec![AuthorityVerb::Spend],
            PaymentShape::new(2_000, &["card"]),
        ));
        scenario.parent = payment_term(
            "parent",
            vec![AuthorityVerb::Reserve, AuthorityVerb::Spend],
            PaymentShape::new(1_000, &["card"]),
        );

        assert_eq!(
            scenario.authorize(),
            Err(PaymentAuthorityError::AuthorityNotSubset)
        );
    }

    #[test]
    fn denies_spend_derived_from_reserve_only_parent() {
        let mut scenario = PaymentScenario::standard();
        scenario.parent.verbs = vec![AuthorityVerb::Reserve, AuthorityVerb::Verify];

        assert_eq!(
            scenario.authorize(),
            Err(PaymentAuthorityError::AuthorityNotSubset)
        );
    }

    #[test]
    fn denies_removing_parent_conditions_or_approvals() {
        let mut scenario = PaymentScenario::standard();
        scenario.parent.conditions = vec![payment_condition()];
        scenario.parent.approvals = vec![payment_approval()];
        scenario.child.conditions = Vec::new();
        scenario.child.approvals = Vec::new();

        assert_eq!(
            scenario.authorize(),
            Err(PaymentAuthorityError::AuthorityNotSubset)
        );
    }

    #[test]
    fn admits_child_that_preserves_parent_conditions_and_approvals() {
        let mut scenario = PaymentScenario::standard();
        let condition = payment_condition();
        let approval = payment_approval();
        scenario.parent.conditions = vec![condition.clone()];
        scenario.parent.approvals = vec![approval.clone()];
        scenario.child.conditions = vec![condition];
        scenario.child.approvals = vec![approval];

        assert_eq!(scenario.authorize(), Ok(()));
    }

    #[test]
    fn denies_missing_reservation_decision() {
        let scenario = PaymentScenario::standard();

        assert_eq!(
            scenario.authorize_with(AuthorizationOverride {
                reservation_decision: Some(None),
                ..AuthorizationOverride::default()
            }),
            Err(PaymentAuthorityError::MissingReservationDecision)
        );
    }

    #[test]
    fn denies_unselected_reservation_decision() {
        let scenario = PaymentScenario::with_decision(unselected_decision());

        assert_eq!(
            scenario.authorize(),
            Err(PaymentAuthorityError::ReservationDecisionNotSelected)
        );
    }

    #[test]
    fn denies_missing_subset_proof() {
        let scenario = PaymentScenario::standard();

        assert_eq!(
            scenario.authorize_with(AuthorizationOverride {
                subset_proof_present: Some(false),
                ..AuthorizationOverride::default()
            }),
            Err(PaymentAuthorityError::MissingSubsetProof)
        );
    }

    #[test]
    fn denies_missing_idempotency_key_for_spend() {
        let scenario = PaymentScenario::standard();

        assert_eq!(
            scenario.authorize_with(AuthorizationOverride {
                idempotency_key: Some(None),
                ..AuthorizationOverride::default()
            }),
            Err(PaymentAuthorityError::MissingIdempotencyKey)
        );
    }

    #[test]
    fn denies_wildcard_counterparty_for_spend() {
        let scenario = PaymentScenario::with_child(child_wildcard_counterparty_term());

        assert_eq!(
            scenario.authorize(),
            Err(PaymentAuthorityError::WildcardCounterpartyDenied)
        );
    }

    #[test]
    fn denies_spend_capability_binding_that_does_not_match_act() {
        let scenario = PaymentScenario::standard();

        assert_eq!(
            scenario.authorize_with(AuthorizationOverride {
                spend_capability_binding: Some(Some(PaymentSpendCapabilityBinding {
                    act_id: "act_payment_other".to_owned(),
                    ..scenario.capability_binding()
                })),
                ..AuthorizationOverride::default()
            }),
            Err(PaymentAuthorityError::SpendCapabilityBindingMismatch)
        );
    }

    #[test]
    fn denies_missing_rail_proof_when_receipt_before_success_required() {
        let scenario = PaymentScenario::standard();

        assert_eq!(
            scenario.authorize_with(AuthorizationOverride {
                rail_proof_refs: Some(&[]),
                ..AuthorizationOverride::default()
            }),
            Err(PaymentAuthorityError::MissingReceiptBeforeSuccess)
        );
    }

    #[test]
    fn denies_sibling_reuse_of_single_use_spend_capability() {
        let mut scenario = PaymentScenario::standard();
        scenario.consumed_spend_capability_refs = vec![scenario.spend_capability_ref.clone()];

        assert_eq!(
            scenario.authorize(),
            Err(PaymentAuthorityError::SpendCapabilityAlreadyConsumed)
        );
    }

    struct PaymentScenario {
        parent: AuthorityTerm,
        child: AuthorityTerm,
        decision: Decision,
        rail_proof_refs: Vec<Reference>,
        consumed_spend_capability_refs: Vec<Reference>,
        child_harness_ref: Reference,
        spend_capability_ref: Reference,
    }

    impl PaymentScenario {
        fn standard() -> Self {
            Self::with_child(child_spend_term())
        }

        fn with_child(child: AuthorityTerm) -> Self {
            Self {
                parent: parent_spend_term(),
                child,
                decision: selected_decision(),
                rail_proof_refs: vec![reference(ReferenceType::Receipt, "runx:receipt:rail-1")],
                consumed_spend_capability_refs: Vec::new(),
                child_harness_ref: reference(
                    ReferenceType::Harness,
                    "runx:harness:harness-payment-rail",
                ),
                spend_capability_ref: reference(
                    ReferenceType::Credential,
                    "runx:payment-capability:spend-1",
                ),
            }
        }

        fn with_decision(decision: Decision) -> Self {
            Self {
                decision,
                ..Self::standard()
            }
        }

        fn authorize(&self) -> Result<(), PaymentAuthorityError> {
            self.authorize_with(AuthorizationOverride::default())
        }

        fn authorize_decision(
            &self,
        ) -> Result<PaymentRailAuthorizationDecision<'_>, PaymentAuthorityError> {
            let binding = self.capability_binding();

            authorize_payment_rail(PaymentRailAuthorization {
                parent_authority: &self.parent,
                child_authority: &self.child,
                reservation_decision: Some(&self.decision),
                subset_proof_present: true,
                child_harness_ref: &self.child_harness_ref,
                act_id: ACT_ID,
                idempotency_key: Some(IDEMPOTENCY_KEY),
                spend_capability_binding: Some(binding),
                rail_proof_refs: &self.rail_proof_refs,
                consumed_spend_capability_refs: &self.consumed_spend_capability_refs,
                spend_capability_ref: Some(&self.spend_capability_ref),
            })
        }

        fn authorize_with(
            &self,
            overrides: AuthorizationOverride<'_>,
        ) -> Result<(), PaymentAuthorityError> {
            let default_binding = self.capability_binding();
            let reservation_decision = overrides
                .reservation_decision
                .unwrap_or(Some(&self.decision));
            let idempotency_key = overrides.idempotency_key.unwrap_or(Some(IDEMPOTENCY_KEY));
            let spend_capability_binding = overrides
                .spend_capability_binding
                .unwrap_or(Some(default_binding));
            let rail_proof_refs = overrides.rail_proof_refs.unwrap_or(&self.rail_proof_refs);
            let subset_proof_present = overrides.subset_proof_present.unwrap_or(true);

            authorize_payment_rail(PaymentRailAuthorization {
                parent_authority: &self.parent,
                child_authority: &self.child,
                reservation_decision,
                subset_proof_present,
                child_harness_ref: &self.child_harness_ref,
                act_id: ACT_ID,
                idempotency_key,
                spend_capability_binding,
                rail_proof_refs,
                consumed_spend_capability_refs: &self.consumed_spend_capability_refs,
                spend_capability_ref: Some(&self.spend_capability_ref),
            })
            .map(|_| ())
        }

        fn capability_binding(&self) -> PaymentSpendCapabilityBinding {
            PaymentSpendCapabilityBinding {
                child_harness_ref: self.child_harness_ref.clone(),
                act_id: ACT_ID.to_owned(),
                reservation_decision_id: "decision_payment_reservation".to_owned(),
                idempotency_key: IDEMPOTENCY_KEY.to_owned(),
                amount_minor: 1_250,
                currency: "USD".to_owned(),
                counterparty: COUNTERPARTY.to_owned(),
                rail: "card".to_owned(),
            }
        }
    }

    #[derive(Default)]
    struct AuthorizationOverride<'a> {
        reservation_decision: Option<Option<&'a Decision>>,
        subset_proof_present: Option<bool>,
        idempotency_key: Option<Option<&'a str>>,
        spend_capability_binding: Option<Option<PaymentSpendCapabilityBinding>>,
        rail_proof_refs: Option<&'a [Reference]>,
    }

    fn parent_spend_term() -> AuthorityTerm {
        payment_term(
            "parent",
            vec![
                AuthorityVerb::Quote,
                AuthorityVerb::Reserve,
                AuthorityVerb::Spend,
                AuthorityVerb::Verify,
            ],
            PaymentShape::new(10_000, &["card", "ach"]),
        )
    }

    fn child_spend_term() -> AuthorityTerm {
        payment_term(
            "child",
            vec![AuthorityVerb::Reserve, AuthorityVerb::Spend],
            PaymentShape::new(2_500, &["card"]),
        )
    }

    fn child_wildcard_counterparty_term() -> AuthorityTerm {
        let mut term = child_spend_term();
        if let Some(payment) = term.bounds.payment.as_mut() {
            payment.counterparty = Some("*".to_owned());
        }
        term
    }

    struct PaymentShape {
        max_per_call_minor: u64,
        rails: Vec<String>,
    }

    impl PaymentShape {
        fn new(max_per_call_minor: u64, rails: &[&str]) -> Self {
            Self {
                max_per_call_minor,
                rails: rails.iter().map(|rail| (*rail).to_owned()).collect(),
            }
        }
    }

    fn payment_term(
        term_id: &str,
        verbs: Vec<AuthorityVerb>,
        shape: PaymentShape,
    ) -> AuthorityTerm {
        AuthorityTerm {
            term_id: term_id.to_owned(),
            principal_ref: reference(ReferenceType::Principal, "runx:principal:merchant-agent"),
            resource_ref: reference(ReferenceType::Grant, "runx:payment-grant:checkout"),
            resource_family: AuthorityResourceFamily::Payment,
            verbs,
            bounds: AuthorityBounds {
                payment: Some(PaymentAuthorityBounds {
                    currency: "USD".to_owned(),
                    max_per_call_minor: Some(shape.max_per_call_minor),
                    max_per_run_minor: Some(25_000),
                    max_per_period_minor: None,
                    period: None,
                    rails: shape.rails,
                    realm: None,
                    counterparty: Some(COUNTERPARTY.to_owned()),
                    operation: Some("checkout".to_owned()),
                    quote_ttl_ms: Some(120_000),
                    approval_threshold_minor: Some(7_500),
                    credential_form: Some(PaymentCredentialForm::SingleUseSpendCapability),
                    quote_required: true,
                    reservation_required: true,
                    idempotency_required: true,
                    recovery_required: true,
                    receipt_before_success: true,
                    single_use_spend: true,
                }),
                ..AuthorityBounds::default()
            },
            conditions: Vec::new(),
            approvals: Vec::new(),
            capabilities: vec![AuthorityCapability::PaymentSingleUseSpend],
            expires_at: Some("2026-05-21T00:00:00Z".to_owned()),
            issued_by_ref: reference(ReferenceType::Grant, "runx:grant:issuer"),
            credential_ref: Some(reference(
                ReferenceType::Credential,
                "runx:credential:payment-session",
            )),
        }
    }

    fn selected_decision() -> Decision {
        Decision {
            decision_id: "decision_payment_reservation".to_owned(),
            choice: DecisionChoice::Continue,
            inputs: DecisionInputs::default(),
            proposed_intent: intent(),
            selected_act_id: Some(ACT_ID.to_owned()),
            selected_harness_ref: None,
            justification: DecisionJustification {
                summary: "reservation selected a bounded spend act".to_owned(),
                evidence_refs: Vec::new(),
            },
            closure: None,
            artifact_refs: Vec::new(),
        }
    }

    fn unselected_decision() -> Decision {
        Decision {
            selected_act_id: None,
            selected_harness_ref: None,
            ..selected_decision()
        }
    }

    fn intent() -> Intent {
        Intent {
            purpose: "complete a bounded checkout payment".to_owned(),
            legitimacy: "authorized by selected reservation decision".to_owned(),
            success_criteria: Vec::new(),
            constraints: Vec::new(),
            derived_from: Vec::new(),
        }
    }

    fn reference(reference_type: ReferenceType, uri: &str) -> Reference {
        Reference {
            reference_type,
            uri: uri.to_owned(),
            provider: None,
            locator: None,
            label: None,
            observed_at: None,
            proof_kind: None,
        }
    }

    fn payment_condition() -> AuthorityCondition {
        AuthorityCondition {
            condition_id: "condition_payment_receipt".to_owned(),
            predicate: AuthorityConditionPredicate::PaymentReceiptPresent,
            refs: Vec::new(),
            parameters: None,
        }
    }

    fn payment_approval() -> AuthorityApproval {
        AuthorityApproval {
            approval_ref: reference(ReferenceType::Decision, "runx:decision:payment-approval"),
            approved_by_ref: Some(reference(
                ReferenceType::Principal,
                "runx:principal:operator",
            )),
            approved_at: Some("2026-05-20T00:00:00Z".to_owned()),
            criterion_ids: vec!["payment_receipt".to_owned()],
        }
    }
}
