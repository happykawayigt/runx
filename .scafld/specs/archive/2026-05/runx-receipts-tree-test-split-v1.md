---
spec_version: '2.0'
task_id: runx-receipts-tree-test-split-v1
created: '2026-05-27T00:00:00Z'
updated: '2026-05-27T00:00:00Z'
status: completed
harden_status: passed
size: small
risk_level: low
---

# runx receipts tree test split v1

## Current State

Status: completed
Current phase: final
Next: done
Reason: task completed
Blockers: none
Allowed follow-up command: `none`
Review gate: pass

## Summary

`crates/runx-receipts/src/tree.rs` still carried the adversarial receipt-tree
test matrix after the production receipt-tree implementation had already been
split into focused modules. This spec moves the tests into focused child
modules while preserving the public receipt-tree API and proof semantics.

## Scope

- Keep the receipt-tree public API unchanged.
- Keep resolver, traversal, finding, and proof behavior unchanged.
- Move structural receipt-tree tests to `tree/structural_tests.rs`.
- Move strict proof tests to `tree/proof_tests.rs`.
- Move fixture builders and custom test resolvers to `tree/test_support.rs`.
- Remove the stale `large-file` waiver from `tree.rs`.
- Do not touch runtime, CLI, MCP, or contract-schema work owned by concurrent
  agents.

## Evidence

Commands run after implementation:

```sh
cargo fmt --manifest-path crates/Cargo.toml --all
CARGO_TARGET_DIR=/tmp/runx-receipts-tree-split-target cargo test --manifest-path crates/Cargo.toml -p runx-receipts --lib tree::
CARGO_TARGET_DIR=/tmp/runx-receipts-tree-split-target cargo clippy --manifest-path crates/Cargo.toml -p runx-receipts --all-targets -- -D warnings
rg -n "rust-style-allow: large-file" crates/runx-receipts/src/tree.rs crates/runx-receipts/src/tree
git diff --check -- .scafld/specs/archive/2026-05/runx-receipts-tree-test-split-v1.md crates/runx-receipts/src/tree.rs crates/runx-receipts/src/tree
```

All commands passed. A broader `cargo test -p runx-receipts` attempt passed
the library tree tests and conformance test, then the unrelated
`receipt_contracts` integration-test binary stalled before Rust test startup
at the macOS loader; other concurrent CLI test binaries in the workspace were
in the same state. This slice used the receipt-tree-specific test filter and
full receipt-crate clippy gate for the implementation evidence.

## Review Notes

- This is a test-organization split only; no receipt digest, signature, child
  resolver, or tree-proof code changed.
- Pre-existing dirty files were present in CLI, runtime MCP, and adapter
  fixtures. This spec did not touch them.
