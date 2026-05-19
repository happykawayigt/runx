---
spec_version: '2.0'
task_id: rust-nitrosend-dogfood
created: '2026-05-18T00:00:00Z'
updated: '2026-05-20T00:00:00Z'
status: draft
harden_status: not_run
size: medium
risk_level: medium
---

# Rust nitrosend dogfood

## Current State

Status: draft
Current phase: plan-refresh
Next: approve
Reason: refreshed against the current local OSS checkout. This is a plan spec,
not an implementation pass.
Blockers: `runx-target-repo-runners` and
`runx-post-merge-outcome-observer` are still draft. The local
Nitrosend-like operational policy fixture also does not yet include the
`nitrosend/api` target required by the real Nitrosend shape.
Allowed follow-up command: none during this refresh; do not run
`scafld harden rust-nitrosend-dogfood`.
Latest runner update: none
Review gate: not_started

## Summary

Use Nitrosend as the external-shaped dogfood for the Rust runtime cutover, but
keep the plan honest about what exists in this OSS checkout today.

Current local facts:

- Generic runtime fixtures exist for the upstream `issue-intake` and
  `issue-to-pr` skills under `fixtures/runtime/skills/issue-intake/` and
  `fixtures/runtime/skills/issue-to-pr/`.
- A reusable operational policy contract exists as
  `runx.operational_policy.v1`, with CLI support in
  `packages/cli/src/commands/policy.ts` and schemas/fixtures under
  `schemas/operational-policy.schema.json` and `fixtures/operational-policy/`.
- `fixtures/operational-policy/nitrosend-like.json` is present, but it is only
  Nitrosend-like. It currently includes `nitrosend/nitrosend` and
  `nitrosend/app`; it does not include the real `nitrosend/api` target.
- No external-shaped Nitrosend replay fixture exists in this checkout:
  `fixtures/external/nitrosend/**` is absent.
- The current reusable follow-on work is not complete: target-repo runner
  support and the post-merge closure observer are both draft specs.

The dogfood goal is preservation, not a new adopter flow. The real Nitrosend
workflow may keep using the human/product names `issue-intake` and
`issue-to-pr`, but cutover execution artifacts must be canonical harness,
decision, act, proof, and sealed `runx.harness_receipt.v1` objects only.

This spec must not invent live Nitrosend traffic capture. Initial validation is
fixture parity against checked-in, sanitized, deterministic external-shaped
fixtures. Production soak is a later confirmation step after fixture, policy,
target-runner, and post-merge observer gates are green.

## Context

CWD: `.` (runx OSS workspace)

Relevant existing local surfaces:

- `skills/issue-intake/SKILL.md`
- `skills/issue-intake/X.yaml`
- `skills/issue-to-pr/SKILL.md`
- `skills/issue-to-pr/X.yaml`
- `fixtures/runtime/skills/issue-intake/cases/*.yaml`
- `fixtures/runtime/skills/issue-to-pr/cases/*.yaml`
- `fixtures/operational-policy/nitrosend-like.json`
- `packages/contracts/src/schemas/operational-policy.ts`
- `schemas/operational-policy.schema.json`
- `packages/cli/src/commands/policy.ts`
- `.scafld/specs/drafts/runx-target-repo-runners.md`
- `.scafld/specs/drafts/runx-post-merge-outcome-observer.md`

Missing local surfaces this plan must create or wait for:

- `fixtures/external/nitrosend/issue-intake/**`
- `crates/runx-runtime/tests/external/nitrosend_issue_intake.rs`
- A Nitrosend-like policy fixture that covers the `nitrosend/api` target as
  well as workspace and app targets.
- Target PR creation through the reusable target-repo runner contract.
- Post-merge/provider observation through sealed closure/proof receipts.

## Invariants

- No compatibility reader or writer translates retired outcome/effect/report
  artifacts into harness receipts at runtime.
- Product copy, skill names, slash-command text, and source-thread comments may
  say `issue-intake` or `issue-to-pr`; persisted execution contract ids,
  fixture artifact kinds, outbox contract payload ids, and Rust structs must not
  use those names as legacy execution artifact identifiers.
- External fixtures are deterministic and sanitized. They are not captured live
  during CI, and they do not depend on private Nitrosend repositories being
  checked out.
- `runx.operational_policy.v1` is the policy boundary. Nitrosend-specific JSON
  can be an input to a one-time conversion, but core runtime behavior must not
  dual-read adopter policy formats.
- Missing source-thread metadata fails closed before any Slack/GitHub final
  publication.
- Target completion, failed verification, source issue closure, and final
  source-thread replies are modeled through contained act closure plus
  proof-bound verification on sealed harness receipts.

## Objectives

- Add an external-shaped Nitrosend fixture suite that layers the existing
  generic `issue-intake`/`issue-to-pr` runtime fixtures into a realistic
  multi-target policy scenario.
