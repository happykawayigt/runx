---
spec_version: '2.0'
task_id: credential-broker-delivery-contract-v1
created: '2026-05-22T00:28:36+10:00'
updated: '2026-05-21T15:19:28Z'
status: review
harden_status: passed
size: large
risk_level: high
---

# Credential broker and delivery contract v1

## Current State

Status: review
Current phase: final
Next: complete
Reason: review gate pass: 2 finding(s), 0 completion blocker(s)
Blockers: none
Allowed follow-up command: `scafld complete credential-broker-delivery-contract-v1`
Latest runner update: 2026-05-21T15:20:33Z
Review gate: pass

## Summary

Define the shared credential broker and delivery contract used by Rust-supervised
execution lanes. The contract starts after policy admission and authority proof:
Rust decides whether a grant may bind to a run, resolves the opaque
`material_ref` through a trusted broker/resolver, maps the material to a
declared delivery profile, and gives the child or provider adapter only the
scoped secret material it is allowed to receive.

This is not a general auth plugin protocol and not a secret store. It is the
runtime handoff primitive between admitted authority and a supervised side-effect
boundary. Raw secret material must never appear in manifests, invocation frames,
authority proofs, receipts, logs, captured output, public projections, or
adapter observations.

The completed `rust-adapter-credential-delivery` spec proved the first local
mechanics: `CredentialDelivery`, `MaterialResolver`, provider env profiles,
`SecretEnv`, and redaction for built-in `cli-tool` and MCP paths. This spec
turns that implementation idea into a stable contract that can be reused by:

- skill author subprocess execution through the existing `CredentialDelivery`
  channel;
- external execution adapters under `external-adapter-plugin-protocol-v1`;
- thread/outbox provider adapters that need provider tokens for comments, PR
  updates, or publication;
- hosted/cloud runtimes that broker secrets outside the local process.

## Context

Existing implemented Rust pieces:
- `crates/runx-runtime/src/credentials.rs` defines `CredentialDeliveryProfile`,
  `MaterialResolver`, `ResolvedCredentialMaterial`, `SecretEnv`, and
  `CredentialDelivery`.
- `crates/runx-runtime/src/adapters/cli_tool.rs` injects
  `CredentialDelivery.secret_env()` only at child process spawn and redacts
  captured stdout/stderr.
- `crates/runx-runtime/src/adapters/mcp/**` passes `SecretEnv` to MCP process
  spawn and redacts tool results.

Existing contract surfaces:
- `packages/contracts/src/schemas/credentials.ts` owns credential envelope and
  authority-proof shapes without raw material.
- `crates/runx-contracts/src/external_adapter.rs` has credential references and
  credential request frames, but no host-to-adapter delivery frame or delivery
  mode.
- `docs/security-authority-proof.md` bans raw tokens and records only
  `material_ref` hashes in public proof.

## Objectives

- Define a stable credential-delivery frame/envelope family in
  `runx-contracts` and `@runxhq/contracts`.
- Define delivery modes for v1. The default should be process environment
  injection because Rust already implements that safely for built-in adapters.
  File/socket/helper-process delivery remains future work unless a v1 consumer
  proves it is required.
- Define a broker response that carries only delivery handles or scoped secret
  material over trusted host-to-supervisor channels. Adapters must not request
  arbitrary credentials at runtime.
- Define delivery profiles: provider, auth mode, purpose, material roles, target
  env/file names, required/optional semantics, and redaction hints.
- Define redaction and non-leakage rules shared by cli-tool, MCP, external
  execution adapters, and outbox/provider adapters.
- Define receipt/proof observations: material is omitted; receipts may record
  credential refs, grant refs, provider, purpose, profile id, delivery mode, and
  material ref hash only.

## Scope

In scope:
- Contract schemas and Rust/TypeScript types for credential delivery requests,
  broker responses, delivery profiles, and redaction policy.
- Runtime mapping from admitted credential envelope + binding decision +
  material resolver to a delivery object.
- External execution-adapter host-to-process delivery semantics.
- Outbox/provider adapter delivery requirements where provider tokens are needed
  for side effects.
- Tests proving secrets do not enter receipts, authority proofs, stdout/stderr,
  metadata, or external adapter response observations.

Out of scope:
- Secret storage implementation, OAuth handshakes, hosted grant lifecycle, and
  BYO credential verification; owned by `byo-credential-foundations` and cloud
  auth specs.
