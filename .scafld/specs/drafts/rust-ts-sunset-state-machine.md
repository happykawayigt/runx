---
spec_version: '2.0'
task_id: rust-ts-sunset-state-machine
created: '2026-05-18T00:00:00Z'
updated: '2026-05-20T00:00:00Z'
status: draft
harden_status: not_run
size: medium
risk_level: high
---

# TS sunset: state-machine

## Current State

Status: draft, blocked
Current phase: prerequisite slicing
Next: unblock ownership and reroute runtime-local graph orchestration before
approval.
Reason: the deletion remains blocked, but a prerequisite slice is executable
against the current tree: move runtime-local sequential graph state creation
through the existing Rust kernel bridge. A fresh 2026-05-20 source scan still
finds live runtime-local imports of `@runxhq/core/state-machine`, fixture
generators still use the TS source as the oracle, and
`packages/core/package.json` still exposes `./state-machine`.
`crates/runx-core` has state-machine fixture parity, but TypeScript remains the
source of truth for the remaining synchronous transition/planning consumers
until separate cutover slices move them.
Blockers:
- Runtime-local graph orchestration still imports `@runxhq/core/state-machine`
  from live execution paths.
- Fixture generation still imports `packages/core/src/state-machine/index.js`
  as the TypeScript oracle; fixture-generator ownership must be reassigned,
  rerouted, or retired before deletion.
- `packages/core/package.json` still exposes `./state-machine` as the public
  package export.
- `docs/api-surface.md` and `scripts/gen-api-index.ts` still reflect the stable
  public `@runxhq/core/state-machine` export; regenerate/remove those entries
  only after the package export is actually removed.
- Deletion stays blocked until runtime-local graph orchestration is rerouted to
  Rust runtime ownership or retired. Do not add legacy or compatibility shapes
  to keep the TS import path alive.
Allowed follow-up command: none while blocked; do not run `scafld harden` for
this draft.
Latest runner update: 2026-05-20T03:15:00Z - prerequisite slice identified:
route runtime-local initial sequential graph state creation through
`packages/runtime-local/src/runner-local/kernel-bridge.ts`; deletion remains
blocked.
Review gate: not_started

## Summary

Future deletion target: remove the TypeScript state-machine implementation and
its public `@runxhq/core/state-machine` export after all live consumers are no
longer depending on it. The current codebase is not there yet:
`crates/runx-core` implements Rust state-machine parity against
`fixtures/kernel/state-machine/`, but it is conformance evidence only and is
not an authoritative replacement for runtime-local graph execution.

This spec must remain blocked until runtime-local graph orchestration no longer
imports the TS state-machine and the fixture generator no longer depends on the
TS implementation as its oracle.

## Executable Prerequisite Slice

This draft is not executable as a deletion spec today. The safe executable
slice is to remove one class of runtime-local value import:
`createSequentialGraphState` used by
`packages/runtime-local/src/runner-local/orchestrator/prepare-run.ts`.

Implementation shape:
- Add local sequential graph state wire types and
  `createSequentialGraphStateViaKernel` to the existing runtime-local kernel
  bridge.
- Change `prepareRun` to await `state-machine.createSequentialGraphState`
  through `runx kernel eval`, passing `options.env`.
- Change runtime-local graph result/context type imports that are only needed
  for this state carrier to use the kernel bridge type instead of
  `@runxhq/core/state-machine`.
- Do not convert synchronous transition, plan, fanout, or ledger hydration code
  in this slice; those need a separate async/runtime ownership design.

Focused validation for this slice:
- `pnpm exec vitest run packages/runtime-local/src/runner-local/kernel-bridge.test.ts`
- `pnpm exec tsc -p tsconfig.typecheck.json --noEmit`
- `rg "from \"@runxhq/core/state-machine\"|from '@runxhq/core/state-machine'" packages/runtime-local/src tests scripts`

## Context

CWD: `.` from the OSS repo root (`/Users/kam/dev/runx/runx/oss`).

Packages:
- `@runxhq/core`
- `crates/runx-core`
- `@runxhq/runtime-local`
- Fixture-generation scripts that currently import the TS state-machine source

Current Rust parity status:
- `crates/runx-core` currently implements state-machine parity against the
  checked-in TypeScript oracle fixtures under `fixtures/kernel/state-machine/`.
- `docs/trusted-kernel-package-truth.md` states that TypeScript remains the
  source of truth until a separate cutover spec changes consumers and passes the
  relevant parity gate.
- Kernel parity is not runtime parity. This draft must not claim the Rust
  runtime is already authoritative for graph execution.

Current TypeScript deletion targets:
- `packages/core/src/state-machine/**`
- `packages/core/package.json` export key `./state-machine`
- Generated API-surface entries that reflect the manifest export, if still
  present at execution time.
