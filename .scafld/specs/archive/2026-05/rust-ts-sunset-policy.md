---
spec_version: '2.0'
task_id: rust-ts-sunset-policy
created: '2026-05-18T00:00:00Z'
updated: '2026-05-20T03:35:01Z'
status: completed
harden_status: not_run
size: medium
risk_level: high
---

# TS sunset: policy

## Current State

Status: completed
Current phase: final
Next: done
Reason: task completed
Blockers: none
Allowed follow-up command: `none`
Latest runner update: 2026-05-20T03:35:01Z
Review gate: pass

## Summary

This draft is not deletion-ready. The first executable slice has landed:
runtime-local retry admission now crosses the Rust kernel eval boundary instead
of calling TS `admitRetryPolicy` directly. The bridge is deliberately narrow:
no-retry steps return locally without spawning Rust, and requested retry
admission requires `RUNX_KERNEL_EVAL_BIN` or an explicit command so runtime code
does not accidentally invoke a TS `runx` from `PATH`.

`packages/core/src/policy/` remains a published TS surface with active
runtime-local imports and parity fixture generation. The next safe work is a
separate graph-scope admission slice. Re-audit imports before reopening any
delete plan.

Rust policy ownership has progressed: `crates/runx-core/src/policy.rs` exports
local admission, retry admission, graph-scope admission, sandbox normalization
and admission, authority-proof metadata, credential binding, public-work policy,
and payment authority subset checks. That progress should replace TS callsites
incrementally. Do not add alternate TS policy shapes or temporary bridge models.

This does not delete contract schemas such as `runx.operational_policy.v1`;
those remain in `@runxhq/contracts` until the contract package has its own
approved Rust ownership path.

## Context

CWD: `.`

Packages:
- `@runxhq/core`
- `crates/runx-core`
- `crates/runx-runtime`
- Every TS package that imports from `@runxhq/core/policy`

Current TypeScript sources:
- `packages/core/src/policy/**` (still live)
- `packages/core/package.json` export `./policy` (still live)
- All TS importers of `@runxhq/core/policy`

Files impacted:
- Retry-admission slice:
  - `packages/runtime-local/src/runner-local/kernel-bridge.ts`
  - `packages/runtime-local/src/runner-local/kernel-bridge.test.ts`
  - `packages/runtime-local/src/runner-local/orchestrator/handle-run-step.ts`
  - `packages/runtime-local/src/runner-local/orchestrator/handle-run-fanout.ts`
  - `tests/graph-retry-idempotency.test.ts`
  - `tests/graph-fanout.test.ts`
- Not deletion-ready: no policy package files are approved for deletion by this
  draft.
- Future migration slices may touch `packages/runtime-local` importers and
  fixture-generation scripts, but those changes belong in separate approved
  slices.

Invariants:
- No policy decision regressions: cross-validated through receipt parity
  before and after.
- Operational policy fixtures and `runx policy inspect|lint` keep validating
  against the same schema/readback shape after the implementation moves.
- The authority-proof helpers in `packages/core/src/policy/authority-proof.ts`
  are only removed after callers use Rust `build_authority_proof*`,
  `build_local_scope_admission`, and `validate_credential_binding`.
- Package export removal only happens after `rg "@runxhq/core/policy"` finds no
  live consumers outside the policy package itself and fixture tooling has a
  Rust-backed replacement.
- Payment authority and runner preflight changes remain owned by the parent
  worker while that worktree is dirty.

## Objectives

- Keep the current importer/export inventory visible in the draft.
- Choose the smallest next migration slice:
  1. Completed: move `admitRetryPolicy` in
     `packages/runtime-local/src/runner-local/orchestrator/handle-run-step.ts`
     and `handle-run-fanout.ts` to Rust because Rust already exposes
     `runx_core::policy::admit_retry_policy`.
  2. Next candidate: graph-scope admission in
     `packages/runtime-local/src/runner-local/graph-governance.ts`, because the
     TS callsite is isolated and Rust kernel eval already supports
     `policy.admitGraphStepScopes`.
  3. Later: sandbox normalization/admission, local admission, credential
     binding, and authority proof only after exact receipt metadata parity is
     protected.
  4. Keep authority proof, public-work policy, and operational policy CLI
     readback on their current surfaces until their consumers are explicitly
     migrated and parity fixtures stay green.
- Re-audit imports after each slice before proposing package export removal.

## Scope

In scope:
- Planning and executing one policy decision migration slice at a time.
- Import audits for `@runxhq/core/policy`.
- The retry-admission bridge and tests listed above.

Out of scope:
- Deleting `packages/core/src/policy/`.
- Removing the `./policy` package export.
- Updating `scripts/check-boundaries.mjs` for policy removal.
- Payment runner preflight and payment execution changes currently being worked
  outside this spec.

## Dependencies

- Rust policy functions available from `crates/runx-core/src/policy.rs`.
- The Rust kernel eval bridge from `runx kernel eval --input - --json`.
- Existing kernel parity fixtures in `fixtures/kernel/policy` and
  `crates/runx-core/tests/policy_fixtures.rs` remain passing for each migrated
  decision.

## Open Questions

- Should the runtime-local Rust policy bridge stay process-based for graph-scope
  admission, or should that next slice wait for a native Rust runtime-local
  owner? Process JSON is acceptable for low-frequency admission, not for hot
  graph state-machine loops.
- Should fixture generation remain TS-authored until all policy callsites move,
  or should a Rust fixture generator become the next slice after retry
  admission?

## Validation

- `pnpm vitest run packages/runtime-local/src/runner-local/kernel-bridge.test.ts`
- `pnpm vitest run tests/graph-retry-idempotency.test.ts`
- `pnpm vitest run tests/graph-fanout.test.ts`
- `cargo test --manifest-path crates/Cargo.toml -p runx-cli --test kernel -- --nocapture`
- `cargo test --manifest-path crates/Cargo.toml -p runx-core --test kernel_eval -- --nocapture`
- `pnpm fixtures:kernel:check`
- `pnpm typecheck`

## Review

Status: completed
Verdict: pass
Mode: verify
Summary: Human-reviewed override accepted: retry-admission slice executed; focused TS/Rust bridge tests, graph retry/fanout tests, kernel eval tests, fixture check, typecheck, and subagent review issues addressed; full policy package deletion remains out of scope

Attack log:
- `review gate`: manual human audit -> clean (retry-admission slice executed; focused TS/Rust bridge tests, graph retry/fanout tests, kernel eval tests, fixture check, typecheck, and subagent review issues addressed; full policy package deletion remains out of scope)

Findings:
- none

