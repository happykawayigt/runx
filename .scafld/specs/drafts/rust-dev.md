---
spec_version: '2.0'
task_id: rust-dev
created: '2026-05-18T00:00:00Z'
updated: '2026-05-20T13:02:08Z'
status: draft
harden_status: in_progress
size: medium
risk_level: medium
---

# Rust dev

## Current State

Status: draft
Current phase: runtime dev core slice implemented
Next: wire native skill/graph fixture execution and CLI watch/presentation cutover
Reason: a narrow Rust runtime slice now exists for dev fixture discovery,
deterministic tool fixture execution, polling watch debounce, presentation, and
dev-mode receipt metadata tagging. This is not complete `runx dev` parity yet.
Blockers: native skill/graph dev fixture execution is not wired in this slice;
the Rust CLI dev command is owned by the CLI cutover worker; the TS command
currently parses `--watch` but does not run a watch loop, so CLI-level watch
parity still needs an owning cutover decision.
Allowed follow-up command: implement the native skill/graph dev executor wiring,
then rerun runtime dev validation; do not mark passed until the remaining
blockers are closed.
Latest runner update: 2026-05-20T13:02:08Z
Review gate: not_started

## Summary

Port `runx dev` to Rust. Dev mode runs a skill or chain in an iterative
loop with file watch, fast-feedback receipts, and harness wiring. Today
this lives in `packages/cli/src/commands/dev/` and consumes runner-local
plus harness primitives.

## Context

CWD: `.`

Packages:
- `@runxhq/cli` (dev command tree)
- `@runxhq/runtime-local` (runner-local, harness)
- `crates/runx-runtime`

Current TypeScript sources:
- `packages/cli/src/commands/dev/**`
- `packages/cli/src/commands/dev.ts`
- `packages/runtime-local/src/harness/runner.ts`

Files impacted:
- `crates/runx-runtime/src/dev/watch.rs`
- `crates/runx-runtime/src/dev/loop.rs`
- `crates/runx-runtime/src/dev/presentation.rs`
- `fixtures/dev/**`

Invariants:
- File watch debounce and ignore patterns match TS.
- Dev mode never silently consumes secrets; reuses connect grants.
- Receipts emitted in dev are clearly tagged as dev-mode in metadata.

## Objectives

- Port dev mode loop with file watch.
- Match presentation (terminal output) to TS via snapshot tests.

## Scope

In scope:
- Dev loop, file watch, presentation.

Out of scope:
- New dev features beyond TS.

## Dependencies

- `rust-runtime-skeleton` (archived completed; review gate pass).
- `rust-harness` (archived completed; harden passed and review gate pass).

## Open Questions

- File watch library choice (notify, watchexec). Defer to Phase 1.

## Harden Rounds

### round-1

Status: in_progress
Started: 2026-05-20T10:34:14Z
Ended: none

Checks:
- `cargo fmt --package runx-runtime` from `crates`: passed.
- `cargo test -p runx-runtime --test dev -- --nocapture` from `crates`: passed
  with 5 tests.
- `cargo check -p runx-runtime` from `crates`: passed.
- `cargo fmt --manifest-path crates/Cargo.toml --package runx-runtime`: passed.
- `cargo test --manifest-path crates/Cargo.toml -p runx-runtime --test dev -- --nocapture`:
  passed with 5 tests.
- `cargo check --manifest-path crates/Cargo.toml -p runx-runtime`: passed.
- Earlier broad filtered check `cargo test -p runx-runtime dev -- --nocapture`
  passed the new 5 dev tests and filtered the rest; initial invocation from repo
  root failed because the Cargo workspace lives under `crates/`.

Issues:
- Runtime slice implemented under `crates/runx-runtime/src/dev/**` with
  deterministic tool fixture execution only.
- Native skill/graph dev fixture execution remains explicit failure metadata,
  not a TS bridge.
- CLI dev routing and release presentation cutover intentionally untouched.
