---
spec_version: '2.0'
task_id: runx-thread-outbox-provider-front-v1
created: '2026-06-04T06:20:35Z'
updated: '2026-06-04T06:20:35Z'
status: draft
harden_status: needs_revision
size: medium
risk_level: high
---

# runx-thread-outbox-provider-front-v1

## Current State

Status: draft
Current phase: phase1
Next: harden
Reason: reframed to parity-first after inventory: the Rust provider supervisor and
protocol exist, but no SourceKind/graph dispatch route exists and `issue-to-pr`
still mutates providers through the live TS `thread.push_outbox` tool. Direct
cutover is not yet safe.
Blockers: Phase 1 must prove fixture/local Rust-front parity before any graph
dispatch or `issue-to-pr` route changes.
Allowed follow-up command: `scafld harden runx-thread-outbox-provider-front-v1`
Latest runner update: 2026-06-04T22:18:00Z
Review gate: not_started

Roadmap: Wave 4 (later). Migrates the provider-mutation boundary into the kernel.
The live TS path ships today; move it only after Rust-front parity is proven.

## Summary

Migrate the provider-mutation boundary from the live TS `thread.push_outbox`
path into the kernel's Rust thread-outbox-provider front, and land
governed-execution-layer item 15 (the post-merge publisher on that front). A
complete Rust supervisor already exists
(`oss/crates/runx-runtime/src/outbox_provider.rs`:
`ThreadOutboxProviderProcessSupervisor` with `invoke_push`/`invoke_fetch`) plus the
`thread-outbox-provider-protocol-v1` contract + JSON schemas, but it is NOT
dispatched as a SourceKind or graph step (inert, unit-tested only). This wires it
into dispatch in stages so provider mutation seals through the governed Rust
front. The first buildable slice is fixture/local parity around the existing Rust
supervisor; the live `issue-to-pr` TS path stays authoritative until parity and a
non-default dispatch route both hold.

## Objectives

- Prove the Rust `ThreadOutboxProviderProcessSupervisor` can supervise the same
  provider-push/readback frames the TS outbox packaging emits, without mutating
  GitHub or cutting over live `issue-to-pr`.
- Add a non-default graph/source dispatch route for the Rust front, fixture-only
  first, with sealed receipts for push and fetch/readback.
- Route `issue-to-pr` provider push through the Rust front only after parity
  holds; keep `thread.push_outbox` as the live path until that moment.
- Land the item-15 post-merge publisher on the same front after the push route is
  proven.
- Provider tokens delivered via Rust-supervised `CredentialDelivery`.

## Scope

In scope:
- Phase 1: fixture/local Rust-front parity using the existing contract fixtures,
  mock provider, and current TS outbox packaging shape.
- Phase 2: a non-default graph/source dispatch route for `thread-outbox-provider`
  against the fixture provider, sealed and receipt-verified.
- Phase 3: route `issue-to-pr` push through the Rust front after Phase 1/2; then
  add the post-merge publisher on the same front.

Out of scope:
- Removing the TS `thread.push_outbox` path before Rust-front parity is proven.
- New providers beyond the GitHub thread/outbox lane.
- A new catch-all plugin surface. This front is only for provider-side
  publication/readback of thread outbox entries.

## Dependencies

- Built-but-inert Rust supervisor: `crates/runx-runtime/src/outbox_provider.rs`
  (`ThreadOutboxProviderProcessSupervisor::invoke_push` / `invoke_fetch`).
- Contract frames: `crates/runx-contracts/src/thread_outbox_provider.rs`.
- Existing tests: `crates/runx-runtime/tests/thread_outbox_provider.rs`.
- Current live graph path: `skills/issue-to-pr/X.yaml` calls
  `thread.push_outbox` for `push-pull-request` and `push-feed-entry`.
- Current TS provider mutation path:
  `tools/thread/push_outbox/src/index.ts` and
  `tools/thread/github_adapter.mjs`.

## Assumptions

- The protocol + supervisor are the right contract; this is dispatch wiring and
  migration, not a protocol rebuild.
- The live `issue-to-pr` path must not be changed until a fixture-only Rust-front
  parity slice proves the same push/readback semantics.
- `SourceKind` and graph-step routing should not grow unless Phase 1 shows the
  dedicated front is ready to carry provider mutation.

## Touchpoints

- `crates/runx-runtime/src/outbox_provider.rs`
- `crates/runx-runtime/tests/thread_outbox_provider.rs`
- `fixtures/contracts/thread-outbox-provider/*.json`
- `fixtures/runtime/thread-outbox-provider/mock-provider.sh`
- SourceKind/graph-step dispatch (`crates/runx-parser/src/skill/source.rs`,
  `crates/runx-parser/src/skill/types.rs`,
  `crates/runx-runtime/src/execution/skill_run.rs`,
  `crates/runx-runtime/src/execution/runner/steps.rs`)
