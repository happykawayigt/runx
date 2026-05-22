---
spec_version: '2.0'
task_id: rust-ts-sunset-state-machine
created: '2026-05-18T00:00:00Z'
updated: '2026-05-22T01:11:59Z'
status: draft
harden_status: not_run
size: medium
risk_level: high
---

# TS sunset: state-machine

## Current State

Status: draft, blocked
Current phase: runtime-local importer child completed; deletion remains blocked
Next: resolve the remaining fixture-oracle, temporary type-parity test, package
export, and generated docs/API-surface references before approval.
Reason: the deletion remains blocked, but the runtime-local importer prerequisite
slice is now complete. A fresh 2026-05-22 importer scan finds zero
`packages/runtime-local/src` references to `@runxhq/core/state-machine`, one root
type-parity test importing the public export while the TS implementation still
exists, and two fixture generator scripts importing
`packages/core/src/state-machine/index.js` as the TypeScript oracle. Public
surface references also remain in `docs/api-surface.md`,
`scripts/gen-api-index.ts`, `docs/trusted-kernel-package-truth.md`, and
`packages/core/package.json` still exposes `./state-machine`. `crates/runx-core`
has state-machine fixture parity and runtime-local now calls the Rust kernel
bridge for state-machine operations, but final TS deletion still needs fixture
oracle ownership and public export/docs cleanup.
Blockers:
- `tests/runtime-local-state-machine-bridge-parity.test.ts` intentionally imports
  `@runxhq/core/state-machine` as a temporary type-parity guard while the TS
  implementation remains present.
- Fixture generation still imports `packages/core/src/state-machine/index.js`
  as the TypeScript oracle; fixture-generator ownership must be reassigned,
  rerouted, or retired before deletion.
- `packages/core/package.json` still exposes `./state-machine` as the public
  package export.
- `docs/api-surface.md` and `scripts/gen-api-index.ts` still reflect the stable
  public `@runxhq/core/state-machine` export; regenerate/remove those entries
  only after the package export is actually removed.
- Do not add legacy or compatibility shapes to keep the TS import path alive;
  the remaining work must remove or reroute the final public/test/script
  references.
Allowed follow-up command: `scafld validate rust-ts-sunset-state-machine --json`
while blocked; do not run `scafld harden` for this draft.
Latest runner update: 2026-05-22T01:11:59Z - importer census refreshed after
the runtime-local child completed. Deletion remains blocked by the temporary
type-parity test, public export/docs surfaces, and fixture-oracle ownership;
runtime-local source importers are no longer blockers.
Review gate: not_started

## Summary

Future deletion target: remove the TypeScript state-machine implementation and
its public `@runxhq/core/state-machine` export after all live consumers are no
longer depending on it. The current codebase is not there yet:
`crates/runx-core` implements Rust state-machine parity against
`fixtures/kernel/state-machine/`, and runtime-local now calls that Rust kernel
boundary for graph execution. Final deletion is still blocked by fixture-oracle,
type-parity, package export, and generated docs/API-surface references.

This spec must remain blocked until the fixture generators no longer depend on
the TS implementation as their oracle, the temporary type-parity test is removed
or rerouted, and the public package/docs/API surfaces no longer expose
`@runxhq/core/state-machine`.

## Completed Prerequisite Slice

This draft is not executable as a deletion spec today. The completed
runtime-local prerequisite slice removed the live runtime-local source imports
from `@runxhq/core/state-machine`.

Implemented shape:
- `packages/runtime-local/src/runner-local/kernel-bridge.ts` owns the explicit
  Rust kernel boundary for state-machine state, plan, transition, fanout sync,
  fanout decision key, and local admission calls.
- Runtime-local graph hydration, orchestration, fanout gates, governance,
  resume, run-step, run-fanout, terminal, paused, and package index paths use
  the kernel bridge surface instead of importing
  `@runxhq/core/state-machine`.
- Success transitions carry `admissionWitness` through the bridge, matching the
  Rust kernel requirement that success cannot be represented without an
  admission/receipt witness.
- Fixture generation remains on the TS oracle and is the next ownership
  decision before final deletion.

Focused regression validation for this slice:
- `pnpm exec vitest run packages/runtime-local/src/runner-local/kernel-bridge.test.ts`
- `pnpm exec tsc -p tsconfig.typecheck.json --noEmit`
- `rg -n "from ['\"]@runxhq/core/state-machine['\"]" packages/runtime-local/src --glob '!**/dist/**'`

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
- Kernel parity is not deletion completeness. This draft must not remove the TS
  package export until fixture-oracle, temporary type-parity, and docs/API
  surfaces are resolved.

Current TypeScript deletion targets:
- `packages/core/src/state-machine/**`
- `packages/core/package.json` export key `./state-machine`
- Generated API-surface entries that reflect the manifest export, if still
  present at execution time.
- `scripts/check-boundaries.mjs` from this CWD, not `oss/scripts/...`; remove
  `state-machine` from `pureCoreDomains` only when the TS domain is actually
  deleted and add/keep a boundary assertion that `./state-machine` is no longer
  exported.

Current live importers found in source by the 2026-05-22 scan after the
prerequisite slice:
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

Importer census command:

```bash
rg -n "from ['\"]@runxhq/core/state-machine['\"]|from ['\"].*state-machine/index\.js['\"]" packages/runtime-local/src tests scripts --glob '!**/dist/**' --glob '!node_modules' --glob '!target'
```

Observed result: 13 files and 13 import statements.

Runtime-local direct public-export importers:
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

Non-runtime-local live importers:
- `tests/graph-hydration-orphan-start.test.ts`
- `scripts/generate-kernel-parity-fixtures.ts`
- `scripts/generate-rust-fanout-fixtures.ts`

The current importer scan matches the expected post-prerequisite-slice shape:
`prepare-run.ts` and `orchestrator/run-context.ts` no longer import
`@runxhq/core/state-machine`, while the remaining synchronous consumers and TS
fixture oracle scripts still do.

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
- `rust-ts-sunset-state-machine-runtime-local-importers` for the runtime-local
  transition/planning importer migration plan.
- A fixture-generator ownership decision: keep TS oracle until final deletion,
  move generation to a new owner, or retire the generator with an explicit
  fixture-freeze policy.
- `rust-cli-rust-cutover` only if CLI/runtime execution still reaches
  runtime-local graph orchestration when this spec is reconsidered.
- A fresh importer scan immediately before approval.

## Open Questions

- Which implementation spec, after
  `rust-ts-sunset-state-machine-runtime-local-importers`, owns actual
  rerouting of runtime-local graph orchestration from the TS state-machine to
  Rust runtime-owned transitions?
- Which spec owns fixture generation after the TS oracle is deleted?
- Should `scripts/check-boundaries.mjs` gain a generic removed-export list so
  `./state-machine` removal is enforced the same way removed runtime-local
  subpaths are enforced today?
