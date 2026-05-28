---
spec_version: '2.0'
task_id: runtime-large-file-decomposition
created: '2026-05-28T23:55:00Z'
updated: '2026-05-28T23:55:00Z'
status: draft
harden_status: not_run
size: large
risk_level: medium
---

# Runtime large-file decomposition

## Current State

Status: draft
Next: pick first target (target_runner.rs)
Reason: A-cutover campaign (Steps 1-7 shipped through `c553caca`) closed the
provider-lockin and open-id work in contracts. The remaining grade-relevant
issue is the runtime has five 40-80KB single files that each carry a
`rust-style-allow: large-file` reason. Decomposing them into well-named
submodules raises the runtime from B to A without changing behavior, but it
must be done as a focused per-file campaign with parity gates because the
files are receipts/sealing hot paths.

## Summary

Split the five runtime giants into per-concern submodules. Each split is one
commit, gated by `cargo test --workspace` plus the receipts oracle so behavior
is preserved bit-for-bit. No public-surface changes.

## Targets

By size, worst first:

| File | LOC | Concerns (proposed submodules) |
|---|---|---|
| `crates/runx-runtime/src/execution/target_runner.rs` | 81KB | commands, execution, validation, mutation, revision, source_publication, projection |
| `crates/runx-runtime/src/execution/harness/runner.rs` | 50KB | env, signing, fixture, runner |
| `crates/runx-runtime/src/execution/runner/steps.rs` | 44KB | step kinds, evaluation, output projection |
| `crates/runx-core/src/policy/payment_authority.rs` | 41KB | bounds, capability check, attenuation |
| `crates/runx-runtime/src/adapters/external_adapter.rs` | 40KB | transport, manifest, invocation, response |

## Why a separate campaign

1. Each file is a receipt-sealing hot path. Cross-function call graphs are
   dense; a clean split needs careful `pub(super)` decisions to avoid leaking
   internals to the parent.
2. Workspace-level behavioral tests must pass at each split. The cost of a
   subtle visibility regression that compiles but breaks idempotency is high.
3. The existing `rust-style-allow: large-file` reasons document why these
   stayed as single files. Splitting them without losing that context means
   each submodule needs a short header explaining its slice of the invariants.

## Phase plan

**Phase 1 — `target_runner.rs` (81KB → ~8 files)**
- Create `crates/runx-runtime/src/execution/target_runner/` directory
- Move public-surface re-exports to `mod.rs`
- Move struct definitions to `commands.rs` (~280 lines)
- Move execute fns + checkout + dedupe lookup to `execution.rs` (~175 lines)
- Move PR observation + mutation + readback validation to `pull_request.rs` (~450 lines)
- Move git mutation + revision receipt to `revision.rs` (~400 lines)
- Move source publication path to `source_publication.rs` (~470 lines)
- Move sealed-receipt projection to `projection.rs` (~200 lines)
- Move adapter trait + errors to `adapter.rs` (~150 lines)
- Verify: `cargo test --workspace`, `pnpm test:fast`, receipts c14n oracle unchanged

**Phase 2 — `harness/runner.rs` (50KB)**
- Submodules: `env.rs`, `signing.rs`, `fixture.rs`, leaving runner orchestration in `mod.rs`

**Phase 3 — `runner/steps.rs` (44KB)**
- Submodules per step-kind family (skill, agent, decision, tool)

**Phase 4 — `core/policy/payment_authority.rs` (41KB)**
- Submodules: `bounds.rs`, `capability.rs`, `attenuation.rs`

**Phase 5 — `adapters/external_adapter.rs` (40KB)**
- Submodules: `transport.rs`, `manifest.rs`, `invocation.rs`, `response.rs`

## Parity gates

For each phase:
1. `cargo build --workspace --all-targets` green before and after.
2. `cargo test --workspace --tests` shows the same passed-count and zero
   failures.
3. `pnpm test:fast` green.
4. Receipt c14n oracle (`fixtures/contracts/canonical-json/runx-receipt-c14n-v1.oracles.json`)
   bit-identical — no receipt body changed.
5. `node scripts/check-integration-test-modules.mjs` green.
6. `cargo clippy --workspace --all-targets --all-features -- -D warnings` green.

## Out of scope

- Behavioral changes (this is pure mechanical decomposition).
- Trait shape changes.
- New tests (existing coverage is the parity gate).
- Phase 6+ (registry/adapter dispatch indirection — separate campaign).

## Related

- A-cutover campaign closed at commit `c553caca` (2026-05-28).
- Companion follow-up: `tool-manifest-named-emits-schema-drift` (the doctor
  cleanup gap discovered during A1).
