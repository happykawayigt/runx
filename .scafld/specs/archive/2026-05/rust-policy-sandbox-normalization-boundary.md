---
spec_version: '2.0'
task_id: rust-policy-sandbox-normalization-boundary
created: '2026-05-20T07:04:43Z'
updated: '2026-05-20T07:06:44Z'
status: completed
harden_status: not_run
size: medium
risk_level: medium
---

# rust-policy-sandbox-normalization-boundary

## Current State

Status: completed
Current phase: final
Next: done
Reason: task completed
Blockers: none
Allowed follow-up command: `none`
Latest runner update: 2026-05-20T07:06:44Z
Review gate: pass

## Summary

Remove the runtime-local process sandbox adapter's executable dependency on the
TypeScript policy package by owning the small sandbox declaration normalization
locally. Rust policy parity and kernel eval already define the same canonical
shape; this slice removes the package edge without making every local process
spawn depend on an external Rust binary.

## Objectives

- Remove `normalizeSandboxDeclaration` import from
  `packages/runtime-local/src/runner-local/process-sandbox.ts`.
- Preserve current sandbox defaults, denial reasons, metadata, cleanup, and
  CLI/MCP adapter behavior.
- Keep sandbox admission, local skill admission, credential binding, and
  package export deletion out of scope.

## Scope

- In scope:
  - `packages/runtime-local/src/runner-local/process-sandbox.ts`
  - focused process sandbox, CLI tool sandbox, and MCP sandbox tests
- Out of scope:
  - async Rust kernel process calls for every sandboxed spawn
  - `admitSandbox`, `admitLocalSkill`, credential binding, and authority proof
  - deleting `@runxhq/core/policy`

## Dependencies

- Completed Rust policy parity for `policy.normalizeSandboxDeclaration`.
- Completed runtime-local policy type-import boundary.

## Assumptions

- Runtime-local process sandboxing is an adapter boundary and may own the
  structural input/defaulting helper while the authoritative Rust policy parity
  remains covered by kernel fixtures.

## Touchpoints

- Local process sandbox cwd/env/write-path defaults.
- CLI tool adapter sandbox metadata.
- MCP server process sandbox metadata.

## Risks

- Risk: local normalizer drifts from Rust parity. Mitigation: keep it tiny,
  typed, and covered by focused sandbox tests plus kernel fixture checks.

## Acceptance

Profile: standard

Validation:
- `pnpm exec vitest run --config vitest.config.ts packages/runtime-local/src/runner-local/process-sandbox.test.ts packages/adapters/src/cli-tool/index.test.ts packages/adapters/src/mcp/index.test.ts tests/cli-tool-sandbox.test.ts tests/mcp-skill-runner.test.ts tests/mcp-import.test.ts`
- `pnpm fixtures:kernel:check`
- `pnpm typecheck`
- `! rg -n '@runxhq/core/policy' packages/runtime-local/src/runner-local/process-sandbox.ts packages/runtime-local/src/mcp/index.ts`
- `git diff --check -- packages/runtime-local/src/runner-local/process-sandbox.ts .scafld/specs/active/rust-policy-sandbox-normalization-boundary.md`

## Phase 1: Implementation

Status: completed
Dependencies: none

Objective: Remove the process sandbox dependency on TypeScript policy.

Changes:
- Add a runtime-local `RequiredSandboxDeclaration` type.
- Add a local `normalizeSandboxDeclaration` helper matching the canonical sandbox defaults.
- Remove the `@runxhq/core/policy` import from the process sandbox adapter.

Acceptance:
- [x] `ac1` command - Focused sandbox tests pass
  - Command: `pnpm exec vitest run --config vitest.config.ts packages/runtime-local/src/runner-local/process-sandbox.test.ts packages/adapters/src/cli-tool/index.test.ts packages/adapters/src/mcp/index.test.ts tests/cli-tool-sandbox.test.ts tests/mcp-skill-runner.test.ts tests/mcp-import.test.ts`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-6
- [x] `ac2` command - Kernel fixtures stay fresh
  - Command: `pnpm fixtures:kernel:check`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-7
- [x] `ac3` command - Typecheck passes
  - Command: `pnpm typecheck`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-8
- [x] `ac4` command - Scoped files no longer import policy package
  - Command: `! rg -n '@runxhq/core/policy' packages/runtime-local/src/runner-local/process-sandbox.ts packages/runtime-local/src/mcp/index.ts`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-9
- [x] `ac5` command - Diff has no whitespace errors
  - Command: `git diff --check -- packages/runtime-local/src/runner-local/process-sandbox.ts .scafld/specs/active/rust-policy-sandbox-normalization-boundary.md`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-10

## Rollback

- Restore the `normalizeSandboxDeclaration` import from `@runxhq/core/policy`
  and delete the local normalizer type/helper.

## Review

Status: completed
Verdict: pass
Mode: verify
Summary: Human-reviewed override accepted: Focused review after green build gate: sandbox tests, kernel fixture check, typecheck, scoped import grep, and diff check passed; no blockers found.

Attack log:
- `review gate`: manual human audit -> clean (Focused review after green build gate: sandbox tests, kernel fixture check, typecheck, scoped import grep, and diff check passed; no blockers found.)

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
