---
spec_version: '2.0'
task_id: rust-ts-sunset-parser
created: '2026-05-18T00:00:00Z'
updated: '2026-05-18T00:00:00Z'
status: draft
harden_status: not_run
size: medium
risk_level: high
---

# TS sunset: parser

## Current State

Status: draft
Current phase: blocked
Next: wait for parser importer migration specs to remove live TS consumers.
Reason: draft created under `plans/rust-takeover.md`. Third TS sunset. On
2026-05-20 this draft was rechecked again and the deletion objective is not
currently valid because 56 live files still import `@runxhq/core/parser`,
relative `packages/core/src/parser` modules, or runtime-local parser type
surfaces.
Blockers: parser importers still live after `rust-ts-sunset-policy` completion.
Allowed follow-up command: `scafld harden rust-ts-sunset-parser`
Latest runner update: 2026-05-20 importer census refreshed; deletion remains
blocked and no harden/build should run for this draft until owning importer
migration specs clear the census.
Review gate: blocked

## Summary

Delete `packages/core/src/parser/`. By the time this spec runs, the Rust
runtime parses skills, graphs, and execution profiles, and no live TS
consumer reads from `@runxhq/core/parser`.

2026-05-20 validation update: this precondition is false in the current
checkout. Do not approve or execute the deletion phase until the importer census
below is clean.

2026-05-20 second validation update: the importer census still finds 56 files.
The largest surviving groups are runtime-local execution/parser-type surfaces,
CLI command readers, fixture/oracle generators, and tests. This update is
inspection evidence only; it does not make deletion executable.

## Context

CWD: `.`

Packages:
- `@runxhq/core`
- `crates/runx-parser`
- Every TS package that imports from `@runxhq/core/parser`

Current TypeScript sources:
- `packages/core/src/parser/**` (to be deleted)
- `packages/core/src/index.ts`
- All TS importers (enumerated in Phase 1)

Files impacted:
- `packages/core/src/parser/` (deleted)
- `packages/core/src/index.ts`

Invariants:
- Parsed AST shape consumers (authoring, marketplaces, registry, executor)
  have either migrated to Rust or are themselves sunsetting.
- The deletion phase must not remove `packages/core/src/parser/**` while any
  production package, script, fixture generator, or test still imports it.
- Until the importer census is clean, parser changes are limited to parity
  maintenance, fixture validation, and this blocker record.

## Objectives

- Enumerate importers; verify migration.
- Delete TS parser implementation.

Current executable slice:
- Enumerate importers and record that the sunset deletion is blocked.
- Keep Rust parser parity tests passing while consumers migrate elsewhere.

## Scope

In scope:
- TS parser deletion.

Out of scope:
- Authoring tools that consume parsed AST today (their own sunset path).

## Dependencies

- `rust-ts-sunset-policy`.
- `rust-parser-parity` complete and consumed.

## Blocker Census

Checked on 2026-05-20:

- `rust-ts-sunset-policy` is archived as completed.
- `rust-parser-parity` is archived as completed.
- The "consumed" part of `rust-parser-parity complete and consumed` is not
  satisfied for TS sunset because parser imports remain live.

Importer commands:

```bash
rg -l "@runxhq/core/parser" packages tests scripts --glob '!packages/core/src/parser/**' | wc -l
rg -l "(\.\./parser|\.\./\.\./parser|packages/core/src/parser)" packages/core scripts tests --glob '!packages/core/src/parser/**' | wc -l
rg -n "@runxhq/core/parser|\.\./parser|packages/core/src/parser" packages tests scripts --glob '!packages/core/src/parser/**' | wc -l
```

Observed results on 2026-05-20:

- 50 files import `@runxhq/core/parser`.
- 7 files refer to relative or direct `packages/core/src/parser` paths outside
  the parser directory.
- 59 total import/reference hits remain outside `packages/core/src/parser/**`.

Representative live production importers:

- `packages/runtime-local/src/runner-local/execution-targets.ts`
- `packages/runtime-local/src/runner-local/index.ts`
- `packages/runtime-local/src/harness/runner.ts`
- `packages/runtime-local/src/sdk/index.ts`
- `packages/cli/src/commands/doctor.ts`
- `packages/cli/src/commands/mcp.ts`
- `packages/cli/src/commands/tool.ts`
- `packages/core/src/config/index.ts`
- `packages/core/src/registry/ingest.ts`
- `packages/adapters/src/agent/json-schema.ts`

Deletion acceptance, when unblocked:

```bash
! rg -n "@runxhq/core/parser|\.\./parser|packages/core/src/parser" packages tests scripts --glob '!packages/core/src/parser/**'
cargo test --manifest-path crates/Cargo.toml -p runx-parser
pnpm exec vitest run packages/core/src/parser/index.test.ts packages/core/src/parser/graph.test.ts
scafld validate rust-ts-sunset-parser
```

This spec remains a draft and must be re-approved only after the importer
census reaches zero or after a narrower parser-bridge spec explicitly owns the
remaining importers.

## Open Questions

- None at draft time.
