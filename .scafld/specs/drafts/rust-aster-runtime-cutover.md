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
Current phase: external-runtime-fixture
Next: cloud-tree binding pass when a checkout with `cloud/**` is available
Reason: added an OSS-local external Aster agent-step replay fixture grounded in
the Aster repo's current Rust bridge scripts and README contract.
Blockers: the cloud tree is not present in this OSS checkout, so hosted/cloud
binding details cannot be verified locally.
Allowed follow-up command: none during this refresh; do not run
`scafld harden rust-aster-runtime-cutover`.
Latest runner update: `fixtures/external/aster/agent-step/rust-bridge-sealed-skill.yaml`
and `crates/runx-runtime/tests/external/aster_agent_step.rs` added.
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
- A runtime external fixture now exists for the local Aster Rust bridge shape:
  `fixtures/external/aster/agent-step/rust-bridge-sealed-skill.yaml`.
- A focused runtime test now exists at
  `crates/runx-runtime/tests/external/aster_agent_step.rs`, wired through
  `crates/runx-runtime/tests/external.rs`.
- The local checkout has no `cloud/` directory and no
  `crates/runx-runtime/src/cloud_client.rs`.
- The readable Aster checkout at `/Users/kam/dev/runx/aster` currently routes
  Rust execution through `scripts/aster-core.mjs` into
  `scripts/runx-agent-bridge.mjs`; the accepted terminal skill report is
  `runx.skill_run.v1` with `status: "sealed"` and a stored
  `runx.harness_receipt.v1` receipt id.
- `cargo test --manifest-path crates/Cargo.toml -p runx-runtime --test external
  aster_agent_step` passes for the new fixture replay.

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
- `cloud/**`

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
- No legacy/compat outcome, effect, verification proof alias, or Aster-only terminal
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
- External Aster runtime fixture definition.
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

- [x] Existing Aster contract fixture coverage remains green for
  `fixtures/contracts/aster-control/public-feed-proof.json`.
- [x] The runtime external fixture
  `fixtures/external/aster/agent-step/**` exists before any Aster runtime
  replay test is claimed.
- [x] The replay test
  `crates/runx-runtime/tests/external/aster_agent_step.rs` is added only after
  the external fixture exists.
- [x] The OSS-hosted boundary is documented against
  `crates/runx-runtime/src/hosted_http.rs` or a reviewed replacement.
- [x] Cloud binding details are marked deferred until `cloud/**` is available
  locally; no acceptance depends on absent cloud paths.
- [x] Aster contract and runtime artifacts use harness receipt closure and
  `proof.verification`, not retired peer terminal artifacts or legacy
  outcome/effect packet fields.
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
test -f fixtures/external/aster/agent-step/rust-bridge-sealed-skill.yaml
test -f crates/runx-runtime/tests/external/aster_agent_step.rs
cargo test --manifest-path crates/Cargo.toml -p runx-contracts aster
cargo test --manifest-path crates/Cargo.toml -p runx-runtime --test external aster_agent_step
! rg -n "runx\\.issue_to_pr_outcome\\.v1|issue_to_pr_outcome|verification[_-]report|target[_-]?effect|\"effect\"\\s*:" crates/runx-contracts/src/aster.rs fixtures/contracts/aster-control
! rg -n "runId|receiptId|issue_to_pr_outcome|verification[_-]report|verificationReport|target[_-]?effect|\"effect\"\\s*:|\"outcome\"\\s*:|/Users/kam" fixtures/external/aster/agent-step/rust-bridge-sealed-skill.yaml
git diff --check -- .scafld/specs/drafts/rust-aster-runtime-cutover.md
```

Latest local validation:

```sh
cargo test --manifest-path crates/Cargo.toml -p runx-contracts aster
# passed: aster_control_fixture_roundtrips

cargo test --manifest-path crates/Cargo.toml -p runx-runtime --test external aster_agent_step
# passed: 2 tests

ruby -ryaml -e 'YAML.load_file("fixtures/external/aster/agent-step/rust-bridge-sealed-skill.yaml"); puts "yaml ok"'
# passed: yaml ok

! rg -n "runId|receiptId|issue_to_pr_outcome|verification[_-]report|verificationReport|target[_-]?effect|\"effect\"\\s*:|\"outcome\"\\s*:|/Users/kam" fixtures/external/aster/agent-step/rust-bridge-sealed-skill.yaml
# passed: no matches
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
