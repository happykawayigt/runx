---
spec_version: '2.0'
task_id: runx-post-merge-outcome-observer
created: '2026-05-19T02:08:02Z'
updated: '2026-05-19T12:11:22Z'
status: draft
harden_status: not_run
size: large
risk_level: high
---

# Runx post-merge closure observer

## Current State

Status: draft
Current phase: none
Next: harden
Reason: issue-to-PR currently tells a good story through PR creation, but the
post-merge closure/proof path is still too dependent on repo-local glue. runx
core needs an observer for merge, close, deploy verification, final
source-thread update, and issue closure.
Blockers: `runx-operational-policy-config`; target/source context from
`runx-target-repo-runners` for cross-repo flows.
Allowed follow-up command: `scafld harden runx-post-merge-outcome-observer`
Latest runner update: 2026-05-20 added Rust contract-level repeated observer
signal idempotency proof for duplicate local provider observations, tightened
missing source-thread routing to fail before provider-state classification in
the pure planner, added a stable webhook/scheduler runtime dedupe plan and a
sealed harness-receipt publication projection gate, then added the local runtime
command projection boundary for source issue comments, source-thread replies,
and policy-authorized source issue close commands. Provider adapters remain
pending.
Review gate: not_started

## Summary

Add a reusable post-merge closure/proof observer for runx issue-to-PR flows. It
observes PR merge/close state, runs policy-defined verification, seals the
observed state and verification proof into harness receipts, updates the source
GitHub issue, posts the final Slack/source-thread reply, and closes or marks
the issue according to policy.

The observer does not auto-merge. Human merge remains the default final gate;
the observer publishes what happened after that gate.

## Context

CWD: `.` (runx OSS workspace)

Production story to support:
1. Intake creates a source issue from Slack/Sentry/GitHub.
2. runx triages and creates or links a target PR.
3. Human reviewer approves/merges or closes the PR.
4. runx observes the result.
5. runx runs verification appropriate to the target.
6. runx posts a concise final reply to the original Slack thread and source
   GitHub issue.
7. runx closes or labels the issue when policy allows.

Candidate touchpoints:
- GitHub adapter/outbox receipt builders.
- `skills/issue-to-pr/**`
- `skills/work-plan/**`
- Runtime receipt projection model.
- Aster observer scheduling and status surfaces.

Invariants:
- Observer is idempotent by source issue, PR, act form, and closure key.
- Source thread metadata must be present before Slack publishing.
- Closed-unmerged, merged-pending-verification, merged-verified,
  failed-verification, and superseded closures are distinct.
- Verification output is reviewer-safe and redacted.
- No hidden auto-merge path is introduced.
- The observer never emits a peer outcome/effect/report packet. It seals a
  follow-on harness receipt whose contained acts use `form: "observation"`,
  `form: "verification"`, `form: "reply"`, or `form: "revision"` as needed.
- Source issue closure and final source-thread publication require a sealed
  harness receipt with proof-bound closure and verification criteria.

## Objectives

- Define the harness receipt closure/proof model for merged, closed-unmerged,
  superseded, verification-passed, and verification-failed observations.
- Define criterion ids, reference roles, closure reason codes, and idempotency
  keys for provider state, PR state, human gate, verification, close policy,
  and source-thread targets.
- Add provider observer for GitHub PR state changes.
- Add policy-driven verification hook that records verification as a contained
  act with `form: "verification"` inside a sealed harness receipt.
- Publish final reply to source GitHub issue and Slack/source thread.
- Add idempotency/dedupe for repeated webhook or scheduled observer runs.
- Add fixtures for merged verified, merged failed verify, closed unmerged,
  missing source thread, and repeated observer signals.

## Scope

In scope:
- Core post-merge observer harness contract.
- GitHub PR state observer.
- Policy-driven verification command/hook contract.
- Final issue and source-thread publishing.
- Issue close/label behavior when policy allows.
- Tests and fixtures.

Out of scope:
- Automatic PR merge.
- Provider-specific deployment integrations beyond a hook boundary.
- Slack listener/reaction intake.
- Nitrosend-only script details except as reference fixtures.

## Dependencies

- `runx-operational-policy-config`.
- `runx-target-repo-runners` for cross-repo source/target context.
- `rust-runtime-receipt-path-discovery` for harness receipt storage.
- `rust-receipt-proof-verification` for sealed receipt proof verification.

## Assumptions

- GitHub is the initial PR provider.
- Deploy verification can start as command/provider hook output with a stable
  contract before richer hosted integrations land.
- Source-thread publishing can use the same outbox act/receipt projection model
  as earlier milestone comments.

## Touchpoints

- Provider adapter for PR state.
- Outbox/feed receipt builders.
- Runtime receipt summaries.
- Policy config.
- Aster observer scheduling/status.

## Risks

- Duplicate webhook deliveries can create noisy final comments.
- Missing source-thread metadata can cause root-channel Slack posts.
- Verification logs can leak secrets or local paths if not redacted.
- Closing issues before verification can hide unresolved bugs.

## Acceptance

Profile: strict

Validation:
- `pnpm test`
- `cargo test --manifest-path crates/Cargo.toml`
- post-merge-observer fixture command
- `git diff --check`

