---
spec_version: '2.0'
task_id: payment-rail-supervisor-proof-v1
created: '2026-05-22T02:04:04Z'
updated: '2026-05-26T04:58:01Z'
status: completed
harden_status: passed
size: medium
risk_level: high
---

# payment rail supervisor proof v1

## Current State

Status: completed
Current phase: final
Next: done
Reason: task completed
Blockers: none
Allowed follow-up command: `none`
Latest runner update: 2026-05-26T04:58:01Z
Review gate: pass

## Settled Architecture

- The Rust runtime owns the supervisor boundary. The skill emits only a *claim*
  (the rail packet in step outputs). It can never produce the proof or the
  supervisor evidence.
- The configured `PaymentRailSupervisor` returns
  `PaymentSupervisorSettlementEvidence` for the admitted spend authority
  (`StepPaymentAuthorityContext`: rail, counterparty, amount_minor, currency,
  idempotency_key, spend_capability_ref) plus the claim's `proof_ref`. The
  default supervisor rejects, so skill output alone cannot mint settlement
  evidence.
- The runtime validates the skill's rail-packet claim against admission and then
  validates the supervisor evidence/proof binding. Any drift in rail,
  counterparty, amount, currency, idempotency key, spend capability ref, act id,
  receipt ref, or receipt digest is denied.
- `output.metadata` is a runtime-controlled channel (it is `sandbox.metadata`
  built by `sandbox_metadata()`; a cli-tool skill cannot write it), so the
  runtime writing supervisor evidence there and the gate reading it is not a
  trust hole. The single rule: the runtime is the writer; the supervisor is the
  evidence source.
- `PaymentSupervisorProof` binds every admitted spend fact plus receipt
  ref/digest and an evidence digest, satisfying dod2. The negative
  proofless-rail fixture must still fail closed (dod1).

## A+ Coding Invariants

This work must hold the runx Rust core invariants (enforced by
`scripts/check-rust-core-style.mjs`, `crates/deny.toml`, and
`[workspace.lints]`):

- Typed errors only via `thiserror` (`PaymentSupervisorError`,
  `PaymentStateError`); no `anyhow`/`eyre`/`Box<dyn Error>` in library code.
- No `unwrap`/`expect`/`panic`/`todo`/`dbg`/`print`; spend gating fails closed
  with typed errors.
- No `serde_json::Value` in public API surfaces; `BTreeMap`/`BTreeSet`, never
  `HashMap`. No wildcard re-exports; `unsafe` forbidden.
- Parse-don't-validate: the supervisor proof match binds all admitted fields by
  type, so a partially-matched proof is unrepresentable.
- Production and tests share the same runtime supervisor boundary. Tests install
  a deterministic supervisor implementation; production defaults to fail-closed
  until a rail/provider supervisor is configured.
- File <=350 lines / fn <=60 lines with documented `// rust-style-allow:`
  escape hatches where a payment transaction genuinely needs it.

## Summary

Close R3 from `runx-security-hardening-v1`: payment spend success must require
a supervisor-verified rail settlement proof, not only a skill-produced
`Reference` typed as `Verification` plus `PaymentRail`.

Today the runtime can tell that a receipt contains a payment-rail-shaped proof
reference, but the proof is still asserted through step output controlled by the
skill. This spec makes the trusted Rust supervisor responsible for verifying the
settlement facts before a payment spend receipt may be sealed as successful,
persisted for idempotency replay, or projected into the payment ledger.

## Context

Packages:
- `crates/runx-runtime`
- `crates/runx-contracts` only if the supervisor proof becomes a public receipt
  or packet shape
- `packages/contracts` and `schemas` only if a generated public schema is needed

Files impacted by this draft:
- `.scafld/specs/drafts/payment-rail-supervisor-proof-v1.md`

