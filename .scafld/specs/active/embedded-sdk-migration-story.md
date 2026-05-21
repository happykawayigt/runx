---
spec_version: '2.0'
task_id: embedded-sdk-migration-story
created: '2026-05-21T12:19:24Z'
updated: '2026-05-22T01:40:00+10:00'
status: active
harden_status: not_run
size: large
risk_level: high
---

# Embedded SDK migration story

## Current State

Status: active
Current phase: planning
Next: complete cloud inventory, then harden
Reason: `@runxhq/runtime-local/sdk` is a real in-process embedding surface, but
the current Rust `runx-sdk` is explicitly CLI-backed and does not execute skills
natively.
Blockers: cloud worker/agent-runner inventory is still unavailable in this OSS
checkout. `skill-author-runtime-contract-v1` is archived complete for the
author-facing subprocess ABI, and `external-adapter-plugin-protocol-v1` now owns
the language-neutral custom adapter path; richer embedded host continuation
semantics still require a Rust host/service/native-binding decision before
runtime-local SDK deletion.
Allowed follow-up command: `scafld harden embedded-sdk-migration-story`
Latest runner update: 2026-05-22T01:40:00+10:00 promoted this executed inventory
spec from drafts to active, revalidated boundary guardrails, and confirmed the
TS-free Rust CLI smoke test exists as the local execution proof. Cloud worker
and agent-runner callers remain unverified in this checkout and are recorded as
a follow-up blocker rather than inferred from stale context.
Previous update: 2026-05-21T22:52:32+10:00 aligned with
`ts-extension-survivorship-boundary` and
`external-adapter-plugin-protocol-v1`; embedded migration must not preserve a
trusted TypeScript runtime fallback or force custom adapter authors into Rust.
Review gate: not_started

## Summary

Plan the migration path for embedded consumers of the TypeScript runtime-local
SDK. The end state must be explicit: host/runtime consumers move to
`runx --json`, a Rust-native runtime embedding surface, hosted HTTP, a
`runx-runtime-service` style boundary, or a Node/native binding around
`runx-runtime`; custom adapter/plugin semantics move to the language-neutral
external adapter/plugin protocol when they need provider-specific userland code.

This spec does not redefine skill author behavior or the adapter/plugin wire
protocol. It depends on `skill-author-runtime-contract-v1` for the subprocess
ABI and `external-adapter-plugin-protocol-v1` for richer adapter/plugin
authoring. No target shape may keep `@runxhq/runtime-local` as a hidden
execution fallback, and no target shape may require custom adapter authors to
rewrite TypeScript or other provider SDK glue as Rust crates.

## Context

Current TypeScript embedding surfaces:
- `oss/packages/runtime-local/src/sdk/index.ts` exports `RunxSdk`, host bridge
  helpers, caller integration, registry access, connect helpers, and direct
  `runLocalSkill` execution.
- `oss/packages/runtime-local/src/runner-local/adapter-types.ts` defines
  adapter-shaped extension points.
- `cloud/packages/worker/src/index.ts` imports `runLocalSkill`,
  `createHostBridge`, `SkillAdapter`, `AuthResolver`, and `Caller`.
- `cloud/packages/agent-runner/src/hosted-agent-adapter.ts` returns custom
  `SkillAdapter` implementations used by hosted runs.

Current Rust surfaces:
- `oss/crates/runx-sdk/src/lib.rs` documents SDK v0 as CLI-backed and not a
  native skill executor.
- `oss/crates/runx-runtime/src/adapter.rs` has a Rust `SkillAdapter` trait, but
  there is no Node or TypeScript compatible embedding layer for existing cloud
  consumers.
- `oss/docs/rust-kernel-architecture.md` records native-runtime SDK work as a
  separate future feature.

New boundary specs:
- `ts-extension-survivorship-boundary` says Rust owns trusted local execution
  while TypeScript remains valid for clients, cloud/product code, scaffolding,
  host adapters, and helper SDKs over stable protocols.
- `external-adapter-plugin-protocol-v1` owns the no-Rust-required
  adapter/plugin process protocol for custom integrations supervised by Rust.

## Objectives

- Inventory every production embedded consumer of `@runxhq/runtime-local/sdk`,
  `runLocalSkill`, `SkillAdapter`, `ToolCatalogAdapter`, `AuthResolver`, and
  host bridge APIs.
- Choose the target migration shape for each consumer: CLI-backed, hosted HTTP,
  `runx-runtime-service`, direct Rust runtime embedding, Node/native binding, or
  external adapter/plugin protocol.
