---
spec_version: '2.0'
task_id: payment-authority-term-v1
created: '2026-05-20T00:00:00Z'
updated: '2026-05-20T00:58:59Z'
status: completed
harden_status: not_run
size: small
risk_level: high
---

# Payment authority term v1

## Current State

Status: completed
Current phase: final
Next: done
Reason: task completed
Blockers: none
Allowed follow-up command: `none`
Latest runner update: 2026-05-20T00:58:59Z
Review gate: pass

## Summary

Define payment as an authority term, not a parallel durable payment object.
Payment challenge, reservation, rail execution, receipt proof, and ledger views
must be represented by existing spine concepts: Signal, Decision, Harness, Act,
receipt payload/reference, and projection.

Current repo-state caveat: Rust policy admission is still string-scope based
(`scopes`, `scope_family`, wildcard/prefix matching). This spec does not pretend
that model is enough for money movement. It introduces the payment authority
algebra and subset rules that Rust policy must enforce before any rail skill can
spend.

## Problem

Connected-auth scope admission can answer "does this skill scope fit this
grant?" It cannot answer "is this attempted payment a subset of the reserved
spend authority?" without typed amount, currency, merchant/rail, recipient,
purpose, expiry, idempotency, and receipt constraints.

The wrong shape would add durable `Payment`, `Reservation`, or `LedgerEntry`
objects beside Harness and Receipts. That would split auditability, retries, and
authority attenuation across two systems. Payment must instead be a domain term
inside the authority algebra, and durable state must remain the harness receipt
tree plus projections.

## Contract

A payment authority term is a regular spine `authorityTerm` with a payment
resource family and payment-specific bounds/conditions.

Term fields:
- `resource_family`: add `payment`.
- `verbs`: add or use payment verbs `quote`, `reserve`, `spend`, `refund`,
  and `verify`. `spend` is the only rail-mutating verb.
- `resource_ref`: identifies the merchant account, wallet/account handle, rail
  profile, or logical payment program. It must not contain wallet secrets.
- `principal_ref`: actor or child harness principal allowed to exercise the
  term.
- `bounds`: payment constraints carried in the existing bounds object plus a
  typed payment extension until the schema is promoted.
- `conditions`: require `decision_selected`, `within_budget`,
  `within_time_window`, `payment_receipt_present`, and rail-proof criteria as
  applicable.
- `approvals`: human/system approvals that authorize the reservation or spend.
- `capabilities`: must include a spend capability token for `spend`; reserve
  terms may omit it.
- `expires_at`, `issued_by_ref`, `credential_ref`: same semantics as current
  authority terms. `expires_at` is the enforced temporal bound in v1.
  `credential_ref` may point to a secret-handle reference but never raw secret
  material.

Payment bounds:
- `currency`: ISO 4217 currency code.
- `max_per_call_minor`: optional cap for a single quote, reservation, spend, or
  refund in minor units.
- `max_per_run_minor`: optional aggregate cap for the enclosing harness run in
  minor units.
- `max_per_period_minor`: optional aggregate cap for the named `period` in
  minor units.
- `period`: optional accounting or rate-limit period. A child may equal or
  narrow the parent period, but may not omit a parent period.
- `rails`: rail/provider allowlist. A child rail set must be a subset of the
  parent rail set.
- `realm`: optional merchant account, wallet/account handle, or logical payment
  program. A child may equal or narrow it, but may not omit a parent realm.
- `counterparty`: optional payee, merchant, wallet, or invoice reference. A
  child may equal or narrow it, but may not omit a parent counterparty.
- `operation`: optional stable business purpose or rail operation label. A
  child may equal or narrow it, but may not omit a parent operation.
- `quote_ttl_ms`: optional quote freshness cap. A child cap must be less than
  or equal to the parent cap when the parent sets one.
- `approval_threshold_minor`: optional approval threshold. A child threshold
  must be less than or equal to the parent threshold when the parent sets one.
- `credential_form`: optional required credential form. In v1 the payment form
  is `single_use_spend_capability`.
- `quote_required`, `reservation_required`, `idempotency_required`,
  `recovery_required`, `receipt_before_success`, `single_use_spend`: required
  boolean constraints. A child must preserve each parent `true` constraint.

Subset / partial-order rules:
- Child term `resource_family` must equal parent `payment`.
- Child verbs must be a subset of parent verbs; `spend` cannot be derived from a
  parent that only has `reserve` or `verify`.
