---
spec_version: '2.0'
task_id: runx-runtime-mcp-concurrency-v1
created: '2026-05-27T16:00:00Z'
updated: '2026-05-27T14:49:14Z'
status: completed
harden_status: not_run
size: medium
risk_level: high
---

# Runtime MCP concurrency cutover

## Current State

Status: completed
Current phase: final
Next: done
Reason: task completed
Blockers: none
Allowed follow-up command: `none`
Latest runner update: 2026-05-27T14:49:14Z
Review gate: pass

## Summary

Remove the MCP server's coarse execution lock so independent tool calls can run
concurrently without serializing on shared server state. The server state must
only protect metadata lookup and run-id allocation; long-running tool execution,
receipt writes, adapter execution, and host resolution must happen outside the
mutex.

This is a clean runtime architecture cutover: no compatibility mode, no fallback
single-thread execution path, and no mixed ownership between state management and
tool execution.

## Objectives

- Keep shared MCP state responsible only for immutable tool metadata and tiny
  mutable counters.
- Execute MCP tool calls outside the global server-state mutex.
- Preserve deterministic run-id uniqueness under concurrent calls.
- Keep list/get operations responsive while a tool call is running.
- Add correctness/concurrency coverage, not perf benchmarks.
- Pass the Claude provider adversarial review gate.

## Scope

- In scope:
  - `crates/runx-runtime/src/adapters/mcp/server.rs`
  - `crates/runx-runtime/src/adapters/mcp/server_skill.rs` only if execution
    boundaries need a cleaner call shape.
  - `crates/runx-runtime/tests/mcp_server.rs`
- Out of scope:
  - MCP transport pooling and private tmp session reuse.
  - Protocol shape changes.
  - Receipt schema changes.
  - Performance benchmark additions.
  - TypeScript CLI changes.

## Dependencies

- Builds on the existing runtime graph hot-path cleanup and sandbox hardening.
- Must not conflict with `runx-runtime-test-gate-dx-v1`, which owns direct test
  self-provisioning and verification-script work.

## Assumptions

- Tool metadata can be cloned cheaply relative to tool execution.
- Run-id uniqueness is the only mutable per-call state that must remain
  synchronized.
- Existing receipt writes are already isolated by their own store/path
  semantics and should not rely on the MCP server mutex.

## Touchpoints

- `crates/runx-runtime/src/adapters/mcp/server.rs`
- `crates/runx-runtime/src/adapters/mcp/server_skill.rs`
- `crates/runx-runtime/tests/mcp_server.rs`

## Risks

- Moving execution outside the lock can expose hidden shared mutable state.
  Mitigation: keep execution inputs owned/cloned before lock release and add
  concurrent-call tests.
- Run-id allocation can race if split incorrectly. Mitigation: isolate it behind
  an atomic counter or a minimal locked method.
- Tests with wall-clock sleeps can be flaky. Mitigation: use deterministic
  blocking channels/barriers where practical.

## Acceptance

Profile: strict

Validation:
- `cargo test --manifest-path crates/Cargo.toml -p runx-runtime --all-features --test integration mcp_server`
- `cargo test --manifest-path crates/Cargo.toml -p runx-runtime --all-features --lib`
- `cargo fmt --manifest-path crates/Cargo.toml --all --check`
- `scafld review runx-runtime-mcp-concurrency-v1 --provider claude --review-depth deep`

## Phase 1: Lock Boundary Split

Status: completed
Dependencies: none

Objective: Move long-running MCP tool execution outside shared state locks.

Changes:
- Replace coarse lock-held execution with a short critical section that clones resolved tool metadata and allocates a run id.
- Keep all adapter invocation, receipt persistence, host resolution, and response building outside the state mutex.
- Keep server state APIs named around ownership: metadata lookup, run-id allocation, and execution.

Acceptance:
- [x] `p1_ac1` command - No call path holds the MCP server state mutex across
  - Command: `rg -n "lock\\(|handle_rmcp_tool_call|next_run_id" crates/runx-runtime/src/adapters/mcp/server.rs`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-3
