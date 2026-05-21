---
spec_version: '2.0'
task_id: x402-pay-phase1-mock-scenario-fixtures-v1
created: '2026-05-21T00:46:25Z'
updated: '2026-05-21T02:43:15Z'
status: completed
harden_status: passed
size: medium
risk_level: medium
---

# x402-pay Phase 1 mock scenario fixtures v1

## Current State

Status: completed
Current phase: final
Next: done
Reason: task completed
Blockers: none
Allowed follow-up command: `none`
Latest runner update: 2026-05-21T02:43:15Z
Review gate: pass

## Summary

Turn the Phase 1 mock eventualities from `x402-pay-dogfood-v1` into explicit
fixture-backed tests. The completed dogfood gate proved the current broad
surface; this spec makes the mock happy path, governed refusal paths,
idempotency, receipt-before-success, and recovery expectations individually
observable.

## Scope And Touchpoints

In scope:

- `tests/x402-pay-dogfood-mock.test.ts`
- `fixtures/harness/payment-approval-graph.yaml`
- `fixtures/harness/oracle/payment-approval-graph.*.json`
- `fixtures/graphs/payment/approval-spend.yaml`
- `fixtures/skills/payment-fulfill/SKILL.md` only to seed deterministic mock
  rail material for receipt leak assertions
- `.scafld/specs/drafts/x402-pay-phase1-mock-scenario-punchlist.md`
- `packages/runtime-local/src/runner-local/index.ts` only to stamp typed
  payment rail proof refs from current mock rail artifacts into receipts
- `packages/runtime-local/src/runner-local/graph-governance.ts` only to pass
  those typed verification refs into the act/criterion receipt contract
- `scripts/dogfood-core-skills.mjs` only to include the new test in dogfood
- `tests/harness-cli.test.ts` and `tests/runtime-local-harness.test.ts` only
  for fixture wiring gaps discovered by the new scenarios
- `tools/outbox/build_pull_request/manifest.json` and
  `tools/thread/push_outbox/manifest.json` only to refresh stale source hashes
  that blocked the global `runx doctor` dogfood gate; tool source remains
  outside this spec.

Out of scope:

- Rust contracts/runtime changes unless a harden round explicitly reopens this.
- Stripe, MPP, x402 network rails, live money, refunds, reversals, disputes.
- New native `runx x402-pay`, `runx receipts`, or `runx ledger` commands.
- `paid-echo` or composer interception.

## Planned Phases

Phase 1: fixture matrix.
: Map P1.1 through P1.17 from `x402-pay-dogfood-v1` to runnable local fixture
cases, marking any unsupported scenario as a punch-list item rather than a
silent pass.

Phase 2: executable tests.
: Add focused tests that run the current harness/skill surfaces and assert
governed success, governed refusal, idempotency, and recovery outcomes without
manual orchestration.

Phase 3: dogfood integration.
: Add the new test to the dogfood proof if it is stable and deterministic.

## Acceptance

Profile: strict

Validation:
- [x] `v1` test - Mock scenario test passes.
  - Command: `cargo build --quiet --manifest-path crates/Cargo.toml -p runx-cli --bin runx && RUNX_KERNEL_EVAL_BIN=crates/target/debug/runx pnpm exec vitest run tests/x402-pay-dogfood-mock.test.ts`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-10
- [x] `v2` dogfood - Core dogfood remains green.
  - Command: `node scripts/dogfood-core-skills.mjs`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-11
- [x] `v3` test - Payment profile validation remains green.
  - Command: `pnpm exec vitest run tests/payment-skill-profile-validation.test.ts`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-12

## Rollback

Strategy: per_phase

Commands:
- `git checkout HEAD -- tests/x402-pay-dogfood-mock.test.ts .scafld/specs/drafts/x402-pay-phase1-mock-scenario-punchlist.md packages/runtime-local/src/runner-local/index.ts packages/runtime-local/src/runner-local/graph-governance.ts fixtures/harness/payment-approval-graph.yaml fixtures/harness/oracle fixtures/graphs/payment/approval-spend.yaml fixtures/skills/payment-fulfill/SKILL.md scripts/dogfood-core-skills.mjs tests/harness-cli.test.ts tests/runtime-local-harness.test.ts tools/outbox/build_pull_request/manifest.json tools/thread/push_outbox/manifest.json`

## Harden Rounds

### round-1

Status: passed
Started: 2026-05-21T01:56:56Z
Ended: 2026-05-21T01:58:01Z

Checks:
- path audit
  - Grounded in: code:fixtures/graphs/payment/approval-spend.yaml:1
  - Result: passed
  - Evidence: The current payment graph fixture exists at the declared path
- command audit
  - Grounded in: code:scripts/dogfood-core-skills.mjs:1
  - Result: passed
  - Evidence: The declared dogfood command exists and already runs the Rust
