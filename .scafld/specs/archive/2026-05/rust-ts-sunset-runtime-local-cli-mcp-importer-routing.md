---
spec_version: '2.0'
task_id: rust-ts-sunset-runtime-local-cli-mcp-importer-routing
created: '2026-05-21T04:11:00Z'
updated: '2026-05-21T04:47:17Z'
status: completed
harden_status: not_run
size: small
risk_level: medium
---

# Runtime-local sunset: CLI MCP importer routing

## Current State

Status: completed
Current phase: final
Next: done
Reason: task completed
Blockers: none
Allowed follow-up command: `none`
Latest runner update: 2026-05-21T04:47:17Z
Review gate: pass

## Summary

Replace the TypeScript CLI MCP command implementation with a native process
delegation boundary. This removes `@runxhq/runtime-local` and
`@runxhq/adapters` imports from `packages/cli/src/commands/mcp.ts` without
adding a runtime-local facade, alias, or compatibility shim. The shipped npm
selector remains unchanged and continues to be the product CLI surface.

## Context

CWD: `.`

Parent drafts:
- `.scafld/specs/drafts/rust-ts-sunset-runtime-local.md`
- `.scafld/specs/drafts/rust-ts-sunset-runtime-local-cli-importers.md`

Completed MCP precedents:
- `.scafld/specs/archive/2026-05/rust-runtime-adapters-mcp.md`
- `.scafld/specs/archive/2026-05/rust-cli-mcp-runner-selection.md`
- `.scafld/specs/archive/2026-05/rust-mcp-server-harness-receipt-seal.md`

Touchpoints:
- `packages/cli/src/commands/mcp.ts`
- `packages/cli/src/commands/mcp.test.ts`
- `crates/runx-runtime/src/adapters/mcp/server.rs` (response flush only)

## Invariants

- Do not modify Rust runtime graph or payment execution files.
- Do not touch `target-runner/post-merge`.
- Do not add a TypeScript runtime-local facade, compatibility package, package
  alias, or workspace shim.
- Do not change `packages/cli/package.json`, `packages/cli/bin/runx`, or native
  selector package metadata.
- Preserve the CLI parser shape accepted by the old TypeScript command handler:
  skill refs, `--receipt-dir`, and `--runner` are forwarded to the native
  `runx mcp serve` boundary.
- The old TypeScript source handler must require an explicit native runx binary
  path; it must not reimplement MCP execution in TypeScript.

## Scope

In scope:
- Remove runtime-local/adapters imports from the CLI MCP command source.
- Route the TS handler to a native `runx mcp serve` child process with stdio
  passed through for MCP JSON-RPC framing.
- Flush native MCP server responses after each framed JSON-RPC message so the
  process boundary is usable before stdin closes.
- Update the focused MCP command test to exercise the native command boundary.
- Add focused validation and negative import evidence.

Out of scope:
- Runtime-local package deletion.
- Root package dependency, TS path alias, vitest alias, lockfile, langchain, and
  IDE-core cleanup.
- Reopening Rust MCP adapter semantics beyond the narrow stdio response flush
  needed for process delegation.
- Rewriting remaining CLI execution importers.

## Acceptance Criteria

- `packages/cli/src/commands/mcp.ts` has zero
  `@runxhq/runtime-local`/`@runxhq/adapters` imports.
- The focused MCP command test drives a compiled native `runx` binary through
  the TypeScript handler and verifies list/call JSON-RPC behavior.
- Native MCP framed responses flush through stdout without waiting for stdin
  EOF.
- Selector package behavior remains untouched.
- Remaining CLI runtime-local/adapters importers are outside this MCP command
  slice and are listed as blockers.

## Validation Commands

```sh
scafld validate rust-ts-sunset-runtime-local-cli-mcp-importer-routing
! rg -n "@runxhq/(runtime-local|adapters)" packages/cli/src/commands/mcp.ts packages/cli/src/commands/mcp.test.ts
rg -n "@runxhq/(runtime-local|adapters)" packages/cli/src --glob '!**/dist/**'
pnpm exec tsc -p tsconfig.typecheck.json --noEmit --pretty false
pnpm exec vitest run packages/cli/src/commands/mcp.test.ts
cargo test --manifest-path crates/Cargo.toml -p runx-runtime mcp_server --features mcp -- --nocapture
git diff --check -- .scafld/specs/drafts/rust-ts-sunset-runtime-local-cli-mcp-importer-routing.md packages/cli/src/commands/mcp.ts packages/cli/src/commands/mcp.test.ts crates/runx-runtime/src/adapters/mcp/server.rs
```

## Remaining Blockers Expected After This Slice

- `packages/cli/src/dispatch.ts` still owns legacy TS execution dispatch for
  skill run, harness, skill add/publish, tool catalog search/inspect, replay,
  diff, and history wiring.
- `packages/cli/src/agent-runtime.ts` still owns legacy managed-agent adapter
  resolution for the TS source backend.
- `packages/cli/src/commands/dev/skill-fixture.ts` still owns legacy TS dev
  skill/graph fixture execution.
- `packages/cli/src/commands/history.ts` still owns local receipt inspection,
  replay seed, diff, and history projections.
- `packages/cli/src/registry-fallback.ts` and `packages/cli/src/skill-refs.ts`
  still own local registry/official-skill helpers.

## Rollback And Repair

- Restore the previous `packages/cli/src/commands/mcp.ts` and focused test if
  native MCP delegation cannot preserve MCP JSON-RPC framing.
- Do not repair by adding a runtime-local compatibility facade; route through a
  durable native CLI contract or keep the broader runtime-local sunset blocked.

## Review

Status: completed
Verdict: pass
Mode: verify
Provider: claude:claude-opus-4-7
Output: claude.mcp_submit_review
Summary: Human-reviewed override accepted: Claude review found no completion-blocking implementation issues; only blocking finding was concurrent workspace mutation outside this MCP slice, and low-severity notes are non-blocking.

Attack log:
- `review gate`: manual human audit -> clean (Claude review found no completion-blocking implementation issues; only blocking finding was concurrent workspace mutation outside this MCP slice, and low-severity notes are non-blocking.)

Findings:
- none

