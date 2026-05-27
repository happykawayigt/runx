---
spec_version: '2.0'
task_id: runx-ts-native-passthrough-cutover-v1
created: '2026-05-27T16:00:00Z'
updated: '2026-05-27T14:52:18Z'
status: completed
harden_status: not_run
size: large
risk_level: medium
---

# TypeScript native passthrough cutover

## Current State

Status: completed
Current phase: final
Next: done
Reason: task completed
Blockers: none
Allowed follow-up command: `none`
Latest runner update: 2026-05-27T14:52:18Z
Review gate: pass

## Summary

Cut TypeScript CLI passthrough commands over to streaming native execution
instead of buffering native stdout/stderr through JS. Commands that Rust owns
should invoke the verified native binary directly and stream output to the
terminal. Buffered native execution remains only where TypeScript must parse the
result.

This is a clean boundary cutover: Rust owns runtime behavior; TypeScript owns
package entrypoint and legacy authoring surfaces only. No duplicated command
responsibilities should remain in passthrough paths.

## Objectives

- Add a streaming native subprocess path for passthrough commands.
- Route Rust-owned commands through streaming passthrough where TypeScript does
  not inspect output.
- Keep buffered native calls only for commands whose JSON output is parsed by
  TypeScript.
- Avoid Node -> selector -> Rust double-spawn for internal native calls when a
  verified platform binary can be resolved safely.
- Preserve exit code, signal, stdout, and stderr semantics.
- Pass the Claude provider adversarial review gate.

## Scope

- In scope:
  - `packages/cli/src/native-runx.ts`
  - `packages/cli/src/dispatch.ts`
  - Focused CLI tests for streaming passthrough/exit behavior.
  - Minimal package script updates if required by the focused tests.
- Out of scope:
  - Removing TypeScript authored tool build support.
  - Registry acquisition semantics.
  - Stable hash helper migration in `@runxhq/core`.
  - Rust command implementation changes unless needed to preserve passthrough
    contracts.
  - Performance benchmark additions.

## Dependencies

- Should run after runtime specs to avoid mixing runtime and TS changes.
- Coordinate with active `runx-runtime-test-gate-dx-v1`; do not clobber
  verification-script work owned by that spec.

## Assumptions

- Package-internal TypeScript code can resolve the native binary through the
  same trusted selector logic used by `bin/runx`, but should invoke the resolved
  binary directly.
- Passthrough commands do not require stdout parsing by TypeScript.
- Existing buffered helper remains for JSON bridge cases.

## Touchpoints

- `packages/cli/src/native-runx.ts`
- `packages/cli/src/dispatch.ts`
- `packages/cli/bin/runx` only if selector resolution needs shared extraction.
- `packages/cli/src/**/*.test.ts` or existing CLI test harness.

## Risks

- Streaming can change stdout/stderr ordering if implemented through separate
  event handlers incorrectly. Mitigation: pipe child streams directly.
- Native binary resolution must not skip verification. Mitigation: share the
  selector's verification path or keep direct binary resolution behind the same
  trust checks.
- Some commands may rely on buffered parsing. Mitigation: keep an explicit
  buffered helper and route only known passthrough commands to streaming.

## Acceptance

Profile: strict

Validation:
- `pnpm --filter @runxhq/cli test`
- `pnpm typecheck`
- `pnpm boundary:check`
- `cargo fmt --manifest-path crates/Cargo.toml --all --check`
- `scafld review runx-ts-native-passthrough-cutover-v1 --provider claude --review-depth deep`

## Phase 1: Native Execution Boundary

Status: completed
Dependencies: none

Objective: Split buffered and streaming native execution APIs.

Changes:
- Add `streamNativeRunx` or equivalent that pipes stdout/stderr and resolves with exact process status.
- Keep `runNativeRunx` buffered for JSON parsing.
- Share native binary resolution rather than shelling through `runx` again when safe.

