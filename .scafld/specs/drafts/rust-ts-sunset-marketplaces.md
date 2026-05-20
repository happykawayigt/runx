---
spec_version: '2.0'
task_id: rust-ts-sunset-marketplaces
created: '2026-05-18T00:00:00Z'
updated: '2026-05-20T00:00:00Z'
status: draft
harden_status: not_run
size: small
risk_level: medium
---

# TS sunset: marketplaces

## Current State

Status: draft, blocked
Current phase: discovery refresh
Next: unblock registry ownership and reroute marketplace consumers before
approval.
Reason: this draft describes a future deletion, not work that can be executed
against the current tree. A fresh 2026-05-20 source scan still finds 14 files
with live imports of `@runxhq/core/marketplaces` or marketplace surfaces across
CLI, runtime-local, registry fallback, SDK, and tests. The prerequisite
`rust-ts-sunset-registry` is also archived as failed, not completed. Deleting
`packages/core/src/marketplaces/**` now would break current package exports and
live consumers.
Blockers:
- `rust-ts-sunset-registry` is archived with `status: failed`; the original
  dependency is not satisfied.
- CLI dispatch and skill refs still import marketplace helpers and fixture
  adapters from `@runxhq/core/marketplaces`.
- Runtime-local skill install and SDK search/install surfaces still accept
  `MarketplaceAdapter` and call `resolveMarketplaceSkill` /
  `searchMarketplaceAdapters`.
- Registry fallback/native registry presentation still reuses
  `SkillSearchResult` from the marketplace package as the shared search result
  model.
- Focused tests still import marketplace fixtures/types directly.
- `packages/core/package.json` still exposes `./marketplaces`; remove it only
  after all importers are rerouted or retired by owning specs.
Allowed follow-up command: none while blocked; do not run `scafld harden rust-ts-sunset-marketplaces`.
Latest runner update: 2026-05-20T22:55:00+10:00 - refreshed source scan
confirmed 14 files still reference marketplace surfaces; deletion remains
blocked.
Review gate: not_started

## Summary

Future deletion target: remove `packages/core/src/marketplaces/` and the public
`@runxhq/core/marketplaces` export after all live consumers no longer depend on
it. The marketplaces domain is small, but it currently owns the shared
marketplace adapter contract, fixture marketplace adapter, marketplace ref
classification, and the `SkillSearchResult` model consumed by registry search
presentation.

This spec must remain blocked until the shared search result model and
marketplace adapter contract have an explicit post-TypeScript owner. That owner
may be `runx-runtime::registry`, a contracts package surface, or a narrow
runtime/CLI boundary, but deletion must not add a compatibility shim or leave
the `@runxhq/core/marketplaces` subpath alive.

## Context

CWD: `.`

Packages:
- `@runxhq/core`
- `crates/runx-runtime` (or merged into registry)

Current TypeScript sources:
- `packages/core/src/marketplaces/**` (future deletion)

Files impacted:
- `packages/core/src/marketplaces/` (future deletion)
- `packages/core/package.json` (`"./marketplaces"` export removal)
- Any generated API-surface docs reflecting the removed export, if still
  present at execution time

Invariants:
- Marketplaces consumers (CLI surfaces, registry resolver, ai-search merge)
  have a Rust path.
- No compatibility shim, re-export, fallback adapter, or legacy TypeScript
  package surface remains after deletion.
- `SkillSearchResult` ownership is explicit before deletion; registry search
  must not keep importing it from the marketplace package.

Current live importers found in the 2026-05-20 source scan:
- `packages/cli/src/dispatch.ts`
- `packages/cli/src/skill-refs.ts`
- `packages/cli/src/native-registry.ts`
- `packages/cli/src/registry-fallback.ts`
- `packages/cli/src/presentation/search.ts`
- `packages/runtime-local/src/runner-local/skill-install.ts`
- `packages/runtime-local/src/sdk/index.ts`
- `packages/core/src/registry/search.ts`
- `tests/skill-add.test.ts`
- `tests/skill-add-profile-metadata.test.ts`

## Objectives

- Keep this draft honest about current blockers.
- Track the exact public export removal target:
  `packages/core/package.json` `exports["./marketplaces"]`.
- Require a fresh source scan before any deletion attempt.
- Delete TS marketplaces only after the marketplace adapter/search-result
  contracts and all consumers have moved to their post-TypeScript owner.

## Scope

In scope:
- Future TS marketplaces deletion plan.
- Future public export removal for `@runxhq/core/marketplaces`.
- Importer verification and deletion gating.

Out of scope:
- Marketplace functionality changes.
- Rerouting CLI/runtime-local/SDK consumers.
- Moving `SkillSearchResult` or marketplace adapter contracts to a new owner.
- Legacy import compatibility, package shims, or fallback adapters.

## Dependencies

- A completed registry ownership/cutover path; the current
  `rust-ts-sunset-registry` archive entry is failed and cannot satisfy this
  dependency.
- A `rust-marketplaces-port` spec, a merger into `runx-runtime::registry`, or a
  contracts/runtime boundary spec that owns `SkillSearchResult`,
  `MarketplaceAdapter`, marketplace ref parsing, and fixture marketplace test
  setup.
- A fresh importer scan immediately before approval.

## Open Questions

- Whether marketplaces ships as its own Rust module, folds into
  `runx-runtime::registry`, or is split between contracts-owned types and
  runtime/CLI-owned adapters.
- Which spec owns moving `SkillSearchResult` out of
  `@runxhq/core/marketplaces` for registry search presentation?
- Which spec owns fixture marketplace test setup after the TS package is
  deleted?