Likely implementation touchpoints if this draft is promoted:
- `crates/runx-runtime/src/execution/runner/authority.rs`
- `crates/runx-runtime/src/execution/runner/steps.rs`
- `crates/runx-runtime/src/payment/packets.rs`
- `crates/runx-runtime/src/payment/state.rs`
- `crates/runx-runtime/src/payment/ledger.rs`
- `crates/runx-runtime/src/payment/supervisor.rs` or equivalent new module
- `crates/runx-runtime/src/lib.rs`
- `crates/runx-runtime/tests/payment/execution.rs`
- `crates/runx-runtime/tests/payment/state.rs`
- `crates/runx-runtime/tests/payment/ledger_projection.rs`
- `crates/runx-runtime/tests/payment/stripe_spt.rs`
- `fixtures/harness/x402-pay-*.yaml`
- `fixtures/graphs/payment/x402-pay-*.yaml`
- `fixtures/skills/x402-pay-*-fulfill/SKILL.md`
- `skills/pay-fulfill-rail/SKILL.md`
- `skills/pay-fulfill-rail/X.yaml`
- `schemas/reference.schema.json`, `schemas/receipt.schema.json`, or
  payment packet schemas only if the public wire shape changes
- `packages/contracts/src/schemas/spine.ts` or
  `packages/contracts/src/schemas/receipt.ts` only if generated schema parity is
  required

Related specs and docs:
- `.scafld/specs/active/runx-security-hardening-v1.md`
- `.scafld/specs/archive/2026-05/rust-payment-execution-boundary-v1.md`
- `.scafld/specs/archive/2026-05/x402-pay-idempotency-recovery-v1.md`
- `.scafld/specs/archive/2026-05/x402-pay-dogfood-v1.md`
- `docs/security-authority-proof.md`

Current alignment:
- `crates/runx-runtime/src/execution/runner/authority.rs` accepts a payment
  proof reference when `reference_type == Verification` and
  `proof_kind == PaymentRail`.
- `crates/runx-runtime/src/payment/packets.rs` reads
  `payment_rail_packet.data.rail_proof.proof_ref` and `idempotency_key` from
  step outputs.
- `crates/runx-runtime/src/payment/state.rs` persists payment idempotency,
  spend-capability consumption, and rail mutation state from payment step
  outputs and receipts.
- `crates/runx-runtime/src/execution/runner/steps.rs` replays sealed payment
  output only after rebuilding the receipt and finding the stored rail proof
  reference.
- `crates/runx-runtime/src/payment/ledger.rs` ties settlement evidence to a
  typed receipt proof reference, but the settlement packet and proof ref are
  still output-derived.
- Existing negative payment fixtures cover missing rail proof references; this
  spec must add coverage for forged or mismatched rail proof references.

## Objectives

- Introduce a runtime-owned supervisor settlement proof path for payment rails.
- Treat skill output as an untrusted settlement claim until corroborated by the
  supervisor.
- Bind the supervisor proof to the admitted payment spend facts: rail,
  counterparty, amount, currency, idempotency key, spend capability ref, act id,
  and receipt being sealed.
- Require the supervisor proof before spend success, idempotency replay, and
  ledger projection can treat payment settlement as terminal.
- Preserve replay and recovery semantics from the existing payment state work:
  same-key sealed replay returns the original receipt, in-flight rail mutation
  recovery escalates before a second rail mutation, and consumed capabilities
  remain single-use.
- Keep rail secrets, raw provider payloads, and raw credential material out of
  receipts, ledgers, public packets, and schema examples.

## Scope

In scope for the future build:
- A trusted supervisor proof model, implemented as runtime-internal state unless
  receipt or ledger consumers require a public schema.
- A verifier boundary that checks the rail settlement claim against
  supervisor-controlled evidence rather than accepting the skill's typed
  reference alone.
- Fail-closed checks for mismatched rail, counterparty, amount, currency,
  idempotency key, proof ref, spend capability ref, act id, receipt ref, and
  recovery state.
- Updates to payment state replay so persisted settlement proof data cannot be
  replayed for a different admitted spend.
- Updates to payment ledger projection so settlement evidence must carry or
  resolve to the supervisor proof, not just a typed proof reference.
- Mock and Stripe SPT test-mode proof paths as non-live validation surfaces.
- Focused docs or skill wording that clarifies rail fulfill skills return
  settlement claims; the runtime supervisor is the verifier.

Out of scope:
- Live-money settlement or new provider integrations.
- Stripe live mode, webhooks, PaymentIntent code, or provider SDK adoption.
- R1 sandbox enforcement and R2 receipt signing fixes, except as dependencies
  and risk notes.
- New native payment CLI commands.
- Refund, dispute, or cross-rail settlement semantics.
- Moving payment authority algebra back into `runx-core`.
- Editing runtime code as part of this draft authoring task.

