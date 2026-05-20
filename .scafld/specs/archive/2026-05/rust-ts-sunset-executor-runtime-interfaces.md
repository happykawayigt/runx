---
spec_version: '2.0'
task_id: rust-ts-sunset-executor-runtime-interfaces
created: '2026-05-20T05:02:40Z'
updated: '2026-05-20T05:11:40Z'
status: completed
harden_status: not_run
size: medium
risk_level: low
---

# Executor sunset: runtime interface ownership

## Current State

Status: completed
Current phase: final
Next: done
Reason: task completed
Blockers: none
Allowed follow-up command: `none`
Latest runner update: 2026-05-20T05:11:40Z
Review gate: pass

## Summary

Move the public TypeScript runtime adapter and tool-catalog interface ownership
out of `@runxhq/core/executor` and into `@runxhq/runtime-local`, then point
adapter/runtime consumers at the runtime-owned surface. Leave `executeSkill`,
executor validators, core registry consumers, registry specs, and Nitrosend
target/observer specs untouched.

## Objectives

- Add runtime-local owned definitions for `SkillAdapter`,
  `AdapterActInvocation`, `ActReceiptEnvelope`, nested skill invocation types,
  and tool-catalog interfaces.
- Update non-registry adapter/runtime importers that only need those interfaces
  to import from `@runxhq/runtime-local` or runtime-local internals.
- Keep the remaining executor imports classified as value/validator/registry
  blockers for a later executor sunset.

## Scope

In scope:
- `packages/runtime-local/src/runner-local/adapter-types.ts`
- `packages/runtime-local/src/runner-local/index.ts`
- `packages/runtime-local/src/runner-local/caller-adapters.ts`
- `packages/runtime-local/src/harness/agent-hook.ts`
- `packages/runtime-local/src/harness/runner.ts`
- `packages/runtime-local/src/sdk/index.ts`
- `packages/runtime-local/src/tool-catalogs/index.ts`
- `packages/adapters/src/**` type-only executor imports
- Focused tests that define local `SkillAdapter` fixtures

Out of scope:
- Registry files, including `packages/core/src/registry/**`.
- Nitrosend target/observer specs.
- Removing `packages/core/src/executor/**`.
- Removing the `@runxhq/core/executor` package export.
- Moving the `executeSkill` implementation or executor validator functions.
- Running `scafld harden`.

## Dependencies

- `@runxhq/runtime-local` already depends on `@runxhq/core` and
  `@runxhq/contracts`, so it can define interfaces using parser and contract
  types without changing package dependencies.
- `@runxhq/adapters` already depends on `@runxhq/runtime-local`, so type-only
  imports can move there without dependency graph churn.

## Assumptions

- TypeScript structural compatibility lets runtime-local owned adapter
  interfaces pass through the legacy `executeSkill` boundary until that value
  function moves in a later slice.
- Existing dirty workspace changes belong to parallel workers and must not be
  reverted or normalized by this slice.

## Touchpoints

- Public runtime-local adapter type exports.
- Adapter package type-only imports.
- Runtime-local tool-catalog interfaces.
- Tests that construct local adapter fixtures.

## Risks

- Accidentally widening into registry cleanup would collide with the active
  registry spec. Mitigation: do not edit registry files; classify registry
  imports as remaining blockers.
- Moving `executeSkill` here would require broader runtime behavior validation.
  Mitigation: keep `executeSkill` imported from core executor and only move
  type ownership.
- Runtime-local self-import cycles could appear if tool-catalog types import
  runner-local types. Mitigation: keep tool-catalog interfaces independent and
  import them type-only from adapter types.

## Acceptance

Profile: standard

