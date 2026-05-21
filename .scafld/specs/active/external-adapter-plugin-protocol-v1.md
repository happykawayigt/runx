---
spec_version: '2.0'
task_id: external-adapter-plugin-protocol-v1
created: '2026-05-21T13:04:12Z'
updated: '2026-05-22T02:07:19+10:00'
status: active
harden_status: not_run
size: large
risk_level: high
---

# External execution adapter protocol v1

## Current State

Status: active
Current phase: Phase 3 SDK/conformance unblocked
Next: implement TypeScript protocol helpers and conformance adapters without
runtime-local/adapters imports
Reason: Phase 1 contract shape and Phase 2 Rust runtime supervision are landed.
`embedded-sdk-migration-story` now records that `runx-sdk` remains CLI-backed,
cloud hosted execution moves to a Rust-supervised runtime service/native
boundary, and custom adapter/provider glue moves to this protocol instead of
runtime-local.
Remaining work: Phase 3 implementation: helper SDKs, TypeScript sample adapter,
non-TypeScript sample adapter, conformance fixtures, and negative import guards.
Registry-backed manifest discovery, source-event ingress, hosted runtime
binding, catalog/read-model access, and thread/outbox provider writes remain
sibling specs or explicit non-goals.
Allowed follow-up command: `scafld handoff external-adapter-plugin-protocol-v1`
Latest runner update: 2026-05-22T02:07:19+10:00 consumed
`embedded-sdk-migration-story`'s target-shape decision. Phase 3 is no longer
blocked on SDK survivorship; it must build protocol-only helpers over generated
contracts and prove they do not import `@runxhq/runtime-local` or
`@runxhq/adapters`.
Previous update: 2026-05-22T01:54:08+10:00 confirmed
`external-adapter-runtime-wiring-v1` is completed and archived after a passing
review. The remaining work is Phase 3 helper SDK/conformance implementation.
Review gate: phase2_runtime_hardening_complete; phase3_ready_for_helper_sdk_conformance

## Summary

Define a language-neutral external execution-adapter process protocol for one
admitted run or step. Rust remains the supervisor: it admits the run, scopes
credentials, starts or connects to the adapter process, enforces
timeout/sandbox/redaction policy, validates returned contract shapes, and seals
receipts. The adapter process remains userland: it may be written in
TypeScript, JavaScript, Python, Rust, or another language, and may contain
execution-time integration-specific provider code.

This is not a replacement TypeScript runtime. It is an out-of-process
execution boundary owned by contracts and supervised by Rust.

## Context

Current built-in Rust adapters cover core execution families under
`oss/crates/runx-runtime/src/adapters/`: `cli_tool`, `mcp`, `agent`, `a2a`,
and `catalog`.

Current TypeScript adapters and cloud hosted adapters prove there are richer
execution-adapter needs than simple CLI tools:
- hosted/durable agent execution;
- custom adapter selection and replacement;
- host continuation and `needs_agent` flows;
- execution-time credential binding;
- execution-time provider-specific API glue.

Those needs should not force provider SDK code into Rust, but they also must
not keep an in-process TypeScript trusted runtime alive.

This spec can consume host, credential, catalog, and SDK surfaces during an
execution invocation, but it does not own those surfaces as general extension
protocols. Source-event ingress, hosted/embedded runtime binding, tool
catalog/read-model access, registry control, auth storage, webhook
verification, artifact-store ownership, and thread/outbox provider writes must
use sibling protocol specs or remain blockers for
`rust-ts-sunset-runtime-local`.

Credential material delivery is owned by
`credential-broker-delivery-contract-v1`. This protocol may reference admitted
credential refs and consume Rust-supervised delivery handles or process-env
delivery, but adapters must not invent arbitrary secret request/response
channels.

## Objectives

- Specify external execution-adapter discovery:
  manifest fields, supported source types, protocol version, command or
  endpoint, startup timeout, lifecycle, declared credential needs, and sandbox
  intent.
