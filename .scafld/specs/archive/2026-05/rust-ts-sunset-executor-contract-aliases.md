---
spec_version: '2.0'
task_id: rust-ts-sunset-executor-contract-aliases
created: '2026-05-20T03:50:42Z'
updated: '2026-05-20T03:51:23Z'
status: completed
harden_status: not_run
size: medium
risk_level: medium
---

# Executor sunset: contract aliases

## Current State

Status: completed
Current phase: final
Next: done
Reason: task completed
Blockers: none
Allowed follow-up command: `none`
Latest runner update: 2026-05-20T03:51:23Z
Review gate: pass

## Summary

Move low-risk consumers of executor contract aliases and validators directly to
`@runxhq/contracts`, without moving adapter interfaces or deleting
`@runxhq/core/executor`.

This is a narrow sunset slice. `SkillAdapter`, `AdapterActInvocation`,
`ActReceiptEnvelope`, `ToolCatalogAdapter`, `ExecuteSkillOptions`, tool catalog
types, adapter runtime types, and the executor package export remain live until
their ownership is explicitly decided.

## Objectives

- Replace pure `ResolutionRequest`, `ResolutionResponse`, `Question`,
  `ApprovalGate`, and validator imports with `@runxhq/contracts` aliases where
  no adapter/runtime interface is needed.
- Keep public names stable through aliases such as
  `ResolutionRequestContract as ResolutionRequest`.
- Do not change approval, adapter, or execution behavior.
- Keep `@runxhq/core/executor` deletion blocked and visible.

## Scope

- `packages/adapters/src/runtime.ts`
- `packages/adapters/src/runtime.test.ts`
- `packages/cli/src/agent-runtime.ts`
- `packages/cli/src/callers.ts`
- `packages/cli/src/presentation/needs-resolution.ts`
- `packages/cli/src/cli-presentation.test.ts`
- `packages/runtime-local/src/runner-local/approval.ts`
- `packages/runtime-local/src/runner-local/graph-fanout-gates.ts`
- `packages/runtime-local/src/runner-local/graph-reporting.ts`
- `packages/runtime-local/src/runner-local/inputs.ts`
- `packages/runtime-local/src/sdk/caller.ts`
- `packages/runtime-local/src/sdk/host-protocol.ts`
- `packages/runtime-local/src/sdk/trusted-host-outcome.ts`
- `tests/host-protocol.test.ts`

Out of scope:
- removing `packages/core/src/executor/**`;
- removing the `@runxhq/core/executor` export;
- migrating `SkillAdapter`, `AdapterActInvocation`, `ActReceiptEnvelope`, or
  tool catalog adapter interfaces;
- changing adapter execution behavior.

## Dependencies

- `@runxhq/contracts` exports the canonical host protocol and resolution
  contracts.
- The broader `rust-ts-sunset-executor` draft remains deletion-blocked.

## Assumptions

- Contract aliases are safe to consume directly from `@runxhq/contracts`.
- Adapter interfaces need a separate owner decision and should not be moved in
  this slice.

## Touchpoints

- CLI presentation/caller types
- Runtime-local approval/fanout/input helpers
- Runtime-local SDK host bridge
- Adapter runtime contract types

## Risks

- Accidentally moving adapter interfaces would create a hidden compatibility
  layer. Mitigation: only pure contracts/validators moved.
- Stale tests may still assert pre-harness receipt fields. Mitigation: updated
  two host/CLI fixtures to canonical harness `seal.disposition` shape.

## Acceptance

Profile: standard

Validation:
- `pnpm typecheck`
- `pnpm vitest run packages/cli/src/cli-presentation.test.ts tests/host-protocol.test.ts packages/adapters/src/runtime.test.ts packages/runtime-local/src/runner-local/inputs.test.ts tests/runtime-local-sdk.test.ts tests/answers-file-shape.test.ts tests/caller-approval-boundary.test.ts tests/graph-fanout.test.ts`
- `git diff --check` for touched files

## Phase 1: Implementation

Status: completed
Dependencies: none

Objective: Move pure executor contract aliases to `@runxhq/contracts`.

Changes:
- Replaced pure resolution/question/approval contract type imports with
  `@runxhq/contracts`.
- Replaced resolution request validators with contract validators.
- Kept `SkillAdapter` and other execution interfaces on `@runxhq/core/executor`.
- Updated stale host/CLI presentation fixtures to canonical harness receipt
  shape.

Acceptance:
- [x] `ac1` command - focused validation commands listed above
  - Command: see Validation
  - Expected kind: `exit_code_zero`
  - Status: passed

## Rollback

- Revert this alias slice if a consumer needs executor-package names for an
  execution interface. Do not add re-export shims; leave the broad executor
  sunset blocked.

## Review

Status: completed
Verdict: pass
Mode: verify
Summary: Human-reviewed override accepted: pure executor contract aliases moved to @runxhq/contracts; adapter/runtime interfaces intentionally left on @runxhq/core/executor; typecheck, focused tests, and diff checks passed

Attack log:
- `review gate`: manual human audit -> clean (pure executor contract aliases moved to @runxhq/contracts; adapter/runtime interfaces intentionally left on @runxhq/core/executor; typecheck, focused tests, and diff checks passed)

Findings:
- none

## Self Eval

- The alias slice is complete. The broad executor sunset remains blocked by
  adapter/runtime interfaces and public SDK surfaces.

## Deviations

- This spec was created after the worker executed the safe importer slice so
  the scafld ledger reflects the already-finished bounded work.

## Metadata

- created_by: scafld

## Origin

Created by: scafld
Source: plan

## Harden Rounds

- none

## Planning Log

- none