- Preserve host-state, approval/continuation, custom adapter, auth resolver,
  tool catalog, receipt, and caller semantics before deleting TypeScript SDK
  surfaces.
- Make any remaining trusted TypeScript runtime dependency an explicit blocker,
  not ambient drift. TypeScript protocol helpers are allowed only when they do
  not execute skills outside Rust supervision.

## Scope

In scope:
- Embedded SDK and host bridge caller inventory, split into verified OSS
  inventory and separate cloud inventory when a cloud checkout is available.
- Cloud worker and agent-runner migration shape.
- Target public package boundary for embedded consumers.
- Fixture and test plan proving behavior without hidden TypeScript fallback.

Out of scope:
- Changing the subprocess skill author ABI; owned by
  `skill-author-runtime-contract-v1`.
- Defining the external adapter/plugin wire protocol; owned by
  `external-adapter-plugin-protocol-v1`.
- Broad `@runxhq/core` and runtime-local deletion; owned by
  `rust-ts-sunset-runtime-local` and related sunset specs.
- Provider-specific agent behavior unless needed as a migration fixture.

## Dependencies

- `skill-author-runtime-contract-v1`
- `ts-extension-survivorship-boundary`
- `external-adapter-plugin-protocol-v1`
- `rust-ts-sunset-runtime-local`
- `rust-kernel-port-orchestration`
- `rust-runtime-skill-execution`

## Touchpoints

- `oss/packages/runtime-local/src/sdk/`
- `oss/packages/runtime-local/src/runner-local/adapter-types.ts`
- `oss/crates/runx-sdk/src/`
- `oss/crates/runx-runtime/src/adapter.rs`
- `oss/crates/runx-runtime/src/execution/runner.rs`
- `cloud/packages/worker/src/index.ts`
- `cloud/packages/agent-runner/src/`
- `cloud/packages/api/src/server-agent-support.ts`

## Risks

- Replacing embedded SDK consumers with `runx --json` can remove in-process
  adapter overrides, host continuations, and auth resolver hooks.
- A Node/native binding can preserve semantics but adds packaging and ABI
  complexity.
- Direct Rust embedding helps Rust consumers but does not by itself migrate
  cloud TypeScript callers.
- Treating the Rust `SkillAdapter` trait as the only custom-adapter answer would
  create a Rust-only extension surface and violate the survivorship boundary.
- An external adapter/plugin protocol that cannot round-trip host continuation,
  auth resolver, tool catalog, or receipt semantics keeps this migration blocked
  rather than justifying a TypeScript runtime shim.

## Acceptance

Profile: strict

Definition of done:
- [ ] `dod1` Every production embedded SDK consumer has a migration disposition.
- [ ] `dod2` The chosen target shape is documented with package/API boundaries.
- [ ] `dod3` Custom `SkillAdapter`, `ToolCatalogAdapter`, `AuthResolver`,
  caller, host bridge, receipt, and continuation semantics have migration
  tests or explicit blockers.
- [ ] `dod4` No TypeScript SDK fallback remains hidden behind the Rust path.
- [ ] `dod5` TypeScript sunset specs can reference this migration state rather
  than rediscovering embedded consumers.
- [ ] `dod6` Custom adapter/plugin authoring has a language-neutral disposition
  and is not reduced to Rust-only `SkillAdapter` implementations.

Validation:
- [x] `v1` OSS embedded consumer inventory is current for the checked-out OSS
  tree.
  - Command: `rg -n "@runxhq/runtime-local/sdk|@runx/sdk|createRunxSdk|RunxSdk|createHostBridge|HostBridge|runLocalSkill|SkillAdapter|ToolCatalogAdapter|AuthResolver|Caller|createDefaultSkillAdapters|resolveDefaultSkillAdapters" packages examples scripts --glob '*.ts' --glob '!**/*.test.ts' --glob '!**/node_modules/**'`
  - Expected kind: `reviewed_output`
  - Status: reviewed
  - Evidence: 2026-05-22 OSS inventory below.
- [ ] `v1_cloud` Cloud embedded consumer inventory is current.
  - Command: `rg -n "@runxhq/runtime-local|@runxhq/runtime-local/sdk|runLocalSkill|createHostBridge|SkillAdapter|ToolCatalogAdapter|AuthResolver|Caller|HostBridge" cloud --glob '*.ts' --glob '!**/node_modules/**'`
  - Expected kind: `reviewed_output`
  - Status: blocked
  - Evidence: Cloud tree is not present in this OSS checkout; do not infer
    migration disposition from stale context.
