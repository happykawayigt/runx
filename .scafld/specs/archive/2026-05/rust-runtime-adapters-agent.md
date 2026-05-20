---
spec_version: '2.0'
task_id: rust-runtime-adapters-agent
created: '2026-05-18T00:00:00Z'
updated: '2026-05-19T13:43:40Z'
status: completed
harden_status: passed
size: large
risk_level: high
---

# Rust runtime agent adapter

## Current State

Status: completed
Current phase: final
Next: done
Reason: fixture-backed Rust managed-agent adapter slice is implemented and
validated against current code. The landed runtime adapter builds the typed
`AgentActInvocation`, invokes an injected resolver, emits deterministic
metadata, sanitizes provider failures, supports `agent` and `agent-step`, and
replays through the Rust harness.
Blockers: none for the current fixture-backed runtime slice.
Allowed follow-up command: `none`
Latest runner update: 2026-05-19T13:43:40Z - verified current Rust/TS adapter
slice and fixture generators.
Review gate: pass

## Summary

Port the fixture-backed `agent` and `agent-step` adapter family to
`runx-runtime` behind the `features = ["agent"]` flag. The landed adapter
resolves a contained agent act inside the current governed harness node by
building the typed `AgentActInvocation`, passing it to an injected resolver,
and returning adapter output plus deterministic metadata to the harness.

The adapter does not sign receipts, own authority, create standalone act
records, or publish provider-specific contracts. It is one source-type adapter
inside a harness boundary. Live provider HTTP loops, runtime catalog tool
dispatch from model tool calls, and receipt-tree proof verification remain
follow-up work outside this completed fixture slice.

## Context

CWD: `.`

Packages:
- `@runxhq/adapters` agent subpath
- `@runxhq/runtime-local` sdk caller and host-protocol path
- `crates/runx-runtime`
- `crates/runx-contracts`
- `crates/runx-receipts`

Current TypeScript sources:
- `packages/adapters/src/agent/index.ts`
- `packages/adapters/src/agent/agent-act-invocation.ts`
- `packages/adapters/src/agent/anthropic.ts`
- `packages/adapters/src/agent/openai.ts`
- `packages/adapters/src/agent/runtime-tools.ts`
- `packages/runtime-local/src/sdk/caller.ts`
- `packages/runtime-local/src/sdk/host-protocol.ts`
- `../cloud/packages/agent-runner/src/hosted-agent-adapter.ts`
- `../cloud/packages/agent-runner/src/durable-step.ts`
- `../cloud/packages/agent-runner/src/anthropic.ts`

Files impacted:
- `crates/runx-runtime/src/adapters/agent.rs`
- `crates/runx-runtime/tests/agent_parity.rs`
- `fixtures/runtime/adapters/agent/**`
- `scripts/generate-agent-adapter-fixtures.ts`

Contract surfaces consumed:
- `runx-contracts::AgentActInvocation`
- `runx-contracts::ResolutionRequest`
- `runx-contracts::ResolutionResponse`

Invariants:
- Host protocol types come from `runx-contracts`; the adapter does not
  redeclare or loosen them.
- `agent` and `agent-step` remain source-type names. They are adapter source
  types, not central domain objects.
- Agent acts are contained harness payloads. They are provable only through
  the sealing harness receipt.
- The injected resolver receives only the contained `ResolutionRequest`
  produced from the current harness invocation. The current Rust adapter emits
  empty allowed-tool/context/provenance arrays until catalog-backed tool-loop
  dispatch is added by a follow-up spec.
- Declared runx tool calls are represented only as resolver telemetry in this
  slice. Runtime catalog dispatch and child harness receipt refs are deferred.
- Nested resolution pauses are not implemented in the Rust adapter slice; the
  resolver must return a final `ResolutionResponse` or sanitized failure.
- Provider behavior is injected. Unit and parity tests use deterministic
  resolver fixtures; no live provider hits are permitted.
- Hosted durable-step behavior remains owned by the cloud cutover. This spec
  may mirror the boundary in fixtures, but it does not replace hosted Durable
  Objects.
- No new schema aliases or alternate contract families are introduced.

## Objectives

- Port managed agent invocation for `agent` and `agent-step` sources.
- Build typed `AgentActInvocation` and consume typed `ResolutionResponse` at
  the Rust boundary.
- Generate deterministic TypeScript fixture oracles for plain success,
  structured agent-step success, and sanitized provider failure.
- Preserve metadata parity for native route, provider/model identity, status,
  and resolver-reported tool telemetry.
