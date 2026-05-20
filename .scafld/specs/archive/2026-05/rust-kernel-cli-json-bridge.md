---
spec_version: '2.0'
task_id: rust-kernel-cli-json-bridge
created: '2026-05-20T03:18:00Z'
updated: '2026-05-20T03:18:49Z'
status: completed
harden_status: not_run
size: medium
risk_level: medium
---

# Rust kernel CLI JSON bridge

## Current State

Status: completed
Current phase: final
Next: done
Reason: task completed
Blockers: none
Allowed follow-up command: `none`
Latest runner update: 2026-05-20T03:18:49Z
Review gate: pass

## Summary

Add a process-JSON bridge for Rust kernel decisions:

```sh
RUNX_RUST_CLI=1 runx kernel eval --input <file|-> --json
```

The input is the existing kernel fixture input shape, either as a raw
`{ "kind": ... }` object or as a full fixture document with an `input` field.
The output envelope is:

```json
{
  "status": "success",
  "result": {
    "kind": "output",
    "value": {}
  }
}
```

This is an interop bridge, not a cutover. It does not remove TypeScript policy
or state-machine imports. The sunset specs can consume this bridge only after
this task is reviewed and completed.

## Context

CWD: `.`

Packages:
- `crates/runx-core`
- `crates/runx-cli`
- `@runxhq/runtime-local`
- `@runxhq/core`

Why this exists:
- `rust-ts-sunset-policy` is blocked by live `runtime-local` policy calls.
- `rust-ts-sunset-state-machine` is blocked by live `runtime-local`
  state-machine calls.
- `docs/ts-interop-boundary.md` allows CLI JSON as a TypeScript/Rust crossing
  and forbids adding a fourth boundary without an explicit doc update.

## Invariants

- TypeScript remains authoritative until the relevant sunset specs complete.
- This bridge exposes kernel fixture semantics only; it does not expose a
  generic Rust eval or plugin boundary.
- Successful policy denials are still successful evaluations and exit `0`.
- Invalid CLI usage exits with the documented CLI usage code.
- Invalid JSON, unknown input kinds, or evaluation serialization failures exit
  non-zero and return a structured JSON error when `--json` is present.
- No N-API, WASM, long-lived daemon, or compatibility TS shim is introduced.
- No registry, receipt, runtime-local deletion, or CLI release cutover happens
  in this spec.

## Scope

In scope:
- `runx-core::kernel_eval` production helper for existing fixture input kinds.
- Candidate-gated Rust CLI wrapper.
- Focused Rust tests for policy and state-machine fixture input.
- TS smoke test that spawns the Rust CLI for one policy fixture and one
  state-machine fixture.

Out of scope:
- Deleting TS policy/state-machine modules.
- Replacing `runtime-local` imports.
- Promoting Rust kernel parity to blocking CI.
- Adding new kernel decision semantics beyond the existing fixture shapes.

## Acceptance Criteria

- `crates/runx-core::kernel_eval` evaluates existing policy and state-machine
  fixture documents.
- `RUNX_RUST_CLI=1 runx kernel eval --input fixtures/kernel/policy/retry-admission-denies-mutating-without-key.json --json`
  exits `0` and returns the expected policy denial value inside
  `result.value`.
- `RUNX_RUST_CLI=1 runx kernel eval --input fixtures/kernel/state-machine/sequential-plan-first-step.json --json`
  exits `0` and returns the expected state-machine plan value inside
  `result.value`.
- The launcher delegates `kernel` unless `RUNX_RUST_CLI` is non-empty and not
  `0`.
- `runtime-local` smoke tests prove TypeScript can call the Rust evaluator by
  process JSON without importing Rust bindings.
- `rust-ts-sunset-policy` and `rust-ts-sunset-state-machine` remain blocked
  until their import migration specs explicitly consume this bridge.

## Validation Commands

```sh
cargo test --manifest-path crates/Cargo.toml -p runx-core --test kernel_eval -- --nocapture
cargo clippy --manifest-path crates/Cargo.toml -p runx-core --tests -- -D warnings
cargo test --manifest-path crates/Cargo.toml -p runx-cli --test launcher -- --nocapture
cargo test --manifest-path crates/Cargo.toml -p runx-cli --test kernel -- --nocapture
pnpm vitest run packages/runtime-local/src/runner-local/kernel-bridge.test.ts
pnpm fixtures:kernel:check
pnpm fixtures:kernel:validate
```

2026-05-20 core evaluator results:
- `cargo test --manifest-path crates/Cargo.toml -p runx-core --test kernel_eval -- --nocapture`
  passed: 3 tests.
- `cargo test --manifest-path crates/Cargo.toml -p runx-core --test policy_fixtures --test state_machine_fixtures -- --nocapture`
  passed: 15 tests.
- `cargo clippy --manifest-path crates/Cargo.toml -p runx-core --tests -- -D warnings`
  passed.

2026-05-20 CLI bridge results:
- `cargo test --manifest-path crates/Cargo.toml -p runx-cli --test launcher -- --nocapture`
  passed: 48 tests.
- `cargo test --manifest-path crates/Cargo.toml -p runx-cli --test kernel -- --nocapture`
  passed: 6 tests.
- `cargo clippy --manifest-path crates/Cargo.toml -p runx-cli --tests -- -D warnings`
  passed.
- `cargo fmt --manifest-path crates/Cargo.toml -p runx-cli -p runx-core -- --check`
  passed.
- `pnpm vitest run packages/runtime-local/src/runner-local/kernel-bridge.test.ts`
  passed: 2 tests.
- `pnpm fixtures:kernel:check` passed: 64 fixtures checked.
- `pnpm fixtures:kernel:validate` passed.
- `git diff --check` passed for bridge-related files.

## Rollback And Repair

- Before CLI consumption, rollback is to remove `runx-core::kernel_eval` and
  its tests.
- After CLI consumption, rollback is to remove the `kernel eval` launcher
  branch and keep TS imports in place.
- If a fixture shape is missing, add the Rust kernel parity implementation and
  fixture first. Do not invent a bridge-only shape.
- If runtime-local needs a different crossing, update
  `docs/ts-interop-boundary.md` and harden a new spec; do not smuggle a fourth
  boundary into this task.

## Review

Status: completed
Verdict: pass
Mode: verify
Summary: Human-reviewed override accepted: Codex review subagent found no blockers; focused Rust CLI, core evaluator, TS process smoke, fixture, clippy, fmt, scafld validate, and diff checks passed.

Attack log:
- `review gate`: manual human audit -> clean (Codex review subagent found no blockers; focused Rust CLI, core evaluator, TS process smoke, fixture, clippy, fmt, scafld validate, and diff checks passed.)

Findings:
- none

