---
spec_version: '2.0'
task_id: runx-post-merge-observer-closure-planner
created: '2026-05-20T05:15:30Z'
updated: '2026-05-20T05:23:52Z'
status: completed
harden_status: not_run
size: small
risk_level: medium
---

# Runx post-merge observer closure planner

## Current State

Status: completed
Current phase: final
Next: done
Reason: task completed
Blockers: none
Allowed follow-up command: `none`
Latest runner update: 2026-05-20T05:23:52Z
Review gate: pass

## Summary

Add a deterministic Rust contract planner for post-merge observer closure
decisions. The planner classifies observed PR closure plus verification state
into stable closure reasons, contained act forms, seal criteria, idempotency
material, publication requirements, and source-issue disposition. This advances
the closed-unmerged and failed-verification states without adding live provider
observation, registry work, executor work, or publication side effects.

## Objectives

- Pin `closed_unmerged` as distinct from `merged_verified` and avoid claiming
  a shipped fix.
- Pin `failed_verification` as a merged-but-not-verified closure that publishes
  a final update and leaves the source issue open under `when_verified` policy.
- Fail closed before planning final publication when policy requires a source
  thread and request metadata does not provide one.
- Produce deterministic observer intent and trigger keys scoped to source issue,
  PR, provider state, and verification state.
- Keep the planner pure and serializable for later runtime receipt construction.

## Scope

In scope:
- New Rust contract module for pure post-merge observer closure planning.
- Additive export from `runx-contracts`.
- Focused Rust tests over closed-unmerged, failed-verification, source-thread
  fail-closed, and verified closure policy behavior.

Out of scope:
- GitHub webhook handling, provider polling, scheduled replay, or API calls.
- Slack/GitHub publication adapters.
- Registry files and behavior.
- Executor files and behavior.
- Full harness receipt construction beyond the already completed merged-verified
  fixture.

## Dependencies

- `.scafld/specs/drafts/runx-post-merge-outcome-observer.md`
- `.scafld/specs/archive/2026-05/runx-post-merge-observer-harness-fixture.md`
- Existing `runx.operational_policy.v1` Rust contract.

## Assumptions

- Planning can be locked before live observer runtime exists.
- `close_source_issue=when_verified` is the safe default for failed
  verification: publish final status, keep the source issue open.
- `closed_unmerged` represents terminal PR state, not terminal source issue
  success.

## Touchpoints

- `.scafld/specs/active/runx-post-merge-observer-closure-planner.md`
- `crates/runx-contracts/src/lib.rs`
- `crates/runx-contracts/src/post_merge_observer.rs`
- `crates/runx-contracts/tests/post_merge_observer.rs`

## Risks

- Medium: closure naming becomes a downstream contract. Mitigated by using the
  names already required by the parent observer draft.
- Medium: touching `src/lib.rs` overlaps ambient contract exports. Mitigated by
  making only additive module/export changes and preserving existing edits.
- Low: planner might imply auto-merge. Mitigated by explicit act forms and
  tests that keep observation separate from mutation.

## Acceptance

Profile: standard

Validation:
- `cargo test --manifest-path crates/Cargo.toml -p runx-contracts --test post_merge_observer`
- `cargo test --manifest-path crates/Cargo.toml -p runx-contracts --test operational_policy`
- `git diff --check -- .scafld/specs/active/runx-post-merge-observer-closure-planner.md crates/runx-contracts/src/lib.rs crates/runx-contracts/src/post_merge_observer.rs crates/runx-contracts/tests/post_merge_observer.rs`
- `! printf '%s\n' crates/runx-contracts/src/lib.rs crates/runx-contracts/src/post_merge_observer.rs crates/runx-contracts/tests/post_merge_observer.rs | rg '(^|/)(registry|executor)(/|\\.|$)'`

## Phase 1: Implementation

Status: completed
Dependencies: none

Objective: Complete the requested change.

Changes:
- `crates/runx-contracts/src/post_merge_observer.rs` - Add request, observation, verification, publication, source issue, criterion, and idempotency plan structs plus `plan_post_merge_observer_closure`.
- `crates/runx-contracts/src/lib.rs` - Export the new contract module and planner symbols.
- `crates/runx-contracts/tests/post_merge_observer.rs` - Pin closed-unmerged and failed-verification planner behavior plus source-thread fail-closed.

Acceptance:
- [x] `ac1` test - focused post-merge observer planner tests
  - Command: `cargo test --manifest-path crates/Cargo.toml -p runx-contracts --test post_merge_observer`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-6
- [x] `ac2` test - adjacent operational policy behavior still passes
  - Command: `cargo test --manifest-path crates/Cargo.toml -p runx-contracts --test operational_policy`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-7
- [x] `ac3` command - scoped diff has no whitespace errors
  - Command: `git diff --check -- .scafld/specs/active/runx-post-merge-observer-closure-planner.md crates/runx-contracts/src/lib.rs crates/runx-contracts/src/post_merge_observer.rs crates/runx-contracts/tests/post_merge_observer.rs`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-8
- [x] `ac4` command - implementation diff avoids registry/executor paths
  - Command: `! printf '%s\n' crates/runx-contracts/src/lib.rs crates/runx-contracts/src/post_merge_observer.rs crates/runx-contracts/tests/post_merge_observer.rs | rg '(^|/)(registry|executor)(/|\\.|$)'`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-9

## Rollback

- Remove `crates/runx-contracts/src/post_merge_observer.rs`, the additive
  `src/lib.rs` exports, and `crates/runx-contracts/tests/post_merge_observer.rs`.

## Review

Status: completed
Verdict: pass
Mode: verify
Summary: Human-reviewed override accepted: Focused post-merge closure planner reviewed against scope; post_merge_observer and operational_policy Rust tests, combined contract tests, fmt, and diff checks passed after tightening closed-unmerged wording assertion. Live observer runtime remains out of scope.

Attack log:
- `review gate`: manual human audit -> clean (Focused post-merge closure planner reviewed against scope; post_merge_observer and operational_policy Rust tests, combined contract tests, fmt, and diff checks passed after tightening closed-unmerged wording assertion. Live observer runtime remains out of scope.)

Findings:
- none

## Self Eval

- Target score: 9.0. Passing means the next observer runtime can consume a
  deterministic contract for non-happy-path closure states without inventing
  closure names or source issue behavior.

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

- 2026-05-20T05:16:00Z: Split from the larger observer draft after confirming
  the completed harness fixture only covers merged-verified.
