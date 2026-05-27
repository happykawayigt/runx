---
spec_version: '2.0'
task_id: runx-core-fanout-gate-split-v1
created: '2026-05-27T00:00:00Z'
updated: '2026-05-27T00:00:00Z'
status: completed
harden_status: passed
size: small
risk_level: medium
---

# runx core fanout gate split v1

## Current State

Status: completed
Current phase: final
Next: done
Reason: task completed
Blockers: none
Allowed follow-up command: `none`
Review gate: pass

## Summary

`crates/runx-core/src/state_machine/fanout.rs` carried quorum handling,
threshold gates, conflict gates, structured-output lookup, and value
normalization in one waived file. This spec decomposed the internal
implementation while preserving the public state-machine API:

- `evaluate_fanout_sync`
- `fanout_sync_decision_key`

No decision strings, `rule_fired` values, gate payloads, or serde-visible
state-machine contracts changed.

## Scope

- Keep the public `runx_core::state_machine` exports stable.
- Keep quorum and branch-failure decisions in `fanout.rs`.
- Move threshold-gate logic to `fanout/threshold.rs`.
- Move conflict-gate logic to `fanout/conflict.rs`.
- Move structured-output value helpers to `fanout/values.rs`.
- Remove the stale large-file waiver from `fanout.rs`.
- Do not touch runtime execution, CLI tests, or active release-readiness work.

## Evidence

Commands run after implementation:

```sh
cargo fmt --manifest-path crates/Cargo.toml --all
cargo test --manifest-path crates/Cargo.toml -p runx-core --test state_machine_fixtures
cargo test --manifest-path crates/Cargo.toml -p runx-core --test state_machine_proptest
cargo test --manifest-path crates/Cargo.toml -p runx-core state_machine
```

All commands passed.

## Review Notes

- A read-only parallel review found no semantic drift in fanout decision
  strings, `rule_fired` values, gate payload fields, or the public API.
- The only pre-existing dirty files observed during execution were CLI test
  files outside this spec's ownership; this spec did not touch them.
