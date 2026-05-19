---
spec_version: '2.0'
task_id: rust-ts-sunset-executor
created: '2026-05-18T00:00:00Z'
updated: '2026-05-20T00:00:00Z'
status: draft
harden_status: not_run
size: medium
risk_level: high
---

# TS sunset: executor

## Current State

Status: draft
Current phase: refresh
Next: split into a migration slice; do not approve deletion yet.
Reason: current code still publishes and consumes `@runxhq/core/executor`.
Blockers:
- `packages/core/package.json` still exports `./executor`.
- `packages/core/src/executor/index.ts` still exports public protocol aliases,
  validators, adapter interfaces, tool-catalog interfaces, and `ExecuteSkillOptions`.
- Live importers remain in `packages/runtime-local`, `packages/adapters`,
  `packages/cli`, and fixture-generation scripts.
- Rust approval handling has progressed in `crates/runx-runtime/src/approval.rs`,
  and host protocol approval/request types live in `runx_contracts::host_protocol`
  and are re-exported from `runx_contracts`, but TS consumers have not been
  migrated away from the executor package.
Allowed follow-up command: none. Do not run `scafld harden`.
Latest runner update: refreshed against code on 2026-05-20.
Review gate: not_started

## Summary

This draft is not deletion-ready. `packages/core/src/executor/` is still a
published TS surface with active imports. The next safe work is to migrate one
small surface at a time to the Rust-owned contract/runtime paths that already
exist, then re-audit the import graph before any delete spec is reopened.

The current Rust side is no longer empty: approval request resolution,
idempotency-key derivation, request/resolved events, and boolean approval
payload enforcement live in `crates/runx-runtime/src/approval.rs`; shared host
protocol types such as `ApprovalGate`, `ResolutionRequest`, and
`ResolutionResponse` live in `crates/runx-contracts/src/host_protocol.rs`.
That progress should be used directly. Do not add alternate TS shapes or bridge
aliases while sunsetting this package.

## Context

CWD: `.`

Packages:
- `@runxhq/core`
- `@runxhq/contracts`
- `crates/runx-contracts` (host protocol and approval gate contracts live here)
- `crates/runx-runtime`

Current TypeScript sources:
- `packages/core/src/executor/index.ts` (still live)
- `packages/core/src/executor/index.test.ts` (still live)
- `packages/core/package.json` export `./executor` (still live)
- All TS importers of `@runxhq/core/executor`

Files impacted:
- Not deletion-ready: no files are approved for deletion by this draft.
- Future migration slices may touch importers in `packages/runtime-local`,
  `packages/adapters`, `packages/cli`, and scripts, but those changes belong in
  their own approved slice.

Invariants:
- `ApprovalGate`, `ResolutionRequest`, `ResolutionResponse`, `Question`, and
  output/receipt validator behavior remain schema-exact with
  `@runxhq/contracts` and `runx-contracts`.
- Runtime approval behavior remains owned by Rust; no duplicate TS approval
  state machine is introduced.
- No new approval semantics introduced in this spec.
- Package export removal only happens after `rg "@runxhq/core/executor"` finds
  no live consumers outside the executor package itself.

## Objectives

- Keep the current importer/export inventory visible in the draft.
- Choose the smallest next migration slice:
  1. Move pure contract type and validator consumers that can already depend on
     `@runxhq/contracts` directly.
  2. Leave runtime adapter interfaces (`SkillAdapter`, `ToolCatalogAdapter`,
     `ExecuteSkillOptions`, nested invocation types) in place until a Rust
     runtime owner or TS runtime-local owner is explicitly selected.
  3. Route approval execution changes through `crates/runx-runtime/src/approval.rs`
     rather than extending TS executor behavior.
- Re-audit imports after that slice before proposing package export removal.

## Scope

In scope:
- Planning and executing one importer migration slice at a time.
- Import audits for `@runxhq/core/executor`.

Out of scope:
- Approval contract changes.
- Deleting `packages/core/src/executor/`.
- Removing the `./executor` package export.
- Payment runner preflight; that is active parent-agent work and must not be
  coupled to this sunset draft.

## Dependencies

- Rust host protocol contract types available from `crates/runx-contracts`.
- Rust approval runtime available from `crates/runx-runtime/src/approval.rs`.
- A separate decision on ownership for TS-only adapter/runtime interfaces that
  are not simple contract aliases.

## Open Questions

- Which package owns TS adapter interfaces after executor contract aliases move
  to `@runxhq/contracts`: `packages/runtime-local`, `packages/adapters`, or a
  Rust-generated surface?
- Should fixture-generation scripts move directly to `@runxhq/contracts` now,
  or wait until runtime importers are moved in the same slice?
