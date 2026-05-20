---
spec_version: '2.0'
task_id: rust-cli-cutover-negative-verifier
created: '2026-05-20T08:08:39Z'
updated: '2026-05-20T08:27:54Z'
status: completed
harden_status: not_run
size: small
risk_level: medium
---

# Rust CLI cutover negative verifier

## Current State

Status: completed
Current phase: final
Next: done
Reason: task completed
Blockers: none
Allowed follow-up command: `none`
Latest runner update: 2026-05-20T08:27:54Z
Review gate: pass

## Summary

Add a read-only release/cutover verifier for Rust CLI candidate artifacts
without changing runtime provider behavior or flipping release authority. The
verifier inspects a supplied candidate binary/package surface and fails closed
when it finds JavaScript fallback hooks, retired receipt/legacy shapes, v2 alias
surfaces, or hidden references to TypeScript runtime packages where static
inspection can see them.

This task is an enabling guard for the blocked `rust-cli-rust-cutover` draft.
It must not edit launcher dispatch, runtime adapters, provider slices, or npm
release authority. It only adds the verifier, deterministic fixtures/tests, and
documentation describing how the final cutover spec should invoke the guard.

## Objectives

- Provide `scripts/check-rust-cli-cutover-negative.mjs` as a deterministic,
  read-only artifact verifier that accepts an explicit `--candidate <path>`.
- Fail closed for missing candidates, empty candidate surfaces, unreadable
  files, unsupported archive paths, symlink escapes, malformed package
  manifests, and detected forbidden release/cutover surfaces.
- Cover no-JS-fallback, no-legacy-shape, no-v2-alias, and no-hidden-package
  reference checks with checked-in fixtures and targeted Vitest coverage.
- Document the verifier as a prerequisite guard for Rust CLI cutover while
  preserving the rule that TypeScript remains authoritative until a separate
  cutover spec flips the release package.

## Scope

- In scope: a standalone Node verifier script under `scripts/`; fixture
  directories under `fixtures/rust-cli-cutover-negative/`; targeted Vitest
  tests under `tests/`; a focused documentation update.
- In scope: static inspection of candidate directories and `.tgz` package
  archives when feasible with local tooling.
- Out of scope: changing `packages/cli/bin/runx.js`, `packages/cli/package.json`,
  `crates/runx-cli/src/**`, runtime providers, adapters, release scripts, or
  package publishing authority.
- Out of scope: declaring the current package cut over. The current
  TypeScript-backed package is expected to fail this verifier until the
  parent cutover spec intentionally changes release artifacts.

## Dependencies

- `rust-cli-rust-cutover` remains blocked and must consume this verifier later.
- Existing package docs in `docs/trusted-kernel-package-truth.md` define the
  Rust parity/cutover authority boundary.
- Existing Node/Vitest tooling is available through `pnpm`.

## Assumptions

- Static inspection cannot prove absence of every possible dynamic fallback,
  but it can deterministically reject known forbidden file names, manifest
  dependencies, package entries, and byte/text tokens.
- The verifier may inspect binary bytes as UTF-8 text for ASCII tokens; it does
  not need to execute candidate binaries.
- The clean fixture is intentionally minimal and is not a real release package.

## Touchpoints

- `scripts/check-rust-cli-cutover-negative.mjs`
- `tests/rust-cli-cutover-negative-verifier.test.ts`
- `fixtures/rust-cli-cutover-negative/**`
- `docs/trusted-kernel-package-truth.md`

## Risks

- Description: false negatives from overly narrow token checks.
  Mitigation: centralize forbidden token groups, cover each group with
  fixtures, and keep docs explicit that this is a negative static guard rather
  than positive parity evidence.
- Description: false positives against legitimate native package metadata.
  Mitigation: distinguish package manifests from byte scans, report exact file
  and rule ids, and keep the forbidden lists tied to the cutover invariants.
- Description: accidental release-authority flip while adding the guard.
  Mitigation: do not edit runtime provider slices or package launcher files.

## Acceptance

Profile: standard

Validation:
- [x] `ac1` command - verifier accepts clean fixture
  - Command: `node scripts/check-rust-cli-cutover-negative.mjs --candidate fixtures/rust-cli-cutover-negative/clean-candidate`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-26
