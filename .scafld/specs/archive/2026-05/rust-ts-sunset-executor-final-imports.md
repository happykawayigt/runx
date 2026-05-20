---
spec_version: '2.0'
task_id: rust-ts-sunset-executor-final-imports
created: '2026-05-20T05:28:11Z'
updated: '2026-05-20T05:31:33Z'
status: completed
harden_status: not_run
size: medium
risk_level: medium
---

# rust-ts-sunset-executor-final-imports

## Current State

Status: completed
Current phase: final
Next: done
Reason: task completed
Blockers: none
Allowed follow-up command: `none`
Latest runner update: 2026-05-20T05:31:33Z
Review gate: pass

## Summary

Finish the TypeScript executor sunset once the runtime-local execution helper
and adapter interfaces have moved out of `@runxhq/core/executor`.

This is intentionally narrower than the failed full registry sunset. It removes
the remaining executor importers and stale export only; registry deletion remains
blocked until `@runxhq/core/registry` consumers are removed by separate slices.

## Objectives

- Remove source-relative imports of `packages/core/src/executor/index.ts` from
  fixture/oracle generation scripts.
- Remove the `packages/core/src/registry/http-client.ts` type import from
  `../executor/index.js`.
- Move the last executor-owned context contract assertion to
  `@runxhq/contracts`.
- Delete `packages/core/src/executor/**` and the `./executor` package export
  only after the import audit is clean.

## Scope

- No behavior changes to adapter invocation, tool catalog execution, or receipt
  envelope validation.
- No registry sunset. `@runxhq/core/registry` remains live until its own failed
  parent spec is split and fixed.

## Dependencies

- Previous completed slices:
  - `rust-ts-sunset-executor-runtime-interfaces`
  - `rust-ts-sunset-executor-runtime-local-execute-skill`

## Assumptions

- `scripts/generate-runtime-mcp-oracles.ts`
- `scripts/generate-agent-adapter-fixtures.ts`
- `scripts/generate-a2a-adapter-fixtures.ts`
- `scripts/generate-runtime-catalog-adapter-oracles.ts`
- `packages/core/src/registry/http-client.ts`
- `packages/core/src/executor/**`
- `packages/core/package.json`
- `packages/contracts/src/index.test.ts`

## Touchpoints

- Script imports can accidentally widen runtime-local public API; this slice
  uses existing contracts and runtime-local owned source modules only.
- Deleting the executor package export before import cleanup would break
  package consumers; the import audit is a hard acceptance gate.

## Risks

- `pnpm typecheck`
- `pnpm vitest run packages/contracts/src/index.test.ts --config vitest.config.ts`
- `pnpm vitest run tests/mcp-import.test.ts --config vitest.config.ts`
- `node scripts/check-boundaries.mjs`
- `! rg -n 'from "../packages/core/src/executor/index.js"|from "@runxhq/core/executor"|packages/core/src/executor|@runxhq/core/executor' scripts packages tests --glob '*.ts' --glob '*.tsx' --glob '*.mjs'`
- `test ! -d packages/core/src/executor`

## Acceptance

Profile: standard

Validation:
- none

## Phase 1: Implementation

Status: completed
Dependencies: none

Objective: Complete the requested change.

Changes:
- Point fixture/oracle generation scripts at `@runxhq/contracts` validators and runtime-local adapter/tool-catalog types.
- Localize core registry remote-tool types away from the executor package.
- Move context envelope assertions to contracts tests.
- Remove the stale executor source directory and package export after the audit.

Acceptance:
- [x] `ac1` command - Typecheck
  - Command: `pnpm typecheck`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-6
- [x] `ac2` command - Contract assertions
  - Command: `pnpm vitest run packages/contracts/src/index.test.ts --config vitest.config.ts`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-7
- [x] `ac3` command - Tool catalog smoke
  - Command: `pnpm vitest run tests/mcp-import.test.ts --config vitest.config.ts`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-8
- [x] `ac4` command - Executor importer audit
  - Command: `! rg -n 'from "../packages/core/src/executor/index.js"|from "@runxhq/core/executor"|packages/core/src/executor|@runxhq/core/executor' scripts packages tests --glob '*.ts' --glob '*.tsx' --glob '*.mjs'`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-9
- [x] `ac5` command - Deleted directory audit
  - Command: `test ! -d packages/core/src/executor`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-10

## Rollback

- Revert this slice by restoring `packages/core/src/executor/**`, the package
  export, and the four script imports. Registry sunset state is not affected.

## Review

Status: completed
Verdict: pass
Mode: verify
Summary: Human-reviewed override accepted: executor import audit clean; typecheck, focused vitest, boundary check, lockfile update, and diff check passed

Attack log:
- `review gate`: manual human audit -> clean (executor import audit clean; typecheck, focused vitest, boundary check, lockfile update, and diff check passed)

Findings:
- none

## Self Eval

- none

## Deviations

- none

## Metadata

- created_by: scafld

## Origin

Created by: scafld
Source: plan

## Harden Rounds

- none

## Planning Log

- none