- Child minor-unit caps must be less than or equal to the parent caps when the
  parent sets them, and payment currency must be identical.
- Child `rails` must be a subset of the parent rail allowlist.
- Child `realm`, `counterparty`, `operation`, and `period` must equal the
  parent or narrow an omitted parent; a child may not omit a parent value. No
  wildcard counterparty for `spend`.
- Child `expires_at` must be less than or equal to the parent `expires_at` when
  the parent sets one. v1 does not define `not_before`.
- Runtime admission must require an idempotency key when the term sets
  `idempotency_required`; key derivation and rail reconciliation belong to the
  child harness runtime guard, not the pure subset comparator.
- Child capabilities must be a subset of parent capabilities. A `spend`
  capability is single-use and cannot be copied into sibling child harnesses.
- Additional child conditions/approvals may narrow the term; removing parent
  conditions or approvals is widening and must fail subset proof.

## Spine Mapping

- Challenge: a payment request is a `Signal` from an invoice, checkout event,
  internal request, or verified operator intent. It carries source/evidence refs
  and no rail secret.
- Reservation: the parent Harness records a `Decision` selecting the payment
  intent. The Decision artifact/metadata contains the reserved payment bounds,
  authority subset proof, and idempotency key.
- Fulfillment: the actual rail call runs in a child `Harness` with attenuated
  payment authority and one terminal `Act` for the rail operation.
- Rail proof: the rail response is stored as a receipt payload or referenced by
  a receipt ref. Sensitive rail data is redacted/committed by hash; the receipt
  ref is the proof surface.
- Ledger: account history, spend reports, and reconciliation are projections
  over Harness receipts and rail proof refs. They are rebuildable views, not
  independent durable payment truth.

## Correctness Rules

- Single-use spend capability: a `spend` term may be consumed once. The consumed
  capability is bound to the child Harness id, Act id, reservation Decision id,
  amount, currency, recipient, rail, and idempotency key.
- No wallet secret to rail skill: rail skills receive only scoped secret handles
  or provider session refs. Raw wallet keys, seed phrases, card PAN/CVV, and bank
  credentials never appear in Signal, Decision, Act, receipt payload, or logs.
- Reserve before rail: a child rail Harness cannot start unless the parent
  Harness has a selected Decision with payment bounds and a passing subset proof.
- Crash recovery by idempotency key: retries reuse the same idempotency key and
  reconcile the rail state before attempting another mutation. A recovered rail
  success seals the Harness from the existing rail proof.
- Receipt before success: runx must not report payment success until the child
  Harness receipt includes the rail proof ref/payload and verifies the Act
  criteria. A successful rail call with missing receipt proof is an incomplete
  Harness, not success.
- No parallel durable payment objects: reservation state, spend outcome, and
  proof live in Harness/Decision/Act/receipt data. Any ledger table or API is a
  projection with source receipt refs.

## Implementation Plan

Current landed surfaces:
- `packages/contracts/src/schemas/spine.ts` defines payment bounds under
  authority bounds and exports payment authority contract types.
- `crates/runx-contracts/src/authority.rs` mirrors `payment` resource family,
  payment verbs, single-use spend capability, payment credential form, and typed
  payment bounds.
- `crates/runx-core/src/policy/payment_authority.rs` implements the pure
  payment authority subset comparator.
- `crates/runx-runtime/src/payment_authority.rs` gates rail admission and
  authorization on reservation decision, subset proof, idempotency,
  single-use spend capability, bounded counterparty, and rail proof.
- `crates/runx-runtime/tests/payment_authority.rs` and
  `crates/runx-runtime/tests/payment_execution.rs` cover the runtime guard
  surface.

Remaining executable slices:

1. Receipt proof gate: landed. Strict parent/child harness receipt proof
   acceptance now exists in `runx-receipts` and runtime receipt-tree
   verification. Payment rail success must keep using that strict path so the
   sealing harness receipt proves rail proof refs and receipt-before-success
   criteria.

2. Kernel fixture parity: landed. TypeScript oracle generation now emits
   representative `is_payment_authority_subset` cases, and
   `crates/runx-core/tests/policy_fixtures.rs` dispatches those cases while the
   existing Rust unit/proptest coverage remains in place.

3. Runtime harness proof fixtures: landed for current runtime coverage.
   Payment execution tests cover allowed spend, amount widening denial, missing
   receipt proof, missing admission inputs, and sealed graph receipt validation
   through strict parent/child harness receipt proof. Sibling reuse of
   single-use spend capability remains covered in the pure runtime authority
   tests.