- [ ] `v2` Cloud worker migration tests pass.
  - Command: `pnpm vitest run packages/worker/src`
  - Expected kind: `exit_code_zero`
  - Status: pending
  - Evidence: none
- [ ] `v3` Rust SDK/native runtime tests pass for the chosen target.
  - Command: `cargo test --manifest-path crates/Cargo.toml -p runx-sdk -p runx-runtime`
  - Expected kind: `exit_code_zero`
  - Status: pending
  - Evidence: none
- [ ] `v4` TypeScript typecheck passes after consumer migration changes.
  - Command: `pnpm typecheck`
  - Expected kind: `exit_code_zero`
  - Status: pending
  - Evidence: none
- [x] `v5` Native Rust CLI can run representative local workflows without a
  Node/TypeScript runtime environment.
  - Command:
    `cargo test --manifest-path crates/Cargo.toml -p runx-cli --test native_no_ts -- --nocapture`
  - Expected kind: `exit_code_zero`
  - Status: passed
  - Evidence: 2026-05-22T01:40:00+10:00 passed 1 test,
    `native_cli_smoke_runs_without_node_or_typescript_env`, covering doctor,
    list, history, agent-step needs-agent, and harness receipt sealing with
    `env_clear()` and no Node/TypeScript runtime variables.

## Phase 1: Inventory

Goal: identify every embedded SDK caller and classify it.

Status: partial
Dependencies: none

Changes:
- Inventory OSS and cloud imports of runtime-local SDK, runtime-local core
  execution, custom adapters, tool catalogs, auth resolvers, and host bridge
  helpers.
- Mark each caller as CLI-backed candidate, Rust-native embedding candidate,
  hosted HTTP or service candidate, Node/native binding candidate,
  external-adapter/plugin-protocol candidate, or runtime-local-retained blocker.
- OSS inventory is recorded below. Cloud inventory remains blocked until the
  cloud tree is available to this spec runner.

Acceptance:
- The inventory is complete enough that `rust-ts-sunset-runtime-local` can fail
  closed on any unclassified import.

### OSS Inventory Slice: 2026-05-22

Command reviewed:
`rg -n "@runxhq/runtime-local/sdk|@runx/sdk|createRunxSdk|RunxSdk|createHostBridge|HostBridge|runLocalSkill|SkillAdapter|ToolCatalogAdapter|AuthResolver|Caller|createDefaultSkillAdapters|resolveDefaultSkillAdapters" packages examples scripts --glob '*.ts' --glob '!**/*.test.ts' --glob '!**/node_modules/**'`

Dispositions are against the harness-spine/Rust rewrite. They are planning
classifications only; this slice intentionally does not implement migrations.

