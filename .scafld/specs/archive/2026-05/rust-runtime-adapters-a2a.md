---
spec_version: '2.0'
task_id: rust-runtime-adapters-a2a
created: '2026-05-18T00:00:00Z'
updated: '2026-05-19T13:43:40Z'
status: completed
harden_status: passed
size: medium
risk_level: medium
---

# Rust runtime a2a adapter

## Current State

Status: completed
Current phase: final
Next: done
Reason: fixture-backed Rust A2A adapter slice is implemented and validated
against current code. The landed runtime adapter requires an explicit
transport, implements deterministic fixture transport, argument mapping,
polling, timeout/cancel handling, sanitized failures, metadata hashes, and
Rust harness replay.
Blockers: none for the current fixture-backed runtime slice.
Allowed follow-up command: `none`
Latest runner update: 2026-05-19T13:43:40Z - verified current Rust/TS adapter
slice and fixture generators.
Review gate: pass

## Summary

Port the fixture-backed `a2a` adapter family to `runx-runtime` behind the
`features = ["a2a"]` flag. A2A dispatch sends a contained act to another agent
surface through an explicit transport while the current harness preserves the
adapter boundary and seals the containing run.

The adapter owns message/task transport and task polling only. The harness owns
authority, decisions, contained acts, receipt signing, receipt tree
verification, and publication of proof. Parent/child harness receipt linkage
and live cross-network transport remain follow-up work outside this completed
fixture slice.

## Context

CWD: `.`

Packages:
- `@runxhq/adapters` a2a subpath
- `@runxhq/runtime-local` harness a2a fixture path
- `crates/runx-runtime`
- `crates/runx-contracts`
- `crates/runx-core`
- `crates/runx-receipts`

Current TypeScript sources:
- `packages/adapters/src/a2a/index.ts`
- `packages/adapters/src/a2a/index.test.ts`
- `packages/runtime-local/src/harness/a2a-fixture.ts`
- `packages/core/src/parser/index.ts`
- `packages/core/src/policy/index.ts`

Files impacted:
- `crates/runx-runtime/src/adapters/a2a.rs`
- `crates/runx-runtime/tests/a2a_parity.rs`
- `fixtures/runtime/adapters/a2a/**`
- `scripts/generate-a2a-adapter-fixtures.ts`

Contract surfaces consumed:
- `runx-contracts::JsonObject`
- `runx-contracts::JsonValue`
- `runx-parser::SkillSource`
- `runx-runtime::SkillInvocation`

Invariants:
- A2A never invents its own authority model. Any future child dispatch must
  pass through runtime admission and policy checks, and child authority must be
  a checked subset of the parent harness authority.
- The adapter requires an explicit transport. Fixture transport is allowed only
  in tests and harness replay.
- No live cross-network calls are permitted in acceptance tests.
- Task id derivation, polling, timeout, cancellation, and failure messages must
  match the TS fixture oracle after deterministic normalization.
- The adapter reports task metadata and hashes. It does not sign receipts,
  validate receipt trees, or claim parent/child receipt proof itself.
- Unsupported live target URLs fail closed through the fixture transport in
  deterministic tests.
- No new schema aliases or alternate contract families are introduced.

## Objectives

- Port A2A source parsing and dispatch to Rust.
- Implement deterministic fixture transport matching the TS harness fixture.
- Preserve success, failed, missing-metadata, timeout, and cancel failure
  behavior covered by current Rust and TS tests.
- Preserve argument-template mapping for raw and resolved inputs.
- Attach source task metadata needed by the sealing harness without leaking
  absolute paths, secrets, or raw provider credentials.
- Keep parent/child harness receipt linkage out of this slice until child
  dispatch exists in the runtime.

## Scope

In scope:
- `a2a` feature-gated runtime adapter.
- Explicit transport trait and fixture transport.
- Argument mapping and output serialization parity.
- Timeout, polling, and cancellation behavior.
- Harness replay coverage for the deterministic fixture transport.

