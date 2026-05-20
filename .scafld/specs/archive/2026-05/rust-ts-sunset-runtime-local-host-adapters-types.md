---
spec_version: '2.0'
task_id: rust-ts-sunset-runtime-local-host-adapters-types
created: '2026-05-20T08:25:11Z'
updated: '2026-05-20T08:28:56Z'
status: completed
harden_status: not_run
size: medium
risk_level: medium
---

# Rust TS sunset runtime-local host adapters types

## Current State

Status: completed
Current phase: final
Next: done
Reason: task completed
Blockers: none
Allowed follow-up command: `none`
Latest runner update: 2026-05-20T08:28:56Z
Review gate: pass

## Summary

Retarget host-adapter type ownership away from `@runxhq/runtime-local/sdk`.
The host-adapter package now owns structural host protocol types locally and
imports only stable resolution contracts from `@runxhq/contracts`. Runtime-local
SDK imports remain only where tests need runtime values for harness creation.

## Objectives

- Remove the `@runxhq/runtime-local` dependency from `@runxhq/host-adapters`.
- Keep host adapter execution and bridge tests behaviorally unchanged.
- Preserve runtime-local value imports in tests where they are not type-only.

## Scope

In scope:
- `packages/host-adapters` public type surface.
- Host adapter tests and shared test utilities.
- Package dependency metadata.

Out of scope:
- Removing `packages/runtime-local`.
- Rewriting host bridge execution.
- Rust CLI or provider runtime changes.

## Dependencies

- `rust-ts-sunset-runtime-local`.
- `rust-ts-sunset-parser-runtime-type-imports` for the runtime-local typecheck
  cleanup that unblocked final validation.

## Assumptions

- Host adapter users can consume structural host protocol types from
  `@runxhq/host-adapters` without importing runtime-local SDK types.

## Touchpoints

- `packages/host-adapters/src/index.ts`
- `packages/host-adapters/src/index.test.ts`
- `packages/host-adapters/package.json`
- `tests/host-protocol-test-utils.ts`
- `pnpm-lock.yaml`

## Risks

- Type-only retargeting can accidentally change public assignability.
  Mitigated by typecheck and host bridge tests.

## Acceptance

Profile: standard

Validation:
- `pnpm exec vitest run --config vitest.config.ts packages/host-adapters/src/index.test.ts tests/host-protocol.test.ts`
- `RUNX_KERNEL_EVAL_BIN=/Users/kam/dev/runx/runx/oss/crates/target/debug/runx pnpm exec vitest run --config vitest.config.ts tests/framework-bridge.test.ts`
- `pnpm exec tsc -p tsconfig.typecheck.json --noEmit`
- `rg -n "@runxhq/runtime-local/sdk|HostBridge|HostRunResult|HostRunState" packages/host-adapters tests/host-protocol-test-utils.ts tests/framework-bridge.test.ts`

## Phase 1: Implementation

Status: completed
Dependencies: none

Objective: Move type-only host-adapter imports off runtime-local SDK.

Changes:
- Added local structural host protocol types in `packages/host-adapters`.
- Replaced the host-adapter package dependency on `@runxhq/runtime-local` with
  `@runxhq/contracts`.
- Updated host-adapter tests and test utilities to import `HostBridge` from the
  host-adapter package surface.

Acceptance:
- [x] `ac1` command - host adapters no longer import runtime-local SDK.
  - Command: `! rg -n "@runxhq/runtime-local/sdk" packages/host-adapters`
  - Expected kind: `exit_code_zero`
  - Status: passed 2026-05-20
- [x] `ac2` command - host adapter tests pass.
  - Command: `pnpm exec vitest run --config vitest.config.ts packages/host-adapters/src/index.test.ts tests/host-protocol.test.ts`
  - Expected kind: `exit_code_zero`
  - Status: passed 2026-05-20
- [x] `ac3` command - framework bridge passes with Rust kernel binary.
  - Command: `RUNX_KERNEL_EVAL_BIN=/Users/kam/dev/runx/runx/oss/crates/target/debug/runx pnpm exec vitest run --config vitest.config.ts tests/framework-bridge.test.ts`
  - Expected kind: `exit_code_zero`
  - Status: passed 2026-05-20
- [x] `ac4` command - TypeScript typecheck passes.
  - Command: `pnpm exec tsc -p tsconfig.typecheck.json --noEmit`
  - Expected kind: `exit_code_zero`
  - Status: passed 2026-05-20

## Rollback

- Restore the `@runxhq/runtime-local` package dependency and runtime-local SDK
  type imports in `packages/host-adapters`.

## Review

Status: completed
Verdict: pass
Mode: verify
Summary: Human-reviewed override accepted: Reviewed host-adapter type retargeting after parser-type cleanup; validations pass, and tests/host-protocol-test-utils.ts is attributed to this runtime-local sunset slice.

Attack log:
- `review gate`: manual human audit -> clean (Reviewed host-adapter type retargeting after parser-type cleanup; validations pass, and tests/host-protocol-test-utils.ts is attributed to this runtime-local sunset slice.)

Findings:
- none

## Self Eval

- none

## Deviations

- none

## Metadata

- created_by: scafld
- completed_by: codex

## Origin

Created by: scafld
Source: plan

## Harden Rounds

- none
- 2026-05-20: Implemented host-adapter type retargeting and validated after
  `rust-ts-sunset-parser-runtime-type-imports` unblocked runtime-local
  typecheck errors.

## Planning Log

- none
