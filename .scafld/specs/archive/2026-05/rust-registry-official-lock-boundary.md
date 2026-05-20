---
spec_version: '2.0'
task_id: rust-registry-official-lock-boundary
created: '2026-05-20T06:34:37Z'
updated: '2026-05-20T06:35:58Z'
status: completed
harden_status: not_run
size: small
risk_level: low
---

# rust-registry-official-lock-boundary

## Current State

Status: completed
Current phase: final
Next: done
Reason: task completed
Blockers: none
Allowed follow-up command: `none`
Latest runner update: 2026-05-20T06:35:58Z
Review gate: pass

## Summary

Remove the official skill lock path's dependency on the retired
`@runxhq/core/registry` surface without changing the public lock shape. The
doctor diagnostic and `scripts/generate-official-lock.mjs` only need the
lock tuple (`skill_id`, `version`, `digest`), so they derive that tuple from
the surviving parser/util boundaries instead of importing the registry package.

## Objectives

- Remove `@runxhq/core/registry` from the official lock generator.
- Remove `@runxhq/core/registry` from the doctor official-lock diagnostic.
- Keep `packages/cli/src/official-skills.lock.json` generated from the same
  strict skill/profile validation and version seed as the old registry builder.

## Scope

- In scope:
  - `scripts/generate-official-lock.mjs`
  - `packages/cli/src/commands/doctor-structure.ts`
  - `packages/cli/src/official-skills.lock.json`
- Out of scope:
  - Full registry store/search/publish migration.
  - SDK, skill-install, and CLI dispatch registry call sites owned by parallel
    slices.

## Dependencies

- `rust-ts-sunset-registry` completed the primary registry package sunset.
- The TS parser still exists; this slice intentionally depends on parser/util
  only until the parser sunset runs.

## Assumptions

- The official lock only needs the stable tuple used by packaged first-party
  skill fetches; it does not need registry store/search/publish behavior.

## Touchpoints

- If the version seed logic drifts, official first-party skill fetches can
  reject packaged skills. The acceptance recomputes the lock through the script
  and keeps the lockfile checked in.

## Risks

- none

## Acceptance

Profile: standard

Validation:
- `node scripts/generate-official-lock.mjs`
- `! rg -n '@runxhq/core/registry' packages/cli/src/commands/doctor-structure.ts scripts/generate-official-lock.mjs`

## Phase 1: Implementation

Status: active
Dependencies: none

Objective: Complete the requested change.

Changes:
- Inline the official-lock tuple derivation in the doctor diagnostic and lock generator using parser/util boundaries.
- Refresh the checked-in lock after recomputation.

Acceptance:
- [ ] `ac1` command - Official lock recomputes successfully
  - Command: `node scripts/generate-official-lock.mjs`
  - Expected kind: `exit_code_zero`
  - Status: passed
- [ ] `ac2` command - Official lock callers no longer import registry
  - Command: `! rg -n '@runxhq/core/registry' packages/cli/src/commands/doctor-structure.ts scripts/generate-official-lock.mjs`
  - Expected kind: `exit_code_zero`
  - Status: passed

## Rollback

- Revert the two caller edits and restore the previous lockfile contents.

## Review

Status: completed
Verdict: pass
Mode: verify
Summary: Human-reviewed override accepted: Local review: official-lock callers now derive only skill_id/version/digest from parser/util, script recomputes lock successfully, and targeted registry-import grep is clean; broader typecheck is deferred because parallel SDK slice is mid-edit.

Attack log:
- `review gate`: manual human audit -> clean (Local review: official-lock callers now derive only skill_id/version/digest from parser/util, script recomputes lock successfully, and targeted registry-import grep is clean; broader typecheck is deferred because parallel SDK slice is mid-edit.)

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
