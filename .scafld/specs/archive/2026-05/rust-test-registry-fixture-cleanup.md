---
spec_version: '2.0'
task_id: rust-test-registry-fixture-cleanup
created: '2026-05-20T06:48:46Z'
updated: '2026-05-20T07:06:59Z'
status: completed
harden_status: not_run
size: small
risk_level: low
---

# Registry test fixture cleanup

## Current State

Status: completed
Current phase: final
Next: done
Reason: task completed
Blockers: none
Allowed follow-up command: `none`
Latest runner update: 2026-05-20T07:06:59Z
Review gate: pass

## Summary

Remove remaining test-only `@runxhq/core/registry` imports that are used to seed
local registry fixtures. Tests should use the runtime-local SDK registry fixture
surface or a local structural store helper so they no longer pin Rust sunset
work to the TS core registry package.

## Objectives

- Add a small test helper that seeds registry fixture records through
  `@runxhq/runtime-local/sdk`.
- Replace fixture-seeding imports in focused tests with that helper.
- Leave true core-registry tests as intentional blockers instead of forcing
  them through the runtime-local surface.

## Scope

In scope:
- `tests/registry-fixtures.ts`
- `tests/skill-add.test.ts`
- `tests/skill-add-profile-metadata.test.ts`
- `tests/graph-registry-refs.test.ts`
- `tests/graph-registry-refs.integration.test.ts`
- `tests/ide-plugin-actions.test.ts`
- `tests/payment-skill-profile-validation.test.ts`
- `tests/registry-ce.test.ts`

Out of scope:
- Production package code.
- `tests/http-cached-registry-store.test.ts` and
  `tests/util-split-skill-id.test.ts`, which directly exercise core registry
  behavior.

## Dependencies

- `rust-registry-sdk-boundary` has added the runtime-local SDK local registry
  helpers.

## Assumptions

- The runtime-local SDK helper is the correct test-facing registry fixture
  surface for non-core tests during the Rust rewrite.
- Core registry unit tests may keep direct core registry imports until that
  package is removed or replaced by a dedicated Rust parity spec.

## Touchpoints

- Tests that seed temporary local registry stores for skill add, graph
  resolution, IDE actions, payment profile validation, and registry CE catalog
  coverage.

## Risks

- Risk: tests accidentally stop covering registry-backed runtime behavior.
  Mitigation: keep fixture stores structural and validate the same focused
  tests after the import cleanup.

## Acceptance

Profile: standard

Validation:
- `scafld validate rust-test-registry-fixture-cleanup --json`
- `pnpm exec vitest run --config vitest.config.ts tests/skill-add.test.ts tests/skill-add-profile-metadata.test.ts tests/graph-registry-refs.test.ts tests/graph-registry-refs.integration.test.ts tests/ide-plugin-actions.test.ts tests/payment-skill-profile-validation.test.ts tests/registry-ce.test.ts`
- `pnpm typecheck`
- `! rg -n '@runxhq/core/registry' tests/skill-add.test.ts tests/skill-add-profile-metadata.test.ts tests/graph-registry-refs.test.ts tests/graph-registry-refs.integration.test.ts tests/ide-plugin-actions.test.ts tests/payment-skill-profile-validation.test.ts tests/registry-ce.test.ts tests/registry-fixtures.ts`

## Phase 1: Implementation

Status: completed
Dependencies: none

Objective: Complete the requested change.

Changes:
- Add runtime-local SDK backed registry fixture helpers.
- Replace direct core registry imports in non-core tests with the helper or structural expectations.

Acceptance:
- [x] `ac1` command - Spec validates
  - Command: `scafld validate rust-test-registry-fixture-cleanup --json`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-13
- [x] `ac2` command - Focused tests pass
  - Command: `pnpm exec vitest run --config vitest.config.ts tests/skill-add.test.ts tests/skill-add-profile-metadata.test.ts tests/graph-registry-refs.test.ts tests/graph-registry-refs.integration.test.ts tests/ide-plugin-actions.test.ts tests/payment-skill-profile-validation.test.ts tests/registry-ce.test.ts`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-14
- [x] `ac3` command - Typecheck passes
  - Command: `pnpm typecheck`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-15
- [x] `ac4` command - Target tests no longer import core registry package
  - Command: `! rg -n '@runxhq/core/registry' tests/skill-add.test.ts tests/skill-add-profile-metadata.test.ts tests/graph-registry-refs.test.ts tests/graph-registry-refs.integration.test.ts tests/ide-plugin-actions.test.ts tests/payment-skill-profile-validation.test.ts tests/registry-ce.test.ts tests/registry-fixtures.ts`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-16

## Rollback

- Revert the test helper and import replacements; no production package code is
  modified by this task.

## Review

