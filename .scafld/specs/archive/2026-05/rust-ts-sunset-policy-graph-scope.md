---
spec_version: '2.0'
task_id: rust-ts-sunset-policy-graph-scope
created: '2026-05-20T03:44:14Z'
updated: '2026-05-20T03:45:42Z'
status: completed
harden_status: not_run
size: medium
risk_level: medium
---

# Rust policy graph-scope bridge

## Current State

Status: completed
Current phase: final
Next: done
Reason: task completed
Blockers: none
Allowed follow-up command: `none`
Latest runner update: 2026-05-20T03:45:42Z
Review gate: pass

## Summary

Move runtime-local graph step scope admission from the TS policy implementation
to Rust kernel eval. This is the second narrow `rust-ts-sunset-policy` slice
after retry admission.

The bridge is intentionally constrained:
- steps with no requested scopes stay local and do not require a Rust process;
- scoped steps call Rust `policy.admitGraphStepScopes`;
- missing or malformed Rust kernel results fail closed into signed
  `policy_denied` harness receipts, not unsealed thrown runtime errors;
- `packages/core/src/policy/**` and the `@runxhq/core/policy` export remain
  live because local admission, sandbox, authority proof, public-work policy,
  fixture generation, and type consumers have not moved.

## Objectives

- Add a typed graph-scope wrapper to
  `packages/runtime-local/src/runner-local/kernel-bridge.ts`.
- Replace runtime-local production calls to TS `admitGraphStepScopes` with the
  Rust kernel eval bridge.
- Preserve receipt metadata shape for allowed and denied graph scope admission.
- Keep unscoped graph execution independent of the Rust binary.
- Fail closed with a signed denial receipt when the Rust kernel command is
  missing or malformed.

## Scope

- `packages/runtime-local/src/runner-local/kernel-bridge.ts`
- `packages/runtime-local/src/runner-local/kernel-bridge.test.ts`
- `packages/runtime-local/src/runner-local/graph-governance.ts`
- `packages/runtime-local/src/runner-local/orchestrator/handle-run-step.ts`
- `packages/runtime-local/src/runner-local/orchestrator/handle-run-fanout.ts`
- Graph governance, retry, fanout, and auth-security tests required to prove
  the bridge.

Out of scope:
- deleting `packages/core/src/policy/**`;
- removing the `@runxhq/core/policy` export;
- migrating sandbox, local admission, authority proof, public-work policy, or
  fixture generation;
- using process JSON for hot state-machine transition loops.

## Dependencies

- Completed Rust kernel eval bridge: `runx kernel eval --input - --json`.
- Rust policy owner for `policy.admitGraphStepScopes` in `runx-core`.
- Existing kernel parity fixtures under `fixtures/kernel/policy`.

## Assumptions

- Process JSON is acceptable for low-frequency admission gates. It is not the
  target shape for the graph state-machine hot loop.
- A missing Rust kernel command is an infrastructure failure that must fail
  closed with a sealed graph receipt at runtime boundaries.

## Touchpoints

- `packages/runtime-local` graph orchestration
- Rust kernel eval fixtures/tests
- Harness receipt governance metadata

## Risks

- Requiring the Rust binary for every graph step would regress local graph
  ergonomics. Mitigation: no-scope graph steps short-circuit locally.
- Letting bridge failures throw would produce unsealed governance failures.
  Mitigation: orchestrators convert admission bridge failures to policy denial
  receipts.

## Acceptance

Profile: standard

Validation:
- `pnpm vitest run packages/runtime-local/src/runner-local/kernel-bridge.test.ts`
- `pnpm vitest run tests/graph-retry-idempotency.test.ts`
- `pnpm vitest run tests/graph-runner-governance.test.ts`
- `pnpm vitest run tests/runtime-local-auth-security.test.ts`
- `pnpm vitest run packages/runtime-local/src/runner-local/kernel-bridge.test.ts tests/graph-receipt-governance.test.ts tests/graph-fanout.test.ts`
- `pnpm typecheck`
- `cargo test --manifest-path crates/Cargo.toml -p runx-core --test policy_fixtures --test kernel_eval -- --nocapture`
- `cargo test --manifest-path crates/Cargo.toml -p runx-cli --test kernel -- --nocapture`
- `pnpm fixtures:kernel:check`
- `git diff --check` for touched files

## Phase 1: Implementation

Status: completed
Dependencies: none

Objective: Migrate graph-scope admission to Rust kernel eval without breaking
unscoped graph execution.

Changes:
- Added `admitGraphStepScopesViaKernel` with strict output parsing and a local
  no-scope fast path.
- Made `buildGraphStepGovernance` async and Rust-backed.
- Updated sequential and fanout graph handlers to await Rust-backed scope
  admission and to fail closed for retry admission bridge failures.
- Added signed-receipt regression coverage for missing Rust kernel command on
  scoped graph admission and retry admission.

Acceptance:
- [x] `ac1` command - focused validation commands listed above
  - Command: see Validation
  - Expected kind: `exit_code_zero`
  - Status: passed

## Rollback

- Revert this slice and the previous retry bridge only as a unit if runtime
  cannot depend on Rust kernel eval. Do not add a TS compatibility shim inside
  runtime-local; reopen the policy sunset sequencing instead.

## Review

Status: completed
Verdict: pass
Mode: verify
Summary: Human-reviewed override accepted: graph-scope admission slice executed; scoped/unscoped behavior verified; missing Rust kernel fails closed into signed receipts; focused runtime, TS, Rust kernel, fixture, typecheck, and diff checks passed; full policy package deletion remains out of scope

Attack log:
- `review gate`: manual human audit -> clean (graph-scope admission slice executed; scoped/unscoped behavior verified; missing Rust kernel fails closed into signed receipts; focused runtime, TS, Rust kernel, fixture, typecheck, and diff checks passed; full policy package deletion remains out of scope)

Findings:
- none

## Self Eval

- Scoped graph admission now uses the Rust owner. Full policy sunset is still
  incomplete and must remain blocked until the remaining TS policy surfaces are
  migrated.

## Deviations

- This spec was captured after implementation started because the parent
  followed the next executable policy slice immediately after completing the
  retry-admission slice. The scafld ledger is being made explicit before review
  and completion.

## Metadata

- created_by: scafld

## Origin

Created by: scafld
Source: plan

## Harden Rounds

- none

## Planning Log

- none