| Consumer | Current surface | Semantics at risk | Disposition |
| --- | --- | --- | --- |
| `packages/runtime-local/src/sdk/index.ts` | `RunxSdk`, `createRunxSdk`, `runSkill`, `createRunxHostBridge`, host run/resume/inspect helpers, registry/search/connect helpers; calls `runLocalSkill` directly. | In-process skill execution, `Caller`, `AuthResolver`, custom `SkillAdapter[]`, `ToolCatalogAdapter[]`, receipts, pending host state, continuation. | Runtime-local-retained blocker until a replacement exists. Split API: non-executing registry/search/connect helpers can remain TypeScript protocol/client helpers; run/host execution must move to Rust-supervised CLI/service/native binding. Native binding or runtime service is the only OSS shape that can preserve host continuation without a hidden TS executor. |
| `packages/runtime-local/src/sdk/host-protocol.ts` | `createHostBridge`, `HostBridge`, `HostBoundaryResolver`, host state/result projections over a caller-wrapped local executor. | Resolver-mediated approvals/input/agent requests, event capture, resume path lookup, inspect state, terminal-state projection. | Data projections can move to contracts/host-adapters. Closure bridge is a runtime concern and cannot be ported as contracts-only. Needs Rust host bridge or service API before TypeScript host helpers can stop depending on runtime-local execution. |
| `packages/runtime-local/src/runner-local/index.ts` | `runLocalSkill`, `runLocalGraph`, `AuthResolver`, run options carrying adapters, callers, tool catalogs, receipts, lineage, auth. | Trusted execution ownership, auth grant/credential resolution, admission, resume, receipts, caller resolution. | Trusted execution must be replaced by harness-spine/Rust runtime. `AuthResolver` becomes a blocker until Rust has an equivalent host-resolution/credential boundary; do not map it to Rust-only `SkillAdapter`. |
| `packages/runtime-local/src/runner-local/adapter-types.ts` | TypeScript `SkillAdapter`, `AdapterActInvocation`, nested skill invoker, `ActReceiptEnvelope`. | Custom provider SDK glue, nested skill calls, tool catalog injection, credential envelope delivery, needs-agent pauses. | Custom adapter authoring must move to `external-adapter-plugin-protocol-v1`. Existing TS interface is a legacy compatibility surface until protocol fixtures prove parity. Rust `SkillAdapter` alone is insufficient because it forces extension authors into Rust. |
| `packages/runtime-local/src/runner-local/execute-skill.ts` | Internal adapter dispatch accepting `readonly SkillAdapter[]`. | Source-type routing and adapter-not-found behavior. | Rust runtime owns dispatch after harness-spine rewrite. Keep only as legacy until adapter/plugin protocol and source-type routing parity are proven. |
| `packages/runtime-local/src/runner-local/execution-targets.ts` | Resolves tool execution targets using `ToolCatalogAdapter[]`. | Catalog-backed tool lookup before execution. | Rust runtime or runtime service must own target resolution. Any provider-side catalog lookup that needs userland SDKs must go through external adapter/plugin protocol instead of in-process TS callbacks. |
| `packages/runtime-local/src/runner-local/caller-adapters.ts` | Converts `Caller` resolution into `SkillAdapter` implementations for agent, agent-step, and approval. | Caller-mediated approvals and agent work currently masquerade as adapters. | Split from adapter execution in the target. Model as Rust host-resolution events/responses, not custom adapter plugins, with fixtures for approval and cognitive-work continuations. |
| `packages/runtime-local/src/tool-catalogs/index.ts`, `packages/runtime-local/src/tool-catalogs/mcp.ts`, `packages/runtime-local/src/tool-catalogs/fixture.ts` | `ToolCatalogAdapter`, env-resolved fixture MCP catalog, MCP-backed catalog search/inspect/invoke. | Imported tool discovery and invocation from TypeScript callbacks. | Blocker for deleting runtime-local tool catalogs. Move stable search/inspect data shapes to contracts/Rust; route userland invocation through external adapter/plugin protocol or a Rust-owned MCP adapter. |
| `packages/runtime-local/src/harness/runner.ts` | Harness runner calls `runLocalSkill`/`runLocalGraph` and accepts `SkillAdapter[]` plus `ToolCatalogAdapter[]`. | Harness fixtures currently prove TS runtime behavior and custom adapter injection. | Harness-spine rewrite owner. Must move fixture execution to Rust runtime or a Rust-supervised service; TS harness runner becomes legacy or a thin fixture parser/client. |
| `packages/runtime-local/src/harness/agent-hook.ts` | Test/development `SkillAdapter` for `harness-hook` source. | Fixture-only hook semantics. | Migrate as a Rust harness fixture adapter or external adapter fixture, not as production runtime API. |
| `packages/adapters/src/index.ts` and `packages/adapters/src/runtime.ts` | `createDefaultSkillAdapters`, `resolveDefaultSkillAdapters`, default local runtime and caller helpers. | Default adapter bundle, managed agent inclusion, temporary runtime paths, caller answers/approvals. | Compatibility layer blocker. Default adapter set must be expressible as built-in Rust adapters plus external adapter/plugin manifests. Runtime path/caller helpers can become client test helpers only after Rust execution is the target. |
| `packages/adapters/src/catalog/index.ts` | `CatalogAdapter extends SkillAdapter`, invokes `ToolCatalogAdapter` results. | Imported tool execution through configured tool catalogs. | Tool catalog resolution needs a Rust/runtime-service boundary. If provider-specific catalog invocation remains userland TS, route it through the external adapter/plugin protocol. |
| `packages/adapters/src/a2a/index.ts`, `packages/adapters/src/agent/index.ts`, `packages/adapters/src/mcp/index.ts` | Custom `SkillAdapter` implementations for A2A, managed agents, and MCP. | Provider SDK calls, MCP execution, managed agent nested work, needs-agent pauses. | Built-in Rust equivalents may replace first-party adapters where already owned by Rust; otherwise each must be classified as an external adapter/plugin protocol fixture. Do not keep them as hidden TS runtime fallbacks. |
| `packages/adapters/src/agent/runtime-tools.ts` | Aggregates `SkillAdapter[]` for managed-agent tool execution. | Agent tool-call execution through TS adapters. | Blocked on external adapter/plugin protocol plus Rust host-resolution semantics for nested tool work. |
| `packages/host-adapters/src/index.ts` | Provider response adapters over a `HostBridge` interface and duplicated host result/state types. | Host run/resume packaging for OpenAI, Anthropic, Vercel AI, LangChain, CrewAI. | Keep as TypeScript client helper package only if it consumes a Rust-backed `HostBridge`/service/client. Its local `HostAuthResolver` is currently `any`-typed and should not become the target auth model. |
| `packages/langchain/src/index.ts` | Legacy/sunset LangChain `ToolCatalogAdapter` entry point now throws and points callers to manifests/CLI JSON. | Existing import compatibility only; no in-process adapter remains. | Already aligned with Rust takeover. Keep as non-executing compatibility/error surface; no embedded runtime blocker unless product policy requires removing the package. |
| `examples/host-protocol/openai.ts` | Imports `createRunxSdk`, `createHostBridge`, `createOpenAiHostAdapter`; executes via `sdk.runSkill`. | Demonstrates hidden in-process TS execution behind host adapter. | Update after target shape is chosen to use Rust-backed host bridge, hosted HTTP, or runtime service. It must not remain the canonical example for post-sunset embedding. |