Required behavior:
- [ ] Merged PR with passing verification posts one final source issue comment,
  one final source-thread reply, and closes/labels according to policy.
- [ ] Merged PR with failing verification posts a final reply projected from a
  failed verification act and leaves the source issue open unless policy
  explicitly says otherwise.
- [ ] Closed-unmerged PR posts a distinct observation closure and does not claim
  a fix shipped.
- [x] Repeated observer signal is idempotent at the Rust contract planning
  layer; webhook and scheduler signals share one runtime dedupe receipt
  identity before publication.
- [x] Missing source Slack thread fails Slack publish cleanly without posting to
  channel root. Rust contract planning now fails closed before provider-state
  classification when source-thread metadata is missing, and the local runtime
  command projection fails closed when sealed source-thread provider/locator
  metadata is missing. Live provider adapters remain pending.
- [ ] Final publication is backed by a sealed harness receipt containing issue
  link, PR link, merge sha when available, verification summary, closure reason,
  and next human action.
- [x] Final publication validates by reading the sealed harness receipt and
  proof-bound verification criteria before it is published or used to close the
  source issue.
- [x] Final publication excludes absolute local paths, raw env vars, secrets,
  and excessive logs at the local runtime command-projection boundary.
- [ ] No fixture, emitted artifact, schema id, or persisted receipt uses
  `runx.issue_to_pr_outcome.v1`, `issue_to_pr_outcome`, `effect`,
  `verification_report`, `verification-report`, or `target_outcome`.

## Phase 1: Closure/Proof Model

Status: pending
Dependencies: `runx-operational-policy-config`

Objective: Define the observer harness, contained acts, closures, references,
criterion ids, and idempotency keys.

Changes:
- Add observer harness receipt fixture shape.
- Add contained act forms for provider observation, deployment verification,
  source-thread reply, and policy-authorized issue close/label revision.
- Add closure reason code rules and criterion id binding to harness receipt
  proof.
- Add idempotency key rules.
- Add policy validation for closure and publication actions.

Acceptance:
- [ ] Fixtures cover every closure state and contain no retired terminal/effect
  peer artifacts.

## Phase 2: Observer

Status: pending
Dependencies: Phase 1

Objective: Observe provider PR state and run verification.

Changes:
- Add GitHub PR observer adapter.
- Add verification hook contract.
- Seal observer harness receipts and link them to the source harness receipt
  tree.

Acceptance:
- [ ] Merged, closed, and repeated signal fixtures produce correct closures,
  verification proof, and idempotent harness receipt refs.

## Phase 3: Publishing

Status: pending
Dependencies: Phase 2

Objective: Publish the final reply and issue updates to the original source
surfaces from sealed receipt projections.

Changes:
- Publish source issue comment from sealed harness receipt projection.
- Publish source Slack/source-thread reply only when thread metadata is present.
- Close/label source issue according to policy through a contained revision act.

Acceptance:
- [ ] Source-thread fixture posts no root-channel messages.
- [ ] Final comment is concise but contains review-gate, closure, and
  verification state projected from the sealed harness receipt.

## Rollback

- Keep repo-local observer scripts until core observer fixtures are green, then
  migrate adopters and remove duplicated observer logic. Do not introduce
  compatibility aliases or shim artifacts; cutover removes duplicated observer
  logic directly.

## Review

Status: not_started
Verdict: none

Findings:
- none

## Self Eval

- Target score: 9.5. Passing means humans get a complete issue-to-PR-to-merge
  story backed by sealed harness receipts without watching multiple channels
  manually.

## Deviations

- none

## Metadata

- created_by: scafld
- planning_reason: make final post-merge publication a reusable runx capability

## Origin

Created by: scafld
Source: plan

## Harden Rounds

- none

## Planning Log

- 2026-05-19: Expanded placeholder into post-merge observer contract.
- 2026-05-19: Reconciled with the harness-spine hard cutover. The observer no
  longer defines a terminal packet; Rust, Aster, and repo wrappers must
  consume sealed harness receipts with contained observation, verification,
  reply, and revision acts.
- 2026-05-20: `runx-post-merge-observer-idempotency-contract` added a pure Rust
  planner proof that repeated merged-and-verified provider observations keep
  the same closure key, act forms, and idempotency identity without live GitHub.
- 2026-05-20: Added a pure Rust planner regression proving missing
  source-thread metadata fails closed before terminal provider-state
  classification, so this local contract slice cannot plan a root-channel
  fallback.
- 2026-05-20: Added the contract runtime dedupe plan that gives webhook and
  scheduler observations the same post-merge observer receipt identity, plus a
  sealed harness-receipt projection gate for final publication and issue close.
- 2026-05-20: Added the runtime/publication slice in `runx-runtime`: it consumes
  `PostMergeObserverRuntimeDedupePlan` plus a sealed harness receipt, emits only
  deterministic source issue comment, source-thread reply, and authorized close
  commands, dedupes repeated webhook/scheduler publication by publication key,
  fails closed on missing source-thread provider/locator metadata, and sanitizes
  public command text for local paths and env-secret assignments. The spec stays
  draft because live provider adapters and end-to-end publication fixtures are
  still pending.
