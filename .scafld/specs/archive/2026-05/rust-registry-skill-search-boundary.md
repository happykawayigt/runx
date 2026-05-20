---
spec_version: '2.0'
task_id: rust-registry-skill-search-boundary
created: '2026-05-20T05:00:37Z'
updated: '2026-05-20T05:02:18Z'
status: completed
harden_status: not_run
size: medium
risk_level: medium
---

# Rust registry skill search boundary

## Current State

Status: completed
Current phase: final
Next: done
Reason: task completed
Blockers: none
Allowed follow-up command: `none`
Latest runner update: 2026-05-20T05:02:18Z
Review gate: pass

## Summary

Move one live TS registry importer onto the native Rust registry process
boundary without deleting the TS registry implementation. This slice covers CLI
skill search only: when explicitly requested, `runx skill search --source
registry` shells to the native `runx registry search --json` command added by
`rust-registry-cli-search`.

This is not the full registry sunset. The default TS path remains in place
until the remaining runtime/CLI registry importers are migrated.

## Objectives

- Add an explicit opt-in boundary for registry skill search:
  `RUNX_RUST_REGISTRY_SEARCH=1` plus `RUNX_RUST_REGISTRY_BIN=<command>`.
- Parse the native Rust registry search envelope
  `{ status: "success", registry: { action: "search", results: [...] } }`.
- Keep fixture marketplace and bundled skill search out of the Rust registry
  process path.
- Preserve the default TS registry search path until full importer deletion is
  safe.
- Record the slice against the active `rust-ts-sunset-registry` blocker.

## Scope

In scope:
- `packages/cli/src/skill-refs.ts`
- `tests/skill-search.test.ts`
- `.scafld/specs/active/rust-ts-sunset-registry.md`

Out of scope:
- Deleting `packages/core/src/registry/**`.
- Removing the `@runxhq/core/registry` export.
- Migrating `runx skill add`, runtime graph registry refs, official cache, SDK,
  or harness registry importers.
- Adding a TS compatibility shim over Rust registry internals.

## Dependencies

- `rust-registry-cli-search` completed native `runx registry search --json`.
- Active `rust-ts-sunset-registry` importer census.

## Assumptions

- A process boundary is acceptable for this CLI importer during the hard
  cutover. The binary path is explicit so tests and adopters cannot silently
  pick up an unintended executable.
- Opt-in gating is intentional for this slice; full cutover flips call sites
  only after the remaining importers have equivalent coverage.

## Touchpoints

- CLI skill search.
- Active registry sunset spec.
- Skill-search CLI tests.

## Risks

- Risk: accidental routing of marketplace or bundled searches through the
  registry process. Mitigation: focused test proves fixture marketplace bypass.
- Risk: accepting malformed native registry JSON. Mitigation: parse the
  canonical search envelope and validate required result fields before
  returning.
- Risk: seeming to close the registry sunset too early. Mitigation: the active
  registry spec still lists remaining importers and keeps deletion blocked.

## Acceptance

Profile: standard

Validation:
- `scafld validate rust-registry-skill-search-boundary --json`
- `pnpm vitest run tests/skill-search.test.ts`
- `pnpm typecheck`
- `cargo test --manifest-path crates/Cargo.toml -p runx-cli registry -- --nocapture`
- `git diff --check -- packages/cli/src/skill-refs.ts tests/skill-search.test.ts .scafld/specs/drafts/rust-registry-skill-search-boundary.md .scafld/specs/active/rust-ts-sunset-registry.md`

## Phase 1: Implementation

Status: completed
Dependencies: none

Objective: Route explicit registry-only skill search through the Rust CLI
registry boundary.

Changes:
- Added `searchRegistryViaRustCli` with spawn timeout and explicit env gating.
- Added native registry search envelope parsing and result field validation.
- Added CLI coverage for opt-in native registry search.
- Added coverage proving fixture marketplace search does not use the native
  registry process.

Acceptance:
- [ ] `ac1` command - Primary validation command
  - Command: see Validation
  - Expected kind: `exit_code_zero`
  - Status: pending

## Rollback

- none

## Review

Status: completed
Verdict: pass
Mode: verify
Summary: Human-reviewed override accepted: Focused registry skill-search boundary reviewed against implemented scope; Vitest skill search, pnpm typecheck, Rust CLI registry tests, and diff checks passed. Full registry sunset remains blocked by remaining importers.

Attack log:
- `review gate`: manual human audit -> clean (Focused registry skill-search boundary reviewed against implemented scope; Vitest skill search, pnpm typecheck, Rust CLI registry tests, and diff checks passed. Full registry sunset remains blocked by remaining importers.)

Findings:
- none

## Self Eval

- none

## Deviations

- Captured after implementation because a parallel worker had already landed
  the bounded importer migration. This spec exists to execute and verify that
  slice through scafld rather than leaving it as ungoverned drift.

## Metadata

- created_by: scafld

## Origin

Created by: scafld
Source: plan

## Harden Rounds

- none

## Planning Log

- none
