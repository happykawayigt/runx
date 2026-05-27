---
spec_version: '2.0'
task_id: runx-runtime-test-gate-dx-v1
created: '2026-05-27T12:58:25Z'
updated: '2026-05-27T14:08:03Z'
status: completed
harden_status: passed
size: large
risk_level: medium
---

# Runtime test and verification gate DX

## Current State

Status: completed
Current phase: final
Next: done
Reason: task completed
Blockers: none
Allowed follow-up command: `none`
Latest runner update: 2026-05-27T14:08:03Z
Review gate: pass

## Summary

Make the Rust runtime test surface and `verify:fast` gate behave like trustworthy
developer tools instead of wrapper-dependent orchestration. Runtime integration
tests must self-provision their fixture signing context and eval binaries when
run directly through Cargo/nextest. The boundary check must scan source, not
stale built artifacts. The fast verification script should preserve signal from
independent checks instead of hiding later failures behind the first red step.

This is a clean infrastructure cutover: no compatibility shims, no fallback test
modes, and no weakening of gates. The existing active
`runx-rust-95-release-readiness` spec is explicitly out of scope.

## Objectives

- Make `runx-runtime` tests that need production-like signing or eval binaries
  self-provision through shared test support, so direct Cargo/nextest invocations
  are understandable and reproducible.
- Harden `scripts/check-boundaries.mjs` against stale `.build/` artifacts and
  emit source-owned findings only.
- Refactor `scripts/verify-fast.mjs` so independent JS/Rust/package checks report
  as separate steps and continue where safe, while preserving a nonzero final
  exit if any required check fails.
- Record the concurrent-agent operational rule in the spec result: worktree
  isolation is recommended for separate agents, but this task uses targeted edits
  in the current checkout because the operator explicitly allowed collisions.
- Audit the cold compile floor and doctest opportunity only far enough to avoid
  accidental dependency broadening; do not perform a dependency swap inside this
  spec.

## Scope

- In scope:
  - `crates/runx-runtime/tests/support.rs` and runtime tests that should consume
    shared fixture signing/runtime helpers.
  - `scripts/check-boundaries.mjs` and focused boundary regression coverage.
  - `scripts/verify-fast.mjs`, limited to check orchestration/reporting.
  - Package scripts needed to expose the new focused checks.
  - Minimal docs/spec notes only when they describe new commands or gates.
- Out of scope:
  - `.scafld/specs/active/runx-rust-95-release-readiness.md`.
  - Changing product behavior, receipt schemas, harness spine contracts, or
    runtime architecture ownership.
  - Replacing reqwest/tokio/rustls/rmcp or changing feature defaults.
  - Merging OSS/cloud repository boundaries or renaming `X.yaml`.
  - Broad decomposition of runtime modules.

## Dependencies

- `test-surface-build-consolidation` remains active and owns the larger CI
  consolidation story. This spec fixes the observed 11 runtime-test
  self-provisioning failures underneath that plan.
- Existing dirty files may be edited with targeted patches, per operator
  instruction. Do not revert or stage unrelated changes.

## Assumptions

- Direct `cargo test` or `cargo nextest` on `runx-runtime` should not require the
  JS `verify:fast` wrapper to inject signing keys or binary paths.
- Test fixture signing keys are non-secret deterministic test material and must
  remain confined to test support.
- Boundary checks should ignore generated/built output directories regardless of
  whether a cache restores them before the build step.

## Touchpoints

- `crates/runx-runtime/tests/support.rs`
- `crates/runx-runtime/tests/{skill_run,skill_issue_intake,skill_issue_to_pr,local_credential_provision,hello_graph,mcp_server}.rs`
- `scripts/check-boundaries.mjs`
- `scripts/verify-fast.mjs`
- `package.json`
- focused tests for the boundary script, if absent

## Risks

- Test support can accidentally mask production signing enforcement if applied
  inside production code. Mitigation: keep helpers under `crates/runx-runtime/tests`.
- A fan-out verification script can overload the Rust linker/eval binary if it
  parallelizes heavy gates. Mitigation: only fan out light independent JS checks;
  keep Rust binary builds and Rust-heavy checks serialized.
- Boundary script fixtures can become another stale mirror. Mitigation: test the
  behavior by creating temporary source/build files during the test, not by
  checking in generated artifacts.

## Acceptance

Profile: strict

Validation:
- `cargo nextest run --manifest-path crates/Cargo.toml -p runx-runtime --all-features --test integration -- skill_run local_credential_provision skill_issue_intake skill_issue_to_pr`
- `cargo test --manifest-path crates/Cargo.toml -p runx-runtime --all-features --test integration -- skill_run local_credential_provision skill_issue_intake skill_issue_to_pr`
- `pnpm boundary:check`
- focused boundary regression test
- `pnpm verify:fast`
- `scafld review runx-runtime-test-gate-dx-v1 --provider claude`

## Phase 1: Runtime test self-provisioning

Status: completed
Dependencies: none

Objective: Runtime tests that need signing/eval context pass under direct Cargo