- Preserve harness replay coverage for a fixture-backed agent skill.
- Keep the adapter narrow: approvals, authority attenuation, receipt signing,
  and receipt tree verification remain harness/runtime responsibilities.

## Scope

In scope:
- Rust `agent` feature and adapter module.
- Injected managed-agent resolver boundary.
- Fixture generation from TypeScript adapter behavior.
- Local harness replay coverage for agent and agent-step cases.

Out of scope:
- Live provider network calls.
- Rust Anthropic/OpenAI HTTP transport implementations.
- Runtime-tool dispatch through the catalog adapter path.
- Nested resolution pause handling.
- Hosted agent-runner Durable Object replacement.
- New provider routing policy.
- Cloud API changes.
- Additional public CLI flags.
- Any second contract reader path.

## Dependencies

- `rust-runtime-skeleton`.
- `rust-approval-gate-parity`.
- `rust-tool-catalogs`.
- `runx-contract-spine-hard-cutover`.
- `rust-receipts-parity` completed against harness receipts.
- Receipt proof/tree APIs before a later spec can claim nested tool receipt
  proof for managed-agent tool calls.

## Sequencing Notes

- This slice does not consume runtime catalog dispatch directly; catalog-backed
  managed-agent tools remain deferred to a follow-up spec.
- Harness replay may use the agent fixture mode only after the adapter returns
  deterministic data and not-yet-supported provider modes fail closed.
- A hosted cutover may consume this module later, but this spec only proves the
  local runtime adapter and host-protocol boundary.

## Acceptance

Profile: strict

Validation:
- [x] `cmd_fixture_oracle` - Agent adapter fixtures are current.
  - Command: `pnpm tsx scripts/generate-agent-adapter-fixtures.ts --check`
  - Expected kind: `exit_code_zero`
- [x] `cmd_ts_agent_adapter` - Existing TypeScript agent adapter behavior still
  passes.
  - Command: `pnpm test -- packages/adapters/src/agent/index.test.ts packages/adapters/src/runtime.test.ts`
  - Expected kind: `exit_code_zero`
- [x] `cmd_runtime_agent` - Rust agent parity tests pass with deterministic
  resolver fixtures.
  - Command: `cargo test --manifest-path crates/Cargo.toml -p runx-runtime --features a2a,agent --test agent_parity`
  - Expected kind: `exit_code_zero`
- [x] `cmd_runtime_combined` - Rust A2A and agent focused parity tests pass
  together.
  - Command: `cargo test --manifest-path crates/Cargo.toml -p runx-runtime --features a2a,agent --test a2a_parity --test agent_parity`
  - Expected kind: `exit_code_zero`
- [x] `cmd_no_cutover_drift` - Touched Rust code and generated fixtures keep
  the post-cutover vocabulary and do not add schema aliases.
  - Command: `rg -n "schema ali[a]s(es)?|dual rea[d]er|alternate receipt fami[l]y|standalone act reco[r]d" crates/runx-runtime/src/adapters/agent.rs crates/runx-runtime/tests/agent_parity.rs fixtures/runtime/adapters/agent && exit 1 || exit 0`
  - Expected kind: `exit_code_zero`

Verification note:
- Running `cargo test -p runx-runtime --features a2a,agent --test a2a_parity --test agent_parity`
  from the OSS root fails because this repository has no root `Cargo.toml`.
  The equivalent command with `--manifest-path crates/Cargo.toml` passes.

Definition of done:
- [x] `dod1` `agent` and `agent-step` source types run through one Rust adapter
  family with separate source-type metadata and shared injected-resolver
  plumbing.
- [x] `dod2` The adapter builds typed `AgentActInvocation` requests and
  consumes typed `ResolutionResponse` values from an injected resolver.
- [x] `dod3` Resolver telemetry can preserve tool names, status, receipt ids,
  and resolution kinds without executing provider-local registries.
- [x] `dod4` Provider failure through the injected resolver produces
  deterministic sanitized adapter output.
- [x] `dod5` Harness replay runs the fixture-backed `agent` source through the
  Rust adapter.
- [x] `dod6` The adapter never signs receipts or emits standalone act proof.
  Harness sealing remains the only proof boundary.
- [x] `dod7` No live network key or provider token is required for tests.

Deferred follow-ups:
- Live Anthropic/OpenAI transports.
- Runtime catalog dispatch for managed-agent tool calls.
- Nested resolution pause semantics.
- Child harness receipt refs and receipt-tree proof verification for nested
  tool runs.
- Malformed provider-payload and cancellation parity beyond the injected
  resolver failure path.

## Phases

### Phase 1 - Fixture oracle

Goal: capture the current TypeScript boundary with deterministic provider
fixtures.

