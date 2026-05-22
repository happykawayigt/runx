---
spec_version: '2.0'
task_id: rust-ts-sunset-parser-runtime-local-importers
created: '2026-05-22T00:26:00+10:00'
updated: '2026-05-22T01:42:29Z'
status: completed
harden_status: not_run
size: small
risk_level: medium
---

# Parser sunset: runtime-local importers

## Current State

Status: completed
Current phase: final
Next: done
Reason: task completed
Blockers: none
Allowed follow-up command: `none`
Latest runner update: 2026-05-22T01:42:29Z
Review gate: pass

## Summary

Migrate runtime-local parser importers away from direct
`@runxhq/core/parser` use and reduce the runtime-local parser-shaped type
surface to Rust/contract-owned inputs where that can be done safely. This is an
importer migration slice only. It must not delete `packages/core/src/parser/**`
and must not perform the broader `packages/runtime-local/` deletion owned by
`rust-ts-sunset-runtime-local`.

The safe first target is runtime-local source code because it has a contained
set of parser value importers and already owns local structural aliases in
`packages/runtime-local/src/parser-types.ts`. Production import changes are
allowed only when the replacement is small, local, and backed by targeted
tests. Otherwise this draft records the migration plan and leaves code intact.

## Objectives

- Classify runtime-local parser value imports by behavior: skill parsing,
  runner manifest parsing, tool manifest parsing, graph parsing, install
  validation, reflect policy, quality profile, and artifact contract handling.
- Replace any small safe importer with a Rust/contract-backed boundary or a
  runtime-local-local helper that no longer imports `@runxhq/core/parser`.
- Keep `parser-types.ts` as a temporary structural surface only while
  runtime-local still exists; do not expand it.
- Prove the runtime-local importer census shrinks without changing parser
  behavior.
- Leave parser implementation deletion to `rust-ts-sunset-parser`.

## Scope

In scope:
- `packages/runtime-local/src/harness/agent-hook.test.ts`
- `packages/runtime-local/src/harness/publish.ts`
- `packages/runtime-local/src/harness/runner.ts`
- `packages/runtime-local/src/runner-local/execution-targets.ts`
- `packages/runtime-local/src/runner-local/index.ts`
- `packages/runtime-local/src/runner-local/skill-install.ts`
- `packages/runtime-local/src/sdk/index.ts`
- `packages/runtime-local/src/tool-catalogs/index.ts`
- Existing runtime-local consumers of `../parser-types.js`.

Out of scope:
- Deleting `packages/core/src/parser/**`.
- Deleting `packages/runtime-local/**` or `packages/adapters/**`.
- CLI, core, script, and root test parser importers.
- Dirty target-runner, payments, MCP, TS-boundary, embedded-sdk,
  rust-dev, and post-merge observer work.

## Dependencies

- `rust-ts-sunset-parser-runtime-type-imports` is archived completed.
- `rust-ts-sunset-parser` remains the deletion parent and stays blocked until
  all importer-class specs clear their censuses.
- `rust-ts-sunset-runtime-local` owns final runtime-local package deletion.

## Importer Census

Checked on 2026-05-22:

```bash
rg -l "@runxhq/core/parser|parser-types\.js" packages/runtime-local/src -g '!node_modules' -g '!crates/target' | sort
rg -n "@runxhq/core/parser|parser-types\.js" packages/runtime-local/src -g '!node_modules' -g '!crates/target'
rg -l "@runxhq/core/parser" packages/runtime-local/src -g '!node_modules' -g '!crates/target' | wc -l
rg -l "parser-types\.js" packages/runtime-local/src -g '!node_modules' -g '!crates/target' | wc -l
```

Observed results:
- 21 runtime-local source files reference parser value imports or
  `parser-types.js`.
- 0 runtime-local source files import `@runxhq/core/parser`.
- 21 runtime-local source files import `parser-types.js`.