- Specify invocation frames:
  adapter identity, skill/source metadata, typed inputs, resolved inputs,
  scoped env, credential delivery references, cwd, receipt directory, run/step
  identifiers, host-resolution channel, and cancellation.
- Specify response frames:
  status, stdout/stderr or structured output, metadata, emitted artifacts,
  requested host resolutions, retry/failure semantics, and adapter-reported
  telemetry.
- Specify host interaction:
  approval/input/agent resolution requests must round-trip through
  `runx-contracts`/`@runxhq/contracts`; adapters do not invent private host
  result shapes.
- Provide TypeScript helper SDKs over the protocol while keeping the Rust
  runtime authoritative.
- Add conformance fixtures that can be implemented by at least TypeScript and
  one non-TypeScript sample adapter.

## Scope

In scope:
- External execution-adapter manifest and process protocol.
- Rust supervisor implementation plan for process lifecycle, validation,
  credential scoping, redaction, timeout, and receipt integration.
- TypeScript author SDK over generated contracts.
- Negative tests proving no runtime-local/adapters fallback is required.

Out of scope:
- Reimplementing built-in trusted adapters in TypeScript.
- Provider-specific integration packages.
- Replacing MCP as the preferred tool-integration protocol.
- Replacing the simpler `cli-tool` skill ABI owned by
  `skill-author-runtime-contract-v1`.
- Source-event ingress protocols for Slack, Sentry, GitHub, file, API, or
  webhook signal admission.
- Hosted/embedded runtime binding for cloud worker, agent-runner, SDK, host
  bridge, continuation, auth resolver, and resume semantics.
- Public tool-catalog/read-model search and inspect protocols.
- Thread/outbox provider protocols for comments, PR updates, or rendered story
  consumers.

## Dependencies

- `ts-extension-survivorship-boundary`
- `skill-author-runtime-contract-v1`
- `credential-broker-delivery-contract-v1`
- `canonical-json-fingerprint-contract-v1`
- `rust-contract-schema-validation-gate`
- `rust-ts-sunset-runtime-local`

## Touchpoints

- `oss/crates/runx-runtime/src/adapters/`
- `oss/crates/runx-runtime/src/adapter.rs`
- `oss/crates/runx-contracts/src/`
- `oss/packages/contracts/src/`
- `oss/packages/authoring/`
- `oss/packages/create-skill/`
- `oss/docs/ts-interop-boundary.md`
- `cloud/packages/agent-runner/src/`
- `cloud/packages/worker/src/`

## Risks

- A too-rich protocol can recreate runtime-local out of process.
- A too-small protocol can make custom hosted adapters impossible and push
  users into forking Rust.
- Treating this as the umbrella plugin protocol can mis-model non-execution
  queues and hide missing source-ingress, hosted-runtime, catalog, or outbox
  specs.
- Credential delivery and redaction must remain Rust-supervised; adapter
  helpers cannot become trusted secret stores.
- Streaming/continuation support must be explicit or custom adapters will be
  limited to one-shot happy paths.

## Acceptance

Profile: strict

Definition of done:
- [x] `dod1` Protocol v1 is documented with manifest, invocation, response,
  host-resolution, credential, timeout, and receipt semantics.
  - Phase 1 evidence: `packages/contracts/src/schemas/external-adapter.ts`,
    `schemas/external-adapter-*.schema.json`,
    `crates/runx-contracts/src/external_adapter.rs`, and
    `fixtures/contracts/external-adapter/*.json`.
  - Receipt boundary: adapter responses are observations only; Rust
    supervision converts accepted observations into sealed harness receipts.
