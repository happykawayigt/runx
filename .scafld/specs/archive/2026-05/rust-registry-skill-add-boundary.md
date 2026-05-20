---
spec_version: '2.0'
task_id: rust-registry-skill-add-boundary
created: '2026-05-20T05:07:56Z'
updated: '2026-05-20T05:10:45Z'
status: completed
harden_status: not_run
size: medium
risk_level: medium
---

# Rust registry skill add boundary

## Current State

Status: completed
Current phase: final
Next: done
Reason: task completed
Blockers: none
Allowed follow-up command: `none`
Latest runner update: 2026-05-20T05:10:45Z
Review gate: pass

## Summary

Move one more CLI registry importer onto the native Rust registry process
boundary without deleting the TS registry implementation. This slice covers
`runx skill add <registry-ref>` only when explicitly requested. Marketplace and
GitHub URL installs keep their existing paths.

This is not the full registry sunset. The default TS path remains in place
until runtime graph resolution, SDK, official cache, and harness registry
importers are migrated.

## Objectives

- Add an explicit opt-in boundary for registry skill install:
  `RUNX_RUST_REGISTRY_INSTALL=1` plus `RUNX_RUST_REGISTRY_BIN=<command>`.
- Parse the native Rust registry install envelope and map the install payload
  back to the existing CLI presentation shape.
- Pass `--to`, `--version`, `--digest`, `--registry`, and remote
  `--installation-id` into the native command when present.
- Keep marketplace installs outside the Rust registry process path.
- Record the slice against the active `rust-ts-sunset-registry` blocker.

## Scope

In scope:
- `packages/cli/src/native-registry.ts`
- `packages/cli/src/skill-refs.ts`
- `packages/cli/src/dispatch.ts`
- `tests/skill-add.test.ts`
- `tests/skill-search.test.ts`
- `crates/runx-cli/src/registry.rs`
- `crates/runx-cli/src/launcher.rs`
- `crates/runx-cli/tests/launcher.rs`
- `.scafld/specs/active/rust-ts-sunset-registry.md`

Out of scope:
- Deleting `packages/core/src/registry/**`.
- Removing the `@runxhq/core/registry` export.
- Migrating runtime graph registry refs, official cache, SDK, harness, dev, or
  MCP registry importers.
- Changing marketplace install behavior.

## Dependencies

- `rust-registry-cli-search` completed native `runx registry install --json`.
- `rust-registry-skill-search-boundary` factored registry process search.
- Active `rust-ts-sunset-registry` importer census.

## Assumptions

- A process boundary is acceptable for this CLI importer during the hard
  cutover. The binary path is explicit so tests and adopters cannot silently
  pick up an unintended executable.
- The `runx skill add` JSON shape should remain stable for the CLI while
  registry ownership moves behind the process boundary.

## Touchpoints

- CLI skill add.
- Native Rust registry install flag parsing.
- Active registry sunset spec.
- Skill-add and skill-search CLI tests.

## Risks

- Risk: native install ignores digest pinning. Mitigation: native registry
  parser accepts `--digest` and passes it into Rust install validation.
- Risk: marketplace installs accidentally route through registry. Mitigation:
  focused test proves fixture marketplace bypass.
- Risk: duplicated process-spawn code. Mitigation: common
  `native-registry.ts` helper owns search/install process calls and envelope
  parsing.

## Acceptance

Profile: standard

Validation:
- `scafld validate rust-registry-skill-add-boundary --json`
- `pnpm vitest run tests/skill-add.test.ts tests/skill-search.test.ts`
- `pnpm typecheck`
- `cargo test --manifest-path crates/Cargo.toml -p runx-cli --test launcher registry -- --nocapture`
- `cargo test --manifest-path crates/Cargo.toml -p runx-cli --test registry -- --nocapture`
- `cargo check --manifest-path crates/Cargo.toml -p runx-cli`
- `git diff --check -- packages/cli/src/native-registry.ts packages/cli/src/skill-refs.ts packages/cli/src/dispatch.ts tests/skill-add.test.ts tests/skill-search.test.ts crates/runx-cli/src/registry.rs crates/runx-cli/src/launcher.rs crates/runx-cli/tests/launcher.rs .scafld/specs/drafts/rust-registry-skill-add-boundary.md .scafld/specs/active/rust-ts-sunset-registry.md`

## Phase 1: Implementation

Status: completed
Dependencies: none

Objective: Route explicit registry-only skill install through the Rust CLI
registry boundary.

Changes:
- Added `native-registry.ts` as the shared TS process boundary helper for
  native registry search/install.
- Refactored registry skill search onto the shared helper.
- Added opt-in native registry install handling in CLI dispatch.
- Added Rust registry `--digest` parsing and install validation plumbing.
- Added CLI tests for native registry install and marketplace bypass.

Acceptance:
- [ ] `ac1` command - Primary validation command
  - Command: see Validation
  - Expected kind: `exit_code_zero`
  - Status: pending

## Rollback

- Revert this slice and leave `runx skill add` on the TS registry path. Do not
  replace it with a TS compatibility shim.

## Review

Status: completed
Verdict: pass
Mode: verify
Summary: Human-reviewed override accepted: Focused registry skill-add boundary reviewed against implemented scope; skill add/search Vitest checks, pnpm typecheck, Rust launcher/registry tests, cargo check, fmt, active registry spec validation, and diff checks passed. Full registry sunset remains blocked by runtime importers.

Attack log:
- `review gate`: manual human audit -> clean (Focused registry skill-add boundary reviewed against implemented scope; skill add/search Vitest checks, pnpm typecheck, Rust launcher/registry tests, cargo check, fmt, active registry spec validation, and diff checks passed. Full registry sunset remains blocked by runtime importers.)

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