Acceptance:
- [x] `p1_ac1` command - Streaming helper does not buffer stdout/stderr.
  - Command: `rg -n "streamNativeRunx|stdio|pipe|spawn" packages/cli/src/native-runx.ts`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-3
- [x] `p1_ac2` command - Buffered helper remains available only for parse-needed
  - Command: `rg -n "runNativeRunx|streamNativeRunx|writeNativeRunx" packages/cli/src/native-runx.ts packages/cli/src/dispatch.ts`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-4

## Phase 2: Dispatch Cutover

Status: completed
Dependencies: phase1

Objective: Route Rust-owned passthrough commands through streaming native

Changes:
- Convert passthrough branches for Rust-owned commands that TypeScript does not parse.
- Keep TypeScript authored-tool and parse-needed paths explicit and narrow.

Acceptance:
- [x] `p2_ac1` command - CLI tests pass.
  - Command: `pnpm --filter @runxhq/cli test`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-9
- [x] `p2_ac2` command - TypeScript typecheck passes.
  - Command: `pnpm typecheck`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-10

## Phase 3: Boundary Validation

Status: completed
Dependencies: phase2

Objective: Complete this phase.

Changes:
- none

Acceptance:
- [x] `p3_ac1` command - Boundary check passes.
  - Command: `pnpm boundary:check`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-15
- [x] `p3_ac2` command - Rust formatting remains clean.
  - Command: `cargo fmt --manifest-path crates/Cargo.toml --all --check`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-16

## Phase 4: Claude Review Gate Preparation

Status: completed
Dependencies: phase3

Objective: Verify the requested provider gate is declared before handing the

Changes:
- none

Acceptance:
- [x] `p4_ac1` command - Claude review command is declared.
  - Command: `rg -n "scafld review runx-ts-native-passthrough-cutover-v1 --provider claude --review-depth deep" .scafld/specs/active/runx-ts-native-passthrough-cutover-v1.md`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-21

## Rollback

Revert the TypeScript passthrough commit. Buffered native execution remains the
fallback implementation during rollback, with no Rust schema or runtime state
changes.

## Review

Status: completed
Verdict: pass
Mode: discover
Provider: claude:claude-opus-4-7
Output: claude.mcp_submit_review
Summary: Streaming cutover is mechanically sound. `streamNativeRunx` pipes child stdout/stderr live, shares verified binary resolution with `runNativeRunx`, and dispatch routes Rust-owned passthrough through streaming while keeping buffered JSON paths (`runNativeRunxJson`, registry/history non-JSON, skill execution) explicit. Exit-code semantics, signal-loss behavior, closed stdin, NO_COLOR injection, and PATH fallback all match the prior buffered code path, so no regressions are introduced. Reviewed: native-runx.ts spawn paths, dispatch.ts cutover branches, bin/runx selector parity, and new streaming test. A few low-severity hygiene observations recorded but none block completion.