Reviewed non-embedded hits:
- `packages/cli/src/cli-runtime-contracts.ts` and `packages/cli/src/callers.ts`
  define CLI-local caller contracts, not `@runxhq/runtime-local` embedded SDK
  consumers. They remain under CLI/Rust parity work, not this migration story.
- `packages/core/src/parser/index.ts` and
  `packages/runtime-local/src/parser-types.ts` define harness fixture caller
  data shapes. They are covered by the harness-spine disposition above and do
  not themselves execute embedded SDK runs.
- `packages/runtime-local/src/runner-local/approval.ts`,
  `graph-reporting.ts`, and `reflect.ts` consume the local `Caller` inside the
  TypeScript runner. They are part of the `runLocalSkill`/`runLocalGraph`
  runtime-local blocker, not standalone public embedding surfaces.

OSS production inventory blockers:
- No cloud tree is available here, so `cloud/packages/worker`,
  `cloud/packages/agent-runner`, and `cloud/packages/api` dispositions remain
  unverified.
- The Rust `runx-sdk` is still CLI-backed and cannot satisfy the in-process
  `RunxSdk.runSkill`/host bridge semantics by itself.
- `AuthResolver` parity is not yet assigned to a concrete Rust host-resolution
  API or service contract.
- Tool catalog and managed-agent adapter behavior cross the boundary between
  built-in Rust adapters and userland provider plugins; final classification
  depends on `external-adapter-plugin-protocol-v1` fixtures.

## Phase 2: Target Shape Decision

Goal: choose the migration shape for embedded consumers.

Status: pending
Dependencies: Phase 1, `skill-author-runtime-contract-v1`,
`external-adapter-plugin-protocol-v1`

Changes:
- Record whether `runx-sdk` remains CLI-backed only or gains a native-runtime
  feature.
- If a Node/native binding is selected, define its package name, build target,
  and minimal API surface.
- If the external adapter/plugin protocol is selected for custom adapter
  behavior, define the manifest, helper package, and host-resolution semantics
  that replace the in-process `SkillAdapter` hook.
- If CLI-backed is selected for a caller, document the lost in-process semantics
  and replacement mechanism.

Acceptance:
- No caller is migrated by assumption.

## Phase 3: Migration Fixtures

Goal: prove embedded behavior after migration.

Status: pending
Dependencies: Phase 2

Changes:
- Add fixtures for custom adapter invocation through the selected stable
  boundary, host continuation, auth resolver, tool catalog resolution, receipt
  production, and denial/needs-agent flow.
- Run fixtures without TypeScript runtime-local fallback on the selected target.

Acceptance:
- The fixtures pass on the selected target and fail if a hidden TypeScript
  fallback is required.

## Rollback

If the selected target cannot preserve required semantics, keep runtime-local
sunset blocked and retain the current SDK only as a pre-cutover legacy
dependency. Do not present the TypeScript SDK as the target architecture.

## Review

Review must reject any migration that only proves `runx --json` happy paths
while dropping custom adapter, host bridge, or auth resolver behavior. It must
also reject any migration that keeps a trusted TypeScript runtime fallback or
requires custom adapter/plugin authors to ship Rust crates.

## Origin

User review of Rust migration risk on 2026-05-21 identified the embedded SDK as
a separate consumer relationship from subprocess `run.js` skills.
