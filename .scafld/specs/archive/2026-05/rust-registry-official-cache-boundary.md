---
spec_version: '2.0'
task_id: rust-registry-official-cache-boundary
created: '2026-05-20T05:39:14Z'
updated: '2026-05-20T05:44:03Z'
status: completed
harden_status: not_run
size: small
risk_level: medium
---

# Rust registry official cache boundary

## Current State

Status: completed
Current phase: final
Next: done
Reason: task completed
Blockers: none
Allowed follow-up command: `none`
Latest runner update: 2026-05-20T05:44:03Z
Review gate: pass

## Summary

Remove the runner-local official cache's live `@runxhq/core/registry`
dependency by moving its narrow acquire/publisher/id helper contract into
`official-cache.ts`. This keeps official cache acquisition behavior stable while
leaving broader registry sunset blockers explicit.

## Objectives

- Remove `@runxhq/core/registry` imports from
  `packages/runtime-local/src/runner-local/official-cache.ts`.
- Preserve cache-hit behavior, including deterministic official cache paths and
  first-party publisher attestations.
- Preserve cache-miss HTTP acquire behavior for locked official skills.
- Keep the remaining registry sunset blocked on `skill-install.ts`, SDK, CLI
  importers, and tests outside this slice.

## Scope

In scope:
- `packages/runtime-local/src/runner-local/official-cache.ts`
- Focused `official-cache` tests if needed.

Out of scope:
- `packages/runtime-local/src/runner-local/skill-install.ts`
- CLI dispatch, CLI skill ref routing, runtime MCP, and contracts.
- Runtime store type and resolver work already completed.
- Deleting `@runxhq/core/registry` or changing package manifests.

## Dependencies

- Completed `rust-registry-runtime-resolver-boundary`.
- Completed `rust-registry-runtime-store-type-boundary`.

## Assumptions

- Official cache only needs `<owner>/<name>` skill ids from the lock file.
- Duplicating the narrow remote acquire payload validator is lower risk than
  changing CLI skill-ref behavior or broad registry store plumbing in this
  slice.

## Touchpoints

- Official skill lock cache fill and reuse.
- CLI callers that consume `ensureOfficialSkillCached` through runtime-local.

## Risks

- Risk: remote acquire payload validation drifts from core registry. Mitigation:
  keep the local validator intentionally narrow and covered by focused cache
  tests.
- Risk: cache-hit reconstructed attestations differ from previous behavior.
  Mitigation: preserve the same first-party publisher attestation fields.

## Acceptance

Profile: standard

Validation:
- `pnpm vitest run packages/runtime-local/src/runner-local/official-cache.test.ts --config vitest.config.ts`
- `pnpm typecheck`
- `! rg -n '@runxhq/core/registry' packages/runtime-local/src/runner-local/official-cache.ts`
- `rg -n '@runxhq/core/registry' packages/runtime-local/src packages/cli/src tests --glob '*.ts'`
- `git diff --check -- packages/runtime-local/src/runner-local/official-cache.ts packages/runtime-local/src/runner-local/official-cache.test.ts .scafld/specs/active/rust-registry-official-cache-boundary.md`

## Phase 1: Implementation

Status: completed
Dependencies: none

Objective: Complete the requested change.

Changes:
- Localize the official cache acquire contract and registry helper types.
- Add focused tests for cache hit, cache miss acquire/write, and verification failure.

Acceptance:
- [x] `ac1` command - Primary validation command
  - Command: `pnpm vitest run packages/runtime-local/src/runner-local/official-cache.test.ts --config vitest.config.ts && pnpm typecheck`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-6
- [x] `ac2` command - Scoped registry import removed
  - Command: `! rg -n '@runxhq/core/registry' packages/runtime-local/src/runner-local/official-cache.ts`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-7

## Rollback

- Restore the `@runxhq/core/registry` imports in `official-cache.ts` and remove
  the focused cache tests. Do not revert unrelated runtime-local or registry
  work.

## Review

Status: completed
Verdict: pass
Mode: verify
Summary: Human-reviewed override accepted: official-cache boundary verified: focused tests, typecheck, scoped import audit, and diff check passed

Attack log:
- `review gate`: manual human audit -> clean (official-cache boundary verified: focused tests, typecheck, scoped import audit, and diff check passed)

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
