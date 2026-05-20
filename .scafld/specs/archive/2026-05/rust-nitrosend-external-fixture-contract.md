---
spec_version: '2.0'
task_id: rust-nitrosend-external-fixture-contract
created: '2026-05-20T05:18:55Z'
updated: '2026-05-20T05:22:30Z'
status: completed
harden_status: not_run
size: medium
risk_level: medium
---

# Rust Nitrosend external fixture contract

## Current State

Status: completed
Current phase: final
Next: done
Reason: task completed
Blockers: none
Allowed follow-up command: `none`
Latest runner update: 2026-05-20T05:22:30Z
Review gate: pass

## Summary

Add the first sanitized external-shaped Nitrosend fixture that ties together
the generic runtime skill fixtures, the Nitrosend-like operational policy, the
target-runner planning/lookup contract, and the post-merge harness receipt
fixture.

This is still not live replay and not provider mutation. It is the deterministic
fixture contract that the later Rust runtime replay can consume.

## Objectives

- Create `fixtures/external/nitrosend/issue-intake/api-source-thread.json`.
- Prove the fixture derives an admitted `nitrosend/api` target runner plan.
- Prove the fixture derives a provider dedupe lookup with source issue/thread
  references.
- Prove the fixture cites the merged-and-verified post-merge harness receipt.
- Update `rust-nitrosend-dogfood` to reflect the new fixture contract.

## Scope

In scope:
- `fixtures/external/nitrosend/issue-intake/api-source-thread.json`
- `crates/runx-contracts/tests/nitrosend_external_fixture.rs`
- `.scafld/specs/drafts/rust-nitrosend-dogfood.md`

Out of scope:
- Live Nitrosend traffic capture.
- Runtime replay.
- Provider API calls or mutations.
- Registry or executor files.

## Dependencies

- `runx-target-runner-planning-contract`
- `runx-target-runner-dedupe-lookup-contract`
- `runx-post-merge-observer-harness-fixture`
- `runx.operational_policy.v1`

## Assumptions

- The first fixture targets `nitrosend/api` because that was the concrete
  multi-repo gap the dogfood spec called out.

## Touchpoints

- External fixture directory.
- Rust contracts fixture tests.
- Nitrosend dogfood draft.

## Risks

- Risk: fixture accidentally becomes live capture. Mitigation: all locators are
  sanitized deterministic fixture values and test asserts no local paths.
- Risk: fixture drifts from the policy/target-runner contracts. Mitigation:
  Rust test derives the plan and lookup from the fixture inputs.

## Acceptance

Profile: standard

Validation:
- `scafld validate rust-nitrosend-external-fixture-contract --json`
- `cargo test --manifest-path crates/Cargo.toml -p runx-contracts --test nitrosend_external_fixture -- --nocapture`
- `cargo test --manifest-path crates/Cargo.toml -p runx-contracts --test target_runner -- --nocapture`
- `cargo test --manifest-path crates/Cargo.toml -p runx-contracts --test harness_spine_fixtures -- --nocapture`
- `git diff --check -- fixtures/external/nitrosend/issue-intake/api-source-thread.json crates/runx-contracts/tests/nitrosend_external_fixture.rs .scafld/specs/drafts/rust-nitrosend-external-fixture-contract.md .scafld/specs/drafts/rust-nitrosend-dogfood.md`

## Phase 1: Implementation

Status: completed
Dependencies: none

Objective: Add and validate the first external-shaped Nitrosend fixture.

Changes:
- Added the sanitized external-shaped fixture.
- Added Rust fixture derivation tests across policy, target runner, dedupe
  lookup, and post-merge harness receipt.
- Updated the dogfood draft with the new local fact.

Acceptance:
- [ ] `ac1` command - Primary validation command
  - Command: see Validation
  - Expected kind: `exit_code_zero`
  - Status: pending

## Rollback

- Remove the fixture and its focused test. Leave `rust-nitrosend-dogfood`
  blocked on missing external fixture again.

## Review

Status: completed
Verdict: pass
Mode: verify
Summary: Human-reviewed override accepted: Focused Nitrosend external fixture contract reviewed against scope; nitrosend fixture derivation, target runner, harness-spine, and post-merge observer Rust tests passed with fmt and diff checks. Live runtime replay remains out of scope.

Attack log:
- `review gate`: manual human audit -> clean (Focused Nitrosend external fixture contract reviewed against scope; nitrosend fixture derivation, target runner, harness-spine, and post-merge observer Rust tests passed with fmt and diff checks. Live runtime replay remains out of scope.)

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