Value importers: no direct runtime-local `@runxhq/core/parser` value importers
remain; migrated runtime-local parser ingress now routes through the Rust parser
bridge or local non-parser helpers listed below.

Migrated value importers:
- `packages/runtime-local/src/runner-local/parser-bridge.ts`
  - Replacement: direct, self-contained structural bridge types for values
    returned by Rust `runx parser eval`; the bridge no longer imports the
    temporary runtime-local `parser-types.js` surface.
  - Evidence:
    `pnpm exec vitest run --config vitest.config.ts packages/runtime-local/src/runner-local/parser-bridge.test.ts`
    and `pnpm typecheck` passed on 2026-05-22T11:27:25+10:00.
- `packages/runtime-local/src/harness/publish.ts`,
  `packages/runtime-local/src/harness/runner.ts`,
  `packages/runtime-local/src/runner-local/execution-targets.ts`,
  `packages/runtime-local/src/runner-local/index.ts`,
  `packages/runtime-local/src/runner-local/skill-install.ts`, and
  `packages/runtime-local/src/sdk/index.ts`
  - Replacement: full parser-validation calls route through
    `../runner-local/parser-bridge.js` / `./parser-bridge.js`, which invokes
    Rust `runx parser eval`; no compatibility aliases were added.
  - Evidence:
    `rg -n "@runxhq/core/parser" packages/runtime-local/src -g '!node_modules' -g '!crates/target'`
    returned no matches on 2026-05-22T11:27:25+10:00.
- `packages/runtime-local/src/harness/publish.ts` and
  `packages/runtime-local/src/harness/runner.ts`
  - Replacement: local `parseSkillFrontmatter` helper for harness-only
    `frontmatter.name` extraction. The helper mirrors the parser frontmatter
    delimiter/YAML-object requirements and intentionally does not validate
    full skill semantics.
  - Evidence:
    `pnpm exec vitest run --config vitest.config.ts packages/runtime-local/src/harness/skill-frontmatter.test.ts packages/runtime-local/src/harness/agent-hook.test.ts`
    and `pnpm typecheck` passed. Broader inline harness execution remains
    blocked without `RUNX_KERNEL_EVAL_BIN`.
- `packages/runtime-local/src/harness/agent-hook.test.ts`
  - Replacement: local `SkillSource` fixture object; the test covers the
    harness-hook adapter surface, not parser behavior.
  - Evidence: `pnpm exec vitest run --config vitest.config.ts packages/runtime-local/src/harness/agent-hook.test.ts`
    passed.
- `packages/runtime-local/src/tool-catalogs/index.ts`
  - Replacement: local tool-catalog structural types and construction for
    imported catalog tools, using the same runtime-local-normalized document
    shape and raw JSON as the previous parser-backed construction.
  - Evidence: `pnpm exec vitest run --config vitest.config.ts packages/runtime-local/src/tool-catalogs/index.test.ts`
    and `pnpm typecheck` passed. Full graph-level MCP import runtime test is
    blocked by missing `RUNX_KERNEL_EVAL_BIN` before imported tool invocation.

Remaining blocked value importers:
- None for direct `@runxhq/core/parser` imports in runtime-local source.
- The remaining `parser-types.js` structural consumers are temporary
  runtime-local type surfaces and are not parser implementation imports.

## Acceptance

Profile: standard

Definition of done:
- [x] `dod1` Runtime-local direct imports from `@runxhq/core/parser` are removed
  only where a small Rust/contract-backed or local helper migration is safe.
- [x] `dod2` `parser-types.ts` remains temporary and does not gain new parser
  shape exports.
- [x] `dod3` No parser implementation file is deleted or renamed.
- [x] `dod4` The parent parser census is updated after this slice lands.

Validation:
- [x] `v1` Scafld validates this spec.
  - Command: `scafld validate rust-ts-sunset-parser-runtime-local-importers --json`
  - Expected kind: `exit_code_zero`
  - Status: passed
  - Evidence: 2026-05-22T11:27:25+10:00 returned
    `{"ok":true,...,"valid":true}`.