- [x] `dod2` Rust runtime has a fail-closed adapter supervisor that validates
  every frame against `runx-contracts`.
  - Phase 2a evidence: `crates/runx-runtime/src/adapters/external_adapter.rs`
    and `crates/runx-runtime/tests/external_adapter.rs` provide an explicit
    feature-gated process-supervisor API and focused tests.
  - Phase 2b evidence: `external-adapter-runtime-wiring-v1` adds the
    feature-gated `ExternalAdapterSkillAdapter`, inline and package-relative
    manifest resolution, injectable manifest resolver/supervisor traits, graph
    routing coverage, credential delivery through
    `credential-broker-delivery-contract-v1`, observation redaction,
    host-resolution frame normalization/routing, and fail-closed tests. Startup
    readiness remains a non-goal for v1 because the frozen contract has no
    ready frame; the one-shot invocation deadline is enforced.
- [ ] `dod3` TypeScript helper SDK exists only as a protocol client/server
  helper and does not import runtime-local/adapters.
- [ ] `dod4` At least one TypeScript sample adapter and one non-TypeScript
  sample adapter pass the same conformance fixture.
- [ ] `dod5` Runtime-local/adapters sunset can point at this protocol for
  custom execution-adapter authoring without preserving the old packages.
- [ ] `dod6` Runtime-local/adapters sunset does not cite this protocol as the
  answer for source ingress, hosted runtime binding, catalog/read-model, or
  thread/outbox provider queues unless those behaviors are explicitly modeled by
  sibling specs.

Validation:
- [x] `v1` Scafld validates this spec.
  - Command: `scafld validate external-adapter-plugin-protocol-v1 --json`
  - Expected kind: `exit_code_zero`
  - Status: passed
  - Evidence: 2026-05-22T00:16:09+10:00 returned
    `{"ok":true,"command":"validate","result":{"task_id":"external-adapter-plugin-protocol-v1","path":"/Users/kam/dev/runx/runx/oss/.scafld/specs/active/external-adapter-plugin-protocol-v1.md","valid":true,"errors":null}}`.
- [x] `v2` Rust protocol tests pass.
  - Command:
    `cargo test --manifest-path crates/Cargo.toml -p runx-runtime --features external-adapter,cli-tool external_adapter`
  - Expected kind: `exit_code_zero`
  - Status: passed
  - Evidence: 2026-05-22T01:22:00+10:00 passed 16 focused
    `tests/external_adapter.rs` tests covering process launch, invocation
    frame serialization, response parsing, mismatched response identity,
    timeout-to-cancellation mapping, unexpected credential-request rejection,
    unknown protocol pre-spawn rejection, crashed-process failure,
    package-relative manifest path success/escape rejection, credential
    delivery/redaction, public credential refs/delivery-observation projection,
    host-resolution frame parsing, and graph-level host routing. The narrower command
    `cargo test --manifest-path crates/Cargo.toml -p runx-runtime --features external-adapter --test external_adapter`
    also passed 16 tests. Running the old no-feature filtered command still hits
    the pre-existing `cli_tool_contract.rs` integration-test discovery import,
    so this feature-gated slice records the explicit feature set.
- [ ] `v3` TypeScript helper tests pass.
  - Command: `pnpm test -- --run packages/authoring`
  - Expected kind: `exit_code_zero`
  - Status: pending
  - Evidence: unblocked by `embedded-sdk-migration-story` at
    2026-05-22T02:07:19+10:00; helper SDK work not implemented yet.
- [ ] `v4` No helper imports deleted runtime packages.
  - Command: `! rg -n "@runxhq/(runtime-local|adapters)|packages/(runtime-local|adapters)" packages/{authoring,create-skill,contracts,host-adapters,langchain} --glob '!**/dist/**'`
  - Expected kind: `exit_code_zero`
  - Status: pending
  - Evidence: unblocked by `embedded-sdk-migration-story`; run after helper
    SDK/conformance adapter implementation.
- [x] `v5` TypeScript protocol schema fixtures pass.
  - Command:
    `pnpm vitest run packages/contracts/src/schemas/external-adapter.test.ts`
  - Expected kind: `exit_code_zero`
  - Status: passed
  - Evidence: 2026-05-21T13:57:28Z re-run passed: 1 test file, 4 tests,
    including rejection of runtime-local `sealed` status and secret material in
    credential request frames.
