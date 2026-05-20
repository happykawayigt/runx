---
spec_version: '2.0'
task_id: rust-receipt-tree-strict-child-proof
created: '2026-05-20T06:55:05Z'
updated: '2026-05-20T06:59:44Z'
status: completed
harden_status: not_run
size: small
risk_level: medium
---

# Strict Rust receipt tree child proof acceptance

## Current State

Status: completed
Current phase: final
Next: done
Reason: task completed
Blockers: none
Allowed follow-up command: `none`
Latest runner update: 2026-05-20T06:59:44Z
Review gate: pass

## Summary

Prove the Rust runtime receipt tree verifier rejects parent graph harness
receipts unless each child harness receipt ref is complete under the strict
post-cutover proof model. The implementation already routes runtime graph tree
acceptance through strict harness receipt proof verification; this slice adds a
focused regression for a structurally resolvable child whose parent ref omits
the required child digest locator.

## Objectives

- Preserve the post-cutover `runx.harness_receipt.v1` receipt tree model.
- Prove parent graph harness receipt acceptance requires complete child refs,
  including a locator matching the child receipt seal digest.
- Keep the slice limited to Rust receipt tree files/tests.

## Scope

- In scope: `crates/runx-runtime/tests/receipt_tree.rs`.
- In scope for verification only: `crates/runx-receipts/src/tree.rs`,
  `crates/runx-receipts/src/verify/finding.rs`, and
  `crates/runx-runtime/src/receipt_tree.rs`.
- Out of scope: TypeScript runtime-local files, registry tests, and scafld
  hardening.

## Dependencies

- Existing strict receipt tree proof implementation from
  `rust-ts-sunset-receipts`.

## Assumptions

- Runtime receipt tree verification remains the live parent/child acceptance
  path for graph harness receipts.
- Legacy `LocalSkillReceipt` and `LocalGraphReceipt` aliases are already
  removed from this Rust receipt-tree surface.

## Touchpoints

- `crates/runx-runtime/tests/receipt_tree.rs`

## Risks

- A broad refactor could collide with concurrent Rust receipt-tree edits.
  Mitigation: add only a focused regression test unless validation exposes an
  implementation gap.

## Acceptance

Profile: standard

Validation:
- `cd crates && cargo test -p runx-receipts tree`
- `cd crates && cargo test -p runx-runtime --test receipt_tree`

## Phase 1: Implementation

Status: completed
Dependencies: none

Objective: Complete the requested change.

Changes:
- Add or adjust focused Rust tests proving strict parent/child receipt tree proof acceptance rejects incomplete child refs.

Acceptance:
- [x] `ac1` command - Receipt tree unit tests pass
  - Command: `cd crates && cargo test -p runx-receipts tree`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-11
- [x] `ac2` command - Runtime receipt tree integration tests pass
  - Command: `cd crates && cargo test -p runx-runtime --test receipt_tree`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-12

## Rollback

- Revert this slice's focused runtime receipt tree test addition.

## Review

Status: completed
Verdict: pass
Mode: discover
Provider: claude:claude-opus-4-7
Output: claude.mcp_submit_review
Summary: The slice adds a focused Rust runtime regression (`runtime_tree_rejects_child_ref_without_digest_locator`) that demonstrates the post-cutover strict proof model rejects a structurally resolvable child whose parent ref omits the required digest locator. The test correctly isolates the finding: it nulls `child_harness_receipt_refs[0].locator`, refreshes the root's body digest and signature so no other proof findings leak in, asserts the structural verifier still says valid, then asserts strict runtime verification emits `ChildReceiptDigestMismatch` at `runtime_receipts[0].locator`. Path/code line up: `RuntimeReceiptResolver::resolve_child` produces `runtime_receipts[0]` for the first matching child, `StrictChildProofPolicy::findings` calls `child_digest_link_findings` with that path, and that helper emits `ChildReceiptDigestMismatch` at `{path}.locator` when `reference.locator.as_deref() != Some(child.seal.digest.as_str())` (including the `None` case). Routing through `verify_runtime_receipt_tree` -> `verify_runtime_receipt_tree_with_policy(local_development)` -> `verify_receipt_tree_proof_with_resolver` does exercise the strict policy. No production-side change was required; recorded acceptance evidence (cargo test on `runx-receipts tree` and `runx-runtime --test receipt_tree`) passed. Ambient drift sits entirely in TypeScript runtime-local files outside the declared scope and is context only. No completion blockers, no scope drift, no invariant violations identified.

Attack log:
- `crates/runx-runtime/tests/receipt_tree.rs::runtime_tree_rejects_child_ref_without_digest_locator`: Spec compliance: verify the new test actually exercises the strict child proof path and asserts the documented digest-locator requirement. -> clean (Test nulls parent locator, refreshes root signature, asserts structural verify is valid, then asserts ChildReceiptDigestMismatch at runtime_receipts[0].locator via verify_runtime_receipt_tree, which routes through StrictChildProofPolicy.)
- `crates/runx-receipts/src/tree.rs::child_digest_link_findings`: Dark patterns: confirm None locator triggers the same finding code/path as a mismatched locator, and that the finding path matches the runtime resolver's path scheme. -> clean (`reference.locator.as_deref() == Some(child.seal.digest.as_str())` is false for None; finding is emitted with code ChildReceiptDigestMismatch at format!("{path}.locator"). With RuntimeReceiptResolver path `runtime_receipts[0]`, the emitted path is `runtime_receipts[0].locator`, matching the test assertion.)
- `crates/runx-runtime/src/receipt_tree.rs::verify_runtime_receipt_tree`: Regression hunt: ensure verify_runtime_receipt_tree still routes through the strict proof verifier (not legacy structural-only) so the new regression actually fires. -> clean (verify_runtime_receipt_tree -> verify_runtime_receipt_tree_with_policy(local_development) -> verify_receipt_tree_proof_with_resolver(... RuntimeReceiptProofContextProvider), which instantiates StrictChildProofPolicy via verify_tree_relationships_with_proof in receipts/src/tree.rs.)
- `crates/runx-runtime/tests/receipt_tree.rs (assert_finding helper)`: Convention check: verify the assertion helper does not accidentally pass on absent findings or wrong paths. -> clean (assert_finding scans verification.findings for an exact (code, path) match and panics with the full findings list on miss, mirroring the receipts-side helper.)
- `Task scope vs workspace changes`: Ambient drift / scope drift: confirm the four ambient-drift TypeScript files in runtime-local/orchestrator are unrelated to this Rust-only slice and that no out-of-scope Rust changes leak in. -> clean (Drift is limited to packages/runtime-local/src/runner-local/{graph-governance,kernel-bridge,orchestrator/handle-run-fanout,orchestrator/handle-run-step}.ts; spec scope is Rust receipt-tree files only and out-of-scope per spec text. Treated as context per provider instruction.)
- `crates/runx-receipts/src/verify/finding.rs::ReceiptFindingCode`: Public API stability: confirm the finding code enum was not silently broadened/renamed in a way that would break downstream pattern matches. -> clean (Enum includes ChildReceiptDigestMismatch among existing variants; no obvious renames. The new test reuses the existing variant, no new code introduced.)
- `crates/runx-runtime/tests/receipt_tree.rs::refresh_local_digest_and_signature`: Test-only logic vs production: confirm the digest/signature refresh stays inside tests and does not bypass real signing/verification in production. -> clean (Helper lives only in the integration test file and operates on local pseudo-signature scheme `sig:{digest}` consumed by RuntimeReceiptSignaturePolicy::local_development. No production code mutation.)

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
