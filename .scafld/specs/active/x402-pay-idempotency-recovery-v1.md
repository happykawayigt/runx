---
spec_version: '2.0'
task_id: x402-pay-idempotency-recovery-v1
created: '2026-05-21T00:00:00Z'
updated: '2026-05-21T00:00:00Z'
status: blocked
harden_status: not_run
size: medium
risk_level: high
---

# x402-pay idempotency recovery v1

## Current State

Status: blocked
Current phase: phase1
Next: state-layer
Reason: P1.7, P1.9, and P1.11 need observable persisted payment state before
fixture-backed replay/recovery assertions can be meaningful.
Blockers: missing persisted idempotency lookup, spend-capability consumption
ledger, and recoverable mock rail mutation record.
Allowed follow-up command: `scafld validate x402-pay-idempotency-recovery-v1`
Latest runner update: 2026-05-21T00:00:00Z
Review gate: not_started

## Summary

Turn the remaining x402 Phase 1 idempotency and recovery eventualities into an
executable scafld contract:

- P1.7: replaying the same idempotency key returns the first sealed receipt and
  does not execute a second mock spend.
- P1.9: reusing the same single-use spend capability is denied by core, not by
  the mock rail.
- P1.11: a crash or abort after a partial mock rail mutation is recoverable by
  idempotency key and either seals the existing mutation or escalates with a
  typed recovery state.

This spec intentionally stops short of adding runnable fixtures while the state
layer is not observable. A fixture that replays static graph inputs would only
prove duplicate YAML, not recovery semantics.

## Context

CWD: `.`

Packages:
- `crates/runx-core`
- `crates/runx-runtime`
- `fixtures/graphs/payment`
- `fixtures/harness`
- `fixtures/skills/payment-fulfill`

Files impacted:
- `.scafld/specs/active/x402-pay-idempotency-recovery-v1.md`
- Future, after the blocker is removed: `fixtures/harness/x402-pay-idempotency-*.yaml`
- Future, after the blocker is removed: `fixtures/graphs/payment/x402-pay-idempotency-*.yaml`
- Future, after the blocker is removed: `fixtures/skills/x402-pay-idempotency-*/SKILL.md`

Invariants:
- Do not depend on a native `runx x402-pay`, `runx receipts`, or `runx ledger`
  command; current observable surfaces are `runx harness`, receipt output, and
  `runx history`.
- Do not touch `.scafld/specs/drafts/rust-nitrosend-dogfood.md`.
- Do not edit `crates/runx-cli/tests/x402_native_dogfood.rs` or
  `tests/x402-pay-dogfood-mock.test.ts` unless coordination confirms no other
  x402 worker owns them.
- New fixtures, when unblocked, must live under clearly named
  `x402-pay-idempotency-*` paths.

Related docs:
- `.scafld/specs/archive/2026-05/x402-pay-dogfood-v1.md`
- `.scafld/specs/archive/2026-05/x402-pay-phase1-mock-scenario-fixtures-v1.md`

## Blocker Evidence

The current implementation has admission-time checks, but not a persisted state
surface that an idempotency/recovery fixture can replay against:

- `crates/runx-runtime/src/execution/runner/authority.rs:56` calls
  `admit_step_authority` with the current step inputs only. It passes
  `consumed_spend_capability_refs` from the fixture input at lines 63-66, but
  does not load consumed refs from a durable ledger.
- `crates/runx-runtime/src/execution/runner/authority.rs:155` parses
  `reserved_payment_authority.consumed_spend_capability_refs`; this proves the
  denial path is input-driven today.
- `fixtures/graphs/payment/approval-spend.yaml:134` hard-codes
  `consumed_spend_capability_refs: []`, and lines 135-139 hard-code one spend
  capability plus one idempotency key. Re-running the fixture starts from the
  same empty consumed set.
- `crates/runx-core/src/policy/payment_authority.rs:287` denies reuse only when
  the submitted `consumed_spend_capability_refs` already contains the spend
  capability. The unit test at
  `crates/runx-core/src/policy/payment_authority.rs:753` seeds that vector by
  hand.
- `fixtures/skills/payment-fulfill/run.sh:2` emits a static mock fulfillment
  packet with `proof_ref`, `idempotency_key`, and
  `rail_session_material_ref`; it does not persist a rail mutation record or a
  recoverable in-flight settlement state.
- `crates/runx-runtime/src/receipts/seal.rs:511` extracts a payment rail proof
  reference and uses `idempotency_key` as the reference locator at line 517, but
  that receipt reference is not an indexed idempotency recovery store.

