---
spec_version: '2.0'
task_id: runx-target-runner-dedupe-lookup-contract
created: '2026-05-20T05:14:15Z'
updated: '2026-05-20T05:16:51Z'
status: completed
harden_status: not_run
size: medium
risk_level: medium
---

# Target runner dedupe lookup contract

## Current State

Status: completed
Current phase: final
Next: done
Reason: task completed
Blockers: none
Allowed follow-up command: `none`
Latest runner update: 2026-05-20T05:16:51Z
Review gate: pass

## Summary

Add a pure Rust target-repo runner dedupe lookup contract. Planning already
computes a target-scoped dedupe key; this slice turns that plan into a provider
lookup shape that can run before branch/PR mutation and can later be sealed in a
harness receipt.

This is not a provider implementation. It does not call GitHub, create/update
PRs, or publish source-thread replies.

## Objectives

- Build a deterministic provider lookup plan from `TargetRepoRunnerPlan`.
- Include target repo, dedupe key/components, provider search query, source
  issue ref, and source thread ref.
- Keep the plan mutation-free and public-output safe: no local checkout paths
  or secret/environment fields.
- Prove lookup planning preserves existing-PR reuse state without changing the
  dedupe key.
- Update the broad `runx-target-repo-runners` draft with the completed slice.

## Scope

In scope:
- `crates/runx-contracts/src/target_runner.rs`
- `crates/runx-contracts/src/lib.rs`
- `crates/runx-contracts/tests/target_runner.rs`
- `.scafld/specs/drafts/runx-target-repo-runners.md`

Out of scope:
- Provider API calls.
- Branch/PR creation.
- Runtime checkout/scafld readiness probing.
- Source-thread publication.
- Registry and executor files.

## Dependencies

- Completed `runx-target-runner-planning-contract` slice.
- `runx.operational_policy.v1` fixture coverage for Nitrosend-like targets.

## Assumptions

- GitHub is the first provider target for this contract. The shape uses shared
  `Reference` values so other providers can add their own lookup planners later
  without changing the target-runner plan.

## Touchpoints

- Target runner contract module.
- Nitrosend-like target runner tests.
- Broad target-repo runner draft.

## Risks

- Risk: dedupe lookup accidentally becomes provider execution. Mitigation:
  pure data contract only; no network, no filesystem.
- Risk: source thread metadata is dropped before publication. Mitigation:
  lookup plan carries both source issue and source thread references.
- Risk: dedupe key is recomputed differently across lookup and plan. Mitigation:
  lookup consumes the already-built plan key/components.

## Acceptance

Profile: standard

Validation:
- `scafld validate runx-target-runner-dedupe-lookup-contract --json`
- `cargo test --manifest-path crates/Cargo.toml -p runx-contracts --test target_runner -- --nocapture`
- `cargo test --manifest-path crates/Cargo.toml -p runx-contracts --test operational_policy -- --nocapture`
- `cargo fmt --manifest-path crates/Cargo.toml --all --check`
- `git diff --check -- crates/runx-contracts/src/target_runner.rs crates/runx-contracts/src/lib.rs crates/runx-contracts/tests/target_runner.rs .scafld/specs/drafts/runx-target-runner-dedupe-lookup-contract.md .scafld/specs/drafts/runx-target-repo-runners.md`

## Phase 1: Implementation

Status: completed
Dependencies: none

Objective: Convert a target runner plan into a provider dedupe lookup plan.

Changes:
- Added lookup plan/query structs.
- Added deterministic `plan_target_repo_runner_dedupe_lookup`.
- Added tests for Nitrosend source issue/thread refs, no-local-path output, and
  existing-PR reuse state.
- Updated the parent target-repo runner draft with the completed slice.

Acceptance:
- [ ] `ac1` command - Primary validation command
  - Command: see Validation
  - Expected kind: `exit_code_zero`
  - Status: pending

## Rollback

- Revert this contract slice and leave provider dedupe lookup as an unresolved
  blocker in `runx-target-repo-runners`.

## Review

Status: completed
Verdict: pass
Mode: verify
Summary: Human-reviewed override accepted: Focused target-runner dedupe lookup contract reviewed against scope; target_runner and operational_policy Rust tests, cargo fmt check, and diff checks passed. Provider API lookup and PR mutation remain out of scope.

Attack log:
- `review gate`: manual human audit -> clean (Focused target-runner dedupe lookup contract reviewed against scope; target_runner and operational_policy Rust tests, cargo fmt check, and diff checks passed. Provider API lookup and PR mutation remain out of scope.)

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