## Dependencies

- `runx-security-hardening-v1` R3 defines the motivating security finding.
- C2 from `runx-security-hardening-v1` is complete and provides typed
  authority subset admission for spend steps.
- `rust-payment-execution-boundary-v1` provides typed payment packet readers and
  payment-domain state/projection boundaries to build on.
- `x402-pay-idempotency-recovery-v1` provides durable idempotency, sealed replay,
  and in-flight recovery expectations.
- R7 from `runx-security-hardening-v1` provides locked payment-state writes.
- R2 receipt signing is not required to implement this local gate, but the
  production security claim depends on signed receipts after this proof is
  sealed.
- `credential-broker-delivery-contract-v1` remains the boundary for any rail
  verifier that needs secret material.

## Assumptions

- The trusted supervisor for v1 is the Rust runtime, not the skill process.
- A rail fulfillment skill may emit a candidate proof ref and settlement packet,
  but that output is a claim until the supervisor verifies it.
- The first implementation can verify mock and Stripe SPT test-mode settlements
  using local, deterministic, supervisor-readable evidence.
- Public receipts can expose opaque proof refs, hashes, rail ids, amounts,
  currencies, counterparties, timestamps, and verifier ids, but not raw rail
  secrets or provider response bodies.
- If no reliable supervisor-readable evidence exists for a rail, that rail must
  fail closed or remain out of scope for this v1.

## Touchpoints

The current draft owns only this file. The following touchpoints are likely
implementation files when promoted; this draft does not modify them:

- `crates/runx-runtime/src/execution/runner/authority.rs` - replace
  proof-kind-only success gating with supervisor proof validation.
- `crates/runx-runtime/src/execution/runner/steps.rs` - carry supervisor proof
  results into receipt sealing, state persistence, and sealed replay.
- `crates/runx-runtime/src/payment/packets.rs` - keep reading skill settlement
  claims, but distinguish them from supervisor attestations.
- `crates/runx-runtime/src/payment/state.rs` - persist replay-safe supervisor
  proof data and reject mismatched replays.
- `crates/runx-runtime/src/payment/ledger.rs` - project settlement only from
  supervisor-attested proof evidence.
- `crates/runx-runtime/src/payment/supervisor.rs` or equivalent - new payment
  rail supervisor verifier boundary.
- `crates/runx-runtime/tests/payment/execution.rs` - positive and negative
  receipt-before-success coverage.
- `crates/runx-runtime/tests/payment/state.rs` - persisted proof and replay
  mismatch coverage.
- `crates/runx-runtime/tests/payment/ledger_projection.rs` - ledger projection
  proof verification coverage.
- `crates/runx-runtime/tests/payment/stripe_spt.rs` - Stripe SPT test-mode proof
  path, if the promoted build includes that rail.
- `fixtures/harness/x402-pay-negative-proofless-rail.yaml` and adjacent
  fixtures - extend from missing proof to forged or mismatched proof cases.

## Risks

- Description: The build could rename skill-produced proof refs as supervisor
  proofs without changing the trust boundary.
  Mitigation: require a negative test where the skill emits a well-typed
  `PaymentRail` reference that fails because no supervisor evidence matches it.
- Description: The verifier could check only idempotency key and miss amount,
  currency, rail, or counterparty drift.
  Mitigation: bind every admitted spend field into the supervisor proof match.
- Description: Replay could accept a previously sealed proof for a different
  spend.
  Mitigation: replay must re-check the stored proof against current admission,
  receipt id, receipt digest, and spend capability ref.
- Description: Public receipts could leak raw rail or provider details.
  Mitigation: receipts expose opaque proof refs, hashes, and normalized
  settlement facts only; raw provider bodies stay private to supervisor state.
- Description: This work can create schema churn if the proof is exposed as a
  public contract too early.
  Mitigation: keep the v1 verifier runtime-internal unless a receipt or ledger
  consumer demonstrably needs a public wire shape.
- Description: R2 remains open, so an unsigned local receipt can still be
  forged after the runtime creates a valid supervisor proof.
  Mitigation: record that this spec closes skill assertion, not cryptographic
  receipt authenticity; production payment claims still require R2.

## Acceptance

Profile: strict

