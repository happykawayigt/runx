---
spec_version: '2.0'
task_id: rust-runtime-kernel-command-provisioning
created: '2026-05-20T07:09:19Z'
updated: '2026-05-20T07:37:04Z'
status: completed
harden_status: not_run
size: medium
risk_level: medium
---

# rust-runtime-kernel-command-provisioning

## Current State

Status: completed
Current phase: final
Next: done
Reason: task completed
Blockers: none
Allowed follow-up command: `none`
Latest runner update: 2026-05-20T07:37:04Z
Review gate: pass

## Summary

Provision a built Rust `runx` kernel eval binary for workspace test harnesses.
Runtime-local graph policy slices already depend on `RUNX_KERNEL_EVAL_BIN`;
future local-admission cutover would otherwise break ordinary `pnpm test` and
`verify:fast` runs. This slice makes the Rust kernel command available to
Vitest without changing runtime behavior for published packages.

## Objectives

- Build `crates/runx-cli` once before workspace Vitest runs.
- Export `RUNX_KERNEL_EVAL_BIN` to Vitest children launched by
  `scripts/test-workspace.mjs`.
- Export the same environment for `pnpm test:fast` inside
  `scripts/verify-fast.mjs`.
- Preserve explicit missing-kernel tests that override `RUNX_KERNEL_EVAL_BIN`
  to the empty string.

## Scope

- In scope:
  - `scripts/test-workspace.mjs`
  - `scripts/verify-fast.mjs`
- Out of scope:
  - changing `resolveKernelCommand`
  - changing published runtime-local package behavior
  - local admission/credential cutover itself

## Dependencies

- Rust CLI kernel eval binary builds with `cargo build -p runx-cli --bin runx`.

## Assumptions

- Test harness scripts may compile the Rust CLI as setup; package runtime code
  should still require explicit `RUNX_KERNEL_EVAL_BIN` or command injection.

## Touchpoints

- `pnpm test`
- `pnpm verify:fast`
- Vitest env used by runtime-local tests

## Risks

- Risk: workspace test startup gets slower. Mitigation: build once and rely on
  Cargo incremental caching.
- Risk: tests intended to exercise missing-kernel behavior stop doing so.
  Mitigation: those tests pass explicit `RUNX_KERNEL_EVAL_BIN: ""`, which
  overrides the harness env.

## Acceptance

Profile: standard

Validation:
- `pnpm test -- packages/runtime-local/src/runner-local/kernel-bridge.test.ts tests/graph-runner-governance.test.ts`
- `RUNX_KERNEL_EVAL_BIN=crates/target/debug/runx pnpm exec vitest run --config vitest.config.ts packages/runtime-local/src/runner-local/kernel-bridge.test.ts tests/graph-runner-governance.test.ts`
- `node scripts/verify-fast.mjs`
- `git diff --check -- scripts/test-workspace.mjs scripts/verify-fast.mjs .scafld/specs/active/rust-runtime-kernel-command-provisioning.md`

## Phase 1: Implementation

Status: completed
Dependencies: none

Objective: Make Rust kernel eval available to workspace test harnesses.

Changes:
- Add a small Cargo build helper in `scripts/test-workspace.mjs`.
- Inject the resulting binary path into Vitest child environments.
- Add equivalent environment provisioning around `pnpm test:fast` in `scripts/verify-fast.mjs`.

Acceptance:
- [x] `ac1` command - Workspace test wrapper provisions kernel bin
  - Command: `pnpm test -- packages/runtime-local/src/runner-local/kernel-bridge.test.ts tests/graph-runner-governance.test.ts`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-6
- [x] `ac2` command - Direct focused tests still pass with an explicit kernel binary
  - Command: `RUNX_KERNEL_EVAL_BIN=crates/target/debug/runx pnpm exec vitest run --config vitest.config.ts packages/runtime-local/src/runner-local/kernel-bridge.test.ts tests/graph-runner-governance.test.ts`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-7
- [x] `ac3` command - Fast verification still passes with kernel provisioning
  - Command: `node scripts/verify-fast.mjs`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-8
- [x] `ac4` command - Diff has no whitespace errors
  - Command: `git diff --check -- scripts/test-workspace.mjs scripts/verify-fast.mjs .scafld/specs/active/rust-runtime-kernel-command-provisioning.md`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-9

## Rollback

- Restore test harness scripts to their prior child environments.

## Review

Status: completed
Verdict: pass
Mode: verify
Summary: Human-reviewed override accepted: Acceptance commands passed: workspace wrapper provisions the Rust kernel binary, direct focused Vitest passes with explicit RUNX_KERNEL_EVAL_BIN, node scripts/verify-fast.mjs passes, and the scoped diff check is clean. No blockers found.

Attack log:
- `review gate`: manual human audit -> clean (Acceptance commands passed: workspace wrapper provisions the Rust kernel binary, direct focused Vitest passes with explicit RUNX_KERNEL_EVAL_BIN, node scripts/verify-fast.mjs passes, and the scoped diff check is clean. No blockers found.)

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
