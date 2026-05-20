---
spec_version: '2.0'
task_id: rust-policy-authority-proof-kernel-boundary
created: '2026-05-20T06:54:12Z'
updated: '2026-05-20T06:59:33Z'
status: completed
harden_status: not_run
size: medium
risk_level: medium
---

# rust-policy-authority-proof-kernel-boundary

## Current State

Status: completed
Current phase: final
Next: done
Reason: task completed
Blockers: none
Allowed follow-up command: `none`
Latest runner update: 2026-05-20T06:59:33Z
Review gate: pass

## Summary

Move graph-step authority-proof metadata generation out of the TypeScript policy
package and through the Rust kernel eval boundary. Graph scope admission already
uses the Rust kernel; this slice makes the denied graph receipt metadata use the
same authority-proof implementation without removing the broader
`@runxhq/core/policy` surface yet.

## Objectives

- Add a typed `authorityProofMetadataViaKernel` bridge in runtime-local.
- Remove the graph-governance runtime import of TypeScript
  `buildAuthorityProofMetadata`.
- Preserve fail-closed graph policy behavior and denied receipt metadata shape.
- Keep local skill admission, sandbox normalization, and policy package export
  removal out of scope.

## Scope

- In scope:
  - `packages/runtime-local/src/runner-local/kernel-bridge.ts`
  - `packages/runtime-local/src/runner-local/graph-governance.ts`
  - graph step/fanout handlers that write denied graph receipts
  - focused kernel bridge and graph-governance tests
- Out of scope:
  - deleting `packages/core/src/policy/**`
  - removing the `@runxhq/core/policy` package export
  - single-skill local admission and sandbox normalization
  - state-machine cutover

## Dependencies

- Completed Rust policy authority-proof parity and kernel eval JSON bridge.
- Existing runtime-local graph scope admission Rust bridge.

## Assumptions

- Runtime-local graph execution may require `RUNX_KERNEL_EVAL_BIN` when a graph
  path needs Rust policy evaluation. Missing or failing kernel eval remains a
  policy-denied/fail-closed condition.

## Touchpoints

- Graph step/fanout policy denial receipts.
- Runtime-local kernel bridge JSON parsing.
- Graph governance authority metadata.

## Risks

- If the bridge parser accepts an invalid value, denied receipts can carry weak
  authority metadata. Mitigation: parse the exact `authority_proof` object.
- If graph handlers forget to await metadata, policy denial receipts can lose
  authority context. Mitigation: focused typecheck and graph receipt tests.

## Acceptance

Profile: standard

Validation:
- `cargo build --quiet --manifest-path crates/Cargo.toml -p runx-cli --bin runx`
- `RUNX_KERNEL_EVAL_BIN="$PWD/crates/target/debug/runx" pnpm exec vitest run --config vitest.config.ts packages/runtime-local/src/runner-local/kernel-bridge.test.ts tests/graph-receipt-governance.test.ts tests/graph-fanout.test.ts tests/graph-runner-governance.test.ts`
- `cargo test --manifest-path crates/Cargo.toml -p runx-core --test policy_fixtures --test kernel_eval -- --nocapture`
- `cargo test --manifest-path crates/Cargo.toml -p runx-cli --test kernel -- --nocapture`
- `pnpm fixtures:kernel:check`
- `pnpm typecheck`
- `! rg -n 'buildAuthorityProofMetadata' packages/runtime-local/src/runner-local/graph-governance.ts packages/runtime-local/src/runner-local/orchestrator/handle-run-step.ts packages/runtime-local/src/runner-local/orchestrator/handle-run-fanout.ts`

## Phase 1: Implementation

Status: completed
Dependencies: none

Objective: Route graph-step authority-proof metadata through Rust kernel eval.

Changes:
- Add bridge request/response types and strict metadata parsing.
- Make `graphStepAuthorityProofMetadata` async and call the Rust bridge.
- Await authority-proof metadata before writing transition, scope, or retry policy-denied graph receipts.
- Remove the graph-governance import from `@runxhq/core/policy`.

Acceptance:
- [x] `ac1` command - Rust CLI binary builds for kernel eval
  - Command: `cargo build --quiet --manifest-path crates/Cargo.toml -p runx-cli --bin runx`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-6
- [x] `ac2` command - Focused graph policy tests pass through Rust kernel eval
  - Command: `RUNX_KERNEL_EVAL_BIN="$PWD/crates/target/debug/runx" pnpm exec vitest run --config vitest.config.ts packages/runtime-local/src/runner-local/kernel-bridge.test.ts tests/graph-receipt-governance.test.ts tests/graph-fanout.test.ts tests/graph-runner-governance.test.ts`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-7
- [x] `ac3` command - Rust policy/kernel fixture tests pass
  - Command: `cargo test --manifest-path crates/Cargo.toml -p runx-core --test policy_fixtures --test kernel_eval -- --nocapture`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-8
- [x] `ac4` command - Rust CLI kernel tests pass
  - Command: `cargo test --manifest-path crates/Cargo.toml -p runx-cli --test kernel -- --nocapture`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-9
- [x] `ac5` command - Kernel fixtures stay fresh
  - Command: `pnpm fixtures:kernel:check`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-10
- [x] `ac6` command - TypeScript typecheck passes
  - Command: `pnpm typecheck`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-11
- [x] `ac7` command - Graph governance no longer uses TS authority metadata
  - Command: `! rg -n 'buildAuthorityProofMetadata' packages/runtime-local/src/runner-local/graph-governance.ts packages/runtime-local/src/runner-local/orchestrator/handle-run-step.ts packages/runtime-local/src/runner-local/orchestrator/handle-run-fanout.ts`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-12

## Rollback

- Restore graph-governance to the TypeScript policy metadata helper and remove
  the bridge wrapper.

## Review

Status: completed
Verdict: pass
Mode: verify
Summary: Human-reviewed override accepted: Focused review after green build gate: Rust policy/kernel tests, focused graph tests, typecheck, kernel fixtures, grep, and diff check passed; no blockers found.

Attack log:
- `review gate`: manual human audit -> clean (Focused review after green build gate: Rust policy/kernel tests, focused graph tests, typecheck, kernel fixtures, grep, and diff check passed; no blockers found.)

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