- [x] `v6` Generated JSON Schemas are fresh.
  - Command: `pnpm contracts:schemas:check`
  - Expected kind: `exit_code_zero`
  - Status: passed
  - Evidence: 2026-05-21T13:57:28Z re-run exited 0 with
    `tsx scripts/generate-contract-schemas.ts --check`.
- [x] `v7` Rust contract fixture parity passes.
  - Command:
    `cargo test --manifest-path crates/Cargo.toml -p runx-contracts --test external_adapter_fixtures -- --nocapture`
  - Expected kind: `exit_code_zero`
  - Status: passed
  - Evidence: 2026-05-21T13:57:28Z re-run passed: 2 tests, including
    rejection of runtime-local `sealed` response status.
- [x] `v8` Contract fixtures validate against generated schemas.
  - Command:
    `cargo test --manifest-path crates/Cargo.toml -p runx-contracts --test schema_validation -- --nocapture`
  - Expected kind: `exit_code_zero`
  - Status: passed
  - Evidence: 2026-05-21T13:57:28Z re-run passed: 5 tests, including the
    mapped external-adapter fixture schema coverage.

## Phase 1: Protocol Shape

Goal: freeze the v1 external execution-adapter wire shape before deleting the
TS adapter package.

Status: completed
Dependencies: `ts-extension-survivorship-boundary`

Changes:
- [x] Define the external execution-adapter manifest schema.
- [x] Define invocation and response envelopes in `runx-contracts` and generated
  `@runxhq/contracts`.
- [x] Define host-resolution, credential request, and cancellation frames.
- [x] Define what extension code may and may not control.

Acceptance:
- The protocol is rich enough for hosted durable execution adapters and simple
  execution-time provider integrations, but not a second local runtime.
- Non-execution queues remain outside this protocol and must not be treated as
  solved by Phase 1 contract parity.
- Phase 1 deliberately does not implement the supervisor. The only executable
  boundary landed here is the language-neutral contract surface plus
  fixture-backed Rust/TypeScript parity.

## Phase 2: Rust Supervisor

Goal: run external execution adapters under Rust authority.

Status: complete for v1 one-shot process supervision and runtime routing
Dependencies: Phase 1
Blocker: helper SDK/conformance work remains in Phase 3. Startup readiness is
not part of v1 because the protocol has no ready frame.

Changes:
- [x] Add an external adapter supervisor behind an explicit runtime feature.
- [x] Wire the supervisor into runtime adapter selection for explicit inline
  manifests or injected resolvers.
- [x] Enforce per-invocation timeout, frame validation, credential delivery
  through `credential-broker-delivery-contract-v1`, redaction, host-resolution
  frame routing, and response metadata mapping into `SkillOutput`.
- [x] Fail closed on unknown protocol version, malformed frames, unexpected
  credential requests, and adapter crashes for the feature-gated one-shot
  process API.
- [x] Route host-resolution frames through the existing host resolution protocol
  before receipt construction. Accepted adapter response metadata maps into
  `SkillOutput` for normal receipt construction.

Acceptance:
- Rust remains the only trusted local execution and receipt authority.

### Phase 2a: Feature-Gated Process Supervisor

Status: complete
Dependencies: Phase 1

Changes:
- [x] Added `external-adapter` runtime feature gating for the new supervisor.
- [x] Added `ExternalAdapterProcessSupervisor::invoke(manifest, invocation)` as
  an explicit API, deliberately not wired into graph execution or adapter
  selection.
- [x] Process transport launches with env cleared and only string-valued scoped
  invocation env plus `RUNX_RECEIPT_DIR` admitted.
- [x] Invocation frames are serialized from `runx-contracts`; response frames
  are parsed back through `ExternalAdapterResponse` and checked for schema,
  protocol, adapter ID, and invocation ID.
- [x] Timeout creates a `runx.external_adapter.cancellation.v1` frame and
  terminates the adapter process group before failing closed.
