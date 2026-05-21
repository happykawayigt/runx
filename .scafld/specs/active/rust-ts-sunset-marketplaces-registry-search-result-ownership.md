---
spec_version: '2.0'
task_id: rust-ts-sunset-marketplaces-registry-search-result-ownership
created: '2026-05-22T00:00:00+10:00'
updated: '2026-05-22T01:36:00+10:00'
status: active
harden_status: not_run
size: small
risk_level: medium
---

# Marketplace sunset: registry search-result ownership

## Current State

Status: active
Current phase: safe import migration completed
Next: parent marketplaces deletion remains blocked on marketplace adapter,
fixture adapter, marketplace ref, runtime-local SDK/install, and test fixture
consumers.
Reason: `SkillSearchResult` ownership is now registry-owned in
`packages/core/src/registry/search-result.ts` and exported from
`@runxhq/core/registry`. Registry search and CLI registry search presentation
import that registry-owned model. `@runxhq/core/marketplaces` re-exports the
type only to preserve existing marketplace adapter API compatibility until the
adapter/ref deletion work is owned by a separate slice.
Blockers: marketplace adapter behavior still has live consumers in CLI
fixture marketplace search, runtime-local install/SDK surfaces, and focused
marketplace fixture tests. Parent marketplace deletion must stay blocked.
Allowed follow-up command: `scafld validate rust-ts-sunset-marketplaces-registry-search-result-ownership --json`
Latest runner update: 2026-05-22T01:36:00+10:00 promoted the executed child
spec from drafts to active. Registry-owned `SkillSearchResult` is already in
place and narrow registry/CLI result-shape imports have moved. Marketplace
adapter behavior was intentionally left in place.
Review gate: result_shape_migrated; adapter_behavior_blocked

## Summary

Move the shared search-result shape out of `@runxhq/core/marketplaces` before
the parent marketplaces deletion draft can advance. This child draft owns only
the registry/search-result ownership migration. It must not delete
`packages/core/src/marketplaces/**`, must not remove
`packages/core/package.json` `exports["./marketplaces"]`, and must not add a
compatibility shim for `@runxhq/core/marketplaces`.

The chosen target is a registry-owned result model exposed from
`@runxhq/core/registry`. Marketplace adapters now reference that model as an
external contract detail, but the marketplace adapter contract and marketplace
ref resolver still need their own migration before deletion.

## Objectives

- Define the post-marketplace owner for registry/search result shape.
- Reroute registry search and CLI presentation imports away from
  `@runxhq/core/marketplaces` only after the owner is explicit.
- Preserve the existing search-result JSON contract expected by Rust registry
  parsing and CLI rendering.
- Keep parent marketplace deletion blocked until marketplace adapters,
  marketplace refs, fixture adapters, and tests are also migrated or retired.

## Scope

In scope:
- `packages/core/src/registry/search.ts`
- `packages/core/src/registry/search-result.ts`
- `packages/core/src/registry/index.ts`
- `packages/core/src/marketplaces/index.ts`, only for type re-exporting the
  registry-owned search result shape while marketplace adapters remain live.
- `packages/cli/src/native-registry.ts`
- `packages/cli/src/registry-fallback.ts`
- `packages/cli/src/presentation/search.ts`
- Search-result type usage inside `packages/cli/src/skill-refs.ts`, without
  moving marketplace adapter behavior in the same step unless the replacement is
  tiny and covered by obvious tests.

Out of scope:
- Deleting `packages/core/src/marketplaces/**`.
- Removing `packages/core/package.json` `exports["./marketplaces"]`.
- Runtime-local SDK or install behavior.
- Marketplace adapter search, fixture marketplace adapters, and marketplace ref
  resolution.
- Payments, MCP, target-runner, post-merge observer, embedded-sdk,
  TS-boundary, parser/runtime-local, state-machine/runtime-local,
  external-adapter, and rust-dev work.

## Dependencies

- `rust-ts-sunset-marketplaces` remains the deletion parent and stays blocked.
- A registry-owned or contracts-owned search-result model that preserves the
  fields currently consumed by CLI presentation and Rust registry parsing.
- Focused CLI/registry tests before any production import migration.

## Importer Census

Checked on 2026-05-22:

```bash
rg -n "@runxhq/core/marketplaces|\\.\\./marketplaces/index\\.js|SkillSearchResult" packages/core/src/registry packages/cli/src --glob '!**/dist/**'
```

Observed result-shape consumers:
- `packages/core/src/registry/search.ts`
  - Imports `SkillSearchResult` from the registry-owned
    `./search-result.js` model and aliases it into `RegistrySearchResult`.
- `packages/cli/src/native-registry.ts`
  - Imports `SkillSearchResult` from `@runxhq/core/registry` for native Rust
    registry search response parsing.
- `packages/cli/src/registry-fallback.ts`
  - Imports `SkillSearchResult` from `@runxhq/core/registry` for fallback
    registry results.