Attack log:
- `packages/cli/src/native-runx.ts streamNativeRunx`: Spawn options/stdio: confirm streaming uses pipe stdio and not inherit; ensure stdout/stderr flow live -> clean (stdio:["ignore","pipe","pipe"] with on('data') forwarders; matches buffered stdio shape so no new stdin regression vs. prior dispatch path.)
- `packages/cli/src/native-runx.ts spawnStreamingNativeRunx terminate/fail`: Timer/kill race: SIGTERM→SIGKILL escalation, settled guard, clearTimers parity with buffered version -> finding (fail() path skips SIGKILL escalation (recorded as low-sev finding stream-fail-no-sigkill-fallback).)
- `packages/cli/src/native-runx.ts forwardOutput backpressure`: Pause/resume coupling and duplicate drain handlers when both streams backpressure -> finding (Self-correcting but coupled; recorded as low-sev finding stream-backpressure-resumes-both.)
- `packages/cli/src/dispatch.ts streamNativeRunxToIo routing`: Spec-compliance: ensure only commands whose stdout TS does not parse are routed through streaming -> clean (harness/dev/policy/tool search+inspect/registry install+publish/history --json stream; skill execution, history non-JSON, native-registry search, history.ts still use buffered runNativeRunx/Json.)
- `packages/cli/src/native-runx.ts resolveNativeRunxBinary`: Binary resolution parity with bin/runx (verified package, checksum, absolute override, no PATH escape) -> clean (Verifies platform package + sha256 + executable bit, mirroring bin/runx; PATH fallback is pre-existing behavior preserved from the buffered code path and outside this spec's scope.)
- `packages/cli/src/native-runx.ts nativeRunxEnv`: Env merge: NO_COLOR/RUNX_RUST_CLI forced; check for leaks or override surprises that would alter passthrough semantics -> clean (Same merge logic as buffered helper; NO_COLOR/RUNX_RUST_CLI=1 were already applied to the buffered path, so streaming preserves prior dispatch behavior.)
- `packages/cli/bin/runx`: Selector parity: streamed path must use the same verified platform binary as the published launcher -> clean (Both verify package name, platform, binary path, and sha256. Streaming dispatch path is dev/test-only (bin/runx is the published entry); behavior aligned.)
- `packages/cli/src/native-runx.test.ts streamNativeRunx`: Test asserts streaming (settled=false after early output) without flake under slow CI -> finding (100ms in-child sleep is tight; recorded as low-sev finding stream-test-timing-tight.)
- `packages/cli/src/dispatch.ts exit-code semantics`: Signal-loss in streamNativeRunxToIo (returns status ?? 1) compared to bin/runx signal re-raise -> clean (Pre-existing behavior: buffered writeNativeRunx also returned status ?? 1 with no signal re-raise. Streaming preserves prior dispatch semantics; bin/runx (published) continues to re-raise.)

Findings:
- [low/non-blocking] `stream-backpressure-resumes-both` Streaming forwarder couples stdout/stderr pause/resume: drain on one stream resumes both even if the other is still backpressured.
  - Location: `packages/cli/src/native-runx.ts:254`
  - Evidence: packages/cli/src/native-runx.ts:254-268 — forwardOutput pauses both child.stdout and child.stderr on any single backpressure and a single drain handler resumes both. If both pipes backpressure, the first drain prematurely resumes the saturated peer; multiple drain handlers can also be queued because once() is registered per write.
  - Impact: Self-correcting churn (extra pause/resume cycles) under combined stdout+stderr backpressure. Not a hang or data loss in practice on a TTY or normal pipe.
  - Validation: Local trace; no failing test required.
- [low/non-blocking] `stream-fail-no-sigkill-fallback` `fail()` in streaming path sends SIGTERM but never escalates to SIGKILL, so a child ignoring SIGTERM can outlive the rejected Promise.
  - Location: `packages/cli/src/native-runx.ts:225`
  - Evidence: packages/cli/src/native-runx.ts:225-231 — fail() clears timers and calls child.kill('SIGTERM') without scheduling the 1s SIGKILL fallback used by terminate(). If a forward-output write throws and the child refuses SIGTERM, the parent rejects while the child stays alive.
  - Impact: Possible zombie child on rare error path (e.g., EPIPE on consumer stream combined with a stubborn child). Buffered terminate() already handles this; streaming fail() is the only exit that does not.
  - Validation: Code review only.
- [low/non-blocking] `stream-test-timing-tight` streamNativeRunx test relies on a 100ms in-child sleep to prove streaming; slow CI could see process exit before the settled assertion runs.
  - Location: `packages/cli/src/native-runx.test.ts:72`
  - Evidence: packages/cli/src/native-runx.test.ts:69-108 — the spawned node script exits 100ms after writing stdout/stderr; the test awaits earlyOutput then asserts `settled === false`. On a slow runner, the 100ms can elapse before the microtask check runs.
  - Impact: Potential flake on saturated CI; does not affect product behavior.
  - Validation: Code review only.