- [x] Unknown protocol/schema, unsupported transport, empty command, non-string
  process env, credential-request frames on the response channel, malformed
  JSON, oversized responses, and crashed adapter processes fail closed.

Remaining v1 limits:
- Startup readiness has no separate ready frame in the frozen contract; the
  current slice validates non-zero startup timeout but only enforces the
  one-shot invocation deadline.
- Credential material delivery, redaction policy, host-resolution routing, and
  normal `SkillOutput` mapping are covered by Phase 2b and the completed
  `external-adapter-runtime-wiring-v1` slice. Helper SDKs and conformance
  adapters remain Phase 3 work.

### Phase 2b: Feature-Gated Runtime Wiring

Status: complete for the runtime-selection and hardening slice
Dependencies: Phase 2a

Changes:
- [x] Added `ExternalAdapterSkillAdapter` behind `features =
  ["external-adapter"]`.
- [x] Added explicit inline-manifest resolution from `SkillSource.raw` and
  package-relative `manifest_path` resolution that canonicalizes below the
  skill directory, plus injectable manifest resolver/supervisor traits for
  tests and future host wiring.
- [x] Built `ExternalAdapterInvocation` frames from `SkillInvocation` without
  provider-specific runtime logic.
- [x] Mapped accepted adapter observations to `SkillOutput` while keeping
  adapter responses as untrusted observations, not receipts.
- [x] Passed `CredentialDelivery` into the supervised process env after scoped
  env admission, and redacted stdout/stderr/output/metadata/errors/artifacts
  before runtime mapping.
- [x] Normalized host-resolution frames into response metadata and routed them
  through `Host::resolve` in graph execution.
- [x] Added graph/skill routing coverage proving `source_type:
  external-adapter` reaches the supervisor and fails closed when manifest
  identity or response identity is unsafe.

Evidence:
- `external-adapter-runtime-wiring-v1` validated at
  2026-05-22T01:22:00+10:00.
- `cargo test --manifest-path crates/Cargo.toml -p runx-runtime --features
  external-adapter --test external_adapter -- --nocapture` passed 16 focused
  tests at 2026-05-22T01:22:00+10:00.

## Phase 3: Author SDKs And Fixtures

Goal: keep adoption easy without keeping runtime-local.

Status: active
Dependencies: Phase 2
Decision: `embedded-sdk-migration-story` keeps `runx-sdk` CLI-backed for v1 and
routes custom adapter/provider glue here. Phase 3 helpers may live in
`@runxhq/authoring` or a future helper package named by this spec, but must be
protocol-only over generated contracts.

Changes:
- Add TypeScript helpers that implement the protocol server/client boilerplate
  using `@runxhq/contracts`.
- Add sample adapters and shared conformance fixtures.
- Add negative tests proving helpers do not import runtime-local/adapters.

Acceptance:
- A custom execution-adapter author can write TypeScript without depending on a
  TypeScript local runtime.
- At least one non-TypeScript sample adapter consumes the same fixture contract,
  proving this is not a TypeScript-only replacement runtime.
- Hosted-agent adapter migration can reference this protocol without pulling
  provider SDK code into Rust.

## Rollback

If the protocol cannot preserve required hosted/custom execution-adapter
behavior, keep the runtime-local/adapters sunset blocked and narrow the
protocol. Do not solve the gap by reviving a TypeScript trusted runtime or by
folding non-execution queues into the execution-adapter protocol.

## Review

Review must reject a protocol that requires execution-adapter authors to write
Rust or link into `runx-runtime`, and must also reject helper SDKs that execute
skills outside Rust supervision. Review must also reject any attempt to use this
protocol as the generic source-ingress, hosted-runtime, catalog, auth,
artifact-store, or thread/outbox provider protocol.

## Origin

User architecture review on 2026-05-21: runx must not drag integration code
into Rust, and Rust-only adapter authoring would harm adoption.