- [x] `ac2` command - targeted verifier tests pass
  - Command: `pnpm exec vitest run tests/rust-cli-cutover-negative-verifier.test.ts`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-27
- [x] `ac3` command - Rust CLI builds without changing release authority
  - Command: `cargo build --manifest-path crates/Cargo.toml -p runx-cli`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-28
- [x] `ac4` command - TypeScript typecheck remains clean
  - Command: `pnpm exec tsc -p tsconfig.typecheck.json --noEmit`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-29

## Phase 1: Verifier and fixtures

Status: completed
Dependencies: none

Objective: Add the standalone negative verifier and deterministic candidate

Changes:
- Create `scripts/check-rust-cli-cutover-negative.mjs`.
- Add clean and failing fixture candidate surfaces under `fixtures/rust-cli-cutover-negative/`.
- Make script output stable JSON for both pass and fail results.

Acceptance:
- [x] `p1_ac1` command - clean fixture passes
  - Command: `node scripts/check-rust-cli-cutover-negative.mjs --candidate fixtures/rust-cli-cutover-negative/clean-candidate`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-6
- [x] `p1_ac2` command - JS fallback fixture fails
  - Command: `node scripts/check-rust-cli-cutover-negative.mjs --candidate fixtures/rust-cli-cutover-negative/js-fallback-candidate`
  - Expected kind: `exit_code_nonzero`
  - Status: pass
  - Evidence: exit code was 1
  - Source event: entry-7
- [x] `p1_ac3` command - legacy/v2/package fixture fails
  - Command: `node scripts/check-rust-cli-cutover-negative.mjs --candidate fixtures/rust-cli-cutover-negative/legacy-v2-package-candidate`
  - Expected kind: `exit_code_nonzero`
  - Status: pass
  - Evidence: exit code was 1
  - Source event: entry-8

## Phase 2: Tests and docs

Status: completed
Dependencies: phase1

Objective: Prove the verifier behavior and document its role in the Rust CLI

Changes:
- Add targeted Vitest coverage for pass/fail behavior, fail-closed missing candidate handling, and stable JSON diagnostics.
- Update `docs/trusted-kernel-package-truth.md` with the verifier command and authority boundary.

Acceptance:
- [x] `p2_ac1` command - targeted tests pass
  - Command: `pnpm exec vitest run tests/rust-cli-cutover-negative-verifier.test.ts`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-13
- [x] `p2_ac2` command - Rust CLI build still passes
  - Command: `cargo build --manifest-path crates/Cargo.toml -p runx-cli`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-14
- [x] `p2_ac3` command - typecheck passes
  - Command: `pnpm exec tsc -p tsconfig.typecheck.json --noEmit`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-15

## Rollback

- Delete `scripts/check-rust-cli-cutover-negative.mjs`, the matching fixtures,
  targeted test file, and the documentation paragraph. No runtime or package
  release behavior is changed by this task.

## Review

Status: completed
Verdict: pass
Mode: verify
Provider: claude:claude-opus-4-7
Output: claude.mcp_submit_review
Summary: Human-reviewed override accepted: Verifier behavior reviewed after fixing oversize-file and archive-traversal fail-closed tests. Remaining workspace mutation findings are attributed to concurrent specs: host-protocol-test-utils to rust-ts-sunset-runtime-local-host-adapters-types and launcher.rs to rust-cli-mcp-runner-selection; no verifier blocker remains.

Attack log:
- `review gate`: manual human audit -> clean (Verifier behavior reviewed after fixing oversize-file and archive-traversal fail-closed tests. Remaining workspace mutation findings are attributed to concurrent specs: host-protocol-test-utils to rust-ts-sunset-runtime-local-host-adapters-types and launcher.rs to rust-cli-mcp-runner-selection; no verifier blocker remains.)

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

- 2026-05-20T08:08:39Z: Created narrow scafld spec for the missing
  Rust CLI cutover negative verifier. Existing `rust-cli-rust-cutover` draft
  remains blocked and explicitly calls for a no-JS/no-legacy/no-v2 release
  artifact guard.
