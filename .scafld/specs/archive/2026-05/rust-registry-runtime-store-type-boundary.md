---
spec_version: '2.0'
task_id: rust-registry-runtime-store-type-boundary
created: '2026-05-20T05:38:00Z'
updated: '2026-05-20T05:37:12Z'
status: completed
harden_status: not_run
size: small
risk_level: medium
---

# Rust registry runtime store type boundary

## Current State

Status: completed
Current phase: final
Next: done
Reason: task completed
Blockers: none
Allowed follow-up command: `none`
Latest runner update: 2026-05-20T05:37:12Z
Review gate: pass

## Summary

Remove type-only `RegistryStore` imports from runtime-local and CLI command
surfaces where only graph/runtime materialization needs a minimal
`getVersion/listVersions` store. This does not delete core registry, change
skill install behavior, or touch official cache acquisition. It narrows one
more registry dependency to the runtime-owned resolver boundary.

## Objectives

- Import `RegistryStore` from `./registry-resolver.js` inside runtime-local
  instead of `@runxhq/core/registry`.
- Import `RegistryStore` from `@runxhq/runtime-local` in CLI command dependency
  types instead of `@runxhq/core/registry`.
- Preserve structural compatibility with existing core registry stores passed
  from CLI dispatch.
- Keep full registry implementation deletion blocked on `skill-install`,
  `official-cache`, SDK, CLI dispatch/skill refs, and test fixtures.

## Scope

In scope:
- `packages/runtime-local/src/runner-local/index.ts`
- `packages/runtime-local/src/harness/runner.ts`
- `packages/cli/src/commands/dev.ts`
- `packages/cli/src/commands/mcp.ts`

Out of scope:
- `packages/runtime-local/src/runner-local/skill-install.ts`
- `packages/runtime-local/src/runner-local/official-cache.ts`
- `packages/runtime-local/src/sdk/index.ts`
- `packages/cli/src/dispatch.ts`
- `packages/cli/src/skill-refs.ts`
- Deleting `packages/core/src/registry/**`

## Dependencies

- Completed `rust-registry-runtime-resolver-boundary`, which introduced the
  runtime-local `RegistryStore` interface.

## Assumptions

- Core `RegistryStore` remains structurally assignable to the runtime-local
  resolver interface because it has `getVersion` and `listVersions`.

## Touchpoints

- Runtime-local graph/harness options.
- CLI dev/MCP dependency types.

## Risks

- Low: a command dependency could rely on broader registry methods through the
  type. Mitigation: this slice only updates dependency types and then runs
  typecheck plus focused graph/harness tests.

## Acceptance

Profile: standard

Validation:
- `pnpm typecheck`
- `pnpm vitest run tests/runtime-local-harness.test.ts tests/graph-registry-refs.test.ts tests/runtime-local-registry-resolver.test.ts --config vitest.config.ts`
- `! rg -n 'import type \\{ RegistryStore \\} from "@runxhq/core/registry"|import \\{ type RegistryStore \\} from "@runxhq/core/registry"' packages/runtime-local/src/runner-local/index.ts packages/runtime-local/src/harness/runner.ts packages/cli/src/commands/dev.ts packages/cli/src/commands/mcp.ts`
- `git diff --check -- packages/runtime-local/src/runner-local/index.ts packages/runtime-local/src/harness/runner.ts packages/cli/src/commands/dev.ts packages/cli/src/commands/mcp.ts .scafld/specs/active/rust-registry-runtime-store-type-boundary.md`

## Phase 1: Implementation

Status: completed
Dependencies: none

Objective: Move type-only graph/harness registry store plumbing to the

Changes:
- Update runtime-local internal imports to use `type RegistryStore` from `./registry-resolver.js`.
- Update CLI command dependency types to use `type RegistryStore` from `@runxhq/runtime-local`.

Acceptance:
- [x] `ac1` command - TypeScript typecheck passes.
  - Command: `pnpm typecheck`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-6
- [x] `ac2` command - Focused runtime registry/harness tests pass.
  - Command: `pnpm vitest run tests/runtime-local-harness.test.ts tests/graph-registry-refs.test.ts tests/runtime-local-registry-resolver.test.ts --config vitest.config.ts`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-7
- [x] `ac3` command - Scoped core-registry type imports are gone.
  - Command: `! rg -n 'import type \{ RegistryStore \} from "@runxhq/core/registry"|import \{ type RegistryStore \} from "@runxhq/core/registry"' packages/runtime-local/src/runner-local/index.ts packages/runtime-local/src/harness/runner.ts packages/cli/src/commands/dev.ts packages/cli/src/commands/mcp.ts`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-8

## Rollback

- Restore the four type-only imports to `@runxhq/core/registry`; no runtime data
  migration is involved.

## Review

Status: completed
Verdict: pass
Mode: verify
Summary: Human-reviewed override accepted: type-only registry store boundary slice verified by typecheck, focused graph/harness tests, scoped import audit, and diff check

Attack log:
- `review gate`: manual human audit -> clean (type-only registry store boundary slice verified by typecheck, focused graph/harness tests, scoped import audit, and diff check)

Findings:
- none

## Self Eval

- none

## Deviations

- none

## Metadata

- created_by: codex

## Origin

Created by: scafld
Source: plan

## Harden Rounds

- none