Tasks:
- Add `scripts/generate-agent-adapter-fixtures.ts`.
- Generate current oracle cases for agent plain success, agent-step structured
  success, and sanitized provider failure.
- Store canonical input, provider transcript, adapter output, and expected
  harness metadata under `fixtures/runtime/adapters/agent/**`.
- Normalize ids, durations, timestamps, and temp paths.

Exit criteria:
- Fixture generation is deterministic and `--check` fails on drift.

### Phase 2 - Contract boundary

Goal: make the Rust boundary typed before invoking resolver behavior.

Tasks:
- Add Rust request/response adapters that convert runtime source metadata into
  `AgentActInvocation`.
- Represent incoming fixture input through `runx-contracts` JSON types.
- Represent resolver final payloads through `ResolutionResponse`.
- Add a negative test for invalid source type.

Exit criteria:
- Boundary tests fail before the resolver is called when the adapter source
  type is invalid.

### Phase 3 - Injected resolver

Goal: support deterministic managed-agent resolution without live network
calls.

Tasks:
- Implement an injected resolver trait that receives the typed
  `ResolutionRequest`.
- Map final `ResolutionResponse` payloads to adapter stdout.
- Preserve resolver telemetry needed by harness metadata without leaking keys,
  absolute paths, or raw secrets.
- Sanitize resolver/provider failures before returning adapter stderr.

Exit criteria:
- The Rust adapter passes resolver success and failure fixtures with exact
  canonical output.

### Phase 4 - Deferred runtime tool dispatch

Goal: explicitly leave managed-agent tool calls for a later runtime catalog
dispatch slice.

Tasks:
- Keep `allowed_tools`, context, historical context, and provenance empty in
  the current generated envelope.
- Preserve resolver-reported tool telemetry as metadata only.
- Do not execute provider-local registries or nested runtime tools in this
  slice.

Exit criteria:
- Runtime tool dispatch is not claimed by this spec and is listed as a
  deferred follow-up.

### Phase 5 - Harness receipt integration

Goal: prove the adapter fits the harness spine.

Tasks:
- Run agent fixtures through harness replay mode.
- Assert contained decisions and acts remain inside the harness receipt.
- Assert adapter output and metadata return to the harness without special-case
  receipt logic.

Exit criteria:
- Adapter fixtures can be used by `rust-harness` without special-case
  receipt logic.

## Risks

- High: provider loops can hide boundary drift behind free-form model text.
  Mitigation: current Rust accepts only injected typed `ResolutionResponse`
  values and defers live provider HTTP loops.
- High: nested tools can accidentally bypass authority attenuation. Mitigation:
  nested tool dispatch is not implemented in this slice.
- Medium: provider APIs differ. Mitigation: keep provider-specific mapping out
  of the Rust runtime slice until live transports are specified.
- Medium: hosted durable-step semantics can be mistaken for local runtime
  scope. Mitigation: hosted replacement is out of scope and explicitly covered
  by cloud cutover specs.

## Rollback

Strategy: per_phase

Commands:
- Revert only the agent adapter files, generated fixtures, and fixture
  generator named in this spec.
- Re-run `pnpm tsx scripts/generate-agent-adapter-fixtures.ts --check` if the
  generator remains.
- Re-run `cargo test --manifest-path crates/Cargo.toml -p runx-runtime --features a2a,agent --test agent_parity` after rollback to confirm no partial adapter registration remains.

## Open Questions

- None. The resolver is injected; live HTTP is outside this spec.

## Harden Rounds

### round-1

Status: passed
Started: 2026-05-19T08:13:37Z
Ended: 2026-05-19T13:43:40Z

Checks:
- source audit
  - Result: passed
  - Evidence: The current TS adapter uses `AgentActInvocation` and
    provider-specific modules; the Rust slice consumes a typed injected
    resolver boundary and leaves catalog tool dispatch deferred.
- harness-spine audit
  - Result: passed
  - Evidence: The spec keeps proof on harness receipts and keeps acts contained
    in the harness.
- execution-readiness audit
  - Result: passed
  - Evidence: Open questions were closed for the fixture-backed runtime slice,
    deterministic fixture generation was added, and focused acceptance commands
    passed with the Rust workspace manifest path.
- current-code scope audit
  - Result: passed
  - Evidence: Current Rust code implements the injected resolver boundary,
    metadata, sanitization, and harness replay. Live provider transports,
    catalog tool dispatch, nested resolution pauses, and child receipt proof
    were moved to explicit deferred follow-ups instead of being claimed by this
    completed slice.

Issues:
- none
