---
spec_version: '2.0'
task_id: runx-post-merge-observer-idempotency-contract
created: '2026-05-20T05:29:31Z'
updated: '2026-05-20T05:34:24Z'
status: completed
harden_status: not_run
size: small
risk_level: medium
---

# Post-merge observer idempotency contract

## Current State

Status: completed
Current phase: final
Next: done
Reason: task completed
Blockers: none
Allowed follow-up command: `none`
Latest runner update: 2026-05-20T05:34:24Z
Review gate: pass

## Summary

Close the contract-level repeated observer signal blocker for the post-merge
observer. The planner should expose idempotency material that is stable across
duplicate webhook/scheduled observations of the same source issue, target PR,
closure key, and contained act forms. This is a pure Rust contract proof; it
does not add a live GitHub observer, scheduler, registry behavior, or executor
behavior.

## Objectives

- Add explicit closure key and contained act forms to the post-merge observer
  idempotency plan.
- Prove repeated merged-and-verified provider observations with different
  delivery timestamps produce the same idempotency identity.
- Preserve distinct idempotency content for materially different closure states
  such as failed verification.
- Update the parent `runx-post-merge-outcome-observer` draft to record this
  completed contract-level slice while leaving live observer work open.

## Scope

In scope:
- `crates/runx-contracts/src/post_merge_observer.rs`
- `crates/runx-contracts/tests/post_merge_observer.rs`
- `.scafld/specs/drafts/runx-post-merge-outcome-observer.md`

Out of scope:
- Registry files, executor files, runtime provider adapters, live GitHub API
  calls, schedulers, source-thread publication, and issue mutation.
- Target runner execution, checkout, and PR creation/update.

## Dependencies

- Completed `runx-post-merge-observer-closure-planner` and
  `runx-post-merge-observer-harness-fixture` slices.
- The parent `runx-post-merge-outcome-observer` draft remains the broad
  runtime/publication blocker.

## Assumptions

- Duplicate provider observations can differ in delivery/observation timestamp
  while representing the same provider terminal state.
- The final closure key and contained act forms are sufficient contract
  material for downstream receipt/outbox dedupe before live provider adapters
  exist.

## Touchpoints

- Post-merge observer Rust contract and its focused tests.
- Parent scafld draft status notes only.

## Risks

- Medium: making idempotency too broad could hide a changed closure state.
  Mitigated by keeping closure state and criterion statuses in the content hash
  and adding a failed-verification contrast assertion.
- Low: exposing act forms in idempotency changes the serialized contract shape.
  The post-merge observer contract is still additive and not a live release
  path.

## Acceptance

Profile: standard

Validation:
- `cargo test --manifest-path crates/Cargo.toml -p runx-contracts --test post_merge_observer -- --nocapture`
- `cargo fmt --manifest-path crates/Cargo.toml --all --check`
- `git diff --check -- crates/runx-contracts/src/post_merge_observer.rs crates/runx-contracts/tests/post_merge_observer.rs .scafld/specs/drafts/runx-post-merge-outcome-observer.md .scafld/specs/active/runx-post-merge-observer-idempotency-contract.md`
- `! printf '%s\n' crates/runx-contracts/src/post_merge_observer.rs crates/runx-contracts/tests/post_merge_observer.rs .scafld/specs/drafts/runx-post-merge-outcome-observer.md | rg '(^|/)(registry|executor)(/|\\.|$)'`

## Phase 1: Implementation

Status: completed
Dependencies: none

Objective: Expose and test repeated observer signal idempotency at the Rust

Changes:
- `crates/runx-contracts/src/post_merge_observer.rs` (partial, exclusive) - Include closure key and act forms in the idempotency plan.
- `crates/runx-contracts/tests/post_merge_observer.rs` (partial, exclusive) - Prove repeated signal stability and changed-state separation.
- `.scafld/specs/drafts/runx-post-merge-outcome-observer.md` (partial, shared) - Record the completed contract-level idempotency proof.

Acceptance:
- [x] `ac1` command - focused post-merge observer tests pass.
  - Command: `cargo test --manifest-path crates/Cargo.toml -p runx-contracts --test post_merge_observer -- --nocapture`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-6
- [x] `ac2` command - Rust formatting is unchanged.
  - Command: `cargo fmt --manifest-path crates/Cargo.toml --all --check`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-7
- [x] `ac3` command - scoped diff has no whitespace errors.
  - Command: `git diff --check -- crates/runx-contracts/src/post_merge_observer.rs crates/runx-contracts/tests/post_merge_observer.rs .scafld/specs/drafts/runx-post-merge-outcome-observer.md .scafld/specs/active/runx-post-merge-observer-idempotency-contract.md`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-8
- [x] `ac4` command - implementation scope avoids registry/executor paths.
  - Command: `! printf '%s\n' crates/runx-contracts/src/post_merge_observer.rs crates/runx-contracts/tests/post_merge_observer.rs .scafld/specs/drafts/runx-post-merge-outcome-observer.md | rg '(^|/)(registry|executor)(/|\\.|$)'`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-9

## Rollback

- Remove the idempotency plan fields and the repeated-signal tests, then revert
  the parent draft status note.

## Review

Status: completed
Verdict: pass
Mode: verify
Provider: local
Output: local.fixture
Summary: Human-reviewed override accepted: contract-level idempotency slice verified by focused cargo tests, fmt, scope audit, and diff check

Attack log:
- `review gate`: manual human audit -> clean (contract-level idempotency slice verified by focused cargo tests, fmt, scope audit, and diff check)

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

- 2026-05-20T05:29:31Z: Split from the broad post-merge observer draft to
  close one locally provable repeated-signal idempotency blocker without live
  provider work.
