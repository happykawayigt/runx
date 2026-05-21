---
spec_version: '2.0'
task_id: rust-ts-sunset-runtime-local-cli-importers
created: '2026-05-21T00:00:00Z'
updated: '2026-05-21T04:31:00Z'
status: completed
harden_status: passed
size: small
risk_level: medium
---

# Runtime-local sunset: CLI importer routing

## Current State

Status: completed
Current phase: final
Next: `rust-ts-sunset-runtime-local-cli-mcp-importer-routing`
Reason: executable first slice carved out of
`rust-ts-sunset-runtime-local`. The npm `@runxhq/cli` package now ships only the
native selector (`bin/runx` plus native package metadata); `packages/cli/src/**`
is no longer the shipped command backend, and the remaining
`@runxhq/runtime-local` / `@runxhq/adapters` imports are pinned by a focused
CLI boundary test to execution-owned blockers only.
Allowed follow-up command: `scafld validate rust-ts-sunset-runtime-local-cli-importers`
Latest runner update: 2026-05-21T04:31:00Z
Review gate: pass

## Summary

Begin the CLI importer-routing slice by removing runtime-local/adapters imports
from CLI code that is not an execution boundary: presentation helpers, local
CLI state helpers, type-only caller/dev surfaces, and stale doctor structure
expectations. This slice does not delete runtime-local/adapters and does not
add a TypeScript compatibility facade. Execution-owned commands that still
need a Rust JSON or native subprocess boundary remain explicit blockers for the
next slice.

## Context

CWD: `.`

Parent draft:
- `.scafld/specs/drafts/rust-ts-sunset-runtime-local.md`

Shipped selector surface:
- `packages/cli/package.json`
- `packages/cli/bin/runx`
- `packages/cli/native/supported-platforms.json`

Old TypeScript source/test surface:
- `packages/cli/src/**`

## Invariants

- Do not modify Rust crates, target-runner, post-merge, dev specs,
  runtime-local package deletion, langchain, or ide-core.
- Do not add a workspace shim, v2 package, path alias, or compatibility import
  for `@runxhq/runtime-local` or `@runxhq/adapters`.
- Keep shipped npm selector behavior unchanged. The selector remains the
  product CLI surface.
- TypeScript source retained in this slice is either test/oracle-only or routes
  through a future explicit native boundary; it must not pretend to be a new
  runtime-local implementation.
- Preserve focused tests that directly exercise CLI TypeScript presentation or
  selector behavior.

## Scope

In scope:
- New local CLI contract/type helpers needed to remove type-only imports.
- CLI presentation rendering helpers.
- CLI install/project state helper ownership.
- CLI caller/dev type annotations.
- CLI doctor structure budget entries.
- Tests that directly exercise the edited CLI TypeScript source.
- `packages/cli/package.json` only if validation proves it necessary.

Out of scope:
- `crates/**`.
- `target-runner/**`.
- post-merge files/specs.
- Rust dev specs.
- Deleting `packages/runtime-local/**` or `packages/adapters/**`.
- `packages/langchain/**`.
- `plugins/ide-core/**`.
- Root package metadata, TS path aliases, vitest aliases, and lockfile cleanup.
- Rewriting execution-owned CLI commands without a separately ratified native
  JSON/subprocess contract.

## Acceptance Criteria

- Runtime-local/adapters imports are removed from CLI source files whose use is
  type-only, presentation-only, local state-only, or stale doctor structure
  expectation-only.
- The npm selector manifest and package contents remain selector-only.
- Focused CLI TypeScript tests covering edited presentation/state surfaces pass.
- A focused negative import check shows the remaining `packages/cli/src/**`
  runtime-local/adapters importers are execution-owned blockers, not accidental
  presentation/type/state imports.
- No compatibility shim or new package alias is introduced.

## Validation Commands

```sh
scafld validate rust-ts-sunset-runtime-local-cli-importers
rg -n "@runxhq/(runtime-local|adapters)" packages/cli/src --glob '!**/dist/**'
pnpm exec tsc -p tsconfig.typecheck.json --noEmit --pretty false
pnpm exec vitest run packages/cli/src/import-boundary.test.ts packages/cli/src/cli-presentation.test.ts packages/cli/src/commands/history.test.ts tests/cli-package.test.ts
git diff --check -- .scafld/specs/archive/2026-05/rust-ts-sunset-runtime-local-cli-importers.md packages/cli/src packages/cli/package.json tests
```

## Remaining Blockers Expected After This Slice

- `packages/cli/src/dispatch.ts` still owns legacy TS execution dispatch for
  skill run, harness, skill add/publish, tool catalog search/inspect, replay,
  diff, and history wiring.
- `packages/cli/src/agent-runtime.ts` still owns legacy managed-agent adapter
  resolution for the TS source backend and must be routed to the native agent
  execution boundary or sunset with the TS backend tests.
- `packages/cli/src/commands/mcp.ts` is covered by
  `rust-ts-sunset-runtime-local-cli-mcp-importer-routing`; after that slice it
  should remain a native process delegation boundary with no runtime-local or
  adapters imports.
- `packages/cli/src/commands/dev/skill-fixture.ts` still owns legacy TS dev
  skill/graph fixture execution and must move to Rust harness/dev execution.
- `packages/cli/src/commands/history.ts` still owns local receipt inspection,
  replay seed, diff, and history projections until Rust exposes the complete
  command surface needed by TS tests or those tests are retired.
- `packages/cli/src/registry-fallback.ts` and `packages/cli/src/skill-refs.ts`
  still own local registry/official-skill helpers until registry install,
  publish, resolve, and official cache flows have an explicit native CLI
  routing contract.

## Rollback And Repair

- Roll back this slice by restoring the edited CLI files and this archived spec.
- If validation finds a removed import was still needed for execution, route
  that execution path through Rust in a follow-up slice rather than adding a
  runtime-local compatibility facade.
