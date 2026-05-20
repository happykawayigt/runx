---
spec_version: '2.0'
task_id: rust-ts-sunset-executor-contract-imports-2
created: '2026-05-20T04:55:12Z'
updated: '2026-05-20T04:58:22Z'
status: completed
harden_status: not_run
size: small
risk_level: low
---

# Executor sunset: second contract import cleanup

## Current State

Status: completed
Current phase: final
Next: done
Reason: task completed
Blockers: none
Allowed follow-up command: `none`
Latest runner update: 2026-05-20T04:58:22Z
Review gate: pass

## Summary

Ledger the already-present second executor import cleanup: pure contract imports move to @runxhq/contracts while runtime adapter interfaces stay on @runxhq/core/executor, then refresh the broad executor sunset importer census.

## Objectives

- Ledger the already-present second pure-contract import cleanup.
- Keep runtime adapter interfaces on `@runxhq/core/executor`.
- Refresh the broad `rust-ts-sunset-executor` importer census.
- Verify that remaining executor imports are adapter/runtime interface users.

## Scope

In scope:
- `packages/adapters/src/agent/agent-act-invocation.ts`
- `packages/adapters/src/agent/index.ts`
- `packages/runtime-local/src/runner-local/index.ts`
- `.scafld/specs/drafts/rust-ts-sunset-executor.md`
- Import audit evidence for `@runxhq/core/executor`

Out of scope:
- Removing `packages/core/src/executor/**`.
- Removing the `@runxhq/core/executor` package export.
- Moving `SkillAdapter`, `AdapterActInvocation`, `ActReceiptEnvelope`,
  `NestedSkillInvoker`, `ToolCatalogAdapter`, or tool catalog interfaces.
- Running `scafld harden`.

## Dependencies

- `@runxhq/contracts` exports the canonical agent act, output, approval, and
  resolution contract names used by these imports.
- Runtime adapter interfaces still need a separate ownership decision.

## Assumptions

- The import cleanup is already present in the working tree and this spec owns
  only the bounded ledger, census refresh, and validation.
- Any unrelated dirty workspace changes belong to parallel workers and are not
  reverted or normalized by this slice.

## Touchpoints

- Agent adapter contract conversion helpers.
- Runtime-local graph runner public types.
- Broad executor sunset draft census.

## Risks

- Accidentally moving adapter interfaces would widen the slice. Mitigation:
  audit remaining `@runxhq/core/executor` import names and leave runtime
  interfaces in place.
- Focused validation may be affected by unrelated workspace drift. Mitigation:
  record the exact failing command and blocker if that happens.

## Acceptance

Profile: standard

Validation:
- `rg 'from "@runxhq/core/executor"|from '\''@runxhq/core/executor'\''' packages tests scripts --glob '*.ts' --glob '*.tsx'`
- `pnpm typecheck`
- `pnpm vitest run packages/adapters/src/runtime.test.ts packages/runtime-local/src/runner-local/voice-profile.test.ts`
- `pnpm vitest run tests/graph-fanout.test.ts -t "denies mutating retry fanout branches without idempotency before adapter execution" --testTimeout 15000`
- `git diff --check -- packages/adapters/src/agent/agent-act-invocation.ts packages/adapters/src/agent/index.ts packages/runtime-local/src/runner-local/index.ts .scafld/specs/drafts/rust-ts-sunset-executor.md .scafld/specs/active/rust-ts-sunset-executor-contract-imports-2.md`

## Phase 1: Implementation

Status: completed
Dependencies: none

Objective: Complete the requested change.

Changes:
- Confirmed pure agent act/output contract imports moved to `@runxhq/contracts` in the agent adapter helper.
- Confirmed agent resolution response/request imports moved to `@runxhq/contracts` in the managed agent adapter.
- Confirmed runtime-local graph runner contract imports moved to `@runxhq/contracts` while `executeSkill`, `ActReceiptEnvelope`, `NestedSkillInvocation`, `NestedSkillInvocationResult`, and `SkillAdapter` remain on `@runxhq/core/executor`.
- Refreshed the broad executor sunset draft with the current importer census.

Acceptance:
- [x] `ac1` command - Remaining executor import audit
  - Command: `rg 'from "@runxhq/core/executor"|from '\''@runxhq/core/executor'\''' packages tests scripts --glob '*.ts' --glob '*.tsx'`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-6
- [x] `ac2` command - Focused TypeScript typecheck
  - Command: `pnpm typecheck`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-7
- [x] `ac3` command - Focused runtime/adapter tests
  - Command: `pnpm vitest run packages/adapters/src/runtime.test.ts packages/runtime-local/src/runner-local/voice-profile.test.ts`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-8
- [x] `ac4` command - Focused graph fanout regression
  - Command: `pnpm vitest run tests/graph-fanout.test.ts -t "denies mutating retry fanout branches without idempotency before adapter execution" --testTimeout 15000`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-9
- [x] `ac5` command - Whitespace check for touched files
  - Command: `git diff --check -- packages/adapters/src/agent/agent-act-invocation.ts packages/adapters/src/agent/index.ts packages/runtime-local/src/runner-local/index.ts .scafld/specs/drafts/rust-ts-sunset-executor.md .scafld/specs/active/rust-ts-sunset-executor-contract-imports-2.md`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-10

## Rollback

- Revert this spec and the broad executor draft census refresh if the
  already-present import cleanup is reverted by its owning worker.

## Review

Status: completed
Verdict: pass
Mode: verify
Summary: Human-reviewed override accepted: Human-reviewed import-only slice: pure contract imports are on @runxhq/contracts, runtime adapter interfaces remain on @runxhq/core/executor, broad executor census refreshed, typecheck/focused tests/diff check passed; combined graph-fanout file timeout recorded as residual suite flakiness.

Attack log:
- `review gate`: manual human audit -> clean (Human-reviewed import-only slice: pure contract imports are on @runxhq/contracts, runtime adapter interfaces remain on @runxhq/core/executor, broad executor census refreshed, typecheck/focused tests/diff check passed; combined graph-fanout file timeout recorded as residual suite flakiness.)

Findings:
- none

## Self Eval

- The bounded import cleanup is already present and validation passes for the
  import audit, typecheck, focused adapter/runtime tests, focused graph fanout
  regression, and touched-file whitespace check.

## Deviations

- The broader combined command
  `pnpm vitest run packages/adapters/src/runtime.test.ts packages/runtime-local/src/runner-local/voice-profile.test.ts tests/graph-fanout.test.ts`
  timed out in `tests/graph-fanout.test.ts` on
  `denies mutating retry fanout branches without idempotency before adapter execution`.
  That same test passes in isolation with `--testTimeout 15000`, so the timeout
  is recorded as residual test-suite flakiness outside this import-only cleanup.

## Metadata

- created_by: scafld

## Origin

Created by: scafld
Source: plan

## Harden Rounds

- none

## Planning Log

- none
