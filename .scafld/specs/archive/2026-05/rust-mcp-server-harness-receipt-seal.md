---
spec_version: '2.0'
task_id: rust-mcp-server-harness-receipt-seal
created: '2026-05-20T05:28:35Z'
updated: '2026-05-20T05:42:31Z'
status: completed
harden_status: not_run
size: small
risk_level: medium
---

# Rust MCP server harness receipt seal

## Current State

Status: completed
Current phase: final
Next: done
Reason: task completed
Blockers: none
Allowed follow-up command: `none`
Latest runner update: 2026-05-20T05:42:31Z
Review gate: pass

## Summary

Make `runx mcp serve` single-skill execution prove the same sealed
`runx.harness_receipt.v1` contract required by the completed Rust MCP adapter
spec. The archived MCP adapter review accepted the port but identified that the
single-skill server path appears to invoke an adapter directly and write a step
receipt, while the graph path already routes through runtime execution. This
slice closes that runtime-local sunset blocker with a focused implementation
and test, without reopening the broader MCP adapter port.

## Objectives

- Route MCP server single-skill execution through the Rust runtime/harness path
  that seals a `runx.harness_receipt.v1` node, or add an equivalent narrow
  harness sealing helper at the MCP server boundary.
- Preserve existing MCP JSON-RPC response shapes for completed, failed,
  paused, denied, and escalated tool results.
- Add a focused regression test that invokes the MCP server single-skill path
  and asserts a sealed harness receipt is written.
- Keep MCP client behavior, graph execution behavior, TypeScript runtime-local,
  and oracle generation out of scope unless the focused test needs fixture
  plumbing.

## Scope

In scope:
- `crates/runx-runtime/src/adapters/mcp.rs` single-skill server execution path.
- Existing Rust MCP server tests under `crates/runx-runtime/tests/`.
- Receipt-store assertions needed to prove a sealed `runx.harness_receipt.v1`
  node exists for the served skill call.

Out of scope:
- Reworking MCP client transport, sandbox, timeout, size-limit, or JSON-RPC
  framing behavior.
- Adding `rmcp` or revisiting the completed MCP library decision.
- TypeScript runtime-local or adapter edits.
- CLI release cutover, runtime-local deletion, or broad MCP oracle refresh.
- Changing public MCP tool response JSON beyond adding the required sealed
  receipt evidence behind the existing runtime contract.

## Dependencies

- `rust-runtime-adapters-mcp` is archived completed; this spec consumes its
  review finding and must not reopen the full adapter port.
- `rust-runtime-skill-execution`, `rust-harness`, `rust-ts-sunset-receipts`,
  and `rust-receipt-proof-verification` are archived completed and define the
  canonical sealed harness receipt contract.
- `rust-ts-sunset-runtime-local` remains blocked until this proof gap and the
  remaining importer/package-routing work are complete.

## Assumptions

- The graph MCP server path already has runtime execution coverage; this slice
  targets the single-skill path called out in the archived review.
- The correct proof is a sealed `runx.harness_receipt.v1` node in the local
  receipt store, not a legacy step receipt with a similar payload.
- Existing JSON-RPC transcript expectations should remain byte-compatible after
  dynamic receipt ids and paths are normalized by existing helpers.

## Touchpoints

- `crates/runx-runtime/src/adapters/mcp.rs`
- `crates/runx-runtime/tests/mcp_server.rs`
- Existing runtime receipt or harness helpers only if needed to avoid
  duplicating sealing logic.

## Risks

- Medium: routing through the harness path may change MCP response timing or
  result envelopes. Mitigation: keep existing MCP server tests green and add
  only receipt-store assertions.
- Medium: duplicating receipt sealing at the MCP boundary could drift from the
  canonical harness implementation. Mitigation: prefer existing harness/runtime
  helpers; add a helper only if it lives beside the canonical sealing code.
- Low: receipt-store tests can become path-sensitive. Mitigation: use temporary
  receipt roots and assert schema/seal properties rather than absolute paths.

## Acceptance

Profile: standard

Validation:
- `cargo test --manifest-path crates/Cargo.toml -p runx-runtime mcp_server --features mcp -- --nocapture`
- `cargo test --manifest-path crates/Cargo.toml -p runx-runtime harness -- --nocapture`
- `cargo test --manifest-path crates/Cargo.toml -p runx-receipts proof -- --nocapture`
- `cargo clippy --manifest-path crates/Cargo.toml -p runx-runtime --all-targets --features mcp -- -D warnings`
- `git diff --check -- crates/runx-runtime/src/adapters/mcp.rs crates/runx-runtime/tests/mcp_server.rs .scafld/specs/drafts/rust-mcp-server-harness-receipt-seal.md`

## Phase 1: Implementation

Status: completed
Dependencies: none

Objective: Prove MCP server single-skill calls emit sealed harness receipts.

Changes:
- `crates/runx-runtime/src/adapters/mcp.rs` (partial, exclusive) - Route or seal the single-skill MCP server execution path through canonical harness receipt behavior.
- `crates/runx-runtime/tests/mcp_server.rs` (partial, exclusive) - Assert a single-skill MCP tool call writes a sealed `runx.harness_receipt.v1` receipt.

Acceptance:
- [x] `ac1` command - MCP server tests prove the single-skill receipt seal.
  - Command: `cargo test --manifest-path crates/Cargo.toml -p runx-runtime mcp_server --features mcp -- --nocapture`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-13
- [x] `ac2` command - existing harness tests remain green.
  - Command: `cargo test --manifest-path crates/Cargo.toml -p runx-runtime harness -- --nocapture`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-14
- [x] `ac3` command - receipt proof checks remain green.
  - Command: `cargo test --manifest-path crates/Cargo.toml -p runx-receipts proof -- --nocapture`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-15
- [x] `ac4` command - MCP runtime clippy stays clean.
  - Command: `cargo clippy --manifest-path crates/Cargo.toml -p runx-runtime --all-targets --features mcp -- -D warnings`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-16

## Rollback

- Revert only the MCP server execution/test changes from this slice. Leave the
  archived `rust-runtime-adapters-mcp` spec completed and keep
  `rust-ts-sunset-runtime-local` blocked on this proof gap.

## Review

Status: completed
Verdict: pass
Mode: verify
Summary: Human-reviewed override accepted: MCP server receipt proof verified; mcp_server, harness, receipt proof, clippy, and diff checks now pass

Attack log:
- `review gate`: manual human audit -> clean (MCP server receipt proof verified; mcp_server, harness, receipt proof, clippy, and diff checks now pass)

Findings:
- none

## Self Eval

- none

## Deviations

- none

## Metadata

- created_by: codex
- source_finding: `.scafld/specs/archive/2026-05/rust-runtime-adapters-mcp.md`

## Planning Log

- 2026-05-20T05:28:35Z: Created from the archived MCP adapter review finding
  `mcp-server-skill-may-skip-harness-receipt-seal` as the smallest executable
  gap before runtime-local deletion can treat MCP server receipt proof as
  closed.
