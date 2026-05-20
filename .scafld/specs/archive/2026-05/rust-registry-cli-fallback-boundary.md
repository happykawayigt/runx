---
spec_version: '2.0'
task_id: rust-registry-cli-fallback-boundary
created: '2026-05-20T06:42:53Z'
updated: '2026-05-20T06:45:26Z'
status: completed
harden_status: not_run
size: medium
risk_level: medium
---

# rust-registry-cli-fallback-boundary

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

Remove the final live CLI `@runxhq/core/registry` import. `skill-refs.ts` and
`dispatch.ts` keep their native Rust registry process boundaries, while the
remaining local fallback imports structural helpers from runtime-local SDK and
uses a narrow HTTP cached store for graph resolution.

## Objectives

- Remove `@runxhq/core/registry` from `packages/cli/src/registry-fallback.ts`.
- Preserve local search, local publish, and remote graph cache behavior.
- Keep native registry search/install boundaries intact.
- Keep the remaining registry package references test-only or comments.

## Scope

- In scope:
  - `packages/cli/src/registry-fallback.ts`
  - `packages/cli/src/skill-refs.ts`
  - `packages/cli/src/dispatch.ts`
  - focused CLI registry tests touched by the migration
- Out of scope:
  - test-only registry helper imports outside CLI behavior
  - deleting the registry package implementation

## Dependencies

- Completed native registry CLI search/install/publish boundary.
- Completed SDK registry boundary.

## Assumptions

- Remote graph registry resolution still needs an acquisition cache until the
  final runtime-local deletion routes graph refs through Rust directly.

## Touchpoints

- CLI skill search/add/publish, graph registry ref resolution, and MCP/dev
  commands that request a registry store.

## Risks

- Fallback code could silently keep old registry semantics alive. Mitigation:
  the file imports no core registry package, focused tests cover CLI behavior,
  and final registry deletion remains tracked separately.

## Acceptance

Profile: standard

Validation:
- `pnpm exec vitest run --config vitest.config.ts tests/skill-search.test.ts tests/skill-publish.test.ts tests/cli-skill-registry-profile.test.ts`
- `pnpm exec vitest run --config vitest.config.ts tests/skill-add.test.ts tests/remote-registry-add.test.ts tests/runtime-local-sdk.test.ts`
- `pnpm exec vitest run --config vitest.config.ts packages/cli/src/index.test.ts`
- `pnpm typecheck`
- `! rg -n '@runxhq/core/registry' packages/runtime-local/src packages/cli/src scripts --glob '*.ts' --glob '*.tsx' --glob '*.mjs'`

## Phase 1: Implementation

Status: active
Dependencies: none

Objective: Complete the requested change.

Changes:
- Route CLI search through native Rust when requested or SDK-backed local search otherwise.
- Route CLI publish through runtime-local SDK registry helpers.
- Add a narrow CLI HTTP cached store for graph registry ref acquisition.

Acceptance:
- [ ] `ac1` command - Focused CLI registry tests pass
  - Command: `pnpm exec vitest run --config vitest.config.ts tests/skill-search.test.ts tests/skill-publish.test.ts tests/cli-skill-registry-profile.test.ts`
  - Expected kind: `exit_code_zero`
  - Status: passed
- [ ] `ac2` command - Live CLI/runtime/script imports are clean
  - Command: `! rg -n '@runxhq/core/registry' packages/runtime-local/src packages/cli/src scripts --glob '*.ts' --glob '*.tsx' --glob '*.mjs'`
  - Expected kind: `exit_code_zero`
  - Status: passed

## Rollback

- Revert CLI fallback/dispatch/skill-ref edits.

## Review

Status: completed
Verdict: pass
Mode: verify
Summary: Human-reviewed override accepted: Local review after integrating subagent work: focused CLI registry tests and CLI index tests passed, typecheck passed, live CLI/runtime/script registry-import grep is clean except a non-import comment.

Attack log:
- `review gate`: manual human audit -> clean (Local review after integrating subagent work: focused CLI registry tests and CLI index tests passed, typecheck passed, live CLI/runtime/script registry-import grep is clean except a non-import comment.)

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
