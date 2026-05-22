---
spec_version: '2.0'
task_id: rust-ts-sunset-state-machine-runtime-local-importers
created: '2026-05-22T00:58:00+10:00'
updated: '2026-05-22T01:11:59Z'
status: completed
harden_status: not_run
size: small
risk_level: high
---

# State-machine sunset: runtime-local importers

## Current State

Status: completed
Current phase: final
Next: done
Reason: task completed
Blockers: none
Allowed follow-up command: `none`
Latest runner update: 2026-05-22T01:11:59Z
Review gate: pass

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

Observed final results:
- 0 runtime-local source files import `@runxhq/core/state-machine`.
- 1 root type-parity test imports `@runxhq/core/state-machine`
  (`tests/runtime-local-state-machine-bridge-parity.test.ts`) to prove the
  bridge type surface remains assignable while the TS implementation still
  exists.
- 2 fixture generator scripts import `packages/core/src/state-machine/index.js`
  directly; final fixture-oracle ownership remains with the parent deletion
  spec.

Runtime-local migration result:
- `packages/runtime-local/src/runner-local/kernel-bridge.ts` owns the explicit
  Rust kernel boundary for state-machine state, plan, transition, fanout sync,
  fanout decision key, and local admission calls.
- The orchestrator, graph hydration, fanout gates, graph governance, resume,
  run-step, run-fanout, terminal, paused, and runtime-local index paths no
  longer import `@runxhq/core/state-machine`.
- Success transitions now carry `admissionWitness` through the bridge, matching
  the Rust kernel requirement that success cannot be represented without an
  admission/receipt witness.
- Deleting `packages/core/src/state-machine/**` remains out of scope until the
  parent deletion spec resolves the fixture generator imports and the temporary
  type-parity test.

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
  - Evidence: 2026-05-22T11:11:00+10:00 returned
    `{"ok":true,"command":"validate","result":{"task_id":"rust-ts-sunset-state-machine-runtime-local-importers","path":"/Users/kam/dev/runx/runx/oss/.scafld/specs/archive/2026-05/rust-ts-sunset-state-machine-runtime-local-importers.md","valid":true,"errors":null}}`.
- [x] `v2` Runtime-local importer census is current.
  - Command: `rg -n "from ['\"]@runxhq/core/state-machine['\"]" packages/runtime-local/src --glob '!**/dist/**'`
  - Expected kind: `no_matches`
  - Status: passed
  - Evidence: final check returned no matches, proving runtime-local source no
    longer imports `@runxhq/core/state-machine`.
- [x] `v3` TS state-machine implementation remains present.
  - Command: `test -d packages/core/src/state-machine`
  - Expected kind: `exit_code_zero`
  - Status: passed
  - Evidence: `test -d packages/core/src/state-machine` printed
    `state machine implementation present`.
- [x] `v4` Focused graph hydration coverage still passes.
  - Command: `pnpm exec vitest run --config vitest.config.ts tests/graph-hydration-orphan-start.test.ts`
  - Expected kind: `exit_code_zero`
  - Status: passed
  - Evidence: 2026-05-22T11:00:00+10:00 passed with 1 file and 4 tests when
    run with the kernel bridge suite.

## Phase 1: Importer Classification

Status: completed
Dependencies: none

Goal: map every runtime-local state-machine import to an ownership decision and
replacement strategy.

Acceptance:
- [x] `ac1` command - Runtime-local state-machine importer census is current.
  - Command: `rg -n "from ['\"]@runxhq/core/state-machine['\"]" packages/runtime-local/src --glob '!**/dist/**'`
  - Expected kind: `no_matches`
  - Status: passed
  - Evidence: final command returned no matches.
- [x] `ac2` command - Importer assignments are recorded in this spec.
  - Command: `rg -n "Runtime-local migration result:" .scafld/specs/archive/2026-05/rust-ts-sunset-state-machine-runtime-local-importers.md`
  - Expected kind: `exit_code_zero`
  - Status: passed
  - Evidence: importer assignments are recorded under `Runtime-local migration result:`.

## Phase 2: Safe Migration

Status: completed
Dependencies: Phase 1

Objective: Complete this phase.

Changes:
- none

Acceptance:
- [x] `ac3` command - Migrated runtime-local imports no longer reference the
  - Command: `rg -n "from ['\"]@runxhq/core/state-machine['\"]" packages/runtime-local/src --glob '!**/dist/**'`
  - Expected kind: `no_matches`
  - Status: pass
  - Evidence: output was empty
  - Source event: entry-3
- [x] `ac4` command - TS state-machine implementation is untouched.
  - Command: `test -d packages/core/src/state-machine`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-4

## Rollback

- Revert only this planning or importer migration slice. Do not restore, remove,
  or replace TS state-machine implementation files from this spec.

## Metadata

- created_by: codex

## Review

Status: completed
Verdict: pass
Mode: verify
Provider: command
Output: command.stdout
Summary: Command review verified no runtime-local source references to @runxhq/core/state-machine and confirmed the TS state-machine implementation remains present.

Attack log:
- `packages/runtime-local/src`: state-machine import census -> clean (no @runxhq/core/state-machine references)
- `packages/core/src/state-machine`: implementation preservation -> clean (implementation directory present)
- `recorded validation`: verify targeted checks were run by operator -> clean (operator ran pnpm typecheck, focused vitest bridge/hydration/parity tests, cargo state_machine tests, and payment ledger projection tests before this review)

Findings:
- none
