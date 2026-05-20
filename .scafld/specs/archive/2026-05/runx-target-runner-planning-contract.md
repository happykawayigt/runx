---
spec_version: '2.0'
task_id: runx-target-runner-planning-contract
created: '2026-05-20T05:02:34Z'
updated: '2026-05-20T05:10:27Z'
status: completed
harden_status: not_run
size: small
risk_level: medium
---

# Runx target runner planning contract

## Current State

Status: completed
Current phase: final
Next: done
Reason: task completed
Blockers: none
Allowed follow-up command: `none`
Latest runner update: 2026-05-20T05:10:27Z
Review gate: pass

## Summary

Add a Rust contract-level target repo runner planning packet. The packet is
derived from operational policy admission and records the selected source,
target repo, runner, owner route, source-thread publication route, and dedupe
key material. It does not execute a runner, inspect registries, check out a
repo, query GitHub, or create/update a PR.

## Objectives

- Define a deterministic Rust `TargetRepoRunnerPlan` contract.
- Build the plan from an `OperationalPolicy` and request-time source/target
  metadata.
- Scope dedupe keys by target repo so the same source signal cannot accidentally
  reuse a PR from another target repo.
- Preserve source-thread locator and source issue URL in the plan for later
  receipt/outbox carry-through.
- Fail closed before planning when operational policy admission denies the
  source, target, runner, or source-thread route.

## Scope

In scope:
- `runx-contracts` Rust types and pure planning helper.
- Focused Rust tests using the existing Nitrosend-like operational policy
  fixture.

Out of scope:
- Registry files and registry behavior.
- Executor files and runner execution.
- Target checkout, scafld readiness probing, provider dedupe lookup, branch
  creation, PR creation, source-thread publication, or Aster scheduling.
- TypeScript runtime/outbox changes.

## Dependencies

- Existing `runx.operational_policy.v1` Rust contract and admission helper.
- `.scafld/specs/drafts/runx-target-repo-runners.md` remains the parent
  blocker for execution/provider work.

## Assumptions

- Planning can produce a stable key and source-thread contract before provider
  lookup exists.
- Provider lookup can later set the dedupe result to reused by passing an
  existing pull request into the planner, without changing the key contract.

## Touchpoints

- `crates/runx-contracts/src/target_runner.rs` (new, exclusive)
- `crates/runx-contracts/src/lib.rs` (partial, export only)
- `crates/runx-contracts/tests/target_runner.rs` (new, exclusive)

## Risks

- Low: contract naming may need TS mirror later. Mitigated by keeping this Rust
  packet additive and isolated.
- Medium: dedupe key material can under-scope cross-repo work. Mitigated by
  always adding `target_repo` to the key material even when policy key_fields
  omit it.

## Acceptance

Profile: standard

Validation:
- `cargo test --manifest-path crates/Cargo.toml -p runx-contracts --test target_runner`
- `cargo test --manifest-path crates/Cargo.toml -p runx-contracts --test operational_policy`
- `git diff --check -- .scafld/specs/active/runx-target-runner-planning-contract.md crates/runx-contracts/src/lib.rs crates/runx-contracts/src/target_runner.rs crates/runx-contracts/tests/target_runner.rs`
- `! printf '%s\n' crates/runx-contracts/src/lib.rs crates/runx-contracts/src/target_runner.rs crates/runx-contracts/tests/target_runner.rs | rg '(^|/)(registry|executor)(/|\\.|$)'`

## Phase 1: Implementation

Status: completed
Dependencies: none

Objective: Complete the requested change.

Changes:
- `crates/runx-contracts/src/target_runner.rs` - Add request, plan, dedupe, source-thread, and existing-PR structs plus `plan_target_repo_runner`.
- `crates/runx-contracts/src/lib.rs` - Export the new contract module.
- `crates/runx-contracts/tests/target_runner.rs` - Pin Nitrosend target runner planning, target-repo-scoped dedupe keys, existing-PR reuse metadata, and fail-closed unknown-target behavior.

Acceptance:
- [x] `ac1` test - focused target runner planning tests
  - Command: `cargo test --manifest-path crates/Cargo.toml -p runx-contracts --test target_runner`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-6
- [x] `ac2` test - adjacent operational policy tests still pass
  - Command: `cargo test --manifest-path crates/Cargo.toml -p runx-contracts --test operational_policy`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-7