- `scripts/check-boundaries.mjs` from this CWD, not `oss/scripts/...`; remove
  `state-machine` from `pureCoreDomains` only when the TS domain is actually
  deleted and add/keep a boundary assertion that `./state-machine` is no longer
  exported.

Current live importers found in source by the 2026-05-20 scan before the
prerequisite slice:
- `packages/runtime-local/src/runner-local/orchestrator.ts`
- `packages/runtime-local/src/runner-local/index.ts`
- `packages/runtime-local/src/runner-local/graph-hydration.ts`
- `packages/runtime-local/src/runner-local/graph-fanout-gates.ts`
- `packages/runtime-local/src/runner-local/graph-governance.ts`
- `packages/runtime-local/src/runner-local/orchestrator/prepare-run.ts`
- `packages/runtime-local/src/runner-local/orchestrator/hydrate-resume.ts`
- `packages/runtime-local/src/runner-local/orchestrator/run-context.ts`
- `packages/runtime-local/src/runner-local/orchestrator/handle-run-step.ts`
- `packages/runtime-local/src/runner-local/orchestrator/handle-run-fanout.ts`
- `packages/runtime-local/src/runner-local/orchestrator/handle-terminal.ts`
- `packages/runtime-local/src/runner-local/orchestrator/handle-paused.ts`
- `tests/graph-hydration-orphan-start.test.ts`
- `scripts/generate-kernel-parity-fixtures.ts`
- `scripts/generate-rust-fanout-fixtures.ts`

Expected remaining importers after the prerequisite slice:
- `packages/runtime-local/src/runner-local/orchestrator.ts`
- `packages/runtime-local/src/runner-local/index.ts`
- `packages/runtime-local/src/runner-local/graph-hydration.ts`
- `packages/runtime-local/src/runner-local/graph-fanout-gates.ts`
- `packages/runtime-local/src/runner-local/graph-governance.ts`
- `packages/runtime-local/src/runner-local/orchestrator/hydrate-resume.ts`
- `packages/runtime-local/src/runner-local/orchestrator/handle-run-step.ts`
- `packages/runtime-local/src/runner-local/orchestrator/handle-run-fanout.ts`
- `packages/runtime-local/src/runner-local/orchestrator/handle-terminal.ts`
- `packages/runtime-local/src/runner-local/orchestrator/handle-paused.ts`
- `tests/graph-hydration-orphan-start.test.ts`
- `scripts/generate-kernel-parity-fixtures.ts`
- `scripts/generate-rust-fanout-fixtures.ts`

Files impacted:
- `packages/core/src/state-machine/` (future deletion)
- `packages/core/package.json` (remove `exports["./state-machine"]`)
- `scripts/check-boundaries.mjs`
- Any generated docs/API index reflecting the removed export
- Every current importer listed above, but only after its owning cutover or
  retirement spec has landed.

Invariants:
- No consumer regression. Deletion is allowed only after a fresh importer scan
  proves there are no live imports from `@runxhq/core/state-machine` and no
  direct imports of `packages/core/src/state-machine/index.js`.
- No false compatibility. Do not preserve `@runxhq/core/state-machine` with a
  shim, adapter, re-export, conditional fallback, or legacy shape.
- Rust runtime ownership must be explicit before deletion; fixture parity alone
  is insufficient.
- Receipts produced before and after deletion remain verifiable.

## Objectives

- Keep this draft honest about current blockers.
- Track the exact public export removal target:
  `packages/core/package.json` `exports["./state-machine"]`.
- Require a fresh source scan before any deletion attempt.
- Delete the TS state-machine implementation only after runtime-local graph
  orchestration is rerouted to Rust runtime ownership or retired.
- Update `scripts/check-boundaries.mjs` from the OSS repo root so removed
  package exports and deleted pure-core domains stay enforced.

## Scope

In scope:
- Future TS state-machine deletion plan.
- Future public export removal for `@runxhq/core/state-machine`.
- Boundary-check updates needed by that deletion.
- Importer verification and deletion gating.

Out of scope:
- Runtime-local graph orchestration migration itself.
- Fixture generator replacement or retirement implementation.
- Rust runtime authority/cutover implementation.
- Legacy import compatibility, package shims, or fallback adapters.

## Dependencies

- A Rust runtime cutover or retirement path for runtime-local graph
  orchestration.
- A fixture-generator ownership decision: keep TS oracle until final deletion,
  move generation to a new owner, or retire the generator with an explicit
  fixture-freeze policy.
- `rust-cli-rust-cutover` only if CLI/runtime execution still reaches
  runtime-local graph orchestration when this spec is reconsidered.
- A fresh importer scan immediately before approval.

## Open Questions

- Which spec owns rerouting runtime-local graph orchestration from the TS
  state-machine to Rust runtime-owned transitions?
- Which spec owns fixture generation after the TS oracle is deleted?
- Should `scripts/check-boundaries.mjs` gain a generic removed-export list so
  `./state-machine` removal is enforced the same way removed runtime-local
  subpaths are enforced today?
