---
spec_version: '2.0'
task_id: rust-ts-sunset-executor-runtime-local-execute-skill
created: '2026-05-20T05:14:55Z'
updated: '2026-05-20T05:20:48Z'
status: completed
harden_status: not_run
size: small
risk_level: medium
---

# Runtime-local executeSkill ownership

## Current State

Status: completed
Current phase: final
Next: done
Reason: task completed
Blockers: none
Allowed follow-up command: `none`
Latest runner update: 2026-05-20T05:20:48Z
Review gate: pass

## Summary

Remove the final live value import of `executeSkill` from
`@runxhq/core/executor` by moving the adapter dispatch helper into
`packages/runtime-local`. This is a migration slice only; the public
`@runxhq/core/executor` export and remaining type-only importer cleanup are left
to later specs.

## Objectives

- Keep runtime-local skill execution behavior unchanged.
- Replace `packages/runtime-local/src/runner-local/index.ts`'s value import from
  `@runxhq/core/executor` with a runtime-local helper.
- Add focused tests for adapter dispatch, missing adapter failure, and
  credential validation pass-through.
- Re-audit the executor import graph after the slice.

## Scope

- In scope:
  - `packages/runtime-local/src/runner-local/index.ts`
  - a runtime-local helper/test next to runner-local ownership
  - this scafld spec
- Out of scope:
  - registry files
  - Nitrosend target/observer specs
  - deleting `packages/core/src/executor`
  - removing `packages/core/package.json`'s `./executor` export
  - broad type-only importer migration

## Dependencies

- Parent draft: `.scafld/specs/drafts/rust-ts-sunset-executor.md`
- Runtime-local adapter interfaces already live in
  `packages/runtime-local/src/runner-local/adapter-types.ts`.
- Contract validation remains owned by `@runxhq/contracts`.

## Assumptions

- `executeSkill` is only a thin adapter dispatcher and does not need to stay in
  `@runxhq/core` for runtime-local execution.
- Remaining executor references after this slice may be package exports,
  executor-local definitions, comments, or type-only importers.

## Touchpoints

- `packages/runtime-local/src/runner-local/index.ts`
- `packages/runtime-local/src/runner-local/execute-skill.ts`
- `packages/runtime-local/src/runner-local/execute-skill.test.ts`

## Risks

- Accidentally changing adapter invocation payload shape.
- Accidentally widening public runtime-local API surface.
- Running into ambient workspace drift unrelated to this slice.

## Acceptance

Profile: standard

Validation:
- `rg -n 'import \{ executeSkill \} from "@runxhq/core/executor"|from "@runxhq/core/executor"' packages/runtime-local/src/runner-local/index.ts` exits non-zero.
- `pnpm vitest run packages/runtime-local/src/runner-local/execute-skill.test.ts --config vitest.config.ts` exits zero.
- `pnpm typecheck` exits zero or any failure is clearly unrelated to the slice.

## Phase 1: Implementation

Status: completed
Dependencies: none

Objective: Complete the requested change.

Changes:
- Add a runtime-local `executeSkill` helper that uses runtime-local adapter interfaces and contract credential validation.
- Update `runner-local/index.ts` to import the helper locally.
- Add focused regression tests for helper behavior.

Acceptance:
- [x] `ac1` command - Value importer audit
  - Command: `rg -n 'import \{ executeSkill \} from "@runxhq/core/executor"|from "@runxhq/core/executor"' packages/runtime-local/src/runner-local/index.ts`
  - Expected kind: `exit_code_nonzero`
  - Status: pass
  - Evidence: exit code was 1
  - Source event: entry-6
- [x] `ac2` command - Focused helper tests
  - Command: `pnpm vitest run packages/runtime-local/src/runner-local/execute-skill.test.ts --config vitest.config.ts`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-7
- [x] `ac3` command - Workspace typecheck
  - Command: `pnpm typecheck`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-8

## Rollback

- Remove the runtime-local helper/test and restore the previous core executor
  value import in `runner-local/index.ts`.

## Review

Status: completed
Verdict: pass
Mode: discover
Provider: claude:claude-opus-4-7
Output: claude.mcp_submit_review
Summary: Runtime-local executeSkill helper is behavior-equivalent to the original @runxhq/core/executor.executeSkill: identical adapter dispatch, identical missing-adapter failure envelope, identical credential validation pass-through (validateCredentialEnvelopeContract vs the core re-export), and identical AdapterActInvocation payload (same field set, same allowedTools fallback). Helper imports stay within domain boundaries (@runxhq/contracts, @runxhq/core/parser type-only, @runxhq/runtime-local/tool-catalogs type-only, sibling adapter-types). runner-local/index.ts now imports executeSkill from ./execute-skill.js and does not re-export it, so no public surface widening. Audit confirms zero remaining `@runxhq/core/executor` references under packages/runtime-local/src (dist matches are out of scope build artifacts). Tests cover all three spec-required cases (dispatch, missing adapter failure, credential validation rejection). Ambient Rust contract changes are unrelated drift, not attributable to this slice.

Attack log:
- `packages/runtime-local/src/runner-local/execute-skill.ts vs packages/core/src/executor/index.ts::executeSkill`: Behavior parity diff: adapter selection, missing-adapter envelope, credential validation, AdapterActInvocation payload field set and ordering -> clean (Field-for-field identical including allowedTools fallback (options.allowedTools ?? options.skill.allowedTools) and credential pass-through via validateCredentialEnvelopeContract)
- `packages/runtime-local/src/runner-local/execute-skill.ts imports`: Domain boundary scan for forbidden cross-domain value imports (cloud, cli, core/executor, host adapters) -> clean (Only @runxhq/contracts (value), @runxhq/core/parser (type-only), @runxhq/runtime-local/tool-catalogs (type-only matching existing pattern), and sibling adapter-types)
- `packages/runtime-local/src/runner-local/index.ts exports`: Did the slice widen the public runtime-local API surface by re-exporting executeSkill? -> clean (executeSkill is internally imported (line 70) and called at line 988; no export * or named re-export from execute-skill.js. Helper remains package-private.)
- `packages/runtime-local/src (all files)`: Audit: any remaining value or live imports from @runxhq/core/executor after the slice -> clean (rg 'from "@runxhq/core/executor"' under src/ returns zero matches. Dist hits are stale build artifacts out of scope per spec.)
- `packages/runtime-local/src/runner-local/execute-skill.test.ts`: Test coverage vs spec objectives (adapter dispatch, missing adapter failure, credential validation pass-through) -> clean (All three spec-required cases present; credential test asserts adapter is not invoked when validation throws, matching pass-through semantics)
- `task scope vs workspace changes`: Ambient drift attribution: are Rust contract changes part of this slice? -> clean (Rust changes are outside declared task scope and unrelated to executeSkill ownership; classified as ambient drift per session manifest)

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

- 2026-05-20T05:14:55Z: Planned as a narrow scafld-backed slice after auditing
  the parent sunset draft and current live value importer.