Changes:
- Add shared runtime test helpers for fixture signing env, signed runtime options, and eval binary discovery/provisioning if a test needs a binary path.
- Replace local duplicated signing env construction in runtime tests with shared helpers.
- Keep the one negative test that checks missing production signing env explicit; do not globally inject signing into process env.

Acceptance:
- [x] `p1_ac1` command - Runtime nextest subset self-provisions.
  - Command: `cargo nextest run --manifest-path crates/Cargo.toml -p runx-runtime --all-features --test integration -- skill_run local_credential_provision skill_issue_intake skill_issue_to_pr`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-6
- [x] `p1_ac2` command - Runtime cargo-test subset self-provisions.
  - Command: `cargo test --manifest-path crates/Cargo.toml -p runx-runtime --all-features --test integration -- skill_run local_credential_provision skill_issue_intake skill_issue_to_pr`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-7

## Phase 2: Boundary source scan hardening

Status: completed
Dependencies: phase1

Objective: Boundary checks report source-owned violations and ignore restored

Changes:
- Keep `.build`, `dist`, `target`, and `target-*` ignored by boundary walks.
- Add a focused regression test that creates a forbidden term under `.build/` and verifies `boundary:check` ignores it while still rejecting the same term under an active source root.
- Improve any failure text needed to point at the owning source file.

Acceptance:
- [x] `p2_ac1` command - Boundary check passes in the real workspace.
  - Command: `pnpm boundary:check`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-12
- [x] `p2_ac2` command - Boundary build-artifact regression passes.
  - Command: `pnpm test:boundary`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-13

## Phase 3: verify:fast signal fan-out

Status: completed
Dependencies: phase2

Objective: `verify:fast` keeps independent check results visible and only

Changes:
- Refactor `scripts/verify-fast.mjs` into named steps with a final summary.
- Run safe independent JS checks with parallel reporting where they do not share generated output or heavy Rust linker work.
- Keep Rust binary builds and Rust-heavy checks serialized.
- Continue executing independent checks after a failure when safe; exit nonzero at the end if any required step failed.

Acceptance:
- [x] `p3_ac1` command - Fast gate passes with the new orchestrator.
  - Command: `pnpm verify:fast`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-23
- [x] `p3_ac2` command - Verify package and Rust-heavy checks stay serialized.
  - Command: `pnpm verify:fast:plan-check`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-24

## Phase 4: Review and completion

Status: completed
Dependencies: phase3

Objective: Record evidence and run the requested Claude review gate.

Changes:
- none

Acceptance:
- [x] `p4_ac1` command - Rust style remains green.
  - Command: `pnpm rust:style`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-35
- [x] `p4_ac2` command - TypeScript remains green.
  - Command: `pnpm typecheck`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-36

## Rollback

- Revert the test-support helper changes and the tests that consume them.
- Restore `scripts/verify-fast.mjs` to the prior serial command loop if the new
  orchestrator hides output or causes false failures.
- Remove the boundary regression test and script changes if it proves flaky.

## Review

Status: completed
Verdict: pass
Mode: discover
Provider: claude:claude-opus-4-7
Output: claude.mcp_submit_review
Summary: Reviewed runtime test self-provisioning, boundary source-scan hardening, and verify:fast fan-out. Test self-provisioning helpers in support.rs are correctly scoped to tests, consumed by all in-scope test files, and the missing-signing-env negative test still bypasses the helper (skill_run.rs:355). verify-fast.mjs serializes Rust builds and rust/package contract groups while only fanning out source-only JS checks; the plan-check enforces that. All acceptance criteria pass per recorded evidence. One medium non-blocking finding: the new boundary regression test uses a .js file under top-level .build/, which is invisible to every walker regardless of the .build ignore, so it does not actually catch a regression that removes .build from ignoredDirectoryNames.