4. Projection discipline: any ledger, account history, spend report, or
   reconciliation view remains a projection over harness receipts and rail proof
   refs. Do not add a durable payment object while implementing payment
   reporting.

## Out Of Scope

- Building a wallet, payment processor, reconciliation service, or accounting
  system.
- Persisting a durable payment domain model outside Harness receipts.
- Selecting a specific rail provider.
- General-purpose finance ledger semantics beyond receipt-backed projections.

## Open Questions

- Exact `payment` bounds schema location inside spine: extend
  `authorityBounds` directly or add a typed `payment` sub-object under bounds.
  The implementation should pick the smallest schema change that preserves
  strict validation.
- Whether reserve-only terms should use a new `reserve` verb or model
  reservation as `approve` plus payment conditions. This draft prefers explicit
  `reserve` because it makes the partial order clearer.

## Review

Status: completed
Verdict: pass
Mode: verify
Provider: claude:claude-opus-4-7
Output: claude.mcp_submit_review
Summary: Verify-mode rerun against the prior discover-mode blockers PAY-V1-001 (verb/bounds drift) and PAY-V1-002 (`not_before` subset rule). Both are repaired in the current Contract section: verbs are now `quote, reserve, spend, refund, verify` matching `AuthorityVerb` in crates/runx-contracts/src/authority.rs:22-40 (no `void`); payment bounds enumerate `currency, max_per_call_minor, max_per_run_minor, max_per_period_minor, period, rails, realm, counterparty, operation, quote_ttl_ms, approval_threshold_minor, credential_form, quote_required, reservation_required, idempotency_required, recovery_required, receipt_before_success, single_use_spend`, field-for-field with `PaymentAuthorityBounds` (authority.rs:62-99) and the TypeScript `paymentAuthorityBoundsSchema` (packages/contracts/src/schemas/spine.ts:372-394); the temporal subset rule now states only `expires_at` and explicitly disclaims `not_before` for v1 (spec line 116), matching `expiry_subset` in crates/runx-core/src/policy/payment_authority.rs:89-95. Grep confirms no residual `void`, `amount_minor`, `recipient_ref`, `merchant_ref`, `idempotency_key`, `single_use`, or `not_before` field references in the Contract section (only in the historical review record). No new regressions surfaced; ambient workspace drift is empty; no task-scope changes since approval baseline. Verify-mode rerun policy `verify_open_blockers` is satisfied.

Attack log:
- `.scafld/specs/active/payment-authority-term-v1.md Contract section vs crates/runx-contracts/src/authority.rs`: Verify PAY-V1-001 repair: do the spec's verbs and payment bounds field names now match AuthorityVerb and PaymentAuthorityBounds field-for-field? -> clean (Spec verbs (quote, reserve, spend, refund, verify) match AuthorityVerb; `void` is gone. Spec bounds list at lines 77-103 matches PaymentAuthorityBounds (currency, max_per_call_minor, max_per_run_minor, max_per_period_minor, period, rails, realm, counterparty, operation, quote_ttl_ms, approval_threshold_minor, credential_form, plus the six *_required booleans + single_use_spend). Grep for legacy names (amount_minor|recipient_ref|merchant_ref|idempotency_key|single_use\b|\bvoid\b) returned only historical review-record matches, not Contract-section matches.)
- `.scafld/specs/active/payment-authority-term-v1.md subset rules vs crates/runx-core/src/policy/payment_authority.rs`: Verify PAY-V1-002 repair: is the `not_before` temporal subset rule either implemented or removed from v1? -> clean (Spec line 116 now states `Child expires_at must be less than or equal to the parent expires_at when the parent sets one. v1 does not define not_before.` This matches expiry_subset (payment_authority.rs:89-95), which only compares expires_at. The contract no longer advertises a two-sided window.)
- `packages/contracts/src/schemas/spine.ts paymentAuthorityBoundsSchema`: TypeScript / Rust parity: do the spine.ts schema fields agree with the Rust PaymentAuthorityBounds and the spec list? -> clean (paymentAuthorityBoundsSchema (spine.ts:372-394) enumerates the same fields as the Rust struct and the spec list, with additionalProperties:false. No drift.)
- `Ambient workspace drift + task scope`: Confirm no review_self_mutation or overlap_drift since approval baseline that could reintroduce regressions. -> clean (workspace_baseline=clean, task_changes=none, ambient_drift=none per the session manifest. No regressions to triage.)

Findings:
- none