Because of these gaps, the current harness can prove receipt-before-success and
single-run authority checks, but it cannot yet prove "same idempotency key
returns prior receipt without second spend" or "crash after partial rail mutation
is recovered from persisted state."

## Objectives

- Specify the executable fixture matrix for P1.7, P1.9, and P1.11.
- Make the state-layer blocker concrete enough that an implementer can unblock
  the fixtures without guessing.
- Keep all future fixture names under `x402-pay-idempotency-*`.

## Scope

- New scafld spec only for this turn.
- Future fixture paths may be added only after the state-layer acceptance below
  is satisfied.
- No shared x402 dogfood tests are edited by this spec.

## Dependencies

- Durable idempotency index keyed by rail family, counterparty or grant, and
  idempotency key.
- Durable spend-capability consumption record keyed by capability ref and
  linked to the sealing receipt or recovery state.
- Durable mock rail mutation record with at least: idempotency key, rail,
  amount, currency, counterparty, mutation status, proof ref when known, and
  recovery classification.

## Assumptions

- The mock rail remains deterministic and local.
- Recovery may return either a sealed receipt or a governed escalation, but it
  must not silently execute an additional spend.
- The first implementation can use file-backed state as long as it survives
  process restart within a harness run and is observable by tests.

## Touchpoints

- `.scafld/specs/active/x402-pay-idempotency-recovery-v1.md`
- Future: `fixtures/harness/x402-pay-idempotency-replay.yaml`
- Future: `fixtures/harness/x402-pay-idempotency-capability-reuse.yaml`
- Future: `fixtures/harness/x402-pay-idempotency-crash-recovery.yaml`
- Future: `fixtures/graphs/payment/x402-pay-idempotency-*.yaml`
- Future: runtime state layer files selected by the state-layer implementer

## Risks

- Description: Static fixtures could falsely pass by duplicating hard-coded
  inputs.
  Mitigation: require an observable persisted state delta and a no-second-spend
  assertion before adding fixtures.
- Description: Recovery could be confused with retry.
  Mitigation: require recovery classification from persisted rail state before a
  second rail execution is allowed.

## Acceptance

Profile: strict

Definition of done:
- [ ] `dod1` P1.7 fixture proves same idempotency key returns the original
  receipt and the mock rail execution count stays one.
- [ ] `dod2` P1.9 fixture proves the second use of a consumed spend capability
  is rejected by core from persisted state.
- [ ] `dod3` P1.11 fixture proves recovery by idempotency key from a partial
  mock rail mutation.
- [ ] `dod4` All new fixture paths are named `x402-pay-idempotency-*`.

Validation:
- [x] `v1` spec - This scafld spec validates.
  - Command: `scafld validate x402-pay-idempotency-recovery-v1`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0 on 2026-05-21
  - Source event: local
- [ ] `v2` state-layer - Durable payment state has executable coverage.
  - Command: `rg -n "idempotency.*lookup|consumed_spend_capability|rail.*mutation|payment.*recovery" crates/runx-runtime crates/runx-core fixtures/harness fixtures/graphs`
  - Expected kind: `exit_code_zero`
  - Status: blocked
  - Evidence: current matches are admission inputs and receipt refs only, not a
    durable replay/recovery state layer
  - Source event: none
- [ ] `v3` fixture - Idempotency replay fixture passes.
  - Command: `runx harness fixtures/harness/x402-pay-idempotency-replay.yaml`
  - Expected kind: `exit_code_zero`
  - Status: blocked
  - Evidence: fixture intentionally not added until v2 exists
  - Source event: none
- [ ] `v4` fixture - Spend capability reuse fixture passes.
  - Command: `runx harness fixtures/harness/x402-pay-idempotency-capability-reuse.yaml`
  - Expected kind: `exit_code_zero`
  - Status: blocked
  - Evidence: fixture intentionally not added until v2 exists
  - Source event: none
- [ ] `v5` fixture - Partial mutation recovery fixture passes.
  - Command: `runx harness fixtures/harness/x402-pay-idempotency-crash-recovery.yaml`
  - Expected kind: `exit_code_zero`
  - Status: blocked
  - Evidence: fixture intentionally not added until v2 exists
  - Source event: none

## Phase 1: State Layer Contract

Goal: expose the durable state needed for P1.7, P1.9, and P1.11.

Status: blocked
Dependencies: none

Changes:
- Runtime state layer (future, shared ownership) - persist idempotency replay
  entries, spend capability consumption, and mock rail mutation/recovery state.