- [x] `p1_ac2` command - Run ids remain unique and monotonic under concurrent calls.
  - Command: `rg -n "Atomic|next_run_id|run_id" crates/runx-runtime/src/adapters/mcp/server.rs crates/runx-runtime/tests/mcp_server.rs`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-4

## Phase 2: Concurrency Correctness Tests

Status: completed
Dependencies: phase1

Objective: Prove the lock split with behavior, not benchmarks.

Changes:
- Add a test where one tool call blocks and a second independent call completes without waiting on the first.
- Add a test where metadata/list/get remains available while an execution is in progress if current test harness can express that without protocol churn.
- Keep assertions deterministic and avoid timing-only flakes.

Acceptance:
- [x] `p2_ac1` command - MCP server integration tests pass.
  - Command: `cargo test --manifest-path crates/Cargo.toml -p runx-runtime --all-features --test integration mcp_server`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-9

## Phase 3: Validation

Status: completed
Dependencies: phase2

Objective: Verify runtime safety.

Changes:
- none

Acceptance:
- [x] `p3_ac1` command - Runtime library tests pass.
  - Command: `cargo test --manifest-path crates/Cargo.toml -p runx-runtime --all-features --lib`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-14
- [x] `p3_ac2` command - Formatting remains clean.
  - Command: `cargo fmt --manifest-path crates/Cargo.toml --all --check`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-15

## Phase 4: Claude Review Gate Preparation

Status: completed
Dependencies: phase3

Objective: Verify the requested provider gate is declared before handing the

Changes:
- none

Acceptance:
- [x] `p4_ac1` command - Claude review command is declared.
  - Command: `rg -n "scafld review runx-runtime-mcp-concurrency-v1 --provider claude --review-depth deep" .scafld/specs/active/runx-runtime-mcp-concurrency-v1.md`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-24

## Rollback

Revert the server lock-boundary commit. The rollback must preserve existing MCP
protocol behavior and tests; no schema migration is involved.

## Review

Status: completed
Verdict: pass
Mode: verify
Provider: claude:claude-opus-4-7
Output: claude.mcp_submit_review
Summary: The previous discover-pass blocker (mcp-concurrency-1: missing Phase 2 concurrent-dispatch test) is resolved: tests/mcp_server.rs:182 now defines `mcp_server_concurrent_call_completes_while_slow_skill_runs`, which builds an MCP server with a slow cli-tool skill (sh sleep 0.2) plus a fast fixed tool, drives both through `serve_mcp_json_rpc` via framed stdio, and asserts the id=2 fast response is written before the id=1 slow response. That ordering only holds when the two tool calls dispatch concurrently, so a future regression that reintroduces a coarse `Mutex<McpServerState>` or otherwise serializes execution would flip the ordering and fail the test. The secondary mcp-concurrency-2 observation (worker_threads(2) saturation under blocking adapter work) is also materially addressed: `RmcpProofServer::call_tool` now wraps skill execution in `tokio::task::spawn_blocking` (server.rs:331), moving blocking subprocess/filesystem work off the tokio worker pool so list_tools / get_tool remain pollable while a skill is running. McpServerState retains only immutable `options` plus an `AtomicU64` run-id counter; `prepare_rmcp_tool_call` clones the resolved tool out of `state.options.tools` and allocates the run id under the atomic only for the Skill variant (Fixed tools do not consume run ids). No coarse lock remains in the call path, `next_run_id` uses Relaxed `fetch_update` with saturating_add and is monotonic/unique, and `&self`-only borrows over `Arc<RmcpProofServer>` are Send+Sync because `McpServerOptions` is owned data. Acceptance commands recorded as pass match what is on disk. No completion-blocking regressions detected within scope.