Out of scope:
- New cross-org trust models.
- Hosted A2A service routing.
- Live network transport acceptance.
- Parent/child harness dispatch and receipt-tree proof.
- Registry acquisition flows.
- Cloud API changes.
- Any second contract reader path.

## Dependencies

- `rust-runtime-skeleton`.
- `rust-runtime-adapters-agent`.
- `runx-contract-spine-hard-cutover`.
- `rust-receipts-parity` completed against harness receipts.
- Receipt proof/tree APIs before a later spec can claim parent/child A2A
  receipt proof.

## Sequencing Notes

- A2A runs after the agent adapter so the runtime has a stable child-agent
  source boundary before cross-agent dispatch is added.
- A2A can land before MCP because its fixture transport is small and explicit.
- Live transport may be introduced by a later hosted or integration spec only
  after local fixture parity is complete.

## Acceptance

Profile: strict

Validation:
- [x] `cmd_fixture_oracle` - A2A adapter fixtures are current.
  - Command: `pnpm tsx scripts/generate-a2a-adapter-fixtures.ts --check`
  - Expected kind: `exit_code_zero`
- [x] `cmd_ts_a2a_adapter` - Existing TypeScript A2A adapter behavior still
  passes.
  - Command: `pnpm test -- packages/adapters/src/a2a/index.test.ts`
  - Expected kind: `exit_code_zero`
- [x] `cmd_runtime_a2a` - Rust A2A parity tests pass.
  - Command: `cargo test --manifest-path crates/Cargo.toml -p runx-runtime --features a2a,agent --test a2a_parity`
  - Expected kind: `exit_code_zero`
- [x] `cmd_runtime_combined` - Rust A2A and agent focused parity tests pass
  together.
  - Command: `cargo test --manifest-path crates/Cargo.toml -p runx-runtime --features a2a,agent --test a2a_parity --test agent_parity`
  - Expected kind: `exit_code_zero`
- [x] `cmd_no_cutover_drift` - Touched Rust code and generated fixtures keep
  the post-cutover vocabulary and do not add schema aliases.
  - Command: `rg -n "schema ali[a]s(es)?|dual rea[d]er|alternate receipt fami[l]y|standalone act reco[r]d" crates/runx-runtime/src/adapters/a2a.rs crates/runx-runtime/tests/a2a_parity.rs fixtures/runtime/adapters/a2a && exit 1 || exit 0`
  - Expected kind: `exit_code_zero`

Verification note:
- Running `cargo test -p runx-runtime --features a2a,agent --test a2a_parity --test agent_parity`
  from the OSS root fails because this repository has no root `Cargo.toml`.
  The equivalent command with `--manifest-path crates/Cargo.toml` passes.

Definition of done:
- [x] `dod1` A2A source metadata validation rejects missing agent card URL and
  missing task with stable diagnostics.
- [x] `dod2` The Rust fixture transport returns deterministic task ids
  and task output shape as the TS fixture transport.
- [x] `dod3` Argument-template mapping matches TS for exact template tokens,
  interpolated template tokens, missing values, and resolved inputs.
- [x] `dod4` Timeout paths attempt cancellation when available and preserve
  cancel failure metadata without hiding the original failure.
- [x] `dod5` Successful fixture dispatch records sanitized task metadata and
  hashes for the parent harness seal.
- [x] `dod6` Failed fixture dispatch closes the adapter invocation with a
  non-success status and does not publish proof as successful.
- [x] `dod7` No live network calls, provider tokens, or real agent-card URLs are
  required for tests.

Deferred follow-ups:
- Live A2A transport.
- Parent/child harness dispatch and receipt refs.
- Receipt-tree proof verification for A2A child runs.
- Unsupported non-fixture status values beyond the current fixture transport
  enum.
- Abort-path parity beyond the deterministic timeout/cancel path.

## Phases

### Phase 1 - Fixture oracle

Goal: capture current TypeScript A2A behavior in deterministic fixtures.

