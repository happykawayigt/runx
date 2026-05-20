---
spec_version: '2.0'
task_id: rust-ts-sunset-parser-runtime-type-imports
created: '2026-05-20T08:17:36Z'
updated: '2026-05-20T08:22:17Z'
status: completed
harden_status: not_run
size: small
risk_level: medium
---

# Rust TS sunset parser runtime type imports

## Current State

Status: completed
Current phase: final
Next: done
Reason: task completed
Blockers: none
Allowed follow-up command: `none`
Latest runner update: 2026-05-20T08:22:17Z
Review gate: pass

## Summary

Remove runtime/package type-only dependencies on `@runxhq/core/parser` from the
runtime-local, adapters, and CLI TypeScript surfaces while preserving parser
validation behavior.

## Objectives

- Replace type-only imports from `@runxhq/core/parser` in `packages/runtime-local/src`,
  `packages/adapters/src`, and `packages/cli/src` with local structural types.
- Leave runtime parser validation and parsing imports in place.
- Keep the slice narrow; do not delete parser code or touch Rust CLI/runtime slices.

## Scope

In scope:
- `packages/runtime-local/src/parser-types.ts`
- Parser type-import consumers under `packages/runtime-local/src`
- `packages/adapters/src/agent/json-schema.ts`
- `packages/cli/src/commands/mcp.ts`

Out of scope:
- Parser implementation deletion.
- Rust CLI/script slices and verifier scripts.
- `crates/runx-cli`
- `crates/runx-runtime/src/target_runner.rs`
- `crates/runx-runtime/src/post_merge_observer.rs`

## Dependencies

- none

## Assumptions

- none

## Touchpoints

- none

## Risks

- none

## Acceptance

Profile: standard

Validation:
- `rg -n "@runxhq/core/parser" packages/runtime-local/src packages/adapters/src packages/cli/src`
- `pnpm exec tsc -p tsconfig.typecheck.json --noEmit`
- `RUNX_KERNEL_EVAL_BIN=/Users/kam/dev/runx/runx/oss/crates/target/debug/runx pnpm exec vitest run --config vitest.config.ts tests/graph-runner.test.ts tests/graph-fanout.test.ts tests/local-skill-runner.test.ts tests/runtime-local-harness.test.ts`

## Phase 1: Implementation

Status: completed
Dependencies: none

Objective: Complete the requested change.

Changes:
- Added runtime-local-owned structural parser type aliases.
- Moved runtime-local parser type consumers to those local aliases.
- Replaced adapters and CLI `SkillInput` type-only parser imports with local structural definitions.
- Left parser validation/parsing imports intact.

Acceptance:
- [x] `ac1` command - Parser import scan
  - Command: `rg -n "@runxhq/core/parser" packages/runtime-local/src packages/adapters/src packages/cli/src`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: remaining matches are parser validation/parsing value imports; type-only parser imports were removed.
- [x] `ac2` command - TypeScript typecheck
  - Command: `pnpm exec tsc -p tsconfig.typecheck.json --noEmit`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
- [x] `ac3` command - Targeted runtime-local tests
  - Command: `RUNX_KERNEL_EVAL_BIN=/Users/kam/dev/runx/runx/oss/crates/target/debug/runx pnpm exec vitest run --config vitest.config.ts tests/graph-runner.test.ts tests/graph-fanout.test.ts tests/local-skill-runner.test.ts tests/runtime-local-harness.test.ts`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: 4 files passed, 23 tests passed

## Rollback

- none

## Review

