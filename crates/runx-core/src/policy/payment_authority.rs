// rust-style-allow: large-file - payment authority comparison is one algebraic boundary; splitting it before the core term model settles would hide subset invariants.
use runx_contracts::{
    AuthorityCapability, AuthorityCondition, AuthorityResourceFamily, AuthorityTerm, AuthorityVerb,
    Decision, DecisionChoice, PaymentAuthorityBounds, PaymentCredentialForm, Reference,
};
use thiserror::Error;

#[derive(Clone, Debug, PartialEq)]
pub struct PaymentRailAuthorization<'a> {
    pub parent_authority: &'a AuthorityTerm,
    pub child_authority: &'a AuthorityTerm,
    pub reservation_decision: Option<&'a Decision>,
    pub subset_proof_present: bool,
    pub child_harness_ref: &'a Reference,
    pub act_id: &'a str,
    pub idempotency_key: Option<&'a str>,
    pub spend_capability_binding: Option<PaymentSpendCapabilityBinding<'a>>,
    pub rail_proof_refs: &'a [Reference],
    pub consumed_spend_capability_refs: &'a [Reference],
    pub spend_capability_ref: Option<&'a Reference>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PaymentRailAdmission<'a> {
    pub parent_authority: &'a AuthorityTerm,
    pub child_authority: &'a AuthorityTerm,
    pub reservation_decision: Option<&'a Decision>,
    pub subset_proof_present: bool,
    pub child_harness_ref: &'a Reference,
    pub act_id: &'a str,
    pub idempotency_key: Option<&'a str>,
    pub spend_capability_binding: Option<PaymentSpendCapabilityBinding<'a>>,
    pub consumed_spend_capability_refs: &'a [Reference],
    pub spend_capability_ref: Option<&'a Reference>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PaymentSpendCapabilityBinding<'a> {
    pub child_harness_ref: &'a Reference,
    pub act_id: &'a str,
    pub reservation_decision_id: &'a str,
    pub idempotency_key: &'a str,
    pub amount_minor: u64,
    pub currency: &'a str,
    pub counterparty: &'a str,
    pub rail: &'a str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PaymentRailAuthorizationDecision<'a> {
    pub parent_term_id: &'a str,
    pub child_term_id: &'a str,
    pub idempotency_key: Option<&'a str>,
    pub spend_capability_ref: Option<&'a Reference>,
    pub rail_proof_refs: &'a [Reference],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PaymentRailAdmissionDecision<'a> {
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
    #[error("payment authority requires a rail receipt before success")]
    MissingReceiptBeforeSuccess,
    #[error("payment authority requires rail proof")]
    MissingRailProof,
}

#[must_use]
pub fn payment_authority_requires_receipt_before_success(term: &AuthorityTerm) -> bool {
    term.bounds
        .payment
        .as_ref()
        .is_some_and(|payment| payment.receipt_before_success)
}

#[must_use]
pub fn payment_authority_spends(term: &AuthorityTerm) -> bool {
    term.verbs
        .iter()
        .any(|verb| matches!(verb, AuthorityVerb::Spend))
}

pub fn authorize_payment_rail(
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

pub fn admit_payment_rail(
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
    binding: &PaymentSpendCapabilityBinding<'_>,
) -> bool {
    same_reference(binding.child_harness_ref, input.child_harness_ref)
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
        && payment.rails.iter().any(|rail| rail == binding.rail)
        && payment
            .counterparty
            .as_deref()
            .is_some_and(|counterparty| counterparty == binding.counterparty)
}

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
