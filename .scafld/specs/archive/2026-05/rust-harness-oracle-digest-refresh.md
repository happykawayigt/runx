---
spec_version: '2.0'
task_id: rust-harness-oracle-digest-refresh
created: '2026-05-20T07:49:43Z'
updated: '2026-05-20T07:50:39Z'
status: completed
harden_status: not_run
size: medium
risk_level: medium
---

# Rust Harness Oracle Digest Refresh

## Current State

Status: completed
Current phase: final
Next: done
Reason: task completed
Blockers: none
Allowed follow-up command: `none`
Latest runner update: 2026-05-20T07:50:39Z
Review gate: pass

## Summary

Refresh the checked-in Rust harness fixture oracle after canonical child
receipt locator changes and local signature-policy cleanup. The harness YAML
`body_digest` and `receipt_digest` fields are Rust byte-oracle expectations.
The TypeScript runtime-local harness runner still uses the same fixture files
for structural compatibility tests, but it must not treat Rust byte digests as
authoritative for its non-pseudo-signature local receipts.

## Objectives

- Regenerate `fixtures/harness/oracle/*.json` and fixture digest expectations.
- Keep Rust harness replay byte-for-byte oracle parity green.
- Keep TypeScript runtime-local harness structural tests green.
- Make the byte-authority boundary explicit: Rust harness assertions validate
  `body_digest`/`receipt_digest`; TypeScript runtime-local validates shape,
  state, closure, acts, and child refs unless it sees a pseudo-local `sig:`
  receipt.

## Scope

In scope:
- `scripts/generate-rust-harness-fixtures.ts`
- `fixtures/harness/**/*.yaml`
- `fixtures/harness/oracle/*.json`
- `crates/runx-runtime/src/harness/**`
- `crates/runx-runtime/src/receipts.rs`
- `crates/runx-runtime/tests/harness_fixtures.rs`
- `packages/runtime-local/src/harness/runner.ts`

Out of scope:
- Reopening the completed `rust-harness` spec.
- Changing harness spine contract shapes.
- Making TypeScript runtime-local byte-identical with Rust before the
  runtime-local sunset.

## Dependencies

- `rust-harness` completed.
- `rust-receipts-parity` completed with sealed harness receipts as the
  canonical receipt shape.

## Assumptions

- The Rust harness is the byte oracle for `fixtures/harness`.
- TypeScript runtime-local remains a transitional execution path until
  `rust-ts-sunset-runtime-local`; it can share fixture structure but not Rust
  byte digest assertions.

## Touchpoints

- Rust harness fixture generation and replay assertions.
- TypeScript harness fixture runner assertion semantics.

## Risks

- Medium: a shared fixture file can imply one byte oracle for both Rust and TS.
  Mitigation: make TS digest assertions conditional on pseudo-local `sig:`
  receipts and keep Rust fixture check authoritative for digest fields.
- Medium: receipt child locators can desync from child receipt digests.
  Mitigation: generator check plus Rust byte oracle replay.

## Acceptance

Profile: standard

Validation:
- `pnpm fixtures:harness:check`
- `cargo test --manifest-path crates/Cargo.toml -p runx-runtime --test harness_fixtures -- --nocapture`
- `pnpm test -- tests/runtime-local-harness.test.ts packages/runtime-local/src/runner-local/kernel-bridge.test.ts tests/graph-runner.test.ts`
- `pnpm typecheck`
- `git diff --check -- scripts/generate-rust-harness-fixtures.ts fixtures/harness crates/runx-runtime/src/harness crates/runx-runtime/src/receipts.rs crates/runx-runtime/tests/harness_fixtures.rs packages/runtime-local/src/harness/runner.ts`

## Phase 1: Implementation

Status: completed
Dependencies: none

Objective: Complete the requested change.

Changes:
- Refresh fixture YAML and oracle JSON digests.
- Preserve Rust canonical byte-oracle checks.
- Keep TypeScript local harness structural assertions compatible with the shared fixture files.

Acceptance:
- [x] `ac1` command - generated harness oracle is fresh.
  - Command: `pnpm fixtures:harness:check`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-6
- [x] `ac2` command - Rust harness byte oracle passes.
  - Command: `cargo test --manifest-path crates/Cargo.toml -p runx-runtime --test harness_fixtures -- --nocapture`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-7
- [x] `ac3` command - TS runtime-local harness remains structurally green.
  - Command: `pnpm test -- tests/runtime-local-harness.test.ts packages/runtime-local/src/runner-local/kernel-bridge.test.ts tests/graph-runner.test.ts`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-8
- [x] `ac4` command - TypeScript types stay green.
  - Command: `pnpm typecheck`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-9
- [x] `ac5` command - diff whitespace is clean.
  - Command: `git diff --check -- scripts/generate-rust-harness-fixtures.ts fixtures/harness crates/runx-runtime/src/harness crates/runx-runtime/src/receipts.rs crates/runx-runtime/tests/harness_fixtures.rs packages/runtime-local/src/harness/runner.ts`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-10

## Rollback

- Revert this oracle refresh and restore the prior fixture digests only if the
  canonical Rust harness oracle is proven wrong. Do not add a second fixture
  dialect.

## Review

Status: completed
Verdict: pass
Mode: verify
Summary: Human-reviewed override accepted: Focused harness oracle refresh reviewed locally: generator check, Rust harness parity, TS harness structural tests, typecheck, and diff checks all pass; no blockers found.

Attack log:
- `review gate`: manual human audit -> clean (Focused harness oracle refresh reviewed locally: generator check, Rust harness parity, TS harness structural tests, typecheck, and diff checks all pass; no blockers found.)

Findings:
- none

## Self Eval

- Target score: 9. The Rust byte oracle is fresh and TS runtime-local no longer
  confuses shared fixture structure with Rust receipt bytes.

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

- 2026-05-20: Created after `pnpm fixtures:harness:check` reported stale
  harness fixture digests and focused TS harness tests exposed the shared
  fixture byte-oracle ambiguity.