- `packages/cli/src/presentation/search.ts`
  - Imports `SkillSearchResult` from `@runxhq/core/registry` for rendering
    search output.
- `packages/cli/src/skill-refs.ts`
  - Imports `SkillSearchResult` from `@runxhq/core/registry`, while
    marketplace adapter helpers remain imported from `@runxhq/core/marketplaces`
    for fixture marketplace behavior.

## Acceptance

Profile: standard

Definition of done:
- [x] `dod1` Search-result ownership is assigned to an explicit registry or
  contracts surface.
- [x] `dod2` Registry/CLI search-result imports no longer depend on
  `@runxhq/core/marketplaces` where marketplace adapter behavior is not needed.
- [x] `dod3` Existing search-result fields and CLI rendering behavior are
  covered by focused tests.
- [x] `dod4` Parent marketplace deletion remains blocked until all non-result
  marketplace consumers are cleared.

Validation:
- [x] `v1` Scafld validates this spec.
  - Command: `scafld validate rust-ts-sunset-marketplaces-registry-search-result-ownership --json`
  - Expected kind: `exit_code_zero`
  - Status: passed
  - Evidence: returned `{"ok":true,...,"valid":true}`.
- [x] `v2` Registry/CLI result-shape importer census is current.
  - Command: `rg -n "@runxhq/core/marketplaces|\\.\\./marketplaces/index\\.js|SkillSearchResult" packages/core/src/registry packages/cli/src --glob '!**/dist/**'`
  - Expected kind: `exit_code_zero`
  - Status: passed
  - Evidence: listed `packages/core/src/registry/search.ts` and CLI
    `native-registry.ts`, `registry-fallback.ts`, `presentation/search.ts`, and
    `skill-refs.ts` result-shape references.
- [x] `v3` Marketplace implementation remains present.
  - Command: `test -d packages/core/src/marketplaces`
  - Expected kind: `exit_code_zero`
  - Status: passed
  - Evidence: command exited 0.
- [x] `v4` TypeScript workspace typecheck passes after the type-owner move.
  - Command: `pnpm typecheck`
  - Expected kind: `exit_code_zero`
  - Status: passed
  - Evidence: `tsc -p tsconfig.typecheck.json --noEmit` completed with exit 0.
- [x] `v5` Focused CLI search rendering behavior still passes.
  - Command: `pnpm vitest run packages/cli/src/index.test.ts -t "renders search results with run and add commands"`
  - Expected kind: `exit_code_zero`
  - Status: passed
  - Evidence: 1 test passed, 46 skipped.
- [x] `v6` Child and parent specs validate after migration notes.
  - Command: `scafld validate rust-ts-sunset-marketplaces-registry-search-result-ownership --json && scafld validate rust-ts-sunset-marketplaces --json`
  - Expected kind: `exit_code_zero`
  - Status: passed
  - Evidence: both commands returned `{"ok":true,...,"valid":true}`.

## Phase 1: Ownership Decision

Status: completed
Dependencies: none

Goal: choose whether the search-result model belongs to registry, contracts, or
a narrow runtime/CLI boundary.

Acceptance:
- [x] `ac1` command - Current result-shape consumers are listed in this spec.
  - Command: `rg -n "Observed result-shape consumers" .scafld/specs/active/rust-ts-sunset-marketplaces-registry-search-result-ownership.md`
  - Expected kind: `exit_code_zero`
  - Status: passed
  - Evidence: section remains present with migrated imports recorded.
- [x] `ac2` manual - Owner choice records field compatibility requirements for
  CLI search rendering and Rust registry parsing.
  - Evidence: `packages/core/src/registry/search-result.ts` preserves the
    existing field set previously declared by `packages/core/src/marketplaces/index.ts`.

## Phase 2: Safe Import Migration

Status: completed
Dependencies: Phase 1

Goal: reroute only result-shape imports with focused tests, leaving marketplace
adapter behavior untouched unless an equally small tested migration exists.

Acceptance:
- [x] `ac3` command - Registry/CLI result-shape imports no longer require the
  marketplace package, with the remaining CLI marketplace import limited to
  fixture marketplace adapter behavior.
  - Command: `rg -n "@runxhq/core/marketplaces|\\.\\./marketplaces/index\\.js|SkillSearchResult" packages/core/src/registry packages/cli/src --glob '!**/dist/**'`
  - Expected kind: `exit_code_zero`
  - Status: passed
  - Evidence: result-shape references point to `@runxhq/core/registry` or
    `packages/core/src/registry/search-result.ts`; only
    `packages/cli/src/skill-refs.ts` still imports `@runxhq/core/marketplaces`
    for adapter helpers.
- [x] `ac4` command - Marketplace source remains undeleted.
  - Command: `test -d packages/core/src/marketplaces`
  - Expected kind: `exit_code_zero`
  - Status: passed
  - Evidence: marketplace implementation remains present.

## Rollback

- Revert only this planning or import migration slice. Do not delete, restore,
  or replace marketplace implementation files from this spec.

## Metadata

- created_by: codex
