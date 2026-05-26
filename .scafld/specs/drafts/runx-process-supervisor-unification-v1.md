---
spec_version: '2.0'
task_id: runx-process-supervisor-unification-v1
created: '2026-05-27T00:00:00Z'
updated: '2026-05-27T00:00:00Z'
status: draft
harden_status: not_run
size: medium
risk_level: high
---

# runx process supervisor unification v1

## Current State

Status: draft
Current phase: none
Next: wait_for_safe_window
Reason: useful follow-up, but must not collide with active S-tier MCP/session work
Blockers: active S-tier cutover may touch MCP transport/session ownership
Allowed follow-up command: inspect overlap, then `scafld approve runx-process-supervisor-unification-v1`
Latest runner update: 2026-05-27T00:00:00Z
Review gate: not_started

## Summary

Unify process-group termination semantics so async MCP supervision does not
shell out to `/bin/kill` while the sync process supervisor uses Rust/rustix.
Process cleanup is part of the trust boundary. A child process group should be
terminated through one internal mechanism with the same TERM, grace, KILL, and
fallback behavior.

This spec is not the S-tier persistent-session work. It must not introduce MCP
session pooling, external-adapter pooling, or new protocol reset behavior.

## Scope

- `crates/runx-runtime/src/process/**`
- `crates/runx-runtime/src/adapters/mcp/transport.rs`
- Focused process-supervision tests under `crates/runx-runtime/tests/**`

Out of scope:

- MCP server contract/type work currently dirty in another agent's lane.
- Persistent MCP sessions and spawn-count perf gates owned by S-tier.
- External adapter protocol reset/session pooling.

## Objectives

- Remove `/bin/kill` shell-out from MCP async process termination on Unix.
- Share signal vocabulary and process-group semantics between sync and async
  supervisors.
- Preserve non-Unix direct-child fallback.
- Add timeout/cleanup tests that fail if descendants outlive termination.

## Acceptance

- `! rg -n 'Command::new\\("/bin/kill"\\)' crates/runx-runtime/src --glob '*.rs'`
- `cargo test --manifest-path crates/Cargo.toml -p runx-runtime --features cli-tool,catalog,mcp process`
- `cargo test --manifest-path crates/Cargo.toml -p runx-runtime --features cli-tool,catalog,mcp --test mcp_server`
- `cargo fmt --manifest-path crates/Cargo.toml --all -- --check`

## Phase 1: Overlap Check

Status: pending
Dependencies: none

Changes:

- Confirm no current dirty diff owns `adapters/mcp/transport.rs` or shared
  process supervisor files.
- If dirty overlap exists, keep this spec draft and do not execute.

## Phase 2: Shared Termination Mechanism

Status: pending
Dependencies: phase1

Changes:

- Extract a shared Rust signal helper usable by sync and async supervisors.
- Replace MCP `/bin/kill` shell-out with the shared helper.
- Keep non-Unix direct-child semantics unchanged.

## Phase 3: Tests And Guards

Status: pending
Dependencies: phase2

Changes:

- Add focused tests/guards for timeout, TERM/KILL fallback, and no shell-out.

## Review

Status: not_started
Verdict: none
