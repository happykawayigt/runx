---
spec_version: '2.0'
task_id: runx-lifecycle-event-reality-v1
created: '2026-05-27T00:00:00Z'
updated: '2026-05-27T00:00:00Z'
status: draft
harden_status: not_run
size: medium
risk_level: high
---

# runx lifecycle event reality v1

## Current State

Status: draft
Current phase: none
Next: wait_for_safe_window
Reason: high-value cleanup, but runtime event producers may overlap S-tier
Blockers: active S-tier cutover may modify runtime execution and services
Allowed follow-up command: inspect overlap, then `scafld approve runx-lifecycle-event-reality-v1`
Latest runner update: 2026-05-27T00:00:00Z
Review gate: not_started

## Summary

Turn lifecycle vocabulary into real emitted evidence or remove it from
production code. The runtime currently defines forward-looking lifecycle
variants with a broad `dead_code` expectation. That is acceptable for one lift
spec, but not for the steady-state codebase. This spec makes each event either
observable in producers/tests or explicitly parked outside production.

This is not a contract rename and not a receipt schema change.

## Scope

- `crates/runx-runtime/src/lifecycle.rs`
- Runtime producers that already call `record_lifecycle`
- Focused lifecycle tests
- Docs only if the event taxonomy changes

Out of scope:

- S-tier scheduler/engine split.
- Receipt writer hot path.
- MCP persistent sessions.
- TypeScript runtime package deletion.

## Objectives

- Remove broad `#[expect(dead_code)]` over the whole lifecycle enum.
- Emit or park `HarnessOpened`, `DecisionRecorded`, `ChildHarnessLinked`,
  `AdapterInvoked`, `VerificationRecorded`, and `PublicationProjected`.
- Add tests proving emitted events project into host/journal surfaces.
- Keep event names aligned with harness, decision, act, receipt, verification,
  and publication vocabulary.

## Acceptance

- `! rg -n '#\\[expect\\(dead_code' crates/runx-runtime/src/lifecycle.rs`
- `cargo test --manifest-path crates/Cargo.toml -p runx-runtime --features cli-tool,catalog,mcp lifecycle`
- `cargo test --manifest-path crates/Cargo.toml -p runx-runtime --features cli-tool,catalog,mcp --test hello_graph`
- `cargo fmt --manifest-path crates/Cargo.toml --all -- --check`

## Phase 1: Producer Map

Status: pending
Dependencies: none

Changes:

- Map each lifecycle variant to an existing producer, planned producer, or
  delete/park decision.
- Do not edit runtime execution files if the S-tier agent currently owns them.

## Phase 2: Emit Or Park

Status: pending
Dependencies: phase1

Changes:

- Wire safe producers for observable lifecycle events.
- Move not-yet-real events out of production enum rather than keeping a broad
  dead-code expectation.

## Phase 3: Evidence Tests

Status: pending
Dependencies: phase2

Changes:

- Add focused tests for event projection and abnormal terminal evidence.

## Review

Status: not_started
Verdict: none
