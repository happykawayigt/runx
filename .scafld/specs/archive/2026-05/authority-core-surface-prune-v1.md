---
spec_version: '2.0'
task_id: authority-core-surface-prune-v1
created: '2026-05-21T00:57:07Z'
updated: '2026-05-21T01:53:50Z'
status: completed
harden_status: passed
size: small
risk_level: medium
---

# Authority core surface prune v1

## Current State

Status: completed
Current phase: final
Next: done
Reason: task completed
Blockers: none
Allowed follow-up command: `none`
Latest runner update: 2026-05-21T01:53:50Z
Review gate: pass

## Summary

Audit and narrow the public `runx_core::policy` and `runx_runtime` authority
exports. The runtime should depend on generic authority admission, while
payment-specific helper functions should either become private test support or
remain public only with a documented consumer.

## Scope And Touchpoints

In scope:

- `crates/runx-core/src/policy.rs`
- `crates/runx-core/src/policy/payment_authority.rs`
- `crates/runx-core/api-snapshot.txt`
- `crates/runx-runtime/src/lib.rs`
- Runtime/core tests that currently import payment-specific helpers

Out of scope:

- Changing runtime enforcement behavior.
- Changing serialized contracts.
- Removing payment authority algebra itself.

## Acceptance

Profile: strict

Validation:
- [x] `v1` audit - Production references are narrow.
  - Command: `bash -lc '! rg -n "authorize_payment_rail|PaymentRailAuthorization|PaymentRailAdmission|admit_payment_rail|payment_authority_spends|payment_authority_requires_receipt_before_success" crates/runx-runtime/src crates/runx-core/src/policy.rs -g "*.rs"'`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-12
- [x] `v2` test - Authority tests pass.
  - Command: `cargo test --manifest-path crates/Cargo.toml -p runx-core policy::payment_authority::tests -- --nocapture`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-13
- [x] `v3` test - Runtime payment execution tests pass.
  - Command: `cargo test --manifest-path crates/Cargo.toml -p runx-runtime --test payment_execution -- --nocapture`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-14
- [x] `v4` audit - Core public API snapshot is current.
  - Command: `node scripts/check-rust-kernel-parity.mjs --api-only`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-15

## Rollback

Strategy: per_phase

Commands:
- `git checkout HEAD -- crates/runx-core/src/policy.rs crates/runx-core/src/policy/payment_authority.rs crates/runx-core/api-snapshot.txt crates/runx-runtime/src/lib.rs crates/runx-runtime/tests/payment_authority.rs`

## Harden Rounds

### round-1

Status: passed
Started: 2026-05-21T01:39:49Z
Ended: 2026-05-21T01:41:05Z

Checks:
- path audit
  - Grounded in: code:crates/runx-core/src/policy.rs:20
  - Result: passed
  - Evidence: Scope covers the core policy re-export surface, the payment
- command audit
  - Grounded in: spec_gap:validation.v1
  - Result: passed
  - Evidence: The production-reference audit now checks public export/runtime
- scope/migration audit
  - Grounded in: code:crates/runx-runtime/src/execution/runner/authority.rs:3
  - Result: passed
  - Evidence: Production runtime already depends on generic
- acceptance timing audit
  - Grounded in: code:crates/runx-runtime/tests/payment_execution.rs:144
  - Result: passed
  - Evidence: Runtime payment execution tests continue to cover
- rollback/repair audit
  - Grounded in: spec_gap:rollback
  - Result: passed
  - Evidence: Rollback covers core policy exports, the payment authority module,
- design challenge
  - Grounded in: code:crates/runx-core/src/policy/payment_authority.rs:170
  - Result: passed
  - Evidence: The legacy `authorize_payment_rail` helper combines admission and
- Should payment-specific rail authorization remain a public runtime API?
  - Grounded in: code:crates/runx-runtime/src/lib.rs:94
  - Result: passed
  - Evidence: The runtime re-export currently exposes payment rail helpers that
- Where should tests for payment-specific algebra live after the public surface is narrowed?
  - Grounded in: code:crates/runx-runtime/tests/payment_authority.rs:8
  - Result: passed
  - Evidence: The current integration test imports through `runx_runtime`

Issues:
- none


## Planning Log

- 2026-05-21T00:57:07Z: Filed after runtime moved to generic
  `admit_step_authority` while compatibility exports remained public.

## Review

Status: completed
Verdict: pass
Mode: verify
Provider: claude:claude-opus-4-7
Output: claude.mcp_submit_review
Summary: Verify pass. The prior F1 blocker (stale crates/runx-core/api-snapshot.txt) is fully resolved: the regenerated snapshot no longer lists `admit_payment_rail`, `authorize_payment_rail`, `payment_authority_spends`, `payment_authority_requires_receipt_before_success`, the four `PaymentRail*` structs, or the `MissingRailProof` / `MissingReceiptBeforeSuccess` enum variants, and `PaymentSpendCapabilityBinding` is now recorded as the owned form with no `<'a>` lifetime and `String`/`u64`/owned `Reference` fields (api-snapshot.txt:238-246). The remaining public payment-authority surface (`is_payment_authority_subset`, `PaymentSpendCapabilityBinding`, `PaymentAuthorityError`, `StepAuthorityAdmission`, `StepAuthorityAdmissionDecision`, `admit_step_authority`, `authority_term_has_verb`) retains real consumers in `crates/runx-core/src/kernel_eval.rs`, `crates/runx-core/tests/policy_*`, and `crates/runx-runtime/src/execution/runner/authority.rs`. Task changes (5 paths) match the declared Scope And Touchpoints exactly; ambient drift is empty. The deleted runtime integration test's behavior is preserved by in-module tests in `payment_authority.rs` and end-to-end coverage in `crates/runx-runtime/tests/payment_execution.rs`. Acceptance v1–v4 evidence is consistent with the file state I inspected. No new release blockers found.

