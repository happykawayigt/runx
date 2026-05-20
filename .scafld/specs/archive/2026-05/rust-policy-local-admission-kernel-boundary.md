---
spec_version: '2.0'
task_id: rust-policy-local-admission-kernel-boundary
created: '2026-05-20T07:11:15Z'
updated: '2026-05-20T07:20:49Z'
status: completed
harden_status: not_run
size: medium
risk_level: medium
---

# rust-policy-local-admission-kernel-boundary

## Current State

Status: completed
Current phase: final
Next: done
Reason: task completed
Blockers: none
Allowed follow-up command: `none`
Latest runner update: 2026-05-20T07:20:49Z
Review gate: pass

## Summary

Move runtime-local single-skill admission, scope admission, credential binding,
and authority-proof metadata generation from `@runxhq/core/policy` to the Rust
kernel eval bridge. This is the final live runtime-local policy package edge
after the graph-governance and sandbox slices.

## Objectives

- Add typed kernel bridge wrappers for `policy.admitLocalSkill`,
  `policy.buildLocalScopeAdmission`, and `policy.validateCredentialBinding`.
- Reuse `authorityProofMetadataViaKernel` for top-level skill receipts.
- Remove the `@runxhq/core/policy` import from `runner-local/index.ts`.
- Preserve structural admission ordering, connected-grant behavior, credential
  binding, graph authority inheritance, and denied receipt metadata.

## Scope

- In scope:
  - `packages/runtime-local/src/runner-local/kernel-bridge.ts`
  - `packages/runtime-local/src/runner-local/kernel-bridge.test.ts`
  - `packages/runtime-local/src/runner-local/index.ts`
  - focused local skill/auth/receipt tests
- Out of scope:
  - state-machine cutover
  - deleting `packages/core/src/policy/**`
  - removing package exports
  - graph-governance code already migrated

## Dependencies

- Rust kernel command provisioning for test harnesses.
- Existing Rust kernel eval operations for local admission, scope admission,
  credential binding, and authority-proof metadata.

## Assumptions

- Workspace tests provide `RUNX_KERNEL_EVAL_BIN`; published runtime-local code
  still fails closed unless callers provide a kernel command or env var.

## Touchpoints

- Local skill execution policy denial and admission.
- Connected auth grant and credential binding receipts.
- Nested graph skill receipt metadata.

## Risks

- Risk: structural admission loses `skipConnectedAuth` or
  `skipSandboxEscalation`. Mitigation: focused auth/security tests.
- Risk: graph parent `authorityScopeAdmission` is used for connected-auth
  skills. Mitigation: derive no-connected-auth from local scope admission
  shape before choosing inherited authority scope.
- Risk: kernel unavailability changes ordinary tests. Mitigation: dependency
  on runtime kernel command provisioning.

## Acceptance

Profile: standard

Validation:
- `pnpm test -- packages/runtime-local/src/runner-local/kernel-bridge.test.ts tests/runtime-local-auth-security.test.ts tests/local-skill-runner.test.ts tests/cli-tool-sandbox.test.ts tests/approval-receipts.test.ts tests/receipt-governance-schema-contract.test.ts`
- `cargo test --manifest-path crates/Cargo.toml -p runx-core --test policy_fixtures --test kernel_eval -- --nocapture`
- `pnpm fixtures:kernel:check`
- `pnpm typecheck`
- `! rg -n '@runxhq/core/policy|admitLocalSkill|buildLocalScopeAdmission|connectedAuthRequirement|validateCredentialBinding|buildAuthorityProofMetadata' packages/runtime-local/src/runner-local/index.ts`
- `git diff --check -- packages/runtime-local/src/runner-local/kernel-bridge.ts packages/runtime-local/src/runner-local/kernel-bridge.test.ts packages/runtime-local/src/runner-local/index.ts .scafld/specs/active/rust-policy-local-admission-kernel-boundary.md`

## Phase 1: Implementation

Status: completed
Dependencies: none

Objective: Route top-level local policy decisions through Rust kernel eval.

Changes:
- Add local admission, local scope admission, and credential binding bridge wrappers and strict parsers.
- Replace top-level skill policy helpers with awaited Rust bridge calls.
- Replace `connectedAuthRequirement` with the already-produced scope admission shape: inherited graph authority scope is used only for the canonical no-connected-auth admission.

Acceptance:
- [x] `ac1` command - Focused local policy/runtime tests pass
  - Command: `pnpm test -- packages/runtime-local/src/runner-local/kernel-bridge.test.ts tests/runtime-local-auth-security.test.ts tests/local-skill-runner.test.ts tests/cli-tool-sandbox.test.ts tests/approval-receipts.test.ts tests/receipt-governance-schema-contract.test.ts`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-6
- [x] `ac2` command - Rust policy/kernel tests pass
  - Command: `cargo test --manifest-path crates/Cargo.toml -p runx-core --test policy_fixtures --test kernel_eval -- --nocapture`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-7
- [x] `ac3` command - Kernel fixtures stay fresh
  - Command: `pnpm fixtures:kernel:check`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-8
- [x] `ac4` command - Typecheck passes
  - Command: `pnpm typecheck`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-9
- [x] `ac5` command - runner-local index no longer imports or calls TS policy helpers
  - Command: `! rg -n '@runxhq/core/policy|admitLocalSkill|buildLocalScopeAdmission|connectedAuthRequirement|validateCredentialBinding|buildAuthorityProofMetadata' packages/runtime-local/src/runner-local/index.ts`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-10
- [x] `ac6` command - Diff has no whitespace errors
  - Command: `git diff --check -- packages/runtime-local/src/runner-local/kernel-bridge.ts packages/runtime-local/src/runner-local/kernel-bridge.test.ts packages/runtime-local/src/runner-local/index.ts .scafld/specs/active/rust-policy-local-admission-kernel-boundary.md`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-11

## Rollback

- Restore the prior `@runxhq/core/policy` import and helper calls in
  `runner-local/index.ts`, and remove the added bridge wrappers.

## Review

Status: completed
Verdict: pass
Mode: verify
Summary: Human-reviewed override accepted: Acceptance commands passed: focused local policy/runtime tests, Rust kernel policy tests, kernel fixture check, typecheck, import grep, and diff check. Denied approval receipt assertion was updated to current harness receipt shape; no blockers found.

Attack log:
- `review gate`: manual human audit -> clean (Acceptance commands passed: focused local policy/runtime tests, Rust kernel policy tests, kernel fixture check, typecheck, import grep, and diff check. Denied approval receipt assertion was updated to current harness receipt shape; no blockers found.)

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
