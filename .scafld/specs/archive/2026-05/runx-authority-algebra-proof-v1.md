---
spec_version: '2.0'
task_id: runx-authority-algebra-proof-v1
created: '2026-05-27T00:00:00Z'
updated: '2026-05-26T22:43:12Z'
status: completed
harden_status: not_run
size: small
risk_level: medium
---

# runx authority algebra proof v1

## Current State

Status: completed
Current phase: final
Next: done
Reason: task completed
Blockers: none
Allowed follow-up command: `none`
Latest runner update: 2026-05-26T22:43:12Z
Review gate: pass

## Summary

Make the checkable authority attenuation invariant less theatrical. The current
`authority_item_subset_is_transitive` property constructs `middle` and `child`
as subsequences of `parent`, so it proves mostly fixture construction. This
spec replaces that with an implication checked over independent candidates, and
adds negative properties for widening.

This is pure `runx-core` work. It must not touch runtime services, MCP, receipt
writer, TypeScript package deletion, or S-tier cutover files.

## Scope

- `crates/runx-core/tests/policy_proptest.rs`
- `crates/runx-core/src/policy/authority_algebra.rs` only if a missing pure
  helper is required.

Out of scope:

- `crates/runx-runtime/**`
- `crates/runx-receipts/**`
- TypeScript package sunset work.
- Active S-tier runtime cutover work.

## Objectives

- Replace tautological subset transitivity with independent generated values.
- Add property coverage for denied widening on item sets and optional bounds.
- Preserve the existing public authority algebra API.
- Keep the test readable and deterministic.

## Acceptance

- `cargo test --manifest-path crates/Cargo.toml -p runx-core --test policy_proptest authority_item_subset`
- `cargo test --manifest-path crates/Cargo.toml -p runx-core --test policy_proptest authority_optional_bounds`
- `cargo test --manifest-path crates/Cargo.toml -p runx-core policy::authority_algebra`
- `cargo fmt --manifest-path crates/Cargo.toml --all -- --check`

## Phase 1: Proof Strengthening

Status: completed
Dependencies: none

Objective: Complete this phase.

Changes:
- Replace `authority_item_subset_is_transitive` with a property whose `parent`, `middle`, and `child` candidates are generated independently and checked as the real implication: if `middle subset parent` and `child subset middle`, then `child subset parent`.
- Add a denied-widening property where a child contains at least one generated item missing from its parent.
- Add optional-bound properties for missing child bound and unbounded parent.

Acceptance:
- none

## Phase 2: Final Gate

Status: completed
Dependencies: phase1

Objective: Complete this phase.

Changes:
- Run the focused unit and formatting gates.

Acceptance:
- none

## Review

Status: completed
Verdict: pass
Mode: verify
Summary: Human-reviewed override accepted: Reviewed focused pure-kernel test change after all listed acceptance commands passed; no runtime or S-tier files touched.

Attack log:
- `review gate`: manual human audit -> clean (Reviewed focused pure-kernel test change after all listed acceptance commands passed; no runtime or S-tier files touched.)

Findings:
- none
