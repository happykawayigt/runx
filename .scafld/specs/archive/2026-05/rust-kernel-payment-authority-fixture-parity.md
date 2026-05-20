---
spec_version: '2.0'
task_id: rust-kernel-payment-authority-fixture-parity
created: '2026-05-20T00:00:00Z'
updated: '2026-05-20T00:56:23Z'
status: completed
harden_status: not_run
size: small
risk_level: high
---

# Rust kernel payment-authority fixture parity

## Current State

Status: completed
Current phase: final
Next: done
Reason: task completed
Blockers: none
Allowed follow-up command: `none`
Latest runner update: 2026-05-20T00:56:23Z
Review gate: pass

## Summary

Add fixture parity for the existing pure payment-authority subset comparator.
The target operation is a kernel fixture input such as
`policy.isPaymentAuthoritySubset` with `child` and `parent` authority terms and
a boolean expected output.

This slice does not make Rust authoritative. TypeScript remains the oracle, and
fixtures remain the cross-language conformance surface.

## Context

Grounded current facts:
- `crates/runx-core/src/policy/payment_authority.rs` exports
  `is_payment_authority_subset`.
- `crates/runx-core/tests/policy_proptest.rs` covers payment-authority subset
  behavior directly in Rust.
- `crates/runx-contracts/src/authority.rs` defines `AuthorityTerm`,
  `PaymentAuthorityBounds`, payment verbs, payment resource family, and
  `PaymentSingleUseSpend`.
- `fixtures/kernel/README.md` says payment-authority subset logic is covered by
  Rust unit/proptest coverage today and fixture parity remains a separate
  executable slice.
- `fixtures/kernel/schema/policy.schema.json`,
  `scripts/generate-kernel-parity-fixtures.ts`, and
  `crates/runx-core/tests/policy_fixtures.rs` now expose
  `policy.isPaymentAuthoritySubset`.

## Scope

In scope:
- Add or expose a TypeScript oracle for the pure payment-authority subset
  decision.
- Add payment-authority fixture cases under `fixtures/kernel/policy/`.
- Extend the policy fixture schema and generator/check mode for the new input
  kind.
- Extend the Rust policy fixture runner to dispatch the new input kind to
  `is_payment_authority_subset`.
- Preserve existing Rust proptests and unit coverage.

Out of scope:
- Runtime payment execution.
- Rail providers, wallets, ledger projections, payment receipts, or adapters.
- Changing payment skill behavior.
- Making Rust policy runtime-authoritative.
- CI promotion from advisory to blocking.

## Fixture Cases

Minimum fixture coverage:
- allows a child with narrower amount bounds, same currency, subset rails,
  preserved required conditions, preserved approvals, and compatible expiry.
- allows reserve/quote behavior without single-use spend capability when the
  child does not request `spend`.
- denies currency widening.
- denies rail widening.
- denies dropping a required payment boolean such as `receipt_before_success`.
- denies omitting a parent-required realm, counterparty, operation, or period.
- denies `spend` without `PaymentSingleUseSpend` and
  `single_use_spend`/`credential_form` evidence.
- denies resource-family or resource-ref mismatch.

## Acceptance

Profile: strict

Validation:
- [x] `v1` command - new fixture kind is wired on both sides.
  - Command: `rg -n 'policy\.isPaymentAuthoritySubset' scripts/generate-kernel-parity-fixtures.ts fixtures/kernel/schema/policy.schema.json crates/runx-core/tests/policy_fixtures.rs`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-40
- [x] `v2` command - payment-authority fixtures exist.
  - Command: `test -f fixtures/kernel/policy/payment-authority-allows-narrower-child.json && test -f fixtures/kernel/policy/payment-authority-denies-currency-widening.json && test -f fixtures/kernel/policy/payment-authority-denies-single-use-spend-without-capability.json`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-41
- [x] `v3` command - fixture generator, validator, and key order are clean.
  - Command: `pnpm fixtures:kernel:check && pnpm fixtures:kernel:validate && pnpm fixtures:kernel:keys`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-42
