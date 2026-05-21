---
spec_version: '2.0'
task_id: rust-ts-sunset-state-machine-runtime-local-importers
created: '2026-05-22T00:58:00+10:00'
updated: '2026-05-22T01:32:00+10:00'
status: active
harden_status: not_run
size: small
risk_level: high
---

# State-machine sunset: runtime-local importers

## Current State

Status: active
Current phase: importer classification complete; migration blocked on Rust
runtime graph boundary
Next: create a Rust runtime graph transition/planning boundary or retire
runtime-local before moving production imports
Reason: `rust-ts-sunset-state-machine` is blocked by live runtime-local
state-machine consumers. The completed prerequisite slice moved only
sequential graph state creation for `prepare-run.ts`; the remaining consumers
are synchronous transition, planning, fanout, hydration, governance, and test
paths that cannot be removed by deleting the TS state-machine package.
Blockers: runtime ownership for transition/planning semantics is not settled,
fanout gate/governance shape ownership is still coupled to TS types, and the
fixture generators still use the TS implementation as their oracle.
Allowed follow-up command: `scafld validate rust-ts-sunset-state-machine-runtime-local-importers --json`
Latest runner update: 2026-05-22T01:32:00+10:00 recorded the importer
classification and validated the fresh census. No production imports moved:
remaining callers own live transition/planning/fanout semantics and need a
Rust runtime graph boundary rather than a compatibility shim.
Review gate: classification_recorded; migration_blocked_on_runtime_boundary

## Summary

Plan the runtime-local importer migration needed before the parent
`rust-ts-sunset-state-machine` deletion draft can advance. This is not a
deletion spec. It must not remove `packages/core/src/state-machine/**`, must
not remove `packages/core/package.json` `exports["./state-machine"]`, and must
not add a compatibility shim for `@runxhq/core/state-machine`.

The safe migration shape is expected to be incremental:
- keep the existing kernel bridge for any operation that can tolerate async
  `runx kernel eval`;
- create an explicit Rust runtime-owned boundary for transition/planning
  operations that cannot be safely handled by one-off kernel calls;
- leave fixture-oracle ownership to the parent or a separate fixture-freeze
  decision.

## Objectives

- Classify each runtime-local `@runxhq/core/state-machine` importer by
  behavior: planning, transition, fanout decision, fanout key, hydration,
  governance type surface, and single-step state carrier.
- Identify the smallest safe runtime-local importer migrations that have
  obvious targeted tests.
- Keep deletion blocked until a fresh parent census proves all live imports are
  gone.
- Avoid production import churn unless the replacement boundary is explicit and
  covered by focused tests.

## Scope

In scope:
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
- `tests/graph-hydration-orphan-start.test.ts`, only as validation or follow-up
  cleanup for migrated runtime-local behavior.

Out of scope:
- Deleting `packages/core/src/state-machine/**`.
- Removing `packages/core/package.json` `exports["./state-machine"]`.
- Changing fixture generators or fixture ownership.
- Payments, MCP, target-runner, post-merge observer, embedded-sdk,
  TS-boundary, parser/runtime-local, external-adapter, and rust-dev work.

## Dependencies

- `rust-ts-sunset-state-machine` remains the deletion parent and stays blocked.
- A Rust runtime ownership decision for synchronous graph transition/planning.
- A separate fixture-generator ownership or freeze decision before final TS
  state-machine deletion.

## Importer Census

Checked on 2026-05-22:

```bash
rg -n "from ['\"]@runxhq/core/state-machine['\"]|from ['\"].*state-machine/index\.js['\"]" packages/runtime-local/src tests scripts --glob '!**/dist/**' --glob '!node_modules' --glob '!target'
rg -l "from ['\"]@runxhq/core/state-machine['\"]" packages/runtime-local/src --glob '!**/dist/**' | sort
```

Observed results:
- 13 live importer files across runtime-local, tests, and scripts.
- 10 runtime-local source files import `@runxhq/core/state-machine`.
- One root graph hydration test imports `@runxhq/core/state-machine`.
- Two fixture generator scripts import `packages/core/src/state-machine/index.js`
  directly.

Runtime-local importers:
- `packages/runtime-local/src/runner-local/orchestrator.ts`
  - Imports: `planSequentialGraphTransition`.
  - Behavior: core graph planning loop.
  - Owner/replacement: Rust runtime graph boundary. Kernel bridge is too
    expensive and async for every loop iteration unless the entire orchestrator
    is rewritten around it.
  - Status: blocked.
- `packages/runtime-local/src/runner-local/index.ts`
  - Imports: state and transition types plus graph status used by exported
    runtime-local API shapes.
  - Behavior: public runtime-local graph run result and resume state surface.
  - Owner/replacement: runtime-local package retirement or a new contracts-owned
    graph run-result surface. Do not re-export TS state-machine types from a new
    compatibility module.
  - Status: blocked.
- `packages/runtime-local/src/runner-local/graph-hydration.ts`
  - Imports: `transitionSequentialGraph`, `SequentialGraphPlan`,
    `SequentialGraphState`.
  - Behavior: ledger hydration and retry/failure reconstruction.
  - Owner/replacement: Rust runtime graph boundary that can replay ledger
    entries into graph state, or keep with runtime-local until package sunset.
  - Status: blocked.
- `packages/runtime-local/src/runner-local/graph-fanout-gates.ts`
  - Imports: `FanoutSyncDecision` type only.
  - Behavior: approval request and receipt metadata projection for fanout gates.
  - Owner/replacement: contracts-owned fanout decision shape once Rust runtime
    emits that shape. Safe to move later as a type-only slice.
  - Status: blocked pending contracts-owned fanout decision.