Status: completed
Verdict: pass
Mode: discover
Provider: claude:claude-opus-4-7
Output: claude.mcp_submit_review
Summary: Type-only `@runxhq/core/parser` dependencies have been removed from runtime-local, adapters, and CLI TypeScript surfaces and replaced with local structural type aliases that match the core shapes byte-for-byte (SkillInput, ValidatedSkill, GraphStep, ExecutionGraph, SkillInstallOrigin, SkillRunnerManifest, etc.). Remaining `@runxhq/core/parser` imports in scope are all value imports (parseSkillMarkdown, parseRunnerManifestYaml, parseToolManifestJson, parseGraphYaml, validateSkill, validateRunnerManifest, validateToolManifest, validateGraph, validateSkillInstall, resolvePostRunReflectPolicy, extractSkillQualityProfile, validateSkillArtifactContract, validateSkillSource). Acceptance evidence records typecheck and runtime-local vitest suite as passing. No boundary violations introduced (core does not depend on runtime-local). Ambient drift is one unrelated negative-verifier test outside scope. No completion-blocking issues found.

Attack log:
- `packages/runtime-local/src/parser-types.ts vs packages/core/src/parser/index.ts + graph.ts + install.ts`: Compare every local structural alias against the core export definitions for shape divergence (field names, optionality, union members). -> clean (SkillInput, SkillRetryPolicy, SkillIdempotencyPolicy, SkillSandboxProfile, SkillSandbox, SkillSource, SkillArtifactContract, SkillQualityProfile, ValidatedSkill, SkillRunnerDefinition, PostRunReflectPolicy, CatalogMetadata, HarnessCallerFixture, HarnessReceiptExpectation, HarnessExpectation, RunnerHarnessCase, RunnerHarnessManifest, SkillRunnerManifest, ValidatedTool, SkillInstallOrigin, GraphContextEdge, GraphRetryPolicy, FanoutGroupPolicy, GraphTransitionGate, GraphPolicy, GraphStep, ExecutionGraph all match the core definitions; ExecutionSemantics is sourced from runtime-local's own copy which is structurally identical to core's.)
- `AC1 — rg matches in packages/runtime-local/src, packages/adapters/src, packages/cli/src`: Re-grep for `@runxhq/core/parser` and confirm every remaining match in source (excluding dist/) is a value-only import. -> clean (All matches are parse*/validate*/resolvePostRunReflectPolicy/extractSkillQualityProfile value imports; no `import type { ... }` from @runxhq/core/parser remains in scoped src files.)
- `packages/adapters/src/agent/json-schema.ts → packages/adapters/src/agent/runtime-tools.ts consumer`: Confirm callers (skillInputsToJsonSchema(target.skill.inputs)) still typecheck when SkillInput is locally redefined and the input comes from a ValidatedSkill returned by @runxhq/core/parser. -> clean (Local SkillInput shape matches core's exactly; structural compatibility holds for Readonly<Record<string, SkillInput>>.)
- `packages/cli/src/commands/mcp.ts local SkillInput shadow + skillInputsToJsonSchema`: Verify the locally redefined SkillInput in mcp.ts is used consistently and the schema builder still accepts skill.inputs returned from @runxhq/runtime-local skill resolution. -> clean (Local interface matches; consumer call sites use the same local definition without shape drift.)
- `Domain boundary invariant`: Check that no @runxhq/core source file pulls in @runxhq/runtime-local types (e.g. via parser-types), and that adapters/CLI do not regress the dependency direction. -> clean (core/src/parser only imports within core; parser-types lives in @runxhq/runtime-local and is consumed only downstream. Adapters/CLI local clones avoid forcing a new dependency on runtime-local types.)
- `Scope drift`: Compare ambient_drift entries against task scope to ensure unrelated dirt is not credited to this slice. -> clean (Only tests/rust-cli-cutover-negative-verifier.test.ts is outside task scope; flagged as ambient context only, not a finding.)

Findings:
- none

## Self Eval

- none

## Deviations

- The targeted vitest command fails without `RUNX_KERNEL_EVAL_BIN`; reran with the existing workspace binary at `crates/target/debug/runx`, matching `scripts/test-workspace.mjs`.

## Metadata

- created_by: scafld

## Origin

Created by: scafld
Source: plan

## Harden Rounds

- none

## Planning Log

- none