Validation:
- [x] `v1` forged proof negative - Skill-only typed payment proof is denied.
  - Command: `cargo test --manifest-path crates/Cargo.toml -p runx-runtime --test payment -- --nocapture`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-24
- [x] `v2` payment state replay - Stored supervisor proof cannot be replayed for
  - Command: `cargo test --manifest-path crates/Cargo.toml -p runx-runtime --test payment -- --nocapture`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-25
- [x] `v3` ledger projection - Ledger refuses unverified settlement evidence.
  - Command: `cargo test --manifest-path crates/Cargo.toml -p runx-runtime --test payment -- --nocapture`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-26
- [x] `v4` Stripe SPT boundary - Test-mode rail remains verified without live
  - Command: `cargo test --manifest-path crates/Cargo.toml -p runx-runtime --test payment -- --nocapture`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-27
- [x] `v5` focused runtime regression - Payment runtime suites pass together.
  - Command: `cargo test --manifest-path crates/Cargo.toml -p runx-runtime --test payment`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-28
- [x] `v6` boundary grep - Old proof-kind-only success helper is gone.
  - Command: `! rg -n "fn is_payment_rail_proof_ref|spend success requires a sealed rail proof reference" crates/runx-runtime/src/execution/runner/authority.rs`
  - Expected kind: `no_matches`
  - Status: pass
  - Evidence: output was empty
  - Source event: entry-29
- [x] `v7` format and lint - Runtime code is formatted and warning-free.
  - Command: `cargo fmt --manifest-path crates/Cargo.toml --all --check && cargo clippy --manifest-path crates/Cargo.toml -p runx-runtime --all-targets -- -D warnings`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-30

## Phase 1: Proof Model And Boundary

Status: completed
Dependencies: none

Objective: Complete this phase.

Changes:
- `crates/runx-runtime/src/payment/supervisor.rs` (new, exclusive) - Define supervisor proof structs, verifier trait, and fail-closed errors.
- `crates/runx-runtime/src/payment/packets.rs` (line-level) - Preserve skill settlement claims as untrusted inputs.
- `crates/runx-runtime/src/lib.rs` (line-level) - Expose the payment supervisor module if needed by integration tests.
- `crates/runx-contracts/src/receipts.rs` and schema files (line-level, only if necessary) - Add public proof shape only after runtime-internal proof is insufficient.

Acceptance:
- none

## Phase 2: Receipt-Before-Success Gate

Status: completed
Dependencies: none

Objective: Complete this phase.

Changes:
- `crates/runx-runtime/src/execution/runner/authority.rs` (line-level) - Replace proof-kind-only acceptance with supervisor proof verification.
- `crates/runx-runtime/src/execution/runner/steps.rs` (line-level) - Thread supervisor proof results into receipt creation and persistence.
- `crates/runx-runtime/tests/payment/execution.rs` (line-level) - Add forged, missing, and mismatched proof tests.

Acceptance:
- none

## Phase 3: State, Replay, And Recovery

Status: completed
Dependencies: none

Objective: Complete this phase.

Changes:
- `crates/runx-runtime/src/payment/state.rs` (line-level) - Persist supervisor proof facts with sealed idempotency entries and rail mutations.
- `crates/runx-runtime/src/execution/runner/authority.rs` (line-level) - Revalidate stored proof binding during sealed replay admission.
- `crates/runx-runtime/src/execution/runner/steps.rs` (line-level) - Refuse replay when receipt digest, proof binding, or spend facts drift.
- `crates/runx-runtime/tests/payment/state.rs` (line-level) - Add replay and recovery mismatch coverage.

Acceptance:
- none

## Phase 4: Ledger And Fixture Hardening

Status: completed
Dependencies: none

Objective: Complete this phase.

Changes:
- `crates/runx-runtime/src/payment/ledger.rs` (line-level) - Require supervisor proof evidence for settlement projection.
- `crates/runx-runtime/tests/payment/ledger_projection.rs` (line-level) - Add projection failure coverage for missing and mismatched supervisor proofs.
- `fixtures/harness/x402-pay-negative-proofless-rail.yaml` and adjacent `x402-pay-negative-*` fixtures (line-level) - Add forged or mismatched proof cases.
- `fixtures/graphs/payment/x402-pay-negative-*.yaml` (line-level) - Wire the negative cases without broad graph churn.
- `fixtures/skills/x402-pay-negative-proofless-fulfill/SKILL.md` (line-level) - Emit a typed but unverified proof claim for the forged-proof negative case.