- [x] `ac3` command - scoped diff has no whitespace errors
  - Command: `git diff --check -- .scafld/specs/active/runx-target-runner-planning-contract.md crates/runx-contracts/src/lib.rs crates/runx-contracts/src/target_runner.rs crates/runx-contracts/tests/target_runner.rs`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-8
- [x] `ac4` command - implementation diff avoids registry/executor paths
  - Command: `! printf '%s\n' crates/runx-contracts/src/lib.rs crates/runx-contracts/src/target_runner.rs crates/runx-contracts/tests/target_runner.rs | rg '(^|/)(registry|executor)(/|\\.|$)'`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-9

## Rollback

- Revert `crates/runx-contracts/src/target_runner.rs`, the `src/lib.rs`
  export, and `crates/runx-contracts/tests/target_runner.rs`.

## Review

Status: completed
Verdict: pass
Mode: discover
Provider: claude:claude-opus-4-7
Output: claude.mcp_submit_review
Summary: The runx-target-runner-planning-contract task adds a self-contained Rust contract module (`target_runner.rs`), an additive `lib.rs` re-export, and a focused integration test suite over the existing Nitrosend operational policy fixture. The plan is built strictly from `admit_operational_policy_request` output and request-time inputs, fails closed when admission denies (e.g. unknown target), enforces target-repo scoping in the dedupe key, preserves the source thread locator and issue URL for downstream receipt carry-through, and marks dedupe reuse when an existing PR is supplied without mutating the key material. No registry/executor/runtime surfaces were touched (ac4 enforced this); cargo tests for both `target_runner` and `operational_policy` pass. No completion blockers found.

Attack log:
- `task scope discipline`: Compare task_changes vs declared touchpoints; confirm registry/executor paths absent and ac4 grep guard satisfied -> clean (Only crates/runx-contracts/src/{lib.rs,target_runner.rs} and tests/target_runner.rs changed within task scope; lib.rs delta is module decl + pub use block as declared.)
- `dedupe key target-repo scoping`: Hand-trace target_scoped_key_fields() and dedupe_key() for nitrosend/api vs nitrosend/app with otherwise identical source material; check whether key truly differs and whether components dedupe loses information -> clean (target_repo is appended exactly once when neither 'target_repo' nor 'target.repo' is configured; sha256 over field/value pairs with \0 and \n separators yields distinct keys per target. Test dedupe_key_is_scoped_by_target_repo asserts this.)
- `fail-closed planning for unknown target / runner / source / source-thread`: Trace plan_target_repo_runner with target_repo not in policy; verify it short-circuits via AdmissionDenied before any plan field is populated -> clean (Admission returns Deny with finding code 'unknown_target_repo'; plan_target_repo_runner immediately returns TargetRepoRunnerPlanError::AdmissionDenied, never reaching the policy.targets.find() lookup. Test unknown_target_denies_before_plan_materializes covers it.)
- `existing PR reuse semantics`: Verify supplying TargetRepoRunnerExistingPullRequest flips dedupe.result to Reused without perturbing key_fields/components and that the structured PR metadata is carried into dedupe.existing_pull_request -> clean (build_dedupe_plan clones existing_pull_request only into the result; key material is derived solely from key_fields+components which do not include PR data. Test existing_pull_request_marks_dedupe_reuse_without_changing_key validates.)
- `serialization safety / secret hygiene`: Inspect Serialize impls and test assertion that JSON of the full plan does not leak local filesystem paths or unexpected secret material -> clean (Plan structs are pure data derived from policy + request; no env, no FS, no host paths. Test asserts JSON contains 'target_repo' and excludes '/Users/' and '/tmp/'.)
- `domain boundaries & dependency surface`: Confirm runx-contracts/Cargo.toml only adds workspace sha2 (already declared) and that target_runner.rs imports stay within the crate (no runtime/cli/cloud reach-through) -> clean (Cargo.toml unchanged for this task; sha2 is preexisting workspace dep. target_runner.rs imports only from crate::operational_policy types and std/serde/sha2.)
- `public API additivity`: Diff lib.rs re-exports; verify only additive exports for new target_runner symbols and no rename/removal of existing exports -> clean (Only `pub mod target_runner;` and a new `pub use target_runner::{...};` block are added; existing pub-use blocks for act, authority, operational_policy, etc. are untouched.)

Findings:
- none

## Self Eval

- 2026-05-20: Split from `runx-target-repo-runners` after inspecting the draft
  blocker. Chosen as the smallest Rust rewrite slice that advances Nitrosend
  dogfood by making target runner planning, dedupe, and source-thread metadata
  deterministic before execution work.

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
