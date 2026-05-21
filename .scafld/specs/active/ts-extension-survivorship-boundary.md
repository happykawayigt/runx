---
spec_version: '2.0'
task_id: ts-extension-survivorship-boundary
created: '2026-05-21T13:04:12Z'
updated: '2026-05-22T01:40:00+10:00'
status: active
harden_status: not_run
size: medium
risk_level: high
---

# TypeScript extension survivorship boundary

## Current State

Status: active
Current phase: boundary language, spec alignment, and static guardrails landed
Next: leave runtime-local/adapters deletion blocked on the child sunset specs
and external adapter/credential/SDK protocol work
Reason: the Rust takeover must delete the trusted TypeScript runtime without
turning the runx extension ecosystem into a Rust-only surface.
Blockers: no blocker remains for this boundary guardrail itself. The broader
runtime-local/adapters deletion remains blocked because
`external-adapter-plugin-protocol-v1`, `credential-broker-delivery-contract-v1`,
and non-execution extension lanes still need their final runtime/SDK protocols.
Allowed follow-up command: `scafld harden ts-extension-survivorship-boundary`
Latest runner update: 2026-05-22T01:40:00+10:00 promoted this guardrail spec
from drafts to active and revalidated `node scripts/check-boundaries.mjs`.
Docs/specs contain the required trusted-runtime/language-neutral extension
boundary language. This spec now serves as a boundary ledger; it must not be
used to delete runtime-local directly.
Review gate: boundary_ready

## Summary

Make the survivorship rule explicit: Rust owns trusted local execution, while
TypeScript remains a valid authoring and integration environment for thin
clients, generated contracts, cloud/product code, host adapters, scaffolding,
and helper SDKs over named language-neutral protocol lanes. No surviving
TypeScript package may reimplement local trusted runtime behavior or hide a
fallback to `@runxhq/runtime-local` or `@runxhq/adapters`.

This spec is the guardrail between two bad outcomes:
- keeping the old TypeScript runtime alive forever as a compatibility shim; and
- forcing every integration or custom adapter author to write Rust or fork the
  runtime.

## Context

Current package intent already points this way:
- `oss/docs/ts-interop-boundary.md` says Rust is canonical for local execution
  and TypeScript remains for client, packaging, product UX, docs, contracts,
  and cloud boundaries.
- `oss/docs/rust-kernel-architecture.md` keeps `runx-sdk` v0 CLI-backed and
  puts side effects in `runx-runtime`.
- `oss/.scafld/specs/drafts/rust-ts-sunset-runtime-local.md` deletes
  `@runxhq/runtime-local` and `@runxhq/adapters`, but must not imply that all
  adapter/integration authors now write Rust.

The missing rule is the authoring surface after deletion.

## Objectives

- Define the TypeScript survivorship categories:
  generated contracts, CLI launcher/client wrappers, host adapters, cloud and
  product integration code, authoring/scaffold tooling, and helper SDKs over
  stable language-neutral protocols.
- Keep extension lanes separate: skill subprocess ABI, external execution
  adapter, source-event ingress, hosted/embedded runtime binding, tool
  catalog/read model, and thread/outbox provider adapters are not one generic
  plugin API.
- Define forbidden TypeScript categories:
  local trusted runtime, graph orchestration, policy admission, sandbox
  enforcement, credential authority, receipt sealing/verification, canonical
  JSON/fingerprinting, hidden runtime-local fallbacks, and compatibility
  packages for deleted runtime surfaces.
- Update docs and sunset specs so built-in trusted adapters move to Rust while
  third-party/custom execution adapters remain authorable through the
  language-neutral external adapter/plugin protocol, and other extension lanes
  remain blocked until they have their own stable boundaries.
- Add static guardrails that reject new runtime-local/adapters shims and
  surviving-package imports or aliases that would recreate trusted TypeScript
  execution.

## Scope

In scope:
- OSS docs that describe the Rust/TypeScript boundary.
- Runtime-local and adapter sunset specs.
- Boundary-check scripts and package-name/import guardrails.
- Scafld specs that currently describe embedded SDK or cloud binding choices.

Out of scope:
- Implementing the adapter process protocol; owned by
  `external-adapter-plugin-protocol-v1`.
- Deleting `@runxhq/runtime-local` or `@runxhq/adapters`; owned by
  `rust-ts-sunset-runtime-local`.
- Rewriting cloud worker/agent-runner; owned by the embedded/cloud binding
  migration specs.

## Dependencies

- `external-adapter-plugin-protocol-v1`
- `skill-author-runtime-contract-v1`
- `embedded-sdk-migration-story`
- `rust-ts-sunset-runtime-local`

## Touchpoints

