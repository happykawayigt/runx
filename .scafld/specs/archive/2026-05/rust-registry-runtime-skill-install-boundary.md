---
spec_version: '2.0'
task_id: rust-registry-runtime-skill-install-boundary
created: '2026-05-20T06:42:53Z'
updated: '2026-05-20T06:45:26Z'
status: completed
harden_status: not_run
size: medium
risk_level: medium
---

# rust-registry-runtime-skill-install-boundary

## Current State

Status: completed
Current phase: final
Next: done
Reason: task completed
Blockers: none
Allowed follow-up command: `none`
Latest runner update: 2026-05-20T06:45:26Z
Review gate: pass

## Summary

Remove `@runxhq/core/registry` from runtime-local skill installation. Local
registry refs are resolved through a structural store interface; remote install
paths use narrow HTTP search/acquire helpers in the install boundary. This
keeps marketplace behavior and install profile validation intact while
unblocking the registry package sunset.

## Objectives

- Remove direct registry package imports from
  `packages/runtime-local/src/runner-local/skill-install.ts`.
- Preserve local registry, remote registry, marketplace, profile metadata, and
  digest verification behavior.
- Keep the install boundary structural so existing stores remain assignable
  without depending on the core registry package.

## Scope

- In scope:
  - `packages/runtime-local/src/runner-local/skill-install.ts`
  - focused skill-add and remote-registry-add tests
- Out of scope:
  - SDK registry helpers
  - CLI dispatch/search fallback
  - test-only registry fixture imports outside install behavior

## Dependencies

- Completed native registry CLI install boundary.
- Completed runtime registry resolver/store type boundary.

## Assumptions

- A structural store with `getVersion` and optional `listSkills` is sufficient
  for install behavior.

## Touchpoints

- Runtime-local skill install, remote acquire, profile-state writing, and
  marketplace install behavior.

## Risks

- Remote registry payload validation could drift. Mitigation: keep the local
  validator narrow and preserve focused remote add coverage.

## Acceptance

Profile: standard

Validation:
- `pnpm exec vitest run tests/skill-add.test.ts tests/remote-registry-add.test.ts --config vitest.config.ts`
- `pnpm exec vitest run tests/skill-add-profile-metadata.test.ts --config vitest.config.ts -t "installs marketplace execution profile when the upstream source provides it|rejects marketplace execution profile"`
- `pnpm typecheck`
- `git diff --check -- packages/runtime-local/src/runner-local/skill-install.ts`
- `! rg -n '@runxhq/core/registry' packages/runtime-local/src/runner-local/skill-install.ts`

## Phase 1: Implementation

Status: active
Dependencies: none

Objective: Complete the requested change.

Changes:
- Replace direct registry helper imports with install-local resolution, remote search/acquire, and strict payload validation inside the install boundary.

Acceptance:
- [ ] `ac1` command - Install behavior remains green
  - Command: `pnpm exec vitest run tests/skill-add.test.ts tests/remote-registry-add.test.ts --config vitest.config.ts`
  - Expected kind: `exit_code_zero`
  - Status: passed
- [ ] `ac2` command - Install boundary has no core registry import
  - Command: `! rg -n '@runxhq/core/registry' packages/runtime-local/src/runner-local/skill-install.ts`
  - Expected kind: `exit_code_zero`
  - Status: passed

## Rollback

- Revert `skill-install.ts` to the previous core-registry helper imports.

## Review

Status: completed
Verdict: pass
Mode: verify
Summary: Human-reviewed override accepted: Subagent and local verification: skill-add and remote-registry-add focused tests passed, typecheck passed, diff check passed, and skill-install has no core registry import.

Attack log:
- `review gate`: manual human audit -> clean (Subagent and local verification: skill-add and remote-registry-add focused tests passed, typecheck passed, diff check passed, and skill-install has no core registry import.)

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
