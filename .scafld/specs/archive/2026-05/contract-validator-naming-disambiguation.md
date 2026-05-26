---
spec_version: '2.0'
task_id: contract-validator-naming-disambiguation
created: '2026-05-24T00:00:00Z'
updated: '2026-05-26T05:04:15Z'
status: completed
harden_status: passed
size: small
risk_level: medium
---

# Disambiguate same-name / divergent-contract helpers

## Current State

Status: completed
Current phase: final
Next: done
Reason: task completed
Blockers: none
Allowed follow-up command: `none`
Latest runner update: 2026-05-26T05:04:15Z
Review gate: pass

## Summary

Rename the helpers so the name states the contract, eliminating the
import-the-wrong-one hazard. Throwing validators keep `require*`/`optional*`
(they validate); lenient extractors get a distinct verb (e.g. `coerce*` /
`read*Field`); the two `firstNonEmpty` shapes keep the `…OrUndefined` suffix
distinction already established in `@runxhq/core/util`.

## Objectives

- One canonical name per contract; no two functions named identically with
  different throw/return behavior anywhere in `packages/`.
- Point all call sites at the canonical helpers in `@runxhq/core/util` (and the
  package-local equivalents that cannot import core).
- Keep behavior identical at every call site (a rename, not a semantics change):
  a throwing site stays throwing, a lenient site stays lenient.

## Scope

In scope: `@runxhq/core/util` validators/accessors and their call sites across
`packages/core`, `packages/runtime-local`, `packages/cli`, `plugins/`.

Out of scope: `packages/authoring` and `packages/adapters` exported public API
names (coordinate separately if they collide); the Rust side.

## Acceptance

- [ ] `dod1` No identifier is defined with two different throw/return contracts
  across `packages/` (verified by a grep/lint over the helper names).
- [ ] `dod2` Every migrated call site preserves its prior runtime behavior
  (throwing stays throwing, lenient stays lenient); `verify:fast` stays green.
- [ ] `dod3` The lenient accessors live once in `@runxhq/core/util`; duplicates
  are removed where the package can import core.

## Origin

A+ roadmap (2026-05-24), step 3. Surfaced by the structural/dark-pattern review
during the shared-helper consolidation.

## Harden Rounds

### round-1

Status: passed
Started: 2026-05-26T04:58:17Z
Ended: 2026-05-26T04:58:54Z

Checks:
- path audit
  - Grounded in: code:packages/core/src/util/validators.ts:17
  - Result: passed
  - Evidence: The core util layer now separates `requireAnyString` from
- command audit
  - Grounded in: spec_gap:acceptance
  - Result: passed
  - Evidence: The implementation has focused TypeScript and fixture checks:
- scope/migration audit
  - Grounded in: code:packages/core/src/util/types.ts:13
  - Result: passed
  - Evidence: Lenient field access is centralized as `readField` /
- acceptance timing audit
  - Grounded in: code:packages/core/src/registry/http-client.ts:5
  - Result: passed
  - Evidence: Registry and parser consumers already import the shared helpers,
- rollback/repair audit
  - Grounded in: spec_gap:rollback
  - Result: passed
  - Evidence: The change is a local naming/consolidation cleanup with no data
- design challenge
  - Grounded in: code:tools/spec/normalize_scafld_frontmatter/src/index.ts:129
  - Result: passed
  - Evidence: The remaining non-core helper that returns undefined is named

Issues:
- none


## Review

Status: completed
Verdict: pass
Mode: verify
Summary: Human-reviewed override accepted: Implementation is a narrow helper naming/consolidation cleanup; pnpm verify:fast passed after regenerating lock/tool metadata, focused helper grep shows one contract per helper name, and focused TS/fixture checks passed.

Attack log:
- `review gate`: manual human audit -> clean (Implementation is a narrow helper naming/consolidation cleanup; pnpm verify:fast passed after regenerating lock/tool metadata, focused helper grep shows one contract per helper name, and focused TS/fixture checks passed.)

Findings:
- none
