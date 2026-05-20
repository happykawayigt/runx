---
spec_version: '2.0'
task_id: rust-runtime-test-coverage
created: '2026-05-21T03:00:00Z'
updated: '2026-05-21T03:00:00Z'
status: draft
harden_status: not_run
size: medium
risk_level: medium
---

# Rust runtime test coverage audit

## Current State

Status: draft
Current phase: planning
Next: design review
Reason: draft read-only audit of runtime test coverage gaps before launcher
promotion.
Blockers: validate the inventory against the current test tree before filing
follow-up executable test specs.
Allowed follow-up command: `scafld harden rust-runtime-test-coverage --provider <provider>`
Latest runner update: none
Review gate: not_started

## Why this exists

The `runx-runtime` crate has 36 integration test files covering ~600 named
tests. Coverage is uneven: some surfaces (`payment_*`, `mcp_*`, `receipt_*`)
have dense matrices; others (`dev/`, `scaffold/`, `harness/assertions`,
`registry/local::build`) have a single happy-path test. Per the rust-takeover
plan §11, the launcher flip needs a baseline coverage signal before
production soak. This spec produces that baseline and prioritizes the gaps.

This is a **read + report** spec, not a code change. The output is a
checklist that drives the next ~5 small "add tests for X" follow-up specs.

## Method

1. For each top-level runtime module, enumerate its integration test file(s)
   and the assertion surface they cover.
2. For each public function and public type, identify whether it's
   exercised in at least one assertion (positive case) and whether failure
   paths are exercised (negative case).
3. Rank the gaps by **production risk**, not by raw coverage percentage.

## Surfaces by current density

### Dense (no immediate action)

| Surface | Test files | Approximate cases |
| --- | --- | --- |
| `payment_*` | `payment_authority.rs`, `payment_execution.rs`, `payment_receipts.rs` | 38 |
| `mcp` adapter | `mcp_adapter.rs`, `mcp_server.rs` | 21 |
| `receipts` (store, tree, paths) | `receipt_store.rs`, `receipt_tree.rs`, `receipt_paths.rs` | 46 |
| `harness` (fixtures) | `harness_fixtures.rs`, `parity.rs` | 40+ |
| `runner.rs` (graph execution) | `hello_graph.rs`, `fanout_parity.rs`, `fanout_proptest.rs`, `parity.rs` | 80+ |
| `connect/*` | `connect_*.rs` (4 files) | 30+ |

### Thin (priority follow-ups)

| Surface | Current tests | Missing |
| --- | --- | --- |
| `dev::*` (run_dev_once, watch, presentation, tool) | `dev.rs` | watch debounce semantics; lane filtering; fixture executor failure paths; render theme variants |
| `scaffold::*` (init, new, templates) | `scaffold.rs` | template ids mismatch; ensure_install_state failure; ensure_project_state preserve+overwrite branches; packet namespace edge cases |
| `harness::assertions` | covered indirectly | direct unit tests for `assert_expectations` against every `HarnessExpectedStatus` and disposition mismatch |
| `registry::local::build` | `registry.rs`, `registry_client.rs` | direct unit tests for `build_registry_skill_version` happy path + missing-publisher; `normalize_registry_skill_version` round-trip on every source-type variant |
| `registry::local::trust` | covered indirectly | direct unit tests for each `*_trust_signal` against verified/declared/not_declared transitions |
| `journal::*` (projection + history filters) | `journal_history.rs` | HistoryFilter combinations (skill+status+source, since+until ranges, artifact_type intersection); empty-store behavior |
| `agent_invocation::*` | `agent_parity.rs` | resolution flow under needs-agent loop; idempotency key derivation; act-ref resolution fixtures |
| `target_runner::*` | `target_runner.rs` | runtime-side execution beyond contract fixture parity; readiness mismatch propagation; PR observation race conditions |
| `post_merge_observer::*` | `post_merge_observer.rs` | publication-from-receipt projection failure modes; runtime dedupe with stale receipt refs |
| `sandbox::*` | covered indirectly | direct unit tests for `prepare_mcp_process_sandbox` against every cwd policy; env allowlist intersection |
| `doctor::*` | `doctor.rs` | each diagnostic severity producing the right exit-code path; repair-confidence ordering |
| `list::*` | none directly | direct unit tests for every `RunxListItemKind` discovery path; ok-only vs invalid-only filtering |

### Untested

| Surface | Why it matters |
| --- | --- |
| `runner/payment.rs::OwnedReservedPaymentAuthority::as_borrowed` and the rest of the Owned* lifetime adapters | These bridge the borrowed PaymentRailAdmission contract to runtime ownership. Mishandled lifetimes here = silent corruption. |
| `runner/inputs.rs` typed-input helpers (`required_typed_input`, `optional_typed_vec_input`, etc.) | They produce `payment_authority_denied` errors with structured reasons. Currently covered by integration tests through full payment flows; no isolated unit. |
| `runner/sync.rs::receipt_strategy` + `receipt_decision` | Pure mappings; trivial but worth a single match-exhaustiveness unit test to lock the wire shape. |
| `dev::watch` change debounce | Time-sensitive; flaky integration test deferred is better than no test. |

## Priority

P1 (production risk): payment.rs lifetime adapters, agent_invocation resolution loop, target_runner runtime execution paths, post_merge_observer projection failure modes, harness::assertions direct units.

P2 (correctness): registry::local::{build,trust} direct units, journal HistoryFilter matrix, sandbox prepare_mcp_process_sandbox direct units, scaffold ensure_*_state branches.

P3 (hygiene): list direct units, doctor diagnostic-to-exit-code matrix, runner::sync exhaustiveness, dev render theme variants, runner::inputs typed-input units.

## Output of this spec

A follow-up filing (one issue or one focused spec per row above) that:

1. References the surface and the gap.
2. Names the file the test should live in (existing or new).
3. Estimates effort in test-cases-per-surface (most are <10 cases).

This spec produces only the ranked list; it does not write the tests.

## Non-goals

- Migrating to `cargo nextest` (deferred per `rust-kernel-architecture.md` §18).
- Coverage instrumentation (`tarpaulin`, `llvm-cov`): adds CI complexity for
  marginal signal once the matrix above is filled in.
- Property-based testing rollout beyond the existing `fanout_proptest.rs` and
  `policy_proptest.rs`. Adopt per-surface only when fixture coverage proves
  insufficient.

## References

- [`crates/runx-runtime/tests/`](../../crates/runx-runtime/tests/) — current
  integration test corpus
- [`oss/docs/rust-kernel-architecture.md`](../../docs/rust-kernel-architecture.md)
  §11 (property + differential testing)
- [`plans/rust-takeover.md`](../../../plans/rust-takeover.md) §11 (outreach
  gating, which assumes a passing test signal)
