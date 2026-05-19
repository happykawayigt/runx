---
spec_version: '2.0'
task_id: rust-aster-runtime-cutover
created: '2026-05-18T00:00:00Z'
updated: '2026-05-20T00:00:00Z'
status: draft
harden_status: not_run
size: large
risk_level: high
---

# Rust aster runtime cutover

## Current State

Status: draft
Current phase: plan-refresh
Next: approve
Reason: refreshed against the current local OSS checkout. This is a plan spec,
not an implementation pass.
Blockers: the cloud tree is not present in this OSS checkout, so hosted/cloud
binding details cannot be verified locally. Runtime external replay fixtures
for Aster are also absent.
Allowed follow-up command: none during this refresh; do not run
`scafld harden rust-aster-runtime-cutover`.
Latest runner update: none
Review gate: not_started

## Summary

Plan the Aster runtime cutover from the local OSS state that is actually
available. This checkout does not include `cloud/**`, so the spec cannot claim
verified cloud package paths, UI paths, hosted agent adapter files, or cloud DB
approval routing. Those bindings stay deferred until the cloud tree is
available to the worker executing that phase.

Current local facts:

- `crates/runx-runtime/src/hosted_http.rs` is the hosted boundary visible in
  this checkout. It defines `HostedHttpClient`, `HostedTransport`, request and
  response types, header validation, curl-backed transport, and redacted debug
  behavior.
- Aster contract types exist in `crates/runx-contracts/src/aster.rs`.
- The contracts crate exports Aster control objects from
  `crates/runx-contracts/src/lib.rs`.
- A structural Aster control fixture exists at
  `fixtures/contracts/aster-control/public-feed-proof.json`.
- No runtime external fixture exists for Aster:
  `fixtures/external/aster/**` and
  `crates/runx-runtime/tests/external/aster_agent_step.rs` are absent.
- The local checkout has no `cloud/` directory and no
  `crates/runx-runtime/src/cloud_client.rs`.

The cutover remains preservation-oriented: Aster should consume the Rust
runtime through a documented boundary and canonical contracts, but this draft
must not invent a cloud binding or claim an agent-step runtime fixture before
those files exist.

## Context

CWD: `.` (runx OSS workspace)

Relevant existing local surfaces:

- `crates/runx-runtime/src/hosted_http.rs`
- `crates/runx-contracts/src/aster.rs`
- `crates/runx-contracts/src/lib.rs`
- `fixtures/contracts/aster-control/public-feed-proof.json`
- `crates/runx-contracts/tests/aster_control_fixtures.rs`
- `fixtures/operational-policy/nitrosend-like.json` as the current
  operational-policy readback proof point, not as an Aster runtime fixture.
- `.scafld/specs/drafts/runx-target-repo-runners.md`
- `.scafld/specs/drafts/runx-post-merge-outcome-observer.md`

Surfaces not present in this checkout:

- `cloud/packages/**`
- `cloud/packages/agent-runner/**`
- `cloud/packages/api/**`
- `cloud/packages/db/**`
- `cloud/packages/receipts-store/**`
- `cloud/packages/ui/**`
- `crates/runx-runtime/src/cloud_client.rs`
- `fixtures/external/aster/agent-step/**`
- `crates/runx-runtime/tests/external/aster_agent_step.rs`

## Invariants

- Cloud binding is deferred until a checkout with the cloud tree is available.
  This spec may name the required boundary, but it must not assert verified
  cloud implementation paths in the OSS-only checkout.
- Aster control objects use the existing `runx-contracts::aster` shapes. Do not
  create parallel Aster JSON shapes for target, opportunity, selection,
  reflection, skill binding, feed entry, or transition records.
- Runtime execution artifacts stay canonical harness, decision, act,
  verification/proof, and sealed `runx.harness_receipt.v1` objects.
- Aster must not read receipts through private local file paths in public or
  hosted projections; receipt access goes through runtime/store APIs or a
  documented hosted boundary.
- `hosted_http.rs` is the current local hosted boundary. Any future cloud
  binding should either use this boundary or explicitly replace it in a
  separate reviewed change.
- No legacy/compat outcome, effect, verification-report, or Aster-only terminal
  packet is introduced.

## Objectives

- Preserve the Aster contract surface already present in
  `crates/runx-contracts/src/aster.rs` and its fixture coverage.
- Define the runtime external fixture that is missing today:
  `fixtures/external/aster/agent-step/**`.
- Add a Rust runtime replay test only after the fixture exists:
  `crates/runx-runtime/tests/external/aster_agent_step.rs`.