Attack log:
- `crates/runx-runtime/tests/support.rs`: Confirm shared helpers expose signing env, signature config, signed RuntimeOptions, and harness RuntimeOptions; verify they import the canonical RUNX_RECEIPT_SIGN_* env name constants from runx_runtime rather than redefining strings. -> clean (Constants imported from runx_runtime (support.rs:7-10). Seed/kid/issuer are deterministic test material kept under tests/. cli-tool gating matches consumer cfgs.)
- `crates/runx-runtime/tests/skill_run.rs`: Verify every run goes through the helper while the negative test still skips it; confirm production signing test does not silently inherit injected env. -> clean (run_skill -> with_test_signing_env -> insert_test_signing_env using or_insert preserves explicit env (skill_run.rs:1249-1259). The negative test at 351-374 calls LocalOrchestrator.run_skill directly with empty env, bypassing the helper.)
- `crates/runx-runtime/tests/{skill_issue_intake,skill_issue_to_pr,hello_graph,mcp_server,local_credential_provision}.rs`: Check each in-scope test self-provisions via support.* and does not reach into process env or the verify:fast wrapper. -> clean (All use crate::support helpers (local_harness_runtime_options, signed_runtime_options, test_signing_env, insert_test_signing_env). mcp_server.rs:655 extends spawned-child env explicitly; no reliance on ambient env.)
- `scripts/check-boundaries.mjs`: Trace ignore behavior for .build, dist, target, and target-* across all three walkers; verify scan-root scope and RUNX_BOUNDARY_WORKSPACE_ROOT override. -> clean (ignoredDirectoryNames + name.startsWith('target-') applied in walk(), walkActiveTypeScriptJavaScript, walkActiveCredentialContract. RUNX_BOUNDARY_WORKSPACE_ROOT is the documented test hook.)
- `scripts/test-boundaries.mjs`: Confirm regression test would fail if .build were removed from ignoredDirectoryNames. -> finding (See F1 — fixture file is .js under top-level .build/, outside all scan roots, so the test is structurally insensitive to the ignore-list invariant it advertises.)
- `scripts/verify-fast.mjs`: Check that independent JS checks fan out while Rust builds and rust/package-contract groups stay serial; verify printSummaryAndExit fails closed on any nonzero step. -> clean (source-check group runs boundary:check, test:boundary, typecheck, integration module guard in parallel via runParallelGroup; package and rust groups via runSerialGroup; cli/oracle builds awaited sequentially; eval-binary-dependent group is skipped only when a required Rust binary fails (build failure still recorded in results, so exit code is nonzero). printSummaryAndExit exits 1 on any failure.)
- `scripts/check-verify-fast-plan.mjs`: Confirm plan-check rejects heavy steps from the parallel source group and requires serial-group markers. -> clean (Forbidden tokens cover authoring/create-skill package contract, rust:crate-graph, rust:style, both Rust binary builds, and test:fast; required tokens enforce the serial-group structure.)
- `package.json`: Verify new scripts (test:boundary, verify:fast:plan-check) are wired to the right files and no unused/dead entries were left behind. -> clean
- `ambient drift`: Confirm baseline-dirty deletions of AGENTS.md, CLAUDE.md, CONVENTIONS.md, README.md, docs/api-surface.md, packages/core/package.json are unrelated to this task and not within scope. -> clean (All listed paths are outside the declared touchpoints; treated as context per provider instruction.)

Findings:
- [medium/non-blocking] `F1` Boundary regression test does not actually validate the .build ignore-list invariant it claims to defend.
  - Location: `scripts/test-boundaries.mjs:18`
  - Evidence: The first phase creates ${fixtureRoot}/.build/runtime/cached.js with a forbidden term (authorize_url) and expects boundary:check to ignore it. But check-boundaries.mjs only reaches that file through walk() (in findSourceFiles) — which filters by sourceExtensions = {.ts,.tsx,.mts,.cts} at scripts/check-boundaries.mjs:713 — or through walkActiveTypeScriptJavaScript / walkActiveCredentialContract, both of which only descend the named scan roots (packages, plugins, scripts, tests, fixtures/contracts, schemas, crates/*/src, crates/*/tests). A top-level .build/ directory is not in any scan root, and the file extension is .js, so the file is invisible regardless of whether .build appears in ignoredDirectoryNames. Removing ".build" from the ignore set in check-boundaries.mjs:34-42 would not make this test fail.
  - Impact: The phase 2 acceptance test passes but does not protect the ignore-list invariant. A future refactor that drops .build (e.g., to scan declaration files emitted to .build/**/cached.d.ts) would silently pass test:boundary while regressing the real check, because walk() does pick up .d.ts files (path.extname('foo.d.ts') === '.ts').
  - Validation: After changing the fixture file extension/location, manually remove ".build" from ignoredDirectoryNames in check-boundaries.mjs and confirm pnpm test:boundary now fails; restore the ignore entry and confirm the test passes again.

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

### round-1

Status: passed
Started: 2026-05-27T12:59:34Z
Ended: 2026-05-27T13:00:28Z

Checks:
- path audit
  - Grounded in: code:crates/runx-runtime/tests/support.rs:15
  - Result: passed
  - Evidence: Runtime fixture signing helpers already exist in test support;
- command audit
  - Grounded in: code:scripts/verify-fast.mjs:12
  - Result: passed
  - Evidence: `verify:fast` currently owns Rust binary prebuild and env
- scope/migration audit
  - Grounded in: code:scripts/check-boundaries.mjs:33
  - Result: passed
  - Evidence: Boundary scanning already ignores `.build`, `dist`, and `target`;
- acceptance timing audit
  - Grounded in: spec_gap:acceptance
  - Result: passed
  - Evidence: The active test-surface spec records the 11 runtime failures as
- rollback/repair audit
  - Grounded in: spec_gap:rollback
  - Result: passed
  - Evidence: Rollback is per surface: test support, boundary regression, and
- design challenge
  - Grounded in: code:crates/runx-runtime/src/execution/runner.rs:66
  - Result: passed
  - Evidence: Production runtime still reads signing config from explicit env;

Issues:
- none


## Planning Log

- none