- Extend the Nitrosend-like operational policy fixture to include
  `nitrosend/api` and prove workspace, API, and app target routing.
- Validate `runx policy lint` and `runx policy inspect` against the
  Nitrosend-like policy without leaking raw provider locators.
- Coordinate with `runx-target-repo-runners` so target PR creation, dedupe, and
  source-thread metadata are reusable core behavior.
- Coordinate with `runx-post-merge-outcome-observer` so merge/close/deploy
  observation, final publication, and source issue closure are sealed
  closure/proof receipts, not a Nitrosend-only observer path.
- Add Rust runtime replay coverage only after the external-shaped fixture and
  the reusable runner/observer contracts exist.

## Scope

In scope:

- Plan and fixture contract for Nitrosend-shaped issue intake.
- Policy fixture gap for `nitrosend/api`.
- Runtime parity expectations for canonical harness receipts.
- Explicit dependency on target-repo runner and post-merge closure observer
  work.

Out of scope:

- Editing Nitrosend production repositories.
- Live external traffic capture.
- Scafld hardening in this refresh.
- Compatibility with `runx.issue_to_pr_outcome.v1`, `effect`,
  `verification_report`, `target_outcome`, or any equivalent retired peer
  artifact.

## Dependencies

- `runx-contract-spine-hard-cutover` for canonical harness, decision, act,
  proof, and receipt shapes.
- `rust-runtime-skeleton` and `rust-runtime-skill-execution`.
- `rust-receipt-proof-verification`, `rust-receipt-tree-resolution`, and
  `rust-runtime-receipt-path-discovery`.
- `runx-operational-policy-config` as the completed policy contract.
- `runx-target-repo-runners` before target dispatch parity can pass.
- `runx-post-merge-outcome-observer` before final closure/proof parity can
  pass.

## Acceptance Criteria

- [ ] Existing generic `issue-intake` and `issue-to-pr` runtime fixtures remain
  green and are referenced by the external-shaped Nitrosend fixture plan.
- [ ] `fixtures/external/nitrosend/issue-intake/**` exists and contains
  sanitized deterministic inputs, not live external capture.
- [ ] The Nitrosend-like operational policy fixture covers
  `nitrosend/nitrosend`, `nitrosend/api`, and `nitrosend/app`.
- [ ] `runx policy lint fixtures/operational-policy/nitrosend-like.json`
  accepts the policy, and `runx policy inspect` redacts raw provider locators.
- [ ] Target PR creation uses `runx-target-repo-runners`; dedupe is represented
  in pull-request outbox `metadata.dedupe` and in the sealed receipt proof path.
- [ ] Target completion and final publication use
  `runx-post-merge-outcome-observer` only through closure/proof on sealed
  harness receipts.
- [ ] No fixture, schema id, persisted receipt, or replay expectation uses
  `runx.issue_to_pr_outcome.v1`, `issue_to_pr_outcome`, `outcome`, `effect`,
  `verification_report`, `verification-report`, `target_outcome`, or
  `target-effect`.
- [ ] Replaying the same external-shaped fixture twice produces identical
  canonical receipt bytes after normalized fixture ids/timestamps and passes
  proof verification.

## Validation Commands

Current local discovery/guard commands:

```sh
find fixtures/runtime/skills -maxdepth 4 -type f | sort
test ! -d fixtures/external
pnpm --filter @runxhq/cli run runx policy lint fixtures/operational-policy/nitrosend-like.json --json
pnpm --filter @runxhq/cli run runx policy inspect fixtures/operational-policy/nitrosend-like.json --json
! rg -n "runx\\.issue_to_pr_outcome\\.v1|issue_to_pr_outcome|verification[_-]report|target[_-]?effect|\"effect\"\\s*:" fixtures/runtime fixtures/operational-policy skills crates/runx-runtime
git diff --check -- .scafld/specs/drafts/rust-nitrosend-dogfood.md
```

Future validation once the missing external fixture and runner/observer specs
land:

```sh
cargo test --manifest-path crates/Cargo.toml -p runx-runtime nitrosend_issue_intake
cargo test --manifest-path crates/Cargo.toml -p runx-receipts
```

## Rollback And Repair

- Before any Nitrosend launcher flip, rollback is to keep the existing
  production path unchanged and disable Rust side-by-side validation.
- If the Nitrosend-like policy fixture omits a real target such as
  `nitrosend/api`, repair the policy fixture and policy admission tests rather
  than adding special-case runtime routing.
- If an external-shaped fixture is accidentally generated from live traffic,
  replace it with a sanitized deterministic fixture before merging.
- If retired artifact fields appear in fixtures or replay output, repair the
  producer and expected receipts. Do not whitelist retired fields.

## Open Questions

- Exact Rust binary pinning mechanism for Nitrosend CI.
- Whether Nitrosend wrapper behavior remains repo-local or is fully absorbed by
  reusable runx target-runner and post-merge observer code.
- Soak duration after fixture parity is green.
