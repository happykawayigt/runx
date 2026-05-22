---
spec_version: '2.0'
task_id: rust-ts-sunset-state-machine
created: '2026-05-18T00:00:00Z'
updated: '2026-05-22T01:54:37Z'
status: completed
harden_status: not_run
size: medium
risk_level: high
---

# TS sunset: state-machine

## Current State

Status: completed
Current phase: final deletion executed
Next: done
Reason: task completed
Blockers: none
Allowed follow-up command: `none`
Latest runner update: 2026-05-22T01:54:37Z
Review gate: pass

## Summary

Delete `packages/core/src/state-machine/**` and make Rust the only
state-machine execution authority. Surviving TypeScript code may define local
wire types at a boundary, but it must not import or re-export a TypeScript
state-machine kernel.

## Implemented Shape

- Runtime-local graph orchestration uses
  `packages/runtime-local/src/runner-local/kernel-bridge.ts`, which shells to
  `runx kernel eval` for state, plan, transition, fanout sync, fanout decision
  key, and local admission operations.
- Fixture generators now use `scripts/rust-kernel-eval.ts` for
  `state-machine.*` fixture cases instead of importing
  `packages/core/src/state-machine/index.js`.
- `scripts/check-boundaries.mjs` rejects `@runxhq/core/state-machine` imports,
  rejects a restored `packages/core/package.json` `./state-machine` export, and
  rejects a restored `packages/core/src/state-machine` directory.
- `tests/runtime-local-state-machine-bridge-parity.test.ts` no longer compares
  against a deleted TypeScript public export. It instead typechecks the
  Rust-owned runtime-local bridge boundary and proves success events are
  non-empty and carry the required admission witness.

## Scope

In scope:
- `packages/core/src/state-machine/**`
- `packages/core/package.json`
- `docs/api-surface.md`
- `docs/trusted-kernel-package-truth.md`
- `scripts/gen-api-index.ts`
- `scripts/check-boundaries.mjs`
- `scripts/generate-kernel-parity-fixtures.ts`
- `scripts/generate-rust-fanout-fixtures.ts`
- `scripts/rust-kernel-eval.ts`
- `tests/runtime-local-state-machine-bridge-parity.test.ts`

Out of scope:
- Runtime-local package deletion.
- Policy TS package deletion.
- Legacy compatibility aliases or shims.
- Rust state-machine algorithm redesign.

## Invariants

- No consumer may import `@runxhq/core/state-machine`.
- No consumer may import `packages/core/src/state-machine/index.js`.
- `packages/core/package.json` must not export `./state-machine`.
- `packages/core/src/state-machine/` must not exist after this deletion.
- Fixture refreshes must use the Rust kernel eval boundary for
  `state-machine.*` cases.
- Success transitions remain receipt/admission-witness backed at the Rust
  boundary.

## Acceptance

Profile: strict

Definition of done:
- [x] `dod1` The public `@runxhq/core/state-machine` export is removed with no
  compatibility alias.
- [x] `dod2` TypeScript state-machine implementation files are deleted.
- [x] `dod3` Fixture generators no longer import the deleted TypeScript oracle.
- [x] `dod4` Boundary checks enforce the removed export, removed import path,
  and deleted source directory.
- [x] `dod5` Runtime-local bridge type coverage remains non-vacuous without
  importing the deleted public export.

Validation:
- [x] `v1` Scafld validates this spec.
  - Command: `scafld validate rust-ts-sunset-state-machine --json`
  - Expected kind: `exit_code_zero`
  - Status: passed
  - Evidence: 2026-05-22T11:50+10:00 returned
    `{"ok":true,...,"valid":true}`.
- [x] `v2` State-machine importer census is clean.
  - Command: `bash -lc '! rg -n "from ['\"'\"']@runxhq/core/state-machine['\"'\"']|from ['\"'\"'].*state-machine/index\\.js['\"'\"']" packages/runtime-local/src tests scripts --glob "!**/dist/**" --glob "!node_modules" --glob "!target"'`
  - Expected kind: `no_matches`
  - Status: passed
  - Evidence: 2026-05-22T11:50+10:00 returned no matches and exited zero.
- [x] `v3` Boundary checks pass.
  - Command: `node scripts/check-boundaries.mjs`
  - Expected kind: `exit_code_zero`
  - Status: passed
  - Evidence: 2026-05-22T11:52+10:00 printed `Boundary check passed.`
- [x] `v4` TypeScript typecheck passes.
  - Command: `pnpm exec tsc -p tsconfig.typecheck.json --noEmit --pretty false`
  - Expected kind: `exit_code_zero`
  - Status: passed
  - Evidence: 2026-05-22T11:52+10:00 exited zero.
- [x] `v5` Focused TypeScript regression tests pass.
  - Command: `RUNX_RUST_CLI_BIN=crates/target/debug/runx RUNX_KERNEL_EVAL_BIN=crates/target/debug/runx pnpm exec vitest run --config vitest.config.ts tests/runtime-local-state-machine-bridge-parity.test.ts tests/upstream-binding.test.ts tests/kernel-parity-fixtures.test.ts tests/caller-approval-boundary.test.ts tests/ide-plugin-actions.test.ts --fileParallelism=false --maxWorkers=1 --testTimeout=60000`
  - Expected kind: `exit_code_zero`
  - Status: passed
  - Evidence: 2026-05-22T11:52+10:00 passed 5 files and 19 tests.
- [x] `v6` Rust kernel eval tests pass.
  - Command: `cargo test --manifest-path crates/Cargo.toml -p runx-core kernel_eval -- --nocapture`
  - Expected kind: `exit_code_zero`
  - Status: passed
  - Evidence: 2026-05-22T11:50+10:00 passed `kernel_eval` unit coverage;
    `cargo test --manifest-path crates/Cargo.toml -p runx-core --test kernel_eval -- --nocapture`
    also passed 6 integration tests.
- [x] `v7` Kernel parity fixtures are Rust-generated and current.
  - Command: `pnpm exec tsx scripts/generate-kernel-parity-fixtures.ts --check`
  - Expected kind: `exit_code_zero`
  - Status: passed
  - Evidence: 2026-05-22T11:52+10:00 checked 65 kernel parity fixtures.

## Rollback

- Restore this spec to blocked, restore `packages/core/src/state-machine/**`,
  restore the public export only if a live importer regression is proven, and
  remove the boundary enforcement that assumes deletion. Do not add a shim or
  compatibility package.

## Review

Status: completed
Verdict: pass
Mode: verify
Summary: Human-reviewed override accepted: Read-only subagent audit plus local validation: scafld validate, clean state-machine import census, boundary check, kernel fixture check, focused Vitest with Rust CLI/kernel env, and runx-core kernel_eval tests passed.

Attack log:
- `review gate`: manual human audit -> clean (Read-only subagent audit plus local validation: scafld validate, clean state-machine import census, boundary check, kernel fixture check, focused Vitest with Rust CLI/kernel env, and runx-core kernel_eval tests passed.)

Findings:
- none