- Provider-specific SDK packages.
- Source-event ingress protocol design.
- Replacing the existing authority proof or credential envelope shape.
- General-purpose auth resolver plugins.

## Dependencies

- `rust-adapter-credential-delivery` archived completed; provides the current
  Rust local implementation mechanics and non-leakage tests.
- `byo-credential-foundations`; owns storage, verification, and hosted material
  availability.
- `external-adapter-plugin-protocol-v1`; consumes this contract for external
  execution-adapter delivery and must not invent a separate credential channel.
- `skill-author-runtime-contract-v1`; the author-facing subprocess ABI must stay
  compatible with the same delivery primitive.
- `github-outbox-receipts`; outbox/provider side effects must use this primitive
  or a named blocker if provider credentials are required.
- `security-authority-proof.md`; public proof remains metadata-only and
  secret-free.

## Touchpoints

- `crates/runx-contracts/src/`
- `packages/contracts/src/schemas/`
- `schemas/credential-*.schema.json`
- `fixtures/contracts/credential-delivery/`
- `crates/runx-runtime/src/credentials.rs`
- `crates/runx-runtime/src/adapters/cli_tool.rs`
- `crates/runx-runtime/src/adapters/mcp/`
- `crates/runx-runtime/src/adapters/external_adapter.rs`
- `docs/security-authority-proof.md`
- `oss/.scafld/specs/active/external-adapter-plugin-protocol-v1.md`

## Risks

- If delivery is left per-protocol, cli-tool, external adapters, hosted runtimes,
  and outbox providers will each grow incompatible secret channels.
- If the contract is too rich, it becomes an auth plugin system or secret store.
- If v1 allows HTTP delivery without auth, retry, and idempotency semantics, it
  can leak secrets or double-use scoped credentials.
- If redaction is exact-string only, transformed or boundary-split secrets can
  leak through captured output. The contract must specify the minimum v1
  guarantee and the limits of that guarantee honestly.

## Acceptance

Profile: strict

Definition of done:
- [x] `dod1` Contract schemas exist for delivery profile, delivery request,
  broker response, and runtime delivery observation.
- [x] `dod2` Rust and TypeScript contract fixtures cross-validate and reject raw
  secret material in public frames.
- [x] `dod3` Runtime delivery uses the contract for cli-tool/MCP without
  weakening the existing `CredentialDelivery` secret channel.
- [x] `dod4` External execution-adapter Phase 2 wiring consumes this contract
  instead of accepting arbitrary credential-request frames from the adapter.
- [x] `dod5` Outbox/provider adapter specs either consume this contract or mark
  provider credentials as an explicit blocker.
- [x] `dod6` Redaction tests cover stdout, stderr, metadata, response
  observations, receipt metadata, and truncation boundaries.

Validation:
- [ ] `v1` Scafld validates this spec.
  - Command: `scafld validate credential-broker-delivery-contract-v1 --json`
  - Expected kind: `exit_code_zero`
  - Status: passed
  - Evidence: 2026-05-22T01:18:00+10:00 returned
    `{"ok":true,"command":"validate","result":{"task_id":"credential-broker-delivery-contract-v1","path":"/Users/kam/dev/runx/runx/oss/.scafld/specs/active/credential-broker-delivery-contract-v1.md","valid":true,"errors":null}}`.
- [ ] `v2` Contract schema generation is fresh.
  - Command: `pnpm contracts:schemas:check`
  - Expected kind: `exit_code_zero`
  - Status: passed
  - Evidence: 2026-05-22T00:43:33+10:00 exited 0 after generating the four
    credential-delivery schemas.
- [ ] `v3` Rust contract fixtures pass.
  - Command:
    `cargo test --manifest-path crates/Cargo.toml -p runx-contracts credential_delivery`
  - Expected kind: `exit_code_zero`
  - Status: passed
  - Evidence: 2026-05-22T00:43:54+10:00 focused fixture command passed
    `credential_delivery_fixtures` and `schema_validation`.