Validation:
- `rg 'from "@runxhq/core/executor"|from '\''@runxhq/core/executor'\''' packages tests scripts --glob '*.ts' --glob '*.tsx'`
- `pnpm typecheck`
- `pnpm vitest run packages/adapters/src/runtime.test.ts packages/runtime-local/src/runner-local/voice-profile.test.ts packages/runtime-local/src/tool-catalogs/index.test.ts`
- `pnpm vitest run tests/graph-fanout.test.ts tests/graph-runner-governance.test.ts tests/graph-retry-idempotency.test.ts tests/history-inspect.test.ts tests/run-diff.test.ts tests/merge-metadata.test.ts --testTimeout 15000`
- `git diff --check -- packages/runtime-local/src/runner-local/adapter-types.ts packages/runtime-local/src/runner-local/index.ts packages/runtime-local/src/runner-local/caller-adapters.ts packages/runtime-local/src/harness/agent-hook.ts packages/runtime-local/src/harness/runner.ts packages/runtime-local/src/sdk/index.ts packages/runtime-local/src/tool-catalogs/index.ts packages/adapters/src tests/graph-fanout.test.ts tests/graph-runner-governance.test.ts tests/graph-retry-idempotency.test.ts tests/history-inspect.test.ts tests/run-diff.test.ts tests/merge-metadata.test.ts .scafld/specs/active/rust-ts-sunset-executor-runtime-interfaces.md`

## Phase 1: Implementation

Status: completed
Dependencies: none

Objective: Complete the requested change.

Changes:
- Define runtime-local owned adapter interface types.
- Define runtime-local owned tool-catalog interface types.
- Update non-registry type-only importers from `@runxhq/core/executor` to the runtime-local owned surfaces.
- Re-run the executor importer audit and classify remaining blockers.

Acceptance:
- [x] `ac1` command - Remaining executor importer audit
  - Command: `rg 'from "@runxhq/core/executor"|from '\''@runxhq/core/executor'\''' packages tests scripts --glob '*.ts' --glob '*.tsx'`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-6
- [x] `ac2` command - TypeScript typecheck
  - Command: `pnpm typecheck`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-7
- [x] `ac3` command - Focused runtime adapter tests
  - Command: `pnpm vitest run packages/adapters/src/runtime.test.ts packages/runtime-local/src/runner-local/voice-profile.test.ts packages/runtime-local/src/tool-catalogs/index.test.ts`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-8
- [x] `ac4` command - Focused adapter fixture tests
  - Command: `pnpm vitest run tests/graph-fanout.test.ts tests/graph-runner-governance.test.ts tests/graph-retry-idempotency.test.ts tests/history-inspect.test.ts tests/run-diff.test.ts tests/merge-metadata.test.ts --testTimeout 15000`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-9
- [x] `ac5` command - Whitespace check for touched files
  - Command: `git diff --check -- packages/runtime-local/src/runner-local/adapter-types.ts packages/runtime-local/src/runner-local/index.ts packages/runtime-local/src/runner-local/caller-adapters.ts packages/runtime-local/src/harness/agent-hook.ts packages/runtime-local/src/harness/runner.ts packages/runtime-local/src/sdk/index.ts packages/runtime-local/src/tool-catalogs/index.ts packages/adapters/src tests/graph-fanout.test.ts tests/graph-runner-governance.test.ts tests/graph-retry-idempotency.test.ts tests/history-inspect.test.ts tests/run-diff.test.ts tests/merge-metadata.test.ts .scafld/specs/active/rust-ts-sunset-executor-runtime-interfaces.md`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-10

## Rollback

- Revert the new runtime-local interface module and importer rewrites. Since
  this slice keeps the legacy executor implementation in place, rollback does
  not require behavior migration.

## Review

Status: completed
Verdict: pass
Mode: verify
Summary: Human-reviewed override accepted: Reviewed completed runtime interface ownership slice; acceptance commands are recorded passing, remaining executor audit is limited to legacy executeSkill/core executor export, and scope avoided registry/Nitrosend files.

Attack log:
- `review gate`: manual human audit -> clean (Reviewed completed runtime interface ownership slice; acceptance commands are recorded passing, remaining executor audit is limited to legacy executeSkill/core executor export, and scope avoided registry/Nitrosend files.)

Findings:
- none

## Self Eval

- Passing means public adapter/runtime type consumers no longer depend on
  `@runxhq/core/executor`, and the remaining executor import audit clearly
  identifies only the legacy implementation, validators, scripts, and
  off-limits registry blockers.

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