Status: completed
Verdict: pass
Mode: discover
Provider: claude:claude-opus-4-7
Output: claude.mcp_submit_review
Summary: The task replaces `@runxhq/core/registry` fixture-seeding imports with a small `tests/registry-fixtures.ts` helper that wraps `@runxhq/runtime-local/sdk` exports (`createFileRegistryStore`, `publishSkillMarkdown`, plus `RegistryStore`/`RegistrySkillVersion`/`PutVersionOptions` types). All seven in-scope tests now import the helper instead of the core registry package. The helper file is pure utility (no `describe`/`it`, no production code touched), so the `no_test_logic_in_production` invariant holds. Spot checks confirm: (1) ac4 grep target paths contain zero `@runxhq/core/registry` imports (the only remaining hit in `reflect-digest-skill.test.ts` is a literal data string, and `http-cached-registry-store.test.ts` / `util-split-skill-id.test.ts` are explicitly out-of-scope core-registry coverage); (2) all symbols the helper consumes exist in `packages/runtime-local/src/sdk/index.ts` and the `./sdk` subpath is declared in the package exports; (3) the structural compatibility between the SDK `RegistryStore` (4 methods) and the narrower `RegistryStore` from `runtime-local/runner-local/registry-resolver` (`getVersion`/`listVersions`) plus `SkillInstallRegistryStore` is preserved — typecheck (ac3) recorded as passing matches this analysis. Ambient drift (Rust crates, runner-local files, mcp index, deny.toml, docs) sits outside the declared task scope and does not interact with the test refactor. Spec acceptance evidence (scafld validate, focused vitest, typecheck, forbidden-import grep) is recorded as exit 0 across the board. No completion blockers found.

Attack log:
- `tests/skill-add.test.ts, tests/skill-add-profile-metadata.test.ts, tests/graph-registry-refs.test.ts, tests/graph-registry-refs.integration.test.ts, tests/ide-plugin-actions.test.ts, tests/payment-skill-profile-validation.test.ts, tests/registry-ce.test.ts, tests/registry-fixtures.ts`: Spec compliance: grep target tests for `@runxhq/core/registry` imports (ac4). -> clean (Zero import matches across the eight target paths; the only repo hit inside scope grep is a string literal in tests/reflect-digest-skill.test.ts:51 (`runx@runxhq/core/registry` repo data) which is out-of-scope and not an import.)
- `tests/registry-fixtures.ts -> @runxhq/runtime-local/sdk`: Helper API surface: verify createFileRegistryStore, publishSkillMarkdown, PutVersionOptions, PublishSkillMarkdownOptions, PublishSkillMarkdownResult, RegistrySkill, RegistrySkillVersion, RegistryStore all exist in the SDK and the ./sdk subpath is exported by packages/runtime-local/package.json. -> clean (All symbols defined in packages/runtime-local/src/sdk/index.ts (RegistryStore L177, putVersion/getVersion/listVersions/listSkills; createFileRegistryStore L791; publishSkillMarkdown L832; types L134-225). package.json declares the `./sdk` exports map entry.)
- `tests/graph-registry-refs.test.ts FixtureRemoteRegistryStore`: Type compatibility: SDK RegistryStore (4 methods) flowing into runLocalGraph/materializeRegistrySkill which use the narrower runner-local RegistryStore (getVersion/listVersions) and SkillInstallRegistryStore (getVersion, optional listSkills) — confirm structural subtyping holds. -> clean (SDK RegistrySkillVersion is a superset of SkillInstallRegistrySkillVersion fields and of the registry-resolver RegistrySkillVersion. ac3 (pnpm typecheck) recorded as exit 0, consistent with the structural check.)
- `Task scope vs workspace drift`: Scope drift: compare ambient drift list (crates/*, packages/runtime-local/src/runner-local/*, packages/runtime-local/src/mcp/index.ts, docs/trusted-kernel-package-truth.md, deny.toml) against declared task scope; verify none of these are required by the test refactor. -> clean (Helper depends only on already-published SDK exports that pre-date this task (per spec dependency on rust-registry-sdk-boundary). The ambient files do not touch the SDK registry helpers.)
- `tests/registry-fixtures.ts`: no_test_logic_in_production invariant: ensure the helper does not leak into production package code and that no production code was modified to satisfy tests. -> clean (Task changes are confined to tests/* per the recorded task_changes manifest; registry-fixtures.ts has no describe/it blocks (pure helper module) and does not import from packages/ source dirs other than via published @runxhq/runtime-local/sdk surface.)
- `tests/registry-fixtures.ts`: Vitest collection regression: confirm the helper file is not picked up as a test (filename `registry-fixtures.ts` outside `*.test.ts` pattern) and re-exports preserve type-vs-value semantics. -> clean (vitest.config.ts include pattern is `tests/**/*.test.ts`; `registry-fixtures.ts` does not match. Helper correctly uses `export { createFileRegistryStore }` for the value and `export type { ... }` for types.)
- `tests/payment-skill-profile-validation.test.ts and tests/registry-ce.test.ts`: Coverage regression: helper variants (buildRegistryFixtureVersion, publishRegistryFixtureSkill, seedRegistrySkill) — confirm they still expose the same record fields the tests assert on (profile_document, profile_digest, runner_names, source_type, etc.). -> clean (publishSkillMarkdown returns PublishSkillMarkdownResult with .record:RegistrySkillVersion containing the fields the tests assert (profile_document, profile_digest, runner_names, source_type, markdown). publishRegistryFixtureSkill returns the full result so registry-ce.test.ts retains access to .link as well.)

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

- 2026-05-20: Found runtime-local SDK now owns createFileRegistryStore,
  publishSkillMarkdown, and searchRegistry helpers; use those for test fixture
  seeding.
