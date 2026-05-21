---
spec_version: '2.0'
task_id: x402-pay-ledger-projection-v1
created: '2026-05-21T00:00:00Z'
updated: '2026-05-21T00:00:00Z'
status: blocked
harden_status: not_run
size: medium
risk_level: medium
---

# x402-pay ledger projection v1

## Current State

Status: blocked
Current phase: planning
Next: implement projection boundary
Reason: no payment-specific ledger projection API or persisted projection file
boundary exists yet
Blockers: missing projection surface
Allowed follow-up command: `implement`
Latest runner update: 2026-05-21T00:00:00Z
Review gate: not_run

## Summary

Add the P1.17 dogfood lane for x402 payment ledger projection evidence. The
projection must distinguish happy settlement accrual from governed refusal:

- happy settlement accrues exactly one bounded spend when a sealed rail proof is
  present;
- governed refusal records the refusal reason and zero accrued spend when
  authority, bounds, or proof validation blocks settlement.

This lane is evidence/projection only. It does not add live x402 rails, ledger
mutation commands, money movement, or public aliases.

## Current Surface Gap

The current native surface can read local run ledgers and produce journal/history
projections in Rust, while the old TypeScript `runtime-local` reflect projection
path remains a sunset surface. Payment ledger projection must therefore land in
`runx-runtime` and be exposed through the native CLI; adding new trusted
orchestration to `packages/runtime-local` would violate `docs/ts-interop-boundary.md`.
The existing ledger can record emitted artifacts, receipt links, step events,
and generic run history, but it does not expose a payment-specific projection
contract that folds x402 payment packets into an accrual/refusal view.

Missing boundary to add:

- Rust API: `runx_runtime::payment_ledger::build_payment_ledger_projection`,
  accepting sealed graph/harness receipts plus typed child receipt evidence.
- Native file output: a deterministic projection artifact under the configured
  receipt directory, keyed by `x402-pay:<receipt-id>`, without requiring
  TypeScript knowledge-store code.
- Ledger event: append a system run event `payment_ledger_projected` after the
  projection write, including the projection artifact id and source receipt id.
- Contract: `runx.payment_ledger_projection.v1` with stable JSON fields for
  `scenario_id`, `disposition`, `accrual`, `refusal`, `evidence_refs`, and
  `source_receipt_id`.

The implementation should live beside the existing Rust journal/history
projection boundary, not in core policy and not in `packages/runtime-local`.
Suggested target paths:

- `crates/runx-runtime/src/payment_ledger.rs`
- `crates/runx-runtime/tests/payment_ledger_projection.rs`
- `crates/runx-cli/src/history.rs` or the native harness completion path for
  projection artifact discovery, whichever is the narrower integration point
- `crates/runx-cli/tests/x402_native_dogfood.rs` for TS-free dogfood assertions
- optional generated schema path:
  `schemas/payment-ledger-projection.schema.json`

## Scope And Touchpoints

In scope:

- `.scafld/specs/active/x402-pay-ledger-projection-v1.md`
- `fixtures/ledger-projections/x402-pay-ledger-*.json`
- A future Rust runtime projection module and tests under
  `crates/runx-runtime/`
- Existing x402 harness fixtures as read-only inputs:
  `fixtures/harness/x402-pay-paid-echo.yaml`,
  `fixtures/harness/x402-pay-negative-cap-exceeded.yaml`,
  `fixtures/harness/x402-pay-negative-ambiguous-bounds.yaml`, and
  `fixtures/harness/x402-pay-negative-proofless-rail.yaml`

Out of scope:

- `.scafld/specs/drafts/rust-nitrosend-dogfood.md`
- Shared x402 test rewrites unrelated to P1.17.
- Live x402, Stripe, MPP, refunds, disputes, or real money rails.
- New public `runx ledger`, `runx x402-pay`, `x402-charge`, or `x402-refund`
  commands.
- Changing the existing ledger chain schema.

## Projection Contract

The v1 projection is a derived evidence document:

- `schema_version`: `runx.payment_ledger_projection.v1`
- `payment_profile`: `x402-pay`
- `source_receipt_id`: graph receipt id used for the projection
- `disposition`: `settled` or `refused`
- `scenario_id`: P1 scenario id when known
- `accrual`: settled amount, currency, rail, idempotency key, and proof refs;
  zero amount and empty proof refs for refusal
- `refusal`: null for settled, otherwise reason code, refused stage, and
  `ledger_spend_recorded: false`
- `evidence_refs`: receipt, harness, and artifact refs sufficient to rerun or
  audit the projection

Required P1.17 distinctions:

- P1.1/P1.5 happy settlement: `disposition: settled`,
  `accrual.amount_minor > 0`, `rail_proof_refs` non-empty, and refusal null.
- P1.3 cap exceeded: `disposition: refused`, `reason_code: cap_exceeded`,
  `accrual.amount_minor: 0`, and no rail proof refs.
- P1.4 ambiguous bounds: `disposition: refused`,
  `reason_code: ambiguous_bounds`, `accrual.amount_minor: 0`, and no rail call.
- P1.12 proofless rail: `disposition: refused`,
  `reason_code: missing_rail_proof`, `accrual.amount_minor: 0`, and no paid echo.

## Planned Phases

Phase 1: projection boundary.
: Add the Rust runtime projection builder, contract type, and unit tests that
fold existing x402 ledger artifact envelopes into the v1 projection.

Phase 2: persistence boundary.
: Persist the native projection artifact under the configured receipt directory,
then append `payment_ledger_projected` to the ledger.

Phase 3: dogfood evidence.
: Run the happy and refusal fixtures, assert projection differences against the
`fixtures/ledger-projections/x402-pay-ledger-*.json` examples, and update the
Phase 1 punch-list only after the executable projection evidence exists.

## Acceptance

Profile: strict

Validation:
- [x] `v1` scafld - Spec validates.
  - Command: `scafld validate x402-pay-ledger-projection-v1 --json`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
- [x] `v2` fixture shape - Golden projection examples are valid JSON.
  - Command: `node -e "const fs=require('node:fs'); for (const f of fs.readdirSync('fixtures/ledger-projections').filter(f => f.startsWith('x402-pay-ledger-') && f.endsWith('.json'))) JSON.parse(fs.readFileSync('fixtures/ledger-projections/'+f,'utf8'))"`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
- [ ] `v3` projection tests - Rust runtime projection distinguishes settlement from
  governed refusal.
  - Command: `cargo test --manifest-path crates/Cargo.toml -p runx-runtime --test payment_ledger_projection`
  - Expected kind: `exit_code_zero`
- [ ] `v4` x402 dogfood - Native x402 fixture lane remains green.
  - Command: `cargo test --manifest-path crates/Cargo.toml -p runx-cli --test x402_native_dogfood native_x402_negative_fixtures_refuse_without_settlement`
  - Expected kind: `exit_code_zero`
- [ ] `v5` projection persistence - Native CLI harness run writes the
  payment-ledger projection artifact and ledger event without TypeScript.
  - Command: `cargo test --manifest-path crates/Cargo.toml -p runx-cli --test x402_native_dogfood native_x402_ledger_projection`
  - Expected kind: `exit_code_zero`

## Rollback

Strategy: per_phase

Commands:
- `rm -f .scafld/specs/active/x402-pay-ledger-projection-v1.md`
- `rm -rf fixtures/ledger-projections`
- Future implementation rollback:
  `git checkout HEAD -- crates/runx-runtime/src/payment_ledger.rs crates/runx-runtime/tests/payment_ledger_projection.rs crates/runx-cli/src/history.rs crates/runx-cli/tests/x402_native_dogfood.rs schemas/payment-ledger-projection.schema.json`

## Harden Rounds

- none

## Planning Log

- 2026-05-21T00:00:00Z: Filed after P1.3, P1.4, and P1.12 refusal fixtures
  existed. The current repo has no Rust-native payment-specific ledger
  projection API, so this active blocked spec records the missing boundary and
  executable acceptance commands.

## Review

Status: not_run
Verdict: pending
Mode: discover
Findings:
- none