- [ ] `v4` Runtime credential delivery tests pass.
  - Command:
    `cargo test --manifest-path crates/Cargo.toml -p runx-runtime --test credential_delivery --features cli-tool,mcp -- --nocapture`
  - Expected kind: `exit_code_zero`
  - Status: passed
  - Evidence: 2026-05-22T00:52:00+10:00 passed 9 focused tests, including
    contract-profile mapping, unsupported role rejection, empty material
    rejection, redact-before-truncate behavior, and MCP real process transport
    delivery/redaction.
- [x] `v5` External adapter supervisor tests prove delivery or fail closed.
  - Command:
    `cargo test --manifest-path crates/Cargo.toml -p runx-runtime --features external-adapter,cli-tool external_adapter`
  - Expected kind: `exit_code_zero`
  - Status: passed
  - Evidence: 2026-05-22T01:19:54+10:00 passed the full filtered runtime
    command; the focused external-adapter file also passed 16 tests, including
    `external_adapter_process_supervisor_delivers_credentials_and_redacts_observations`,
    `external_adapter_skill_adapter_projects_public_credential_refs_and_observation`,
    `external_adapter_process_supervisor_rejects_unexpected_credential_request`,
    and graph-level host-resolution routing. The adapter still rejects
    adapter-originated credential request frames; raw material is delivered only
    through the private `CredentialDelivery` env channel.

## Phase 1: Contract Shape

Status: completed
Dependencies: rust-adapter-credential-delivery

Objective: Complete this phase.

Changes:
- [x] Add contract types for:
- [x] Declare v1 process-env delivery mode and reserve future modes explicitly.
- [x] Declare that public frames carry refs, hashes, provider, purpose, delivery mode, and profile ids only. Raw secret material is private to the trusted broker/supervisor channel.

Acceptance:
- none

## Phase 2: Runtime Adoption

Status: completed
Dependencies: Phase 1

Objective: Complete this phase.

Changes:
- [x] Map `CredentialDeliveryProfile` and `CredentialDelivery` to/from the contract where appropriate without adding serialization to secret-bearing Rust types.
- [x] Reject empty material values before env injection.
- [x] Redact before truncation for cli-tool process output.
- [x] Add an MCP real-spawn credential delivery integration test, not only fixture transport coverage.

Acceptance:
- none

## Phase 3: External Adapter And Outbox Consumption

Status: completed
Dependencies: Phase 2, `external-adapter-plugin-protocol-v1`

Objective: Complete this phase.

Changes:
- [x] Replace or narrow external adapter credential-request handling so adapters receive only host-delivered credential refs/handles/material through the approved delivery mode.
- [x] External adapter supervisor injects process-env delivery after audited scoped env and redacts stdout/stderr/response observations.
- [x] Outbox/provider adapter specs declare whether provider credentials are delivered through this primitive or are not in scope.
- `thread-outbox-provider-protocol-v1` now owns the provider outbox lane, blocks provider mutations until `CredentialDelivery` is consumed, and keeps the current local file-thread helper credential-free.

Acceptance:
- none

## Rollback

If the shared contract cannot cover a consumer, keep that consumer blocked or
give it a named sibling credential-delivery extension. Do not smuggle raw secret
material through existing invocation metadata, adapter env fields, receipts, or
provider-specific JSON blobs.

## Review

Status: completed
Verdict: pass
Mode: discover
Provider: claude:claude-opus-4-7
Output: claude.mcp_submit_review
Summary: Reviewed the credential broker and delivery contract v1. Acceptance criteria are met: contract schemas exist (Rust + TS + JSON-schema + fixtures), runtime delivery uses the contract for cli-tool/MCP/external adapter via `CredentialDelivery`, external supervisor rejects adapter-initiated credential requests and process-env-injects + redacts observations, and the `thread-outbox-provider-protocol-v1` spec is created and explicitly blocks provider mutations on this primitive. Found two low-severity, non-blocking findings: (1) `env_bindings[*].required` is required by the contract schema but the Rust runtime always treats every binding as required, producing a contract-vs-runtime semantic gap that will trip future refresh-token-style profiles; (2) cli-tool `capture_stream` truncates raw bytes at 1MB before `redact_text` runs, so a secret straddling the 1MB capture boundary can leak its prefix into `SkillOutput.stdout/stderr` — the spec acknowledges this redaction limit as a known v1 risk, but no doc states the limit honestly as required by the spec's Review/Risks language. No completion blockers; ambient drift (file-thread, docs, runner steps, external_adapter integration tests) is task-adjacent context, not findings.

