---
spec_version: '2.0'
task_id: rust-registry-sdk-boundary
created: '2026-05-20T06:42:53Z'
updated: '2026-05-20T06:45:26Z'
status: completed
harden_status: not_run
size: medium
risk_level: medium
---

# rust-registry-sdk-boundary

## Current State

Status: completed
Current phase: final
Next: done
Reason: task completed
Blockers: none
Allowed follow-up command: `none`
Latest runner update: 2026-05-20T06:45:26Z
Review gate: pass

## Summary

Remove `@runxhq/core/registry` from the runtime-local SDK. The SDK now owns
its structural registry types and local registry helpers for search, publish,
file-backed storage, and remote read/search behavior used by the public SDK
surface.

## Objectives

- Remove direct core registry imports from `packages/runtime-local/src/sdk`.
- Preserve SDK search/add/publish behavior.
- Keep SDK tests seeding through public SDK behavior, not core registry
  fixtures.

## Scope

- In scope:
  - `packages/runtime-local/src/sdk/index.ts`
  - `tests/runtime-local-sdk.test.ts`
- Out of scope:
  - CLI fallback/dispatch
  - runtime-local skill-install
  - deleting `packages/core/src/registry/**`

## Dependencies

- Completed runtime store/resolver boundary slices.

## Assumptions

- The SDK can own structural registry helpers during the TS sunset; final
  runtime-local deletion remains tracked separately.

## Touchpoints

- SDK search/add/publish consumers and public SDK package surface.

## Risks

- Duplicated registry helper logic can drift. Mitigation: focused SDK tests,
  typecheck, and the follow-up CLI fallback now reuses SDK local helpers.

## Acceptance

Profile: standard

Validation:
- `pnpm exec vitest run --config vitest.config.ts tests/runtime-local-sdk.test.ts`
- `pnpm typecheck`
- `git diff --check -- packages/runtime-local/src/sdk/index.ts tests/runtime-local-sdk.test.ts`
- `! rg -n '@runxhq/core/registry' packages/runtime-local/src/sdk tests/runtime-local-sdk.test.ts`

## Phase 1: Implementation

Status: active
Dependencies: none

Objective: Complete the requested change.

Changes:
- Add SDK-local registry store/search/publish/read helpers and update focused SDK tests to use the SDK surface.

Acceptance:
- [ ] `ac1` command - SDK focused tests pass
  - Command: `pnpm exec vitest run --config vitest.config.ts tests/runtime-local-sdk.test.ts`
  - Expected kind: `exit_code_zero`
  - Status: passed
- [ ] `ac2` command - SDK has no core registry import
  - Command: `! rg -n '@runxhq/core/registry' packages/runtime-local/src/sdk tests/runtime-local-sdk.test.ts`
  - Expected kind: `exit_code_zero`
  - Status: passed

## Rollback

- Revert SDK helper changes and restore previous test seeding.

## Review

Status: completed
Verdict: pass
Mode: verify
Summary: Human-reviewed override accepted: Subagent and local verification: runtime-local SDK tests passed, typecheck passed, diff check passed, and SDK source/tests have no core registry import.

Attack log:
- `review gate`: manual human audit -> clean (Subagent and local verification: runtime-local SDK tests passed, typecheck passed, diff check passed, and SDK source/tests have no core registry import.)

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