- `skills/issue-to-pr/X.yaml` (`thread.push_outbox` push steps)
- `tools/thread/push_outbox/src/index.ts`
- `tools/thread/github_adapter.mjs`
- Post-merge publisher

## Risks

- **Skill-safety (highest).** `issue-to-pr` must keep working through the
  migration. Mitigation: Phase 1/2 are additive and fixture-only; do not cut the
  TS path until parity holds; keep the contract surface frozen.
- **Duplicate mutation path.** Running both Rust and TS provider pushes against
  live GitHub could double-post comments or PR updates. Mitigation: live cutover
  must be single-owner; Phase 1/2 use fixture providers only.
- **Credential leakage.** Provider observations must not expose raw token
  material. Mitigation: keep `CredentialDelivery` structured refs and secret-field
  rejection mandatory in all new dispatch tests.

## Acceptance

Profile: strict

Validation:
- Phase 1 proves the Rust supervisor can push/fetch the existing provider fixture
  frames, including idempotency, provider locator, readback summary, credential
  delivery observation, redaction, and secret-field rejection.
- Phase 2 proves the front can be dispatched from a graph/source route without
  changing `issue-to-pr`.
- Phase 3 proves `issue-to-pr` push and post-merge publishing seal through the
  Rust front with no duplicate TS provider mutation and no SourceKind/protocol
  surface removed.

## Phase 1: Rust-front parity, fixture-only

Status: pending
Dependencies: the inert supervisor, the protocol contract

Objective: prove the existing Rust supervisor can carry provider push/readback
frames at parity with the current packaging contract without touching live
`issue-to-pr`.

Changes:
- Keep `skills/issue-to-pr/X.yaml` unchanged.
- Add or tighten Rust fixture parity around
  `ThreadOutboxProviderProcessSupervisor::invoke_push` / `invoke_fetch`.
- Feed the supervisor frames equivalent to the current TS outbox packaging
  contract and assert operation, request id, idempotency, provider locator,
  provider event hash, readback summary, delivery observations, and redaction.

Acceptance:
- [ ] `ac1` command - Rust provider fixture parity holds
  - Command: `cargo nextest run --manifest-path crates/Cargo.toml -p runx-runtime --all-features thread_outbox_provider`
  - Expected kind: `exit_code_zero`
  - Status: pending
- [ ] `ac2` command - live issue-to-pr route is not cut over in Phase 1
  - Command: `rg -n "tool: thread\\.push_outbox" skills/issue-to-pr/X.yaml`
  - Expected kind: `exit_code_zero`
  - Status: pending

## Phase 2: Non-default dispatch route, fixture-only

Status: pending
Dependencies: Phase 1

Objective: dispatch a thread-outbox-provider graph/source route through the Rust
front against a fixture provider, without cutting over `issue-to-pr`.

Changes:
- Add the smallest parser/runtime dispatch surface for
  `thread-outbox-provider`, fixture-only first.
- Seal push/fetch observations as receipts and preserve `CredentialDelivery`
  behavior.

Acceptance:
- [ ] `ac3` command - fixture graph dispatch seals provider push/readback
  - Command: `runx harness examples/thread-outbox-provider/<case>.yaml --json`
  - Expected kind: `exit_code_zero`
  - Status: pending
- [ ] `ac4` command - protocol surface remains frozen
  - Command: `pnpm vitest run packages/contracts/src/schemas/thread-outbox-provider.test.ts packages/contracts/src/index.test.ts && cargo nextest run --manifest-path crates/Cargo.toml -p runx-contracts --all-features thread_outbox_provider`
  - Expected kind: `exit_code_zero`
  - Status: pending

## Phase 3: issue-to-pr cutover + post-merge publisher

Status: pending
Dependencies: Phase 2

Objective: move the live provider push and post-merge publisher onto the Rust
front with no duplicate mutation path.

Changes:
- Route `issue-to-pr` push through the Rust front.
- Remove or disable the TS provider mutation path only after the Rust route is
  authoritative.
- Implement the post-merge publisher on the same front.

Acceptance:
- [ ] `ac5` command - issue-to-pr push seals via Rust front and graph still works
  - Command: `runx harness skills/issue-to-pr/<push-case>.yaml --json`
  - Expected kind: `exit_code_zero`
  - Status: pending
- [ ] `ac6` command - post-merge publish seals
  - Command: `runx harness examples/post-merge-publish/<case>.yaml --json`
  - Expected kind: `exit_code_zero`
  - Status: pending

## Rollback

- Phase 1 is test/spec-only and reverts cleanly.
- Phase 2 is additive fixture dispatch; remove the route/example if it fails.
- Phase 3 must be a single-owner cutover: revert routing to the TS
  `thread.push_outbox` path if the Rust front regresses. No contract churn.

## Review

Status: not_started
Verdict: none

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