- `packages/runtime-local/src/runner-local/graph-governance.ts`
  - Imports: `FanoutSyncDecision` type only.
  - Behavior: receipt sync-point projection.
  - Owner/replacement: same contracts-owned fanout decision shape as
    `graph-fanout-gates.ts`.
  - Status: blocked pending contracts-owned fanout decision.
- `packages/runtime-local/src/runner-local/orchestrator/hydrate-resume.ts`
  - Imports: `transitionSequentialGraph`, `SequentialGraphState`.
  - Behavior: resume from paused fanout gate and persisted ledger state.
  - Owner/replacement: Rust runtime resume boundary; cannot be replaced by a
    local type alias because it mutates live graph state.
  - Status: blocked.
- `packages/runtime-local/src/runner-local/orchestrator/handle-run-step.ts`
  - Imports: `transitionSequentialGraph`, `SequentialGraphPlan`.
  - Behavior: single-step transition after admission, execution, retry, and
    receipt writing.
  - Owner/replacement: Rust runtime step execution/transition boundary.
  - Status: blocked.
- `packages/runtime-local/src/runner-local/orchestrator/handle-run-fanout.ts`
  - Imports: `evaluateFanoutSync`, `planSequentialGraphTransition`,
    `transitionSequentialGraph`, `SequentialGraphPlan`.
  - Behavior: fanout branch execution, sync-gate planning, and follow-up graph
    planning.
  - Owner/replacement: Rust runtime fanout planning boundary; no safe local
    helper because branch/gate semantics are load-bearing.
  - Status: blocked.
- `packages/runtime-local/src/runner-local/orchestrator/handle-terminal.ts`
  - Imports: `transitionSequentialGraph`, `SequentialGraphPlan`.
  - Behavior: terminal graph transition and fanout escalation receipt state.
  - Owner/replacement: Rust runtime graph termination boundary.
  - Status: blocked.
- `packages/runtime-local/src/runner-local/orchestrator/handle-paused.ts`
  - Imports: `fanoutSyncDecisionKey`, `transitionSequentialGraph`,
    `SequentialGraphPlan`.
  - Behavior: persisted fanout pause/resume state and pending resolution
    ledger entries.
  - Owner/replacement: Rust runtime fanout pause/resume boundary plus
    contracts-owned fanout decision key.
  - Status: blocked.

## Acceptance

Profile: standard

Definition of done:
- [x] `dod1` Runtime-local state-machine importers are classified with explicit
  owner, replacement boundary, and test target.
- [x] `dod2` No TS state-machine implementation files are deleted or renamed.
- [x] `dod3` No compatibility shim, re-export, fallback adapter, or legacy
  `@runxhq/core/state-machine` shape is added.
- [x] `dod4` Parent deletion draft remains blocked until runtime-local and
  fixture-oracle imports are cleared.

Validation:
- [x] `v1` Scafld validates this spec.
  - Command: `scafld validate rust-ts-sunset-state-machine-runtime-local-importers --json`
  - Expected kind: `exit_code_zero`
  - Status: passed
  - Evidence: 2026-05-22T01:32:00+10:00 returned
    `{"ok":true,"command":"validate","result":{"task_id":"rust-ts-sunset-state-machine-runtime-local-importers","path":"/Users/kam/dev/runx/runx/oss/.scafld/specs/active/rust-ts-sunset-state-machine-runtime-local-importers.md","valid":true,"errors":null}}`.
- [x] `v2` Runtime-local importer census is current.
  - Command: `rg -n "from ['\"]@runxhq/core/state-machine['\"]" packages/runtime-local/src --glob '!**/dist/**'`
  - Expected kind: `exit_code_zero`
  - Status: passed
  - Evidence: 2026-05-22T01:32:00+10:00 listed the 10 runtime-local importer
    files recorded above.
- [x] `v3` TS state-machine implementation remains present.
  - Command: `test -d packages/core/src/state-machine`
  - Expected kind: `exit_code_zero`
  - Status: passed
  - Evidence: `test -d packages/core/src/state-machine` printed
    `state machine implementation present`.

## Phase 1: Importer Classification

Status: completed
Dependencies: none

Goal: map every runtime-local state-machine import to an ownership decision and
replacement strategy.

Acceptance:
- [x] `ac1` command - Runtime-local state-machine importer census is current.
  - Command: `rg -n "from ['\"]@runxhq/core/state-machine['\"]" packages/runtime-local/src --glob '!**/dist/**'`
  - Expected kind: `exit_code_zero`
  - Status: passed
  - Evidence: the command listed the 10 runtime-local importers recorded in
    this spec.
- [x] `ac2` command - Importer assignments are recorded in this spec.
  - Command: `rg -n "Runtime-local importers:" .scafld/specs/active/rust-ts-sunset-state-machine-runtime-local-importers.md`
  - Expected kind: `exit_code_zero`
  - Status: passed
  - Evidence: importer assignments are recorded under `Runtime-local importers:`.

## Phase 2: Safe Migration

Status: blocked
Dependencies: Phase 1

Goal: move only importer classes with an explicit Rust-owned boundary and
focused tests.

Acceptance:
- [ ] `ac3` command - Migrated runtime-local imports no longer reference the
  TS public export.
  - Command: `rg -n "from ['\"]@runxhq/core/state-machine['\"]" packages/runtime-local/src --glob '!**/dist/**'`
  - Expected kind: `no_matches`
  - Status: pending
  - Evidence: none
- [ ] `ac4` command - TS state-machine implementation is untouched.
  - Command: `test -d packages/core/src/state-machine`
  - Expected kind: `exit_code_zero`
  - Status: pending
  - Evidence: none

## Rollback

- Revert only this planning or importer migration slice. Do not restore, remove,
  or replace TS state-machine implementation files from this spec.

## Metadata

- created_by: codex
