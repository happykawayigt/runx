---
spec_version: '2.0'
task_id: rust-ts-sunset-parser-runtime-local-importers
created: '2026-05-22T00:26:00+10:00'
updated: '2026-05-22T01:36:00+10:00'
status: active
harden_status: not_run
size: small
risk_level: medium
---

# Parser sunset: runtime-local importers

## Current State

Status: active
Current phase: partial safe migration complete
Next: resolve remaining full parser-validation importers through a
Rust/contract-owned parser ingress before removing more runtime-local imports
Reason: `rust-ts-sunset-parser` is blocked by live parser consumers. The prior
`rust-ts-sunset-parser-runtime-type-imports` slice removed type-only imports
from `@runxhq/core/parser`, but runtime-local still has parser value imports
and a local structural `parser-types.js` surface that keeps parser-shaped types
live in the runtime-local package.
Blockers: replacement Rust/contract parser ingress must be identified for each
full skill, graph, runner manifest, tool manifest, install, and reflect-policy
value importer before production imports move.
Allowed follow-up command: `scafld validate rust-ts-sunset-parser-runtime-local-importers --json`
Latest runner update: 2026-05-22T01:36:00+10:00 promoted the executed child
spec from drafts to active and revalidated the current importer census. The
remaining parser value importers are explicit blockers, not hidden work.
Review gate: partial_migration_recorded; remaining_importers_blocked

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
rg -l "@runxhq/core/parser|parser-types\.js" packages/runtime-local/src --glob '!packages/core/src/parser/**' | sort
rg -n "@runxhq/core/parser|parser-types\.js" packages/runtime-local/src --glob '!packages/core/src/parser/**'
rg -l "@runxhq/core/parser" packages/runtime-local/src --glob '!packages/core/src/parser/**' | wc -l
rg -l "parser-types\.js" packages/runtime-local/src --glob '!packages/core/src/parser/**' | wc -l
```

Observed results:
- 23 runtime-local source files reference parser value imports or
  `parser-types.js`.
- 6 runtime-local source files import `@runxhq/core/parser`.
- 21 runtime-local source files import `parser-types.js`.

Migrated value importers:
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
  - Replacement: local `ValidatedTool` construction for imported catalog tools,
    using the same runtime-local-normalized document shape and raw JSON as the
    previous parser-backed construction.
  - Evidence: `pnpm exec vitest run --config vitest.config.ts packages/runtime-local/src/tool-catalogs/index.test.ts`
    and `pnpm typecheck` passed. Full graph-level MCP import runtime test is
    blocked by missing `RUNX_KERNEL_EVAL_BIN` before imported tool invocation.

Remaining blocked value importers:
- `packages/runtime-local/src/harness/publish.ts`
  - Imports: `parseRunnerManifestYaml`, `validateRunnerManifest`.
  - Blocker: publish harness validation still needs parser-owned runner
    manifest validation semantics. A local harness-only YAML parser would skip
    unrelated runner manifest validation and could accept profiles the parser
    rejects.
- `packages/runtime-local/src/harness/runner.ts`
  - Imports: `parseGraphYaml`, `parseRunnerManifestYaml`, `validateGraph`,
    `validateRunnerManifest`.
  - Blocker: deterministic harness receipt ids and inline harness execution
    currently depend on parser-owned graph and runner manifest semantics.
    Replacing only the name/step/case extraction locally would change validation
    coverage.
- `packages/runtime-local/src/runner-local/execution-targets.ts`
  - Imports: `extractSkillQualityProfile`, `parseGraphYaml`,
    `parseRunnerManifestYaml`, `parseSkillMarkdown`, `parseToolManifestJson`,
    `validateGraph`, `validateRunnerManifest`, `validateSkill`,
    `validateSkillArtifactContract`, `validateSkillSource`,
    `validateToolManifest`.
  - Blocker: this is the main runtime ingress for graph, skill, runner, inline
    run, artifact, quality profile, and local tool execution. It needs a
    contract/Rust parser ingress before migration.
- `packages/runtime-local/src/runner-local/index.ts`
  - Imports: `parseSkillMarkdown`, `resolvePostRunReflectPolicy`,
    `validateSkill`.
  - Blocker: `runLocalSkill` strict skill loading and reflect policy projection
    are production runtime behavior.
- `packages/runtime-local/src/runner-local/skill-install.ts`
  - Imports: `parseRunnerManifestYaml`, `validateRunnerManifest`,
    `validateSkillInstall`.
  - Blocker: install validation binds remote/marketplace skill markdown,
    provenance, and optional profile runner names. Requires an install contract
    parser boundary before replacing parser-owned validation.
- `packages/runtime-local/src/sdk/index.ts`
  - Imports: `parseRunnerManifestYaml`, `parseSkillMarkdown`,
    `parseToolManifestJson`, `validateRunnerManifest`, `validateSkill`,
    `validateToolManifest`.
  - Blocker: SDK publish/inspect flows validate skill markdown, runner
    manifests, and local tool manifests. No generated/contract equivalent was
    found in this slice.

## Acceptance

Profile: standard

Definition of done:
- [x] `dod1` Runtime-local direct imports from `@runxhq/core/parser` are removed
  only where a small Rust/contract-backed or local helper migration is safe.
- [x] `dod2` `parser-types.ts` remains temporary and does not gain new parser
  shape exports.
- [x] `dod3` No parser implementation file is deleted or renamed.
- [ ] `dod4` The parent parser census is updated after this slice lands.

Validation:
- [x] `v1` Scafld validates this spec.
  - Command: `scafld validate rust-ts-sunset-parser-runtime-local-importers --json`
  - Expected kind: `exit_code_zero`
  - Status: passed
  - Evidence: returned `{"ok":true,...,"valid":true}`.
- [ ] `v2` Runtime-local parser direct-import census.
  - Command: `rg -n "@runxhq/core/parser" packages/runtime-local/src --glob '!packages/core/src/parser/**'`
  - Expected kind: `no_matches`
  - Status: blocked
  - Evidence: remaining imports in `harness/publish.ts`, `harness/runner.ts`,
    `runner-local/execution-targets.ts`, `runner-local/index.ts`,
    `runner-local/skill-install.ts`, and `sdk/index.ts`. `parseSkillMarkdown`
    is no longer imported by `packages/runtime-local/src/harness/**`.
- [ ] `v3` Parser implementation remains present.
  - Command: `test -d packages/core/src/parser`
  - Expected kind: `exit_code_zero`
  - Status: passed
  - Evidence: `test -d packages/core/src/parser && printf 'parser implementation present\n'`
    printed `parser implementation present`.
- [ ] `v4` Targeted runtime-local tests cover migrated importers.
  - Command: `pnpm exec vitest run --config vitest.config.ts tests/local-skill-runner.test.ts tests/runtime-local-harness.test.ts packages/runtime-local/src/harness/agent-hook.test.ts`
  - Expected kind: `exit_code_zero`
  - Status: partial
  - Evidence: `pnpm exec vitest run --config vitest.config.ts packages/runtime-local/src/harness/agent-hook.test.ts`
    and `pnpm exec vitest run --config vitest.config.ts packages/runtime-local/src/tool-catalogs/index.test.ts`
    passed in the prior slice. `pnpm exec vitest run --config vitest.config.ts packages/runtime-local/src/harness/skill-frontmatter.test.ts packages/runtime-local/src/harness/agent-hook.test.ts`
    and `pnpm typecheck` passed for this slice. `pnpm exec vitest run --config vitest.config.ts tests/inline-x-harness.test.ts tests/skill-publish.test.ts packages/runtime-local/src/harness/agent-hook.test.ts`
    failed in this dirty tree: inline harness cases require
    `RUNX_KERNEL_EVAL_BIN`, and `tests/skill-publish.test.ts` returned an
    unexpected publish report shape before producing the expected publish
    payload.

## Phase 1: Importer Classification

Status: completed
Dependencies: none

Goal: map each runtime-local parser value import to a safe replacement or an
explicit blocker.

Acceptance:
- [x] `ac1` command - Runtime-local parser importer census is current.
  - Command: `rg -n "@runxhq/core/parser|parser-types\.js" packages/runtime-local/src --glob '!packages/core/src/parser/**'`
  - Expected kind: `exit_code_zero`
  - Status: passed
  - Evidence: remaining direct imports are the six blocked files listed above.
- [x] `ac2` command - Value importer assignments are recorded in this spec.
  - Command: `rg -n "Value importers:" .scafld/specs/active/rust-ts-sunset-parser-runtime-local-importers.md`
  - Expected kind: `exit_code_zero`
  - Status: passed
  - Evidence: migrated and blocked assignments recorded under Importer Census.

## Phase 2: Safe Migration

Status: partial
Dependencies: Phase 1

Goal: make only the small importer changes that have a clear replacement and
targeted tests.

Acceptance:
- [ ] `ac3` command - Runtime-local direct parser imports are removed.
  - Command: `rg -n "@runxhq/core/parser" packages/runtime-local/src --glob '!packages/core/src/parser/**'`
  - Expected kind: `no_matches`
  - Status: blocked
  - Evidence: six production importers remain blocked by full parser-validation
    behavior.
- [x] `ac4` command - Parser implementation is untouched.
  - Command: `test -d packages/core/src/parser`
  - Expected kind: `exit_code_zero`
  - Status: passed
  - Evidence: `test -d packages/core/src/parser` succeeded.

## Rollback

- Revert only this importer slice's runtime-local edits. Do not restore or
  delete parser implementation files from this spec.

## Metadata

- created_by: codex