- [x] `v2` Runtime-local parser direct-import census.
  - Command: `bash -lc '! rg -n "@runxhq/core/parser" packages/runtime-local/src -g "!node_modules" -g "!crates/target"'`
  - Expected kind: `no_matches`
  - Status: passed
  - Evidence: 2026-05-22T11:38:00+10:00 returned no matches and exited zero.
- [x] `v3` Parser implementation remains present.
  - Command: `test -d packages/core/src/parser`
  - Expected kind: `exit_code_zero`
  - Status: passed
  - Evidence: parser implementation remains present; no parser source was
    deleted or renamed in this lane.
- [x] `v4` Targeted runtime-local tests cover migrated importers.
  - Command: `pnpm exec vitest run --config vitest.config.ts packages/runtime-local/src/runner-local/parser-bridge.test.ts`
  - Expected kind: `exit_code_zero`
  - Status: passed
  - Evidence: 2026-05-22T11:27:25+10:00 passed with 1 file and 4 tests.
    `pnpm typecheck` also passed.

## Phase 1: Importer Classification

Status: completed
Dependencies: none

Goal: map each runtime-local parser value import to a safe replacement or an
explicit blocker.

Acceptance:
- [x] `ac1` command - Runtime-local parser importer census is current.
  - Command: `rg -n "@runxhq/core/parser|parser-types\.js" packages/runtime-local/src -g '!node_modules' -g '!crates/target'`
  - Expected kind: `exit_code_zero`
  - Status: passed
  - Evidence: no direct `@runxhq/core/parser` imports remain; 21
    `parser-types.js` structural consumers remain.
- [x] `ac2` command - Value importer assignments are recorded in this spec.
  - Command: `rg -n "Migrated value importers:|Remaining blocked value importers:" .scafld/specs/active/rust-ts-sunset-parser-runtime-local-importers.md`
  - Expected kind: `exit_code_zero`
  - Status: passed
  - Evidence: migrated and blocked assignments recorded under Importer Census.

## Phase 2: Safe Migration

Status: completed
Dependencies: Phase 1

Objective: Complete this phase.

Changes:
- none

Acceptance:
- [x] `ac3` command - Runtime-local direct parser imports are removed.
  - Command: `bash -lc '! rg -n "@runxhq/core/parser" packages/runtime-local/src -g "!node_modules" -g "!crates/target"'`
  - Expected kind: `no_matches`
  - Status: pass
  - Evidence: no direct runtime-local `@runxhq/core/parser` imports remain.
  - Source event: entry-3
- [x] `ac4` command - Parser implementation is untouched.
  - Command: `test -d packages/core/src/parser`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-4

## Rollback

- Revert only this importer slice's runtime-local edits. Do not restore or
  delete parser implementation files from this spec.

## Metadata

- created_by: codex

## Review

Status: completed
Verdict: pass
Mode: verify
Provider: command
Output: command.stdout
Summary: Verified runtime-local has no direct @runxhq/core/parser imports, the parser implementation remains present, and the replacement bridge is a Rust parser eval boundary. No completion-blocking findings.

Attack log:
- `packages/runtime-local/src`: Search for direct @runxhq/core/parser imports. -> clean (rg returned no matches.)
- `packages/core/src/parser`: Verify the TS parser implementation was not deleted by this importer slice. -> clean (directory exists)
- `packages/runtime-local/src/runner-local/parser-bridge.ts`: Inspect bridge for hidden parser implementation dependency and Rust CLI boundary. -> clean (no direct parser import; bridge calls parser eval)
- `packages/runtime-local/src`: Confirm remaining parser-types.js consumers are structural runtime-local surface, not direct parser implementation imports. -> clean (1 parser-types.js references remain, matching the spec temporary-surface allowance.)

Findings:
- none