Acceptance:
- none

## Phase 5: Hardening Closure

Status: completed
Dependencies: none

Objective: Complete this phase.

Changes:
- `.scafld/specs/active/runx-security-hardening-v1.md` (line-level, only after this spec is active and green) - Mark R3/dod7 complete with evidence.
- `docs/security-authority-proof.md` (line-level, optional) - Clarify that payment settlement proof is supervisor-attested.

Acceptance:
- none

## Rollback

Strategy: per_phase

This draft-only task has no runtime rollback. Delete this draft file if the
planning direction is rejected before promotion.

For a promoted implementation:
- Phase 1 rollback removes the new supervisor proof module and any public schema
  additions.
- Phase 2 rollback restores the prior receipt-before-success gate, but must
  reopen R3 because proof-kind-only acceptance is the security finding.
- Phase 3 rollback removes new payment-state proof fields and bumps or restores
  the state schema consistently.
- Phase 4 rollback removes the new negative fixtures and ledger proof checks.
- Phase 5 rollback reopens `runx-security-hardening-v1` R3/dod7.

## Origin

Source:
- User request on 2026-05-22: create a draft scafld spec for R3 from
  `runx-security-hardening-v1`, payment rail supervisor proof, without editing
  runtime code.
- `.scafld/specs/active/runx-security-hardening-v1.md` R3: payment proof is
  skill-asserted and must be bound to out-of-band rail settlement verified by
  the supervisor.

Repo:
- `/Users/kam/dev/runx/runx/oss`

Grounded in read-only inspection:
- `.scafld/specs/active/runx-security-hardening-v1.md`
- `docs/security-authority-proof.md`
- `crates/runx-runtime/src/execution/runner/authority.rs`
- `crates/runx-runtime/src/execution/runner/steps.rs`
- `crates/runx-runtime/src/payment/packets.rs`
- `crates/runx-runtime/src/payment/state.rs`
- `crates/runx-runtime/src/payment/ledger.rs`
- `.scafld/specs/archive/2026-05/rust-payment-execution-boundary-v1.md`
- `.scafld/specs/archive/2026-05/x402-pay-idempotency-recovery-v1.md`
- `.scafld/specs/archive/2026-05/x402-pay-dogfood-v1.md`

## Harden Rounds

### round-1

Status: passed
Started: 2026-05-26T04:39:14Z
Ended: 2026-05-26T04:40:48Z

Checks:
- path audit
  - Grounded in: code:crates/runx-runtime/src/payment/supervisor.rs:20
  - Result: passed
  - Evidence: The implementation paths named by the draft exist and own the
- command audit
  - Grounded in: spec_gap:validation
  - Result: passed
  - Evidence: The validation commands are executable in this checkout and were
- scope/migration audit
  - Grounded in: code:crates/runx-runtime/src/payment/supervisor.rs:99
  - Result: passed
  - Evidence: The proof model stays runtime-internal and typed. State lookups
- acceptance timing audit
  - Grounded in: code:crates/runx-runtime/tests/payment/execution.rs:519
  - Result: passed
  - Evidence: Acceptance is checkable now because the implementation had
- rollback/repair audit
  - Grounded in: spec_gap:rollback
  - Result: passed
  - Evidence: The repair was limited to regression coverage plus one clippy
- design challenge
  - Grounded in: code:crates/runx-runtime/src/payment/supervisor.rs:56
  - Result: passed
  - Evidence: The design is the right trust-boundary fix rather than a rename:

Issues:
- none


## Review

Status: completed
Verdict: pass
Mode: verify
Provider: claude:claude-opus-4-7
Output: claude.mcp_submit_review
Summary: Human-reviewed override accepted: Claude review found no completion-blocking payment supervisor proof issues; only failure was workspace-mutation guard while concurrent graph/runtime edits were active. Focused payment tests, clippy, fmt, and diff checks passed.

Attack log:
- `review gate`: manual human audit -> clean (Claude review found no completion-blocking payment supervisor proof issues; only failure was workspace-mutation guard while concurrent graph/runtime edits were active. Focused payment tests, clippy, fmt, and diff checks passed.)

Findings:
- none
