---
spec_version: '2.0'
task_id: rust-registry-cli-search
created: '2026-05-20T04:44:21Z'
updated: '2026-05-20T04:53:04Z'
status: completed
harden_status: not_run
size: medium
risk_level: medium
---

# Rust registry CLI boundary

## Current State

Status: completed
Current phase: final
Next: done
Reason: task completed
Blockers: none
Allowed follow-up command: `none`
Latest runner update: 2026-05-20T04:53:04Z
Review gate: pass

## Summary

Add a native `runx registry` boundary behind `RUNX_RUST_CLI` so registry
search/read/resolve/install/publish can call `runx-runtime::registry` directly.
This is not the full `@runxhq/core/registry` sunset. It is the executable slice
that gives TS CLI/runtime importers a Rust-owned process boundary to migrate to
next.

## Objectives

- Add native `runx registry search|read|resolve|install|publish`.
- Route local registry operations through `runx-runtime::registry`.
- Route remote registry search/read/resolve/acquire through the shared runtime
  hosted HTTP transport, without reintroducing `runx-hosted-http`,
  `runx-registry-client`, `reqwest`, `hyper`, or `serde_yml`.
- Preserve candidate gating: native registry commands are only reachable when
  `RUNX_RUST_CLI` is set.
- Prove local publish/search/resolve/install end to end from the Rust binary.

## Scope

In scope:
- `crates/runx-cli/src/registry.rs`
- registry launcher parsing/routing in `crates/runx-cli/src/launcher.rs`
- `crates/runx-cli/src/lib.rs`
- `crates/runx-cli/src/main.rs`
- focused CLI registry tests
- serialization derives needed for registry CLI JSON output

Out of scope:
- deleting `packages/core/src/registry/**`
- removing `@runxhq/core/registry`
- migrating TS runtime/CLI importers to the native boundary
- changing hosted registry HTTP routes
- adding compatibility shims or v2 registry shapes

## Dependencies

- Existing `runx-runtime::registry` local and hosted registry implementation.
- Existing shared hosted HTTP runtime transport.
- Active `rust-ts-sunset-registry` importer census.

## Assumptions

- A process boundary is acceptable for CLI registry operations.
- Install/publish remain explicit local CLI actions; execution evidence belongs
  to the enclosing harness receipt when registry install is part of execution.
- The next sunset slice will migrate TS importers to this native boundary
  before deleting TS registry code.

## Touchpoints

- Rust CLI launcher
- Rust runtime registry module
- Active registry sunset spec

## Risks

- The new command could accidentally make the Rust CLI look release-authoritative.
  Mitigation: native routing remains behind `RUNX_RUST_CLI`.
- Registry output could duplicate TS semantics incorrectly. Mitigation: output
  uses Rust runtime registry structs, and the end-to-end test exercises the
  runtime boundary directly.

## Acceptance

Profile: standard

Validation:
- `cargo check --manifest-path crates/Cargo.toml -p runx-cli`
- `cargo test --manifest-path crates/Cargo.toml -p runx-cli --test registry -- --nocapture`
- `cargo test --manifest-path crates/Cargo.toml -p runx-cli --test launcher registry -- --nocapture`
- `cargo clippy --manifest-path crates/Cargo.toml -p runx-cli --all-targets -- -D warnings`
- `cargo fmt --manifest-path crates/Cargo.toml --all --check`
- `cargo deny --manifest-path crates/Cargo.toml check bans licenses sources`
- `git diff --check` for touched files

## Phase 1: Implementation

Status: completed
Dependencies: none

Objective: Make the native registry CLI boundary real and compilable.

Changes:
- Added `crates/runx-cli/src/registry.rs`.
- Wired `LauncherAction::RunRegistry` into launcher and main.
- Added `registry search|read|resolve|install|publish` parser support behind
  `RUNX_RUST_CLI`.
- Added serde output support for install and resolved-ref registry structs.
- Added native CLI test coverage for local publish/search/resolve/install.

Acceptance:
- [x] `ac1` command - focused validation
  - Command: see Validation
  - Expected kind: `exit_code_zero`
  - Status: passed

## Rollback

- Revert this slice and leave `rust-ts-sunset-registry` blocked. Do not replace
  it with a TS compatibility shim.

## Review

Status: completed
Verdict: pass
Mode: verify
Summary: Human-reviewed override accepted: Focused registry CLI boundary reviewed against implementation scope; cargo check, targeted registry/launcher tests, clippy, fmt, cargo-deny, and diff checks passed; full TS registry deletion remains explicitly out of scope.

Attack log:
- `review gate`: manual human audit -> clean (Focused registry CLI boundary reviewed against implementation scope; cargo check, targeted registry/launcher tests, clippy, fmt, cargo-deny, and diff checks passed; full TS registry deletion remains explicitly out of scope.)

Findings:
- none

## Self Eval

- This closes the missing native registry CLI boundary. It does not close the
  registry sunset; live TS importers still need migration to this boundary.

## Deviations

- Captured after implementation began because the missing `registry.rs` module
  was already referenced by `runx-cli` and blocked Rust checks.

## Metadata

- created_by: scafld

## Origin

Created by: scafld
Source: plan

## Harden Rounds

- none

## Planning Log

- none
