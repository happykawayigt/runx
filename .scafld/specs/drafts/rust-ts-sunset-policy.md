---
spec_version: '2.0'
task_id: rust-ts-sunset-policy
created: '2026-05-18T00:00:00Z'
updated: '2026-05-20T00:00:00Z'
status: draft
harden_status: not_run
size: medium
risk_level: high
---

# TS sunset: policy

## Current State

Status: draft
Current phase: refresh
Next: split into a migration slice; do not approve deletion yet.
Reason: current code still publishes and consumes `@runxhq/core/policy`.
Blockers:
- `packages/core/package.json` still exports `./policy`.
- `packages/core/src/policy/**` still contains public TS policy decisions,
  sandbox normalization, authority-proof helpers, public-work policy helpers,
  tests, and fixture generation inputs.
- Live importers remain in `packages/runtime-local` and scripts.
- Rust policy has progressed in `crates/runx-core/src/policy.rs` and submodules,
  but the TS runtime still calls TS policy exports in several execution paths.
- Payment runner preflight is active parent-agent work; do not overlap it from
  this draft.
Allowed follow-up command: none. Do not run `scafld harden`.
Latest runner update: refreshed against code on 2026-05-20.
Review gate: not_started

## Summary

This draft is not deletion-ready. `packages/core/src/policy/` remains a
published TS surface with active runtime-local imports and parity fixture
generation. The next safe work is to migrate one narrow policy decision path to
the existing Rust policy owner, then re-audit before reopening any delete plan.

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
- Not deletion-ready: no files are approved for deletion by this draft.
- Future migration slices may touch `packages/runtime-local` importers and
  fixture-generation scripts, but those changes belong in their own approved
  slice.

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
  1. Start with `admitRetryPolicy` in
     `packages/runtime-local/src/runner-local/orchestrator/handle-run-step.ts`
     and `handle-run-fanout.ts`, because Rust already exposes
     `runx_core::policy::admit_retry_policy`.
  2. After retry admission is Rust-owned, consider graph-scope or sandbox
     normalization/admission as the next separate slice.
  3. Keep authority proof, public-work policy, and operational policy CLI
     readback on their current surfaces until their consumers are explicitly
     migrated and parity fixtures stay green.
- Re-audit imports after each slice before proposing package export removal.

## Scope

In scope:
- Planning and executing one policy decision migration slice at a time.
- Import audits for `@runxhq/core/policy`.

Out of scope:
- Deleting `packages/core/src/policy/`.
- Removing the `./policy` package export.
- Updating `scripts/check-boundaries.mjs` for policy removal.
- Payment runner preflight and payment execution changes currently being worked
  outside this spec.

## Dependencies

- Rust policy functions available from `crates/runx-core/src/policy.rs`.
- A runtime call path that can use Rust policy results without reimplementing
  those decisions in TS.
- Existing kernel parity fixtures in `fixtures/kernel/policy` and
  `crates/runx-core/tests/policy_fixtures.rs` remain passing for each migrated
  decision.

## Open Questions

- What is the intended TS-to-Rust call boundary for runtime-local policy
  decisions during the transition?
- Should fixture generation remain TS-authored until all policy callsites move,
  or should a Rust fixture generator become the next slice after retry
  admission?