- `oss/docs/ts-interop-boundary.md`
- `oss/docs/rust-kernel-architecture.md`
- `oss/README.md`
- `oss/scripts/check-boundaries.mjs`
- `oss/.scafld/specs/active/embedded-sdk-migration-story.md`
- `oss/.scafld/specs/drafts/rust-ts-sunset-runtime-local.md`
- `oss/.scafld/specs/drafts/rust-aster-runtime-cutover.md`

## Acceptance

Profile: strict

Definition of done:
- [x] `dod1` Docs state that Rust is the trusted runtime and TypeScript remains
  a supported authoring/integration language only through stable protocols.
- [x] `dod2` Docs state that external adapters/plugins must not require a Rust
  crate, runtime fork, or core-linking path.
- [x] `dod3` Runtime-local/adapters sunset docs distinguish built-in Rust
  adapters from language-neutral external adapter/plugin authoring.
- [x] `dod4` Static checks reject new runtime-local/adapters v2, shim, or
  compat package names, shims, aliases, or surviving-package imports while
  allowing the current dual-tree package internals to remain deletion blockers.
- [x] `dod5` Existing embedded/cloud binding specs no longer claim cloud
  adapter migration is settled when the binding mode is still open.

Validation:
- [ ] `v1` Scafld validates this spec.
  - Command: `scafld validate ts-extension-survivorship-boundary --json`
  - Expected kind: `exit_code_zero`
  - Status: passed
  - Evidence: 2026-05-21T14:15:21Z returned
    `{"ok":true,"command":"validate","result":{"task_id":"ts-extension-survivorship-boundary","path":"/Users/kam/dev/runx/runx/oss/.scafld/specs/active/ts-extension-survivorship-boundary.md","valid":true,"errors":null}}`.
- [ ] `v2` Boundary checks pass after guardrail changes.
  - Command: `node scripts/check-boundaries.mjs`
  - Expected kind: `exit_code_zero`
  - Status: passed
  - Evidence: 2026-05-22T00:47:00+10:00 exited 0 with
    `Boundary check passed.`
- [ ] `v3` Docs contain the required extension-boundary vocabulary.
  - Command: `rg -n "language-neutral external adapter|external adapter/plugin|trusted local runtime|no Rust crate" docs README.md .scafld/specs/drafts`
  - Expected kind: `reviewed_output`
  - Status: passed
  - Evidence: 2026-05-22T00:47:00+10:00 output included `trusted local
    runtime`, `external adapter/plugin`, `language-neutral external adapter`,
    and `external execution-adapter` hits in `README.md`,
    `docs/ts-interop-boundary.md`, `docs/rust-kernel-architecture.md`, and
    active/draft specs.

## Phase 1: Boundary Language

Goal: make the architecture unambiguous.

Status: complete
Dependencies: none

Changes:
- Update `ts-interop-boundary.md` to add language-neutral extension protocols
  as stable crossings, with `external-adapter-plugin-protocol-v1` limited to
  the external execution-adapter lane.
- Update package dispositions so `@runxhq/adapters` sunset means "built-in
  trusted adapters move to Rust", not "custom adapters must become Rust".
- Update `README.md` and `rust-kernel-architecture.md` with the same split.

Acceptance:
- No active doc describes TypeScript as a local runtime fallback.
- No active doc implies integrations must be ported into Rust.

## Phase 2: Spec Alignment

Goal: align pending work with the boundary.

Status: complete
Dependencies: Phase 1

Changes:
- Update `embedded-sdk-migration-story` to treat cloud in-process semantics as
  a binding/process-protocol problem, not as a reason to keep runtime-local.
- Update `rust-ts-sunset-runtime-local` so cloud agent-runner is a blocker
  until it has a stable boundary, not a settled fact.
- Update `rust-aster-runtime-cutover` open questions to reference the allowed
  boundary categories.

Acceptance:
- Sunset specs block on missing extension/cloud boundaries without preserving a
  TypeScript runtime shim.

## Phase 3: Static Guardrails

Goal: make the standard hard to regress.

Status: complete
Dependencies: Phase 1

Changes:
- Extend boundary checks so surviving TypeScript packages cannot import
  runtime-local/adapters or introduce shim/v2/compat compatibility packages or
  aliases.
- Keep current runtime-local/adapters package internals and tests classified as
  deletion blockers rather than failing the guardrail before the sunset lands.

Acceptance:
- The guardrail passes in the current dual-tree state and fails for a new
  surviving-package runtime-local import or compatibility package.

## Rollback

If a doc or guardrail blocks valid authoring/integration code, relax it only by
adding a named stable protocol category. Do not reintroduce trusted TypeScript
runtime execution as the escape hatch.

## Review

Review must reject any wording that equates "delete TS runtime" with "all
extension authors must write Rust", and must also reject any wording that keeps
`@runxhq/runtime-local` as a hidden compatibility runtime.

## Origin

User architecture review on 2026-05-21: Rust should own trust and execution,
while TypeScript remains a first-class integration and extension authoring
environment through stable protocols.