- [x] `v4` command - Rust policy fixture and proptest coverage pass.
  - Command: `cargo test --manifest-path crates/Cargo.toml -p runx-core --test policy_fixtures && cargo test --manifest-path crates/Cargo.toml -p runx-core --test policy_proptest payment_authority`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-43
- [x] `v5` command - runtime-local adapters were not touched by this slice.
  - Command: `test -z "$(git diff --name-only -- packages/runtime-local packages/adapters)"`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-44

## Phase 1: Oracle And Fixture Generator

Status: completed
Dependencies: none

Objective: Complete this phase.

Changes:
- none

Acceptance:
- none

## Phase 2: Rust Fixture Runner

Status: completed
Dependencies: Phase 1

Objective: Complete this phase.

Changes:
- none

Acceptance:
- none

## Review

Status: completed
Verdict: pass
Mode: discover
Provider: claude:claude-opus-4-7
Output: claude.mcp_submit_review
Summary: Reviewed payment-authority fixture parity slice. The new TS oracle inside scripts/generate-kernel-parity-fixtures.ts mirrors crates/runx-core/src/policy/payment_authority.rs branch-for-branch (resource family/ref, verb/capability subset, conditions/approvals preservation, expiry, currency equality, minor-unit caps with usesMinorUnits gate, rails subset, optional-exact-or-narrower, required booleans, single_use_spend capability gate driven by Spend verb). All twelve required denial/allow cases listed in the spec's Fixture Cases are present and JSON shapes deserialize cleanly against AuthorityTerm/PaymentAuthorityBounds (deny_unknown_fields tolerated; reserved verbs/families snake_case). Schema oneOf for policy.isPaymentAuthoritySubset uses additionalProperties:false with required child/kind/parent and a unique kind const, so it does not collide with sibling branches. Rust fixture runner adds the IsPaymentAuthoritySubset arm and dispatches to is_payment_authority_subset; proptest payment_authority_comparison_is_deterministic and unit tests are preserved. Acceptance v1–v5 all recorded as passing. No findings.

Attack log:
- `fixtures/kernel/policy + scripts/generate-kernel-parity-fixtures.ts`: Spec Compliance: verify the 12 fixture cases enumerated in the spec exist and match the declared kind/expected shape -> clean (All allow/deny cases present (narrower child, reserve-without-single-use, currency/rail widening, dropped receipt_before_success, omitted realm/counterparty/operation/period, spend-without-capability, resource-family/-ref mismatch).)
- `scripts/generate-kernel-parity-fixtures.ts vs crates/runx-core/src/policy/payment_authority.rs`: Cross-language oracle parity: trace every branch of the TS isPaymentAuthoritySubset against the Rust function, including capability gate triggered only by Spend verb, optional_cap_subset asymmetry, expiry comparison, and required boolean subset -> clean (Every helper has a 1:1 counterpart with matching truth tables; capability gate uses Spend-only in both implementations.)
- `fixtures/kernel/schema/policy.schema.json + Rust deserialization`: Schema oneOf disambiguation and Rust deserialization: confirm new policy.isPaymentAuthoritySubset branch is unique, additionalProperties:false, and that fixture JSON deserializes into AuthorityTerm with deny_unknown_fields -> clean (kind const is unique across oneOf branches; fixture fields are all known AuthorityTerm/PaymentAuthorityBounds fields; verbs/families are valid snake_case enum variants.)
- `packages/runtime-local + packages/adapters scope guard`: Scope drift: ensure no runtime/adapter mutations leaked into a fixture-parity slice -> clean (Acceptance v5 recorded clean diff; spec Out of Scope reaffirms no runtime/adapter changes.)

Findings:
- none

## Metadata

Estimated effort hours: 6
Actual effort hours: none
AI model: none
React cycles: none

Tags:
- rust
- trusted-kernel
- payment-authority
- fixtures
- parity

## Origin

Source:
- split from obsolete `rust-kernel-port-orchestration` after observing that the
  Rust helper exists but fixture parity remains explicitly separate.

Repo:
- runxhq/runx

Git:
- none

Sync:
- none

Supersession:
- follows: rust-kernel-port-orchestration
- related: payment-authority-term-v1

## Harden Rounds

- none
