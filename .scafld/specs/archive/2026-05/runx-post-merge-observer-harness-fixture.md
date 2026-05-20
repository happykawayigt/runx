---
spec_version: '2.0'
task_id: runx-post-merge-observer-harness-fixture
created: '2026-05-20T05:02:26Z'
updated: '2026-05-20T05:12:30Z'
status: completed
harden_status: not_run
size: small
risk_level: medium
---

# Post-merge observer harness fixture

## Current State

Status: completed
Current phase: final
Next: done
Reason: task completed
Blockers: none
Allowed follow-up command: `none`
Latest runner update: 2026-05-20T05:12:30Z
Review gate: pass

## Summary

Add a deterministic harness-spine fixture for a Nitrosend issue-to-PR flow after
a human PR merge. The fixture records provider observation, verification,
source-thread publication, and source-issue close policy as contained harness
acts inside one sealed harness receipt.

This is a contract/fixture slice only. It does not add a live provider observer,
scheduled replay, registry behavior, executor behavior, or any automatic merge
path.

## Objectives

- Pin the merged-and-verified post-merge closure shape in harness receipt terms.
- Prove the fixture round-trips through Rust contract types.
- Prove TypeScript contract validation accepts the same fixture.
- Assert the receipt uses contained `observation`, `verification`, `reply`, and
  `revision` acts rather than a peer terminal packet.
- Assert source-thread publication has a Slack thread reference and never
  models a root-channel fallback.

## Scope

In scope:
- `fixtures/contracts/harness-spine/post-merge-observer-merged-verified.json`
- Focused Rust fixture assertions in
  `crates/runx-contracts/tests/harness_spine_fixtures.rs`.
- Focused TypeScript fixture validation in
  `packages/contracts/src/schemas/post-merge-observer-fixture.test.ts`.

Out of scope:
- GitHub webhooks or scheduled observer runtime.
- Slack or GitHub publication adapters.
- Registry, executor, or live replay files.
- Full closed-unmerged or failed-verification matrix.

## Dependencies

- Existing harness-spine contract types.
- Existing TypeScript contract validator for `runx.harness_receipt.v1`.

## Assumptions

- A single merged-and-verified fixture advances Nitrosend dogfood by making the
  post-merge proof closure observable without requiring external systems.
- Additional closure states can be added as separate fixtures after this
  contract shape is reviewed.

## Touchpoints

- `fixtures/contracts/harness-spine/post-merge-observer-merged-verified.json`
- `crates/runx-contracts/tests/harness_spine_fixtures.rs`
- `packages/contracts/src/schemas/post-merge-observer-fixture.test.ts`

## Risks

- The fixture could accidentally imply auto-merge; assertions must keep the
  human merge gate as observation-only.
- The fixture could allow source-thread publication without thread metadata;
  assertions must require a concrete Slack thread reference.
- The fixture could reintroduce retired terminal packet names; tests must reject
  those tokens in the fixture payload.

## Acceptance

Profile: standard

Validation:
- `pnpm vitest run packages/contracts/src/schemas/post-merge-observer-fixture.test.ts`
- `cargo test --manifest-path crates/Cargo.toml -p runx-contracts --test harness_spine_fixtures -- --nocapture`
- `pnpm fixtures:contracts:keys`
- `git diff --check`

2026-05-20 results:
- `pnpm vitest run packages/contracts/src/schemas/post-merge-observer-fixture.test.ts`
  passed: 1 test.
- `cargo test --manifest-path crates/Cargo.toml -p runx-contracts --test harness_spine_fixtures -- --nocapture`
  passed: 5 tests.
- `pnpm fixtures:contracts:keys` passed.
- `cargo fmt --manifest-path crates/Cargo.toml -p runx-contracts -- --check`
  passed.
- `git diff --check` passed.

## Phase 1: Merged Verified Fixture

Status: active
Dependencies: none

Objective: Add the deterministic merged-and-verified observer receipt fixture

Changes:
- Add one harness-spine fixture with provider observation, verification, source-thread reply, and source issue close/label acts.
- Add Rust round-trip and semantic assertions for the post-merge observer fixture.
- Add TypeScript contract validation and semantic assertions for the same fixture.

Acceptance:
- [ ] `ac1` command - TypeScript contract validator accepts the fixture and
  - Command: `pnpm vitest run packages/contracts/src/schemas/post-merge-observer-fixture.test.ts`
  - Expected kind: `exit_code_zero`
  - Status: passed
- [ ] `ac2` command - Rust contract types round-trip the fixture and check the
  - Command: `cargo test --manifest-path crates/Cargo.toml -p runx-contracts --test harness_spine_fixtures -- --nocapture`
  - Expected kind: `exit_code_zero`
  - Status: passed
- [ ] `ac3` command - Contract fixtures remain canonical JSON.
  - Command: `pnpm fixtures:contracts:keys`
  - Expected kind: `exit_code_zero`
  - Status: passed
- [ ] `ac4` command - No whitespace errors are introduced.
  - Command: `git diff --check`
  - Expected kind: `exit_code_zero`
  - Status: passed

## Rollback

- Remove the new fixture and its two focused fixture tests.

## Review

Status: completed
Verdict: pass
Mode: verify
Summary: Human-reviewed override accepted: Focused post-merge harness fixture reviewed against scope; TS contract validation, Rust harness-spine fixture roundtrip/semantic assertions, fixture key ordering, and diff checks passed. Live observer runtime remains out of scope.

Attack log:
- `review gate`: manual human audit -> clean (Focused post-merge harness fixture reviewed against scope; TS contract validation, Rust harness-spine fixture roundtrip/semantic assertions, fixture key ordering, and diff checks passed. Live observer runtime remains out of scope.)

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

- 2026-05-20T05:08:00Z: Narrowed the large post-merge observer blocker to a
  deterministic harness receipt fixture and focused Rust/TS contract checks.
- 2026-05-20T05:10:54Z: Added the merged-verified Nitrosend observer receipt
  fixture, Rust round-trip/semantic assertions, and TypeScript validation.
- 2026-05-20T05:13:00Z: Narrowed review scope language to exact fixture and
  test files so unrelated dirty contract work stays ambient.