Attack log:
- `crates/runx-core/api-snapshot.txt`: Verify F1: re-grep snapshot for symbols the prune was supposed to remove (admit_payment_rail, authorize_payment_rail, payment_authority_spends, payment_authority_requires_receipt_before_success, PaymentRailAdmission*, PaymentRailAuthorization*, MissingRailProof, MissingReceiptBeforeSuccess) and for the obsolete borrowed PaymentSpendCapabilityBinding<'a> shape. -> clean (Zero matches for any removed symbol; PaymentSpendCapabilityBinding entry now owned (no lifetime, String/u64/Reference fields), consistent with payment_authority.rs:61-72.)
- `task_changes vs Scope And Touchpoints`: Scope/ambient drift: cross-check the 5 task_changes paths against the spec scope and confirm ambient_drift is empty. -> clean (All 5 paths (api-snapshot.txt, policy.rs, policy/payment_authority.rs, runtime/src/lib.rs, deleted runtime/tests/payment_authority.rs) match declared scope; ambient_drift list reports none.)
- `runx_core::policy public re-exports`: Regression hunt: confirm each still-exported payment symbol has a real consumer per spec summary's 'documented consumer' rule. -> clean (is_payment_authority_subset: kernel_eval.rs:310, tests/policy_fixtures.rs:393, tests/policy_proptest.rs. authority_term_has_verb / StepAuthorityAdmission / StepAuthorityAdmissionDecision / admit_step_authority / PaymentSpendCapabilityBinding: crates/runx-runtime/src/execution/runner/authority.rs:1-204. PaymentAuthorityError surfaces via the runtime's `source.to_string()` mapping in the same file (line 68).)
- `crates/runx-runtime/src/execution/runner/{steps.rs,authority.rs}`: Convention/regression: ensure the runtime authority enforcement path (admission + receipt-before-success) still wires through generic admit_step_authority and is invoked by steps.rs after the prune. -> clean (steps.rs:39 calls enforce_step_authority_admission and steps.rs:63 calls enforce_step_authority_receipt_before_success; authority.rs:48-69 invokes admit_step_authority with StepAuthorityAdmission and maps PaymentAuthorityError via to_string(), avoiding exhaustive matches that would care about cfg(test) variants.)
- `PaymentAuthorityError public enum + cfg(test) variants`: Dark pattern: check whether the test-only MissingRailProof / MissingReceiptBeforeSuccess variants leak into the published public API or break exhaustive matchers. -> clean (Both variants are gated `#[cfg(test)]` (payment_authority.rs:112-117) and absent from api-snapshot.txt (lines 66-75 list only the 9 production variants). The sole runtime consumer maps the error via `to_string()`, so no exhaustive match is affected.)
- `Deleted crates/runx-runtime/tests/payment_authority.rs`: Coverage regression: confirm the removed integration tests' behaviors are still exercised by the in-module unit tests and runtime end-to-end tests. -> clean (payment_authority.rs tests cover admit_subset, missing reservation/decision selection, missing subset proof, missing idempotency key, wildcard counterparty, binding mismatch, missing-receipt-before-success, sibling reuse of single-use capability. End-to-end runtime paths exist in tests/payment_execution.rs:236+ (`reserved_payment_authority` required, rail proof, scope-string detection).)
- `PaymentSpendCapabilityBinding borrowed→owned rewrite`: Convention/contract check: verify the borrowed→owned struct change preserves the serialized JSON contract and does not break call sites. -> clean (Struct keeps `#[serde(deny_unknown_fields)]` with String/u64/owned Reference fields; only Rust caller is runner/authority.rs which builds an owned binding and forwards by value into StepAuthorityAdmission (which still takes Option<PaymentSpendCapabilityBinding>). Serialized shape unchanged.)

Findings:
- [medium/non-blocking] `F1-stale-public-api-snapshot` crates/runx-core/api-snapshot.txt regenerated to match pruned payment-authority surface.
  - Location: `crates/runx-core/api-snapshot.txt:238`
  - Evidence: rg confirms api-snapshot.txt no longer contains PaymentRailAdmission|PaymentRailAuthorization|admit_payment_rail|authorize_payment_rail|payment_authority_spends|payment_authority_requires_receipt_before_success|MissingRailProof|MissingReceiptBeforeSuccess (0 matches). PaymentSpendCapabilityBinding entry at lines 238-246 now lists owned fields (act_id: String, amount_minor: u64, child_harness_ref: Reference, …) with no `<'a>` lifetime, matching the current struct in crates/runx-core/src/policy/payment_authority.rs:61-72. Acceptance v4 (`node scripts/check-rust-kernel-parity.mjs --api-only`) is recorded as exit 0.
  - Impact: Previously documented blocker is resolved; pnpm rust:check no longer rejects the snapshot as stale.
  - Validation: rg returned no matches for the removed symbol set inside the snapshot; v4 acceptance evidence is exit 0.