Attack log:
- `spec acceptance items dod1-dod6`: verify Rust + TS + JSON-schema + fixtures cover profile/request/broker-response/observation and confirm runtime adopts them for cli-tool/MCP/external adapter; verify outbox/provider spec exists -> clean (All four schemas under schemas/, fixtures under fixtures/contracts/credential-delivery/, Rust types in crates/runx-contracts/src/credential_delivery.rs, TS types in packages/contracts/src/schemas/credential-delivery.ts, and runtime mapping in credentials.rs:44-67. thread-outbox-provider-protocol-v1 spec is active and references this primitive.)
- `external adapter supervisor`: confirm adapter-initiated credential request frames are rejected and process-env delivery is supervisor-injected after scoped env, with stdout/stderr/metadata/response observations redacted before runtime mapping -> clean (external_adapter.rs:855-868 routes CREDENTIAL_REQUEST_SCHEMA to UnexpectedCredentialRequest. process_env (external_adapter.rs:799-819) inserts credential_delivery.secret_env after scoped env so the delivered binding wins. redact_response (external_adapter.rs:925-974) walks stdout, stderr, output, metadata, artifacts.summary, errors.code/message, telemetry, observed_at. Tests at tests/external_adapter.rs:319-598 cover both the supervisor and SkillAdapter paths.)
- `cli-tool + MCP credential delivery`: confirm secret env injected only at child spawn, redaction applied before truncation, empty material rejected, role mapping fail-closed -> finding (Redaction-before-truncation holds at the API surface (credentials.rs:280-285), but cli-tool's pre-API capture_stream truncates raw bytes at 1MB before redaction runs (F2). Empty-material rejection (credentials.rs:338-342) and unsupported role rejection (credentials.rs:87-96) are wired and tested.)
- `contract <-> runtime field consistency`: diff every contract field against its runtime usage to find silent drops -> finding (CredentialDeliveryEnvBinding.required is required by the contract schema but never read by from_contract_profile or apply_profile (F1). All other fields (provider, auth_mode, purpose, delivery_mode, material_roles, env_var, redaction_policy_ref, harness_ref, host_ref, grant_ref, credential_ref, profile_id, material_ref_hash, redaction_refs, observed_at) flow through as expected. delivery_mode is checked against ProcessEnv even though it is the only variant — defensive and harmless.)
- `scope drift / ambient classification`: compare declared task scope and touchpoints against modified files; verify ambient drift is not silently load-bearing on this task -> clean (Task-scoped changes are crates/runx-runtime/src/adapters/external_adapter.rs and crates/runx-runtime/src/credentials.rs. Ambient drift (tests/external_adapter.rs, execution/runner/steps.rs, docs/thread-story-contract.md, docs/ts-interop-boundary.md, packages/core/src/knowledge/file-thread.ts, packages/core/src/knowledge/index.test.ts) belongs to the outbox/provider lane (thread-outbox-provider-protocol-v1) and does not change credential-delivery semantics — runner steps.rs:73 still passes CredentialDelivery::none() to all adapter invocations, so end-to-end runner remains broker-free as the spec describes.)
- `regression hunt for downstream consumers`: trace CredentialDelivery, CredentialDeliveryProfile, SecretEnv, redact_text, and the four contract types to all callers -> clean (CredentialDelivery::none() is used in skill_run.rs:192, runner/steps.rs:73, harness/runner.rs:910 — all entrypoints fail-closed to no-secret delivery. SecretEnv only escapes via secret_env().iter() at adapter spawn sites (cli_tool.rs:64, external_adapter.rs:815, mcp adapter env wiring). redact_text is invoked at every cli-tool/MCP/external adapter observable surface. SecretString never implements Serialize and its Debug prints '[redacted-credential]' (credentials.rs:170-174).)
- `schema fidelity / public-frame purity`: verify fixtures and TS/JSON schemas reject raw secret material in profile/request/broker-response/observation; confirm material_ref/material_ref_hash semantics -> clean (All four schemas use additionalProperties:false and only carry refs, hashes, profile ids, env bindings, status enums, and timestamps. Fixtures (fixtures/contracts/credential-delivery/*.json) use opaque uris (e.g. runx:credential-delivery-handle:req_cred_1:access_token, sha256:4ab3). docs/security-authority-proof.md explicitly delegates runtime secret handoff to this contract and bans raw material in proofs/receipts/logs.)

Findings:
- [low/non-blocking] `F1-required-binding-ignored` Rust runtime ignores contract `env_bindings[*].required`, treating every binding as required
  - Location: `crates/runx-runtime/src/credentials.rs:327`
  - Evidence: `CredentialDeliveryEnvBinding` in `crates/runx-contracts/src/credential_delivery.rs:48-54`, `packages/contracts/src/schemas/credential-delivery.ts:47-54`, and `schemas/credential-delivery-profile.schema.json:93-130` all require `required: bool`. `from_contract_profile` (credentials.rs:44-67) drops `required` and constructs a runtime `CredentialEnvBinding` without it; `apply_profile` (credentials.rs:327-346) then errors `MissingRole` whenever `material.values` lacks a role, regardless of whether the contract declared the binding optional.
  - Impact: Forward-compatibility hazard. The first hosted/cloud broker that legitimately publishes an optional binding (e.g., refresh-token role marked `required: false`) will be rejected by the Rust supervisor even though the contract permits the material to be absent. The discrepancy is also a Liskov-style trap for skill authors reading the contract: the `required` flag is observable in the schema but silently inert in v1.
- [low/non-blocking] `F2-capture-truncation-redaction-limit` cli-tool capture truncates raw bytes to 1MB before redaction, so secrets straddling the 1MB boundary can leak their prefix
  - Location: `crates/runx-runtime/src/adapters/cli_tool.rs:133`
  - Evidence: `capture_stream` (cli_tool.rs:133-151) bounds the captured `Vec<u8>` to `OUTPUT_LIMIT_BYTES = 1_048_576` while reading from the child pipe. `collect_redacted_output` (cli_tool.rs:84-91) calls `CredentialDelivery::redact_bytes_to_string` (credentials.rs:280-285), which does an exact substring `String::replace(secret, REDACTED_CREDENTIAL)` after the capture-side truncation has already happened. If a delivered secret straddles the 1MB capture boundary, the captured bytes contain only the prefix of the secret, `redact_text` does not match it, and the prefix is published into `SkillOutput.stdout`/`stderr` and downstream into `output_object`/receipts. The spec's Risks section calls this out explicitly: 'If redaction is exact-string only, transformed or boundary-split secrets can leak through captured output. The contract must specify the minimum v1 guarantee and the limits of that guarantee honestly.'
  - Impact: Adversarial or pathological skill output >1MB whose write happens to cut a delivered secret across the capture boundary can publish a partial secret into receipts and metadata. External adapter is not affected because its supervisor refuses truncated stdout via `ResponseTooLarge` (`external_adapter.rs:335-339`). MCP is not affected because its tool result string is bounded by transport framing, not bulk pipe capture. cli-tool is the asymmetric surface.

## Origin

User architecture review on 2026-05-22: after the external execution-adapter
scope was corrected, the remaining cross-cutting gap is credential
broker/delivery. It must be a shared primitive consumed by cli-tool, external
adapters, and outbox/provider side effects rather than a per-protocol
afterthought.

## Harden Rounds

### round-1

Status: passed
Started: 2026-05-21T15:05:44Z
Ended: 2026-05-21T15:07:50Z

Checks:
- path audit
  - Grounded in: code:crates/runx-runtime/src/credentials.rs:199
  - Result: passed
  - Evidence: The runtime secret-bearing object is `CredentialDelivery`, which
- command audit
  - Grounded in: spec_gap:acceptance
  - Result: passed
  - Evidence: The acceptance section names executable scafld, schema, contract,
- scope/migration audit
  - Grounded in: code:crates/runx-runtime/src/adapters/external_adapter.rs:110
  - Result: passed
  - Evidence: `ExternalAdapterSupervisor` currently receives only the manifest
- acceptance timing audit
  - Grounded in: spec_gap:acceptance.dod4
  - Result: passed
  - Evidence: The external adapter acceptance item remains open until the
- rollback/repair audit
  - Grounded in: spec_gap:rollback
  - Result: passed
  - Evidence: Rollback is explicit: keep a consumer blocked or create a named
- design challenge
  - Grounded in: code:crates/runx-contracts/src/external_adapter.rs:98
  - Result: passed
  - Evidence: The external adapter contract still contains an adapter-emitted

Issues:
- none