- scope/migration audit
  - Grounded in: archive:x402-pay-dogfood-v1
  - Result: passed
  - Evidence: The archived v1 dogfood spec explicitly deferred Stripe,
- acceptance timing audit
  - Grounded in: spec_gap:acceptance
  - Result: passed
  - Evidence: Acceptance now distinguishes passing executable fixture cases
- rollback/repair audit
  - Grounded in: spec_gap:rollback
  - Result: passed
  - Evidence: Rollback remains limited to the declared test, fixture, script,
- design challenge
  - Grounded in: code:fixtures/graphs/payment/approval-spend.yaml:1
  - Result: passed
  - Evidence: The current deterministic payment graph is a two-step approval

Issues:
- none


## Planning Log

- 2026-05-21T00:46:25Z: Filed after `x402-pay-dogfood-v1` completed broad
  mock dogfood proof and deferred finer-grained P1 fixture coverage.

## Review

Status: completed
Verdict: pass
Mode: verify
Provider: claude:claude-opus-4-7
Output: claude.mcp_submit_review
Summary: Verify mode confirms both prior findings are resolved and acceptance evidence is intact. F1 (manifest scope drift) was fixed by adding `tools/outbox/build_pull_request/manifest.json` and `tools/thread/push_outbox/manifest.json` to the spec's Scope And Touchpoints with an explicit rationale (source_hash refresh to clear `runx doctor`). F2 (vacuous `rail_session_material` assertion) was fixed by updating `fixtures/skills/payment-fulfill/SKILL.md` to emit `rail_session_material_ref:'rail-session-material:mock:payment-execution-001'` and tightening `tests/x402-pay-dogfood-mock.test.ts:119-130` to assert (a) the ledger contains the raw token and (b) the fulfill receipt does NOT contain `rail_session_material_ref` or the literal material ref — the assertion now exercises a real redaction path (key matches `material_ref` in `isSecretKey` and the runtime extracts only `proof_ref`+`idempotency_key` into verification_refs). Task changes since baseline are all inside the declared scope, acceptance v1/v2/v3 are pass, and the punch-list keeps the 12 uncovered P1.x ids open. No completion blockers or regressions found.

Attack log:
- `tests/x402-pay-dogfood-mock.test.ts:119-130 + fixtures/skills/payment-fulfill/SKILL.md`: Verify F2: confirm rail_session_material non-leakage assertion is no longer vacuous — fixture must emit the field, runtime must filter it, and test must enforce both observations. -> clean (SKILL.md now emits rail_session_material_ref inside payment_rail_packet.data.rail_proof. paymentRailProofVerificationRefs (index.ts:1173-1188) only extracts proof_ref + idempotency_key into the receipt's verification_refs, and isSecretKey (graph-governance.ts:1277-1283) matches the material_ref token so any direct stamping would be redacted. Test now asserts ledger contains the raw value AND receipt does not — non-vacuous redaction guard.)
- `Spec Scope And Touchpoints vs task_changes_since_baseline`: Verify F1: confirm tools/outbox/build_pull_request/manifest.json and tools/thread/push_outbox/manifest.json are no longer scope drift. -> clean (Spec lines 51-54 now list both manifests in Scope And Touchpoints with the source_hash rationale. Rollback command (line 107) also lists both paths. Workspace classifier confirms no remaining unexplained task_changes outside the declared scope.)
- `packages/runtime-local/src/runner-local/graph-governance.ts redaction filter`: Regression hunt: confirm isSecretKey still excludes material_ref_hash and that new in-scope edits to graph-governance.ts/index.ts do not weaken redaction or false-positive-redact the new proof_kind/locator fields. -> clean (Negative-match clause for material[_-]?ref[_-]?hash is intact (line 1279-1281). proof_kind, locator, uri, idempotency_key do not match the secret-key regex. ReferenceContract schema (spine.ts) supports proof_kind+locator without additionalProperties violations.)
- `Punch-list completeness vs covered scenarios (tests/x402-pay-dogfood-mock.test.ts:208-222)`: Regression hunt: confirm the 'missing' check still fails closed for all 17 P1.x ids and the punch-list rows still pin Open + Missing for the 12 uncovered scenarios. -> clean (punchlist .scafld/specs/drafts/x402-pay-phase1-mock-scenario-punchlist.md has all 12 expected rows (P1.2-4, P1.7-14, P1.17) each with 'Open' and 'Missing' tokens. Covered set (P1.1, P1.5, P1.6, P1.15, P1.16) plus punchlisted union covers 1..17.)
- `Acceptance evidence v1/v2/v3`: Verify acceptance commands remain valid and recorded results match the current workspace state. -> clean (All three acceptance criteria show status=pass with exit_code_zero evidence and source events entry-10/11/12. v1 runs the new vitest file, v2 runs dogfood-core-skills.mjs (which now includes the new test as step 'prove x402 mock payment fixtures'), v3 runs payment-skill-profile-validation.)

Findings:
- none

