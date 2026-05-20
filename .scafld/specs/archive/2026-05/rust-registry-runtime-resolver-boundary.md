---
spec_version: '2.0'
task_id: rust-registry-runtime-resolver-boundary
created: '2026-05-20T05:27:37Z'
updated: '2026-05-20T05:31:23Z'
status: completed
harden_status: not_run
size: small
risk_level: medium
---

# Rust registry runtime resolver boundary

## Current State

Status: completed
Current phase: final
Next: done
Reason: task completed
Blockers: none
Allowed follow-up command: `none`
Latest runner update: 2026-05-20T05:31:23Z
Review gate: pass

## Summary

Remove the runtime-local registry resolver's live @runxhq/core/registry dependency by resolving configured RegistryStore records locally while preserving the existing native runx registry resolve process boundary.

## Objectives

- Remove the remaining live `@runxhq/core/registry` runtime dependency from
  `packages/runtime-local/src/runner-local/registry-resolver.ts`.
- Preserve store-backed graph registry ref materialization for configured
  `RegistryStore` callers.
- Preserve the existing opt-in native process boundary
  `RUNX_RUST_REGISTRY_RESOLVE=1` / `RUNX_RUST_REGISTRY_BIN=<command>`.
- Keep the fallback behavior honest: full registry sunset remains blocked by
  runtime-local `skill-install.ts`, `official-cache.ts`, SDK, CLI, and test
  importers.

## Scope

In scope:
- `packages/runtime-local/src/runner-local/registry-resolver.ts`
- Focused resolver/graph registry ref tests.

Out of scope:
- The failed umbrella `rust-ts-sunset-registry` spec.
- `packages/runtime-local/src/runner-local/skill-install.ts`
- `packages/runtime-local/src/runner-local/official-cache.ts`
- Executor files and broad runtime-local rewrites.
- Removing the `@runxhq/core/registry` export.

## Dependencies

- Completed native registry CLI resolve boundary in `runx registry resolve
  --json`.
- Existing runtime-local graph registry ref test coverage.

## Assumptions

- Graph/runtime materialization only needs owner/name registry refs because
  `parseRegistryRef` rejects bare names before resolution.
- Store-backed resolution can preserve the prior `resolveRegistrySkill` mapping
  by calling `RegistryStore.getVersion(skillId, version)` directly.

## Touchpoints

- Runtime-local graph registry ref materialization.
- Runtime-local exported registry resolver types.

## Risks

- Risk: Store-backed remote caches no longer resolve through the core helper.
  Mitigation: focused graph registry tests cover local stores and
  `HttpCachedRegistryStore` behavior through the structural store interface.
- Risk: Native resolve boundary regresses while removing the core import.
  Mitigation: keep the existing Rust-boundary test in acceptance.

## Acceptance

Profile: standard

Validation:
- `scafld validate rust-registry-runtime-resolver-boundary --json`
- `pnpm vitest run tests/runtime-local-registry-resolver.test.ts tests/graph-registry-refs.test.ts tests/graph-registry-refs-rust-boundary.test.ts`
- `pnpm typecheck`
- `rg '@runxhq/core/registry' packages/runtime-local/src/runner-local/registry-resolver.ts`
- `git diff --check -- packages/runtime-local/src/runner-local/registry-resolver.ts tests/runtime-local-registry-resolver.test.ts .scafld/specs/drafts/rust-registry-runtime-resolver-boundary.md`

## Phase 1: Implementation

Status: completed
Dependencies: none

Objective: Complete the requested change.

Changes:
- Replace the dynamic core-registry import with local store-backed resolution.
- Add a focused resolver test that uses only runtime-local registry types.
- Validate existing graph registry refs and native Rust resolve boundary tests.

Acceptance:
- [x] `ac1` command - Primary validation command
  - Command: `pnpm vitest run tests/runtime-local-registry-resolver.test.ts tests/graph-registry-refs.test.ts tests/graph-registry-refs-rust-boundary.test.ts && pnpm typecheck`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-6

## Rollback

- Restore the dynamic core-registry fallback in `registry-resolver.ts` and
  remove the focused resolver test. Do not revert unrelated registry work.

## Review

Status: completed
Verdict: pass
Mode: discover
Provider: command
Output: command.stdout
Summary: Command review passed for the scoped runtime-local resolver boundary.

Attack log:
- `no core registry import in registry-resolver`: deterministic command check -> clean
- `diff check scoped files`: deterministic command check -> clean

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