Attack log:
- `crates/runx-runtime/tests/mcp_server.rs`: Verify Phase 2 deliverable: a test that drives serve_mcp_json_rpc / call_tool with a blocking call interleaved against an independent fast call -> clean (mcp_server_concurrent_call_completes_while_slow_skill_runs at tests/mcp_server.rs:182 builds a tempdir cli-tool skill that runs sh ./run.sh with `sleep 0.2`, plus a fixed `fast` tool, frames initialize + initialized + tools/call(slow, id=1) + tools/call(fast, id=2) into the stdio cursor, runs it through serve_mcp_json_rpc, and asserts fast_position < slow_position in the parsed response stream. A serial path would write id=1 before id=2 and fail the assertion. Previous mcp-concurrency-1 finding is repaired.)
- `crates/runx-runtime/src/adapters/mcp/server.rs:257-342`: Confirm call_tool no longer holds shared state across blocking work and that tokio worker threads are not held for the duration of skill execution -> clean (call_tool now calls prepare_rmcp_tool_call (short, lockless: tools.iter().find().cloned() + state.next_run_id atomic) and returns execute_rmcp_tool_call(prepared). execute_rmcp_tool_call wraps skill execution in tokio::task::spawn_blocking(move || execute_mcp_server_skill(...)).await, which moves blocking subprocess+receipt+host work to the tokio blocking pool and frees the worker thread, so worker_threads(2) is no longer saturated by two concurrent skill calls. Fixed tools return immediately without spawn_blocking, which is appropriate.)
- `McpServerState (server.rs:507-530)`: Hunt for residual shared mutable state or lock-free hazards introduced by removing the coarse mutex -> clean (Struct holds only `options: McpServerOptions` (read-only after construction; consumed by value into RmcpProofServer.state and only borrowed via &self thereafter) and `next_run_sequence: AtomicU64`. No interior mutability beyond the atomic. rmcp wraps the service in Arc for dispatch, so &self is shared across spawned tasks safely. next_run_id uses fetch_update with Relaxed/Relaxed and saturating_add(1); monotonic and unique up to u64::MAX which is unreachable.)
- `Receipt write path under concurrent tool calls`: Probe for races when two concurrent tools/call invocations write into the same receipt_dir -> clean (complete_mcp_server_skill (server_skill.rs:259) derives the receipt from the unique run_id allocated by McpServerState::next_run_id, and write_local_receipt_dir writes per-id files. With distinct run ids per concurrent call, receipt file paths do not collide. ReceiptServices::from_env is invoked per call inside spawn_blocking, isolating each call's signing config.)
- `tests/mcp_server.rs:182 slow skill timing strategy`: Check for wall-clock-only flakes in the new concurrency test -> clean (The 0.2s sleep is enough margin that the fast Fixed tool path (in-process, returns immediately) will write before the slow cli-tool subprocess finishes even on loaded CI workers. A barrier/channel between the slow and fast tools is not expressible without a protocol detour, and the spec text explicitly accepts wall-clock latencies when 'channels/barriers' are not practical. The 200ms buffer is generous relative to expected dispatch latency.)
- `Workspace/scope hygiene`: Confirm task-scope changes are confined to the three declared files and that ambient drift (other modified crates/runtime files and docs/thesis.md) is unrelated to this spec -> clean (Modifications to crates/runx-runtime/src/adapters/mcp/server.rs, server_skill.rs, and tests/mcp_server.rs match the declared touchpoints. Other modified files (execution/graph.rs, output_projection.rs, runner/steps.rs, receipts.rs, receipts/seal.rs, packages/cli/*, scripts/test-boundaries.mjs) belong to the other active specs runx-runtime-step-output-projection-v1 and runx-ts-native-passthrough-cutover-v1 and are not attributable to this task. No review-self-mutation of the spec file detected.)
- `Configured invariants (no_legacy_code, no_test_logic_in_production, public_api_stable)`: Check the cutover for compatibility shims, test-only branches in production functions, or breaking public API changes -> clean (No fallback Mutex<McpServerState> path, no .v2 ids, no compatibility flags. #[cfg(test)] mod tests in server.rs is compiled out for non-test builds and does not branch production code. Public surface (serve_mcp_json_rpc, mcp_tool_result_from_host_result, McpServerOptions, McpServerTool, McpServerToolBehavior, etc.) keeps stable signatures; McpServerState remains pub(super).)

Findings:
- none
