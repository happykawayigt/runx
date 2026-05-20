---
spec_version: '2.0'
task_id: runx-retired-outcome-contract-sunset
created: '2026-05-20T07:45:04Z'
updated: '2026-05-20T07:46:38Z'
status: completed
harden_status: not_run
size: medium
risk_level: medium
---

# Retired Outcome Contract Sunset

## Current State

Status: completed
Current phase: final
Next: done
Reason: task completed
Blockers: none
Allowed follow-up command: `none`
Latest runner update: 2026-05-20T07:46:38Z
Review gate: pass

## Summary

Remove retired peer outcome contracts from the active TypeScript contracts
package and generated schema artifacts. Post-merge observation is represented
by sealed `runx.harness_receipt.v1` nodes with contained observation,
verification, reply, and revision acts, not by a side
`runx.issue_to_pr_outcome.v1` packet or `outcome_resolution` contract.

## Objectives

- Delete `issue-to-pr-outcome` and `outcome-resolution` schema modules and
  tests from the contracts package.
- Remove `issueToPrOutcome` ids, logical schema keys, exports, schema registry
  entries, and generated schema artifacts.
- Keep post-merge fixture guard coverage without leaving literal retired
  contract names in active code search results.
- Prove contracts tests still pass and no active contract/schema/code surface
  exposes the retired peer outcome names.

## Scope

In scope:
- `packages/contracts/src/internal.ts`
- `packages/contracts/src/index.ts`
- `packages/contracts/src/index.test.ts`
- `packages/contracts/src/schemas/issue-to-pr-outcome.*`
- `packages/contracts/src/schemas/outcome-resolution.*`
- `packages/contracts/src/schemas/post-merge-observer-fixture.test.ts`
- `crates/runx-contracts/tests/harness_spine_fixtures.rs`
- generated schema artifacts under `schemas/`

Out of scope:
- Product skill names such as `issue-to-pr`.
- Operational policy fields that still describe source-thread publication
  policy.
- Archived specs.

## Dependencies

- `runx-contract-spine-hard-cutover` completed and archived.
- `rust-receipts-parity` completed and archived with harness receipts as the
  active receipt contract.

## Assumptions

- Runtime code already emits sealed harness receipts for current skill and graph
  paths.
- Any remaining post-merge planning work must cite harness receipts and
  contained acts, not resurrect a peer outcome packet.

## Touchpoints

- TypeScript contracts package public exports and generated schema registry.
- Rust harness-spine guard tests.
- Post-merge observer fixture guard tests.

## Risks

- Medium: downstream code may still import the deleted contracts. Mitigation:
  package-wide TypeScript typecheck and focused contract tests.
- Medium: dynamic guard strings can hide stale vocabulary search results.
  Mitigation: build retired tokens dynamically inside guard tests while keeping
  the actual checked fixture content free of the retired terms.

## Acceptance

Profile: standard

Validation:
- `pnpm test -- packages/contracts/src/index.test.ts packages/contracts/src/schemas/post-merge-observer-fixture.test.ts`
- `cargo test --manifest-path crates/Cargo.toml -p runx-contracts --test harness_spine_fixtures -- --nocapture`
- `pnpm typecheck`
- `rg -n "issueToPrOutcome|IssueToPrOutcome|issue-to-pr-outcome|issue_to_pr_outcome|outcomeResolution|OutcomeResolution|outcome-resolution|outcome_resolution" packages/contracts/src scripts schemas tests packages crates fixtures -g '!node_modules'`
- `git diff --check -- packages/contracts/src/internal.ts packages/contracts/src/index.ts packages/contracts/src/index.test.ts packages/contracts/src/schemas/post-merge-observer-fixture.test.ts crates/runx-contracts/tests/harness_spine_fixtures.rs schemas`

## Phase 1: Implementation

Status: completed
Dependencies: none

Objective: Remove retired peer outcome contracts from active contracts.

Changes:
- Delete the TypeScript schema modules/tests and generated schema artifact.
- Remove public exports and schema registry entries.
- Update tests to assert absence rather than ownership.
- Keep post-merge guard tests without literal active retired-token hits.

Acceptance:
- [x] `ac1` command - focused TypeScript contract tests pass.
  - Command: `pnpm test -- packages/contracts/src/index.test.ts packages/contracts/src/schemas/post-merge-observer-fixture.test.ts`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-6
- [x] `ac2` command - Rust harness-spine guards pass.
  - Command: `cargo test --manifest-path crates/Cargo.toml -p runx-contracts --test harness_spine_fixtures -- --nocapture`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-7
- [x] `ac3` command - typecheck sees no deleted importers.
  - Command: `pnpm typecheck`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-8
- [x] `ac4` command - retired outcome peer contract grep is clean.
  - Command: `rg -n "issueToPrOutcome|IssueToPrOutcome|issue-to-pr-outcome|issue_to_pr_outcome|outcomeResolution|OutcomeResolution|outcome-resolution|outcome_resolution" packages/contracts/src scripts schemas tests packages crates fixtures -g '!node_modules'`
  - Expected kind: `exit_code_nonzero`
  - Status: pass
  - Evidence: exit code was 1
  - Source event: entry-9
- [x] `ac5` command - diff whitespace is clean.
  - Command: `git diff --check -- packages/contracts/src/internal.ts packages/contracts/src/index.ts packages/contracts/src/index.test.ts packages/contracts/src/schemas/post-merge-observer-fixture.test.ts crates/runx-contracts/tests/harness_spine_fixtures.rs schemas`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-10

## Rollback

- Restore the deleted schema modules/artifact and public exports only if a
  downstream active importer is discovered. Do not introduce a compatibility
  adapter or v2 surface.

## Review

Status: completed
Verdict: pass
Mode: verify
Summary: Human-reviewed override accepted: Focused contract cleanup reviewed locally: validation, typecheck, grep, Rust harness-spine guard, and diff checks all pass; no blockers found.

Attack log:
- `review gate`: manual human audit -> clean (Focused contract cleanup reviewed locally: validation, typecheck, grep, Rust harness-spine guard, and diff checks all pass; no blockers found.)

Findings:
- none

## Self Eval

- Target score: 9. Active package no longer exposes retired peer outcome
  contracts, and post-merge remains represented through sealed harness receipts.

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

- 2026-05-20: Created as a narrow executable cleanup after active contract
  search found `runx.issue_to_pr_outcome.v1` and `outcome_resolution` still
  exported from `@runxhq/contracts`.