Tasks:
- Add `scripts/generate-a2a-adapter-fixtures.ts`.
- Generate current oracle cases for success, sanitized fixture failure,
  missing metadata, embedded template mapping, exact template mapping, resolved
  inputs, and unsupported non-fixture URL.
- Normalize ids, durations, timestamps, and temp paths.
- Store canonical inputs and adapter outputs under
  `fixtures/runtime/adapters/a2a/**`.

Exit criteria:
- Fixture generation is deterministic and `--check` fails on drift.

### Phase 2 - Rust transport trait

Goal: make transport explicit and testable.

Tasks:
- Define a narrow Rust transport trait for send, get, and cancel.
- Implement fixture transport with deterministic task id derivation.
- Reject construction without a transport.
- Add tests for send failure, timeout polling, and cancel failure paths.

Exit criteria:
- The adapter cannot accidentally perform live network work in tests.

### Phase 3 - Argument mapping and polling

Goal: match TS runtime behavior for inputs and task lifecycle.

Tasks:
- Port exact and interpolated template-token mapping.
- Preserve raw-input fallback behavior.
- Implement polling, timeout, and cancellation.
- Preserve sanitized error messages.

Exit criteria:
- Rust parity tests pass for all non-receipt A2A fixture cases.

### Phase 4 - Deferred child harness linkage

Goal: explicitly leave parent/child proof for a later child-dispatch slice.

Tasks:
- Report deterministic task metadata and hashes to the containing harness.
- Do not claim parent/child receipt refs until runtime child-harness creation
  exists.
- Keep receipt-tree verification out of this adapter slice.

Exit criteria:
- A2A fixtures prove transport parity, and child receipt proof is listed as a
  deferred follow-up.

### Phase 5 - Verification

Goal: leave the adapter ready for later hosted transport work.

Tasks:
- Run all acceptance commands.
- Document unsupported live transport behavior as an explicit diagnostic.
- Confirm no code outside this spec's declared paths is required.

Exit criteria:
- All validation commands pass and unsupported live paths fail closed through
  fixture transport diagnostics.

## Risks

- Medium: cross-agent dispatch can bypass the harness if implemented as a raw
  HTTP client. Mitigation: this slice requires an explicit transport and
  defers live transport plus child harness receipt refs.
- Medium: timeout and cancellation behavior can become flaky. Mitigation:
  deterministic fixture transport and normalized oracle fields.
- Medium: source metadata names differ across provider surfaces. Mitigation:
  parser parity and exact fixture assertions.

## Rollback

Strategy: per_phase

Commands:
- Revert only the A2A adapter files, generated fixtures, and fixture generator
  named in this spec.
- Re-run `pnpm tsx scripts/generate-a2a-adapter-fixtures.ts --check` if the
  generator remains.
- Re-run `cargo test --manifest-path crates/Cargo.toml -p runx-runtime --features a2a,agent --test a2a_parity` after rollback to confirm no partial adapter registration remains.

## Open Questions

- None for approval. Live transport is intentionally outside this spec.

## Harden Rounds

### round-1

Status: passed
Started: 2026-05-19T08:13:37Z
Ended: 2026-05-19T13:43:40Z

Checks:
- source audit
  - Result: passed
  - Evidence: The current TS adapter already requires an explicit transport and
    has deterministic fixture transport behavior.
- harness-spine audit
  - Result: passed
  - Evidence: The spec keeps authority and proof on harness receipt nodes and
    limits A2A to message/task transport.
- execution-readiness audit
  - Result: passed
  - Evidence: Open questions were closed for the fixture-backed runtime slice,
    fixture generation was added, and focused acceptance commands passed with
    the Rust workspace manifest path.
- current-code scope audit
  - Result: passed
  - Evidence: Current Rust code implements explicit transport, deterministic
    fixture transport, mapping, polling, timeout/cancel metadata, sanitization,
    metadata hashing, and harness replay. Live transport and parent/child
    receipt proof were moved to explicit deferred follow-ups instead of being
    claimed by this completed slice.

Issues:
- none