- Use `hosted_http.rs` as the locally verified hosted boundary for any OSS-side
  runtime-to-host interaction.
- Defer cloud package binding details until the cloud tree is available.
- Ensure Aster-run issue-to-PR and post-merge paths use
  `runx-target-repo-runners` and `runx-post-merge-outcome-observer` when those
  contracts exist, with final state represented as sealed closure/proof
  receipts.

## Scope

In scope:

- OSS-local plan for Aster contract preservation.
- Missing external runtime fixture definition.
- Hosted boundary notes grounded in `hosted_http.rs`.
- Dependency sequencing for target-runner and post-merge observer flows.

Out of scope:

- Editing or verifying `cloud/**` paths in this checkout.
- Implementing the cloud binding shim.
- Aster UI, feed curation, selector product behavior, or brand work.
- Scafld hardening in this refresh.
- Legacy/compat execution artifact shapes.

## Dependencies

- `runx-contract-spine-hard-cutover`.
- `rust-runtime-skeleton`.
- `rust-runtime-skill-execution`.
- `rust-approval-gate-parity` for any hosted approval gates that Aster consumes.
- `rust-runtime-receipt-path-discovery`,
  `rust-receipt-tree-resolution`, and `rust-receipt-proof-verification`.
- `runx-operational-policy-config` for policy/admin readback.
- `runx-target-repo-runners` for Aster-scheduled source-to-target PR flows.
- `runx-post-merge-outcome-observer` for final closure/proof observation and
  source-thread updates.
- A future cloud-tree binding pass that can inspect the real `cloud/**`
  implementation.

## Acceptance Criteria

- [ ] Existing Aster contract fixture coverage remains green for
  `fixtures/contracts/aster-control/public-feed-proof.json`.
- [ ] The runtime external fixture
  `fixtures/external/aster/agent-step/**` exists before any Aster runtime
  replay test is claimed.
- [ ] The replay test
  `crates/runx-runtime/tests/external/aster_agent_step.rs` is added only after
  the external fixture exists.
- [ ] The OSS-hosted boundary is documented against
  `crates/runx-runtime/src/hosted_http.rs` or a reviewed replacement.
- [ ] Cloud binding details are marked deferred until `cloud/**` is available
  locally; no acceptance depends on absent cloud paths.
- [ ] Aster contract and runtime artifacts do not contain
  `runx.issue_to_pr_outcome.v1`, `issue_to_pr_outcome`, `outcome`, `effect`,
  `verification_report`, `verification-report`, `target_outcome`, or
  `target-effect`.
- [ ] Aster final publication and issue-to-PR completion, once implemented, use
  sealed harness receipt closure/proof through the reusable observer/runner
  specs rather than Aster-only terminal packets.

## Validation Commands

Current local discovery/guard commands:

```sh
test ! -d cloud
test -f crates/runx-runtime/src/hosted_http.rs
test -f crates/runx-contracts/src/aster.rs
test -f fixtures/contracts/aster-control/public-feed-proof.json
test ! -d fixtures/external/aster
cargo test --manifest-path crates/Cargo.toml -p runx-contracts aster
! rg -n "runx\\.issue_to_pr_outcome\\.v1|issue_to_pr_outcome|verification[_-]report|target[_-]?effect|\"effect\"\\s*:" crates/runx-contracts/src/aster.rs fixtures/contracts/aster-control
git diff --check -- .scafld/specs/drafts/rust-aster-runtime-cutover.md
```

Future validation once the external runtime fixture and cloud binding exist:

```sh
cargo test --manifest-path crates/Cargo.toml -p runx-runtime aster_agent_step
```

## Rollback And Repair

- If cloud binding assumptions are wrong, repair the cloud binding spec after
  inspecting a checkout that contains `cloud/**`; do not encode guessed cloud
  paths in this OSS-only spec.
- If the external runtime fixture is missing, keep Aster cutover blocked rather
  than treating the Aster control contract fixture as runtime execution proof.
- If a future binding bypasses `hosted_http.rs`, require an explicit reviewed
  replacement boundary and update this spec.
- If retired artifact fields appear in Aster fixtures or runtime output, repair
  the producer and expected sealed receipts. Do not add compatibility shims.

## Open Questions

- Which concrete cloud binding mode wins once the cloud tree is available:
  hosted HTTP, subprocess JSON over `runx-cli`, or an in-process service/FFI
  bridge.
- Where hosted approval routing lives in the cloud tree after the Aster v1 reset
  work is available for inspection.
- Whether Aster needs a dedicated runtime fixture generator or can share the
  generic hosted fixture machinery once that exists.