Acceptance:
- [ ] `ac1_1` state - A sealed payment receipt can be looked up by
  idempotency key after process restart.
  - Command: `runx harness fixtures/harness/x402-pay-idempotency-replay.yaml`
  - Expected kind: `exit_code_zero`
  - Status: blocked
  - Evidence: no fixture until durable lookup exists
  - Source event: none
- [ ] `ac1_2` state - A consumed spend capability ref is rejected when reused
  without requiring the fixture to seed `consumed_spend_capability_refs`.
  - Command: `runx harness fixtures/harness/x402-pay-idempotency-capability-reuse.yaml`
  - Expected kind: `exit_code_zero`
  - Status: blocked
  - Evidence: current denial requires input-seeded consumed refs
  - Source event: none
- [ ] `ac1_3` state - A partial mock rail mutation is recoverable by
  idempotency key without issuing a second rail mutation.
  - Command: `runx harness fixtures/harness/x402-pay-idempotency-crash-recovery.yaml`
  - Expected kind: `exit_code_zero`
  - Status: blocked
  - Evidence: current mock rail fixture emits no persisted mutation state
  - Source event: none

## Phase 2: Executable Fixtures

Goal: add the three fixture-backed eventualities once Phase 1 is unblocked.

Status: pending
Dependencies:
- phase1

Changes:
- `fixtures/harness/x402-pay-idempotency-replay.yaml` (future, exclusive) -
  executes one payment, replays the same idempotency key, and asserts the first
  receipt is returned with one rail mutation.
- `fixtures/harness/x402-pay-idempotency-capability-reuse.yaml` (future,
  exclusive) - executes one payment, attempts a second spend with the same
  capability ref, and asserts a core denial before rail execution.
- `fixtures/harness/x402-pay-idempotency-crash-recovery.yaml` (future,
  exclusive) - simulates a crash after partial mock rail mutation, invokes
  recovery by idempotency key, and asserts seal-or-escalate classification.

Acceptance:
- [ ] `ac2_1` fixture - P1.7 has a runnable fixture.
  - Command: `test -f fixtures/harness/x402-pay-idempotency-replay.yaml`
  - Expected kind: `exit_code_zero`
  - Status: pending
  - Evidence: none
  - Source event: none
- [ ] `ac2_2` fixture - P1.9 has a runnable fixture.
  - Command: `test -f fixtures/harness/x402-pay-idempotency-capability-reuse.yaml`
  - Expected kind: `exit_code_zero`
  - Status: pending
  - Evidence: none
  - Source event: none
- [ ] `ac2_3` fixture - P1.11 has a runnable fixture.
  - Command: `test -f fixtures/harness/x402-pay-idempotency-crash-recovery.yaml`
  - Expected kind: `exit_code_zero`
  - Status: pending
  - Evidence: none
  - Source event: none

## Rollback

Strategy: per_phase

Commands:
- `rm .scafld/specs/active/x402-pay-idempotency-recovery-v1.md`

## Review

Status: not_started
Verdict: none
Timestamp: none
Review rounds: none
Reviewer mode: none
Reviewer session: none
Round status: none
Override applied: none
Override reason: none
Override confirmed at: none
Reviewed head: none
Reviewed dirty: none
Reviewed diff: none
Blocking count: none
Non-blocking count: none

Findings:
- none

Passes:
- none

## Self Eval

Status: complete
Completeness: blocker specified with path evidence and executable acceptance
Architecture fidelity: current harness and runtime state surfaces respected
Spec alignment: P1.7, P1.9, and P1.11 mapped directly
Validation depth: spec validation only, implementation intentionally blocked
Total: blocked pending state layer
Second pass performed: yes

Notes:
No implementation fixtures were added because the current state surface cannot
prove replay or crash recovery without false positives.

Improvements:
- Add fixtures after durable payment state is observable.

## Deviations

- Implementation is intentionally blocked rather than adding static fixtures.

## Metadata

Estimated effort hours: 4
Actual effort hours: 1
AI model: gpt-5-codex
React cycles: 0

Tags:
- x402
- payments
- idempotency
- recovery

## Origin

Source:
- Worker E OSS x402 idempotency/recovery spec lane

Repo:
- `/Users/kam/dev/runx/runx/oss`

Git:
- dirty worktree; unrelated `rust-nitrosend-dogfood.md` modification preserved

Sync:
- none

Supersession:
- none

## Harden Rounds

- none

## Planning Log

- 2026-05-21T00:00:00Z: Filed blocked spec for P1.7/P1.9/P1.11 after code
  evidence showed missing observable persisted payment state.
