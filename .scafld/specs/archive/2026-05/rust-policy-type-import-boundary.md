---
spec_version: '2.0'
task_id: rust-policy-type-import-boundary
created: '2026-05-20T07:01:01Z'
updated: '2026-05-20T07:03:08Z'
status: completed
harden_status: not_run
size: medium
risk_level: medium
---

# rust-policy-type-import-boundary

## Current State

Status: completed
Current phase: final
Next: done
Reason: task completed
Blockers: none
Allowed follow-up command: `none`
Latest runner update: 2026-05-20T07:03:08Z
Review gate: pass

## Summary

Remove the remaining policy-package type-only imports from runtime-local
surfaces that do not need TypeScript policy behavior. This is a narrow cleanup
slice before the larger sandbox/local-admission cutover.

## Objectives

- Move graph-scope grant typing to the runtime-local Rust kernel bridge type.
- Own the local process sandbox declaration type in runtime-local instead of
  importing it from `@runxhq/core/policy`.
- Preserve the executable sandbox normalization import for a later Rust policy
  slice.

## Scope

- In scope:
  - `packages/runtime-local/src/runner-local/runner-helpers.ts`
  - `packages/runtime-local/src/mcp/index.ts`
  - `packages/runtime-local/src/runner-local/process-sandbox.ts`
- Out of scope:
  - removing `normalizeSandboxDeclaration`
  - removing local-skill admission policy imports
  - deleting `@runxhq/core/policy`

## Dependencies

- Completed graph-scope Rust kernel bridge exposes `GraphScopeGrant`.

## Assumptions

- Runtime-local may define structural policy input types while executable
  behavior is still routed through Rust or pending a later Rust bridge.

## Touchpoints

- MCP sandbox option typing.
- Runtime graph grant default helper.
- Local process sandbox option typing.

## Risks

- Risk: structural sandbox type drifts from normalized policy behavior.
  Mitigation: keep it colocated with the process sandbox adapter and typecheck
  current callers.

## Acceptance

Profile: standard

Validation:
- `pnpm typecheck`
- `pnpm exec vitest run --config vitest.config.ts tests/mcp-skill-runner.test.ts tests/mcp-import.test.ts tests/graph-receipt-governance.test.ts`
- `! rg -n '@runxhq/core/policy.*GraphScopeGrant|@runxhq/core/policy.*SandboxDeclaration|import type .*@runxhq/core/policy' packages/runtime-local/src/runner-local/runner-helpers.ts packages/runtime-local/src/mcp/index.ts packages/runtime-local/src/runner-local/process-sandbox.ts`
- `git diff --check -- packages/runtime-local/src/runner-local/runner-helpers.ts packages/runtime-local/src/mcp/index.ts packages/runtime-local/src/runner-local/process-sandbox.ts .scafld/specs/active/rust-policy-type-import-boundary.md`

## Phase 1: Implementation

Status: completed
Dependencies: none

Objective: Remove non-executable runtime-local policy type imports.

Changes:
- Import `GraphScopeGrant` from the kernel bridge.
- Export runtime-local `SandboxDeclaration` / `SandboxProfile` types from the local process sandbox adapter.
- Import MCP sandbox typing from the local process sandbox adapter.

Acceptance:
- [x] `ac1` command - TypeScript typecheck passes
  - Command: `pnpm typecheck`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-6
- [x] `ac2` command - Focused MCP and graph tests pass
  - Command: `pnpm exec vitest run --config vitest.config.ts tests/mcp-skill-runner.test.ts tests/mcp-import.test.ts tests/graph-receipt-governance.test.ts`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-7
- [x] `ac3` command - Type-only policy imports are gone from scoped files
  - Command: `! rg -n '@runxhq/core/policy.*GraphScopeGrant|@runxhq/core/policy.*SandboxDeclaration|import type .*@runxhq/core/policy' packages/runtime-local/src/runner-local/runner-helpers.ts packages/runtime-local/src/mcp/index.ts packages/runtime-local/src/runner-local/process-sandbox.ts`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-8
- [x] `ac4` command - Diff has no whitespace errors
  - Command: `git diff --check -- packages/runtime-local/src/runner-local/runner-helpers.ts packages/runtime-local/src/mcp/index.ts packages/runtime-local/src/runner-local/process-sandbox.ts .scafld/specs/active/rust-policy-type-import-boundary.md`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-9

## Rollback

- Restore the prior type imports from `@runxhq/core/policy`.

## Review

Status: completed
Verdict: pass
Mode: verify
Summary: Human-reviewed override accepted: Focused review after green build gate: typecheck, MCP/graph tests, scoped policy-import grep, and diff check passed; no blockers found.

Attack log:
- `review gate`: manual human audit -> clean (Focused review after green build gate: typecheck, MCP/graph tests, scoped policy-import grep, and diff check passed; no blockers found.)

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
