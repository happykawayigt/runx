---
spec_version: '2.0'
task_id: rust-ts-sunset-executor
created: '2026-05-18T00:00:00Z'
updated: '2026-05-20T05:34:01Z'
status: completed
harden_status: not_run
size: medium
risk_level: high
---

# TS sunset: executor

## Current State

Status: completed
Current phase: final
Next: done
Reason: task completed
Blockers: none
Allowed follow-up command: `none`
Latest runner update: 2026-05-20T05:34:01Z
Review gate: pass

## Summary

Close the TypeScript executor sunset parent. The old
`@runxhq/core/executor` surface is gone: package imports were moved to
`@runxhq/contracts`, runtime-local owns adapter invocation types and
`executeSkill`, fixture/oracle scripts import the owning packages directly, and
the stale `packages/core/src/executor/**` source directory plus `./executor`
package export have been removed.

This spec does not delete registry, runtime-local, adapters, parser,
state-machine, or marketplace surfaces. Those remain separate sunset tracks.

## Context

CWD: `.`

Packages:
- `@runxhq/core`
- `@runxhq/contracts`
- `@runxhq/runtime-local`
- `@runxhq/adapters`

Completed child slices:
- `rust-ts-sunset-executor-contract-aliases`
- `rust-ts-sunset-executor-contract-imports-2`
- `rust-ts-sunset-executor-runtime-interfaces`
- `rust-ts-sunset-executor-runtime-local-execute-skill`
- `rust-ts-sunset-executor-final-imports`

Final state:
- `packages/core/package.json` no longer exports `./executor`.
- `packages/core/src/executor/` no longer exists.
- No live TS/script/test import references `@runxhq/core/executor` or
  `packages/core/src/executor`.
- Runtime-local adapter interfaces live under `@runxhq/runtime-local`.
- Contract validators and envelope types live under `@runxhq/contracts`.
- Adapters now declare their direct `@runxhq/contracts` dependency.

## Objectives

- Verify the executor package surface is fully removed.
- Verify moved imports typecheck and obey package boundaries.
- Keep registry and runtime-local sunset work separate from executor closeout.

## Scope

In scope:
- Parent closeout validation for the executor sunset.
- Import/export/directory audits proving the old executor surface is gone.

Out of scope:
- Registry deletion or registry importer cleanup.
- Runtime-local/adapters deletion.
- Approval contract changes.
- New Rust runtime behavior.

## Dependencies

- Completed executor child slices listed above.

## Assumptions

- Archived specs may mention historical `@runxhq/core/executor` references as
  audit evidence; active code and active specs must not depend on them.

## Touchpoints

- `packages/core/package.json`
- `packages/core/src/executor/**`
- `packages/contracts/src/index.test.ts`
- fixture/oracle generation scripts
- boundary checks for direct workspace package dependencies

## Risks

- Medium: stale imports can survive in scripts because they are not always part
  of focused runtime tests. Mitigation: broad import audit over scripts,
  packages, and tests.
- Low: direct adapter imports from `@runxhq/contracts` can drift without package
  manifest declaration. Mitigation: `node scripts/check-boundaries.mjs`.

## Acceptance

Profile: standard

Validation:
- `pnpm typecheck`
- `pnpm vitest run packages/contracts/src/index.test.ts --config vitest.config.ts`
- `pnpm vitest run tests/mcp-import.test.ts --config vitest.config.ts`
- `node scripts/check-boundaries.mjs`
- `test ! -d packages/core/src/executor`
- `! rg -n 'from "../packages/core/src/executor/index.js"|from "@runxhq/core/executor"|packages/core/src/executor|@runxhq/core/executor' scripts packages tests .scafld/specs/drafts --glob '*.ts' --glob '*.tsx' --glob '*.mjs' --glob '*.md'`

## Phase 1: Closeout Verification

Status: completed
Dependencies: none

Objective: Verify the executor surface has been removed without preserving a

Changes:
- No new implementation beyond completed child slices.
- Parent spec records the final state and runs closeout audits.

Acceptance:
- [x] `ac1` command - TypeScript typecheck remains green.
  - Command: `pnpm typecheck`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-6
- [x] `ac2` command - Contract assertions remain green.
  - Command: `pnpm vitest run packages/contracts/src/index.test.ts --config vitest.config.ts`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-7
- [x] `ac3` command - Runtime tool-catalog smoke remains green.
  - Command: `pnpm vitest run tests/mcp-import.test.ts --config vitest.config.ts`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-8
- [x] `ac4` command - Package boundaries declare direct dependencies.
  - Command: `node scripts/check-boundaries.mjs`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-9
- [x] `ac5` command - Executor directory is removed.
  - Command: `test ! -d packages/core/src/executor`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-10
- [x] `ac6` command - No live executor imports remain.
  - Command: `! rg -n 'from "../packages/core/src/executor/index.js"|from "@runxhq/core/executor"|packages/core/src/executor|@runxhq/core/executor' scripts packages tests .scafld/specs/drafts --glob '*.ts' --glob '*.tsx' --glob '*.mjs' --glob '*.md'`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-11

## Rollback

- Restore the archived child-slice changes that removed `packages/core/src/executor/**`
  and `./executor` if a downstream consumer proves the surface still exists.
  Do not add an executor compatibility proxy.

## Review

Status: completed
Verdict: pass
Mode: verify
Summary: Human-reviewed override accepted: parent closeout verified: no executor package/export/source/imports remain; typecheck, focused vitest, boundary check, and audits passed

Attack log:
- `review gate`: manual human audit -> clean (parent closeout verified: no executor package/export/source/imports remain; typecheck, focused vitest, boundary check, and audits passed)

Findings:
- none

## Self Eval

- none

## Deviations

- none

## Metadata

- created_by: codex

## Origin

Created by: scafld
Source: plan

## Harden Rounds

- none

## Planning Log

- 2026-05-20T05:34:00Z: Refreshed from stale draft into parent closeout after
  `rust-ts-sunset-executor-final-imports` removed the final importers, source
  directory, and package export.
