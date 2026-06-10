---
spec_version: '2.0'
task_id: runx-demo-surface-prune-v1
created: '2026-06-05T03:25:35Z'
updated: '2026-06-10T11:10:58Z'
status: completed
harden_status: passed
size: medium
risk_level: medium
---

# runx-demo-surface-prune-v1

## Current State

Status: completed
Current phase: complete
Next: done
Reason: finalization receipt passed
Blockers: none
Allowed follow-up command: `none`
Latest runner update: 2026-06-10T11:09:45Z
Review gate: not_started

## Summary

Prune and rank the example/demo surface so a fresh evaluator sees only polished,
runnable proof paths. Runx now has a strong set of featured demos:
`hello-world`, `http-graph`, `openapi-graph`, `github-mcp-hero`,
`governed-spend`, and the payment dogfood lanes. Other example-like material may
still be useful, but it should not sit in the same first-class window unless it
has a README/run command/gate and proves a current product surface.

This is not about deleting fixtures that protect contracts. It is about separating
featured demos, runnable examples, fixtures, and archived scaffolding with no
ambiguous leftovers.

## Objectives

- Define the public demo list in one place and make docs match it.
- Move fixture-only or protocol-only example material out of the featured demo
  lane, or promote it with a runnable script and gate.
- Ensure every `examples/*` directory has an explicit classification: featured,
  runnable-preview, fixture-support, or archived/deleted.
- Remove stale wrappers and stale docs that imply non-A-grade demos are featured.

## Scope

In scope:
- `examples/**`, `docs/demos.md`, `examples/README.md`, README references.
- `scripts/check-demos.mjs` and any new demo inventory/guard script.
- Thread-outbox and post-merge example directories only for classification; product
  wiring belongs to the thread-outbox spec.

Out of scope:
- Building new demos for payment, A2A, hosted, or gallery UX.
- Deleting contract fixtures that have test coverage value.

## Dependencies

- Gate hardening should define the demo inventory guard.
- Existing demos: `examples/github-mcp-hero/run.sh`, `examples/http-graph/run.sh`,
  `examples/openapi-graph/run.sh`, `examples/governed-spend/*`,
  `scripts/check-demos.mjs`.
- Current gap: `docs/demos.md` marks several demos as harness-gated, while
  `pnpm demos:check` only runs the payment receipt demos. This spec closes that
  mismatch by either expanding the gate or changing the public label.

## Assumptions

- A featured demo must run from a fresh checkout, emit or verify a receipt where
  applicable, and have a clear README path.
- Preview examples may stay if they are explicitly non-featured and runnable.

## Risks

- **Accidental fixture deletion.** Mitigation: classify first, delete only when no
  test or doc consumer exists.
- **Demo set becomes too narrow.** Mitigation: keep a runnable-preview section for
  real but secondary examples.

## Acceptance

Profile: strict

Validation:
- `docs/demos.md` and `examples/README.md` agree on the featured demo list.
- Every `examples/*` directory is classified or deleted.
- `pnpm demos:check`, `pnpm x402:dogfood:local`, and focused featured demo
  scripts pass.
- No stale "compatibility wrapper" or obsolete demo wording remains in featured
  docs unless the wrapper is intentionally retained with a deletion date.

## Phase 1: Inventory and classify examples

Status: pass
Dependencies: runx-readiness-gate-hardening-v1

Objective: create a single source of truth for demo classification.

Changes:
- Add or update a demo inventory guard.
- Classify each `examples/*` directory as featured, runnable-preview, fixture-support, or remove.
- Align `docs/demos.md`, `examples/README.md`, and root README links.

Acceptance:
- [x] `p1_ac1` command - example inventory is complete
  - Command: `node scripts/check-demo-inventory.mjs`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-18

## Phase 2: Prune or promote non-featured material

Status: pass
Dependencies: Phase 1

Objective: no ambiguous example debris.

Changes:
- Delete truly orphaned example directories.
- Move fixture-support material under a fixture/support path if that is cleaner.
- Add `run.sh` and docs only for examples intentionally promoted to runnable-preview.

Acceptance:
- [x] `p2_ac1` command - featured demos run
  - Command: `DEMO_RECEIPTS="$(mktemp -d)" && CARGO_TARGET_DIR=crates/target/demo-gate cargo build --quiet --manifest-path crates/Cargo.toml -p runx-cli --bin runx && RUNX_BIN=crates/target/demo-gate/debug/runx sh examples/github-mcp-hero/run.sh && RUNX_BIN=crates/target/demo-gate/debug/runx sh examples/http-graph/run.sh && RUNX_BIN=crates/target/demo-gate/debug/runx sh examples/openapi-graph/run.sh && RUNX_RECEIPT_DIR="$DEMO_RECEIPTS/overspend" RUNX_RECEIPT_SIGN_KID=runx-demo-key RUNX_RECEIPT_SIGN_ED25519_SEED_BASE64=QkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkI= RUNX_RECEIPT_SIGN_ISSUER_TYPE=hosted crates/target/demo-gate/debug/runx harness examples/governed-spend/skills/overspend-refused --json && pnpm demos:check && pnpm x402:dogfood:local`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-19
- [x] `p2_ac2` command - full fast gate remains green
  - Command: `pnpm verify:fast`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-20

## Rollback

- Restore any deleted example directory only with its classification and gate.

## Review

Status: not_started
Verdict: none

Findings:
- none

## Self Eval

- none

## Deviations

- none

## Metadata

- created_by: codex

## Origin

Created by: Codex
Source: operator readiness queue

## Harden Rounds

### round-1

Status: passed
Started: 2026-06-05T03:50:03Z
Ended: 2026-06-05T03:50:03Z
Verdict: pass
Provider: claude
Model: claude-opus-4-7
Output format: claude.mcp_submit_harden
Summary: Draft spec is executable. The demo inventory guard (`scripts/check-demo-inventory.mjs`) and source-of-truth file (`docs/demo-inventory.json`) already exist as untracked files and align with the classification in `examples/README.md` and `docs/demos.md`. All featured run scripts exist; acceptance commands are real. The original advisories were handled after review: p2_ac1 now builds and uses an isolated native binary first, and `docs/demos.md` no longer uses stale "compatibility wrapper" wording.

Checks:
- path audit
  - Grounded in: code:oss/docs/demo-inventory.json:1
  - Result: passed
  - Evidence: Verified every path the spec depends on: examples/github-mcp-hero/run.sh, examples/http-graph/run.sh, examples/openapi-graph/run.sh, examples/governed-spend/run.sh, examples/governed-spend/x402.sh, examples/governed-spend/stripe-spt.sh, examples/governed-spend/skills/overspend-refused (SKILL.md + X.yaml), examples/governed-spend/verify.mjs, docs/demos.md, examples/README.md, docs/demo-inventory.json, scripts/check-demo-inventory.mjs, scripts/check-demos.mjs all exist. The inventory file lists 5 featured, 7 runnable_preview, and 10 fixture_support entries that match the actual examples/* directories on disk.
- command audit
  - Grounded in: code:oss/package.json:72
  - Result: passed
  - Evidence: package.json defines `demos:check` -> scripts/check-demos.mjs, `x402:dogfood:local` -> scripts/x402-local-dogfood.mjs, and `verify:fast` -> scripts/verify-fast.mjs. scripts/verify-fast.mjs (line 43) already runs `node scripts/check-demo-inventory.mjs` as part of the source-checks group, so p1_ac1 will run inside the existing fast lane. The four featured run.sh scripts shell out to a native runx binary (with RUNX_BIN override) and node, which are the binaries the acceptance commands assume.
- scope/migration audit
  - Grounded in: code:oss/.scafld/specs/drafts/runx-demo-surface-prune-v1.md:47
  - Result: passed
  - Evidence: Scope is bounded to examples/**, docs/demos.md, examples/README.md, README references, and the demo guard scripts. Thread-outbox/post-merge directories are explicitly only in scope for classification (already covered by inventory entries thread-outbox-provider-* and post-merge-final-outcome-publisher), with product wiring deferred to the thread-outbox spec. Phase 1 depends on runx-readiness-gate-hardening-v1, which is active with harden_status: passed (`.scafld/specs/active/runx-readiness-gate-hardening-v1.md:6`), so the dependency edge is real and current.
- acceptance timing audit
  - Grounded in: code:oss/.scafld/specs/drafts/runx-demo-surface-prune-v1.md:127
  - Result: passed
  - Evidence: p1_ac1 (`node scripts/check-demo-inventory.mjs`) is pure I/O — it reads docs/demo-inventory.json, examples/, docs/demos.md, examples/README.md — so it can run at the end of phase 1 without external setup. p2_ac1 builds an isolated native binary, chains four shell run scripts plus the Rust harness, then runs `pnpm demos:check` and `pnpm x402:dogfood:local`. p2_ac2 (`pnpm verify:fast`) is the canonical fast gate and includes the demo inventory guard, so a green p2_ac2 also proves p1_ac1 still passes after phase 2 churn.
- rollback/repair audit
  - Grounded in: code:oss/.scafld/specs/drafts/runx-demo-surface-prune-v1.md:135
  - Result: passed
  - Evidence: Rollback rule ("Restore any deleted example directory only with its classification and gate") is the right shape for medium-risk classification/pruning work: any restoration must re-enter docs/demo-inventory.json, the guard catches inconsistency, and git history is the underlying repair path. Risks section calls out accidental fixture deletion and over-narrowing the demo set, mitigated by classifying-before-deleting and the runnable-preview tier. No destructive non-reversible step (database migration, signed-receipt format change) is bundled, so a brief rollback is proportional.
- design challenge
  - Grounded in: code:oss/examples/README.md:20
  - Result: passed
  - Evidence: Codifying the featured demo list in docs/demo-inventory.json with a guard script is the durable architectural move, not a bandaid: it removes the docs-vs-code drift the spec's Dependencies section explicitly names ("docs/demos.md marks several demos as harness-gated, while pnpm demos:check only runs the payment receipt demos"). It leaves contract fixtures untouched and adds three explicit tiers (featured / runnable-preview / fixture-support) that match how a fresh evaluator reads the repo. Not bloat: the guard is ~80 lines of Node and already integrated into verify:fast. Not short-sighted: classification scales as new demos land without re-litigating which page is authoritative.

Issues:
- [low/advisory] `harden-1` acceptance_precondition - p2_ac1 implicitly requires a pre-built crates/target/debug/runx binary
  - Status: resolved
  - Grounded in: code:oss/.scafld/specs/drafts/runx-demo-surface-prune-v1.md:127
  - Evidence: The Phase 2 acceptance string now builds `runx-cli` into `crates/target/demo-gate` and passes that binary through `RUNX_BIN` before running featured demo scripts and the governed-spend harness directly.
  - Recommendation: Either prepend `pnpm rust:build` (or a `cargo build -p runx-cli` invocation) to p2_ac1, or add a Phase 2 "Prerequisites" note stating that the native runx binary must be built first (which is implicit anywhere `pnpm verify:fast` has run).
  - Question: Should p2_ac1 include an explicit build step, or rely on the convention that verify:fast or a prior cargo build has already produced crates/target/debug/runx?
  - Recommended answer: Rely on verify:fast having been run (p2_ac2 is in the same acceptance set) but add a one-line note under Phase 2 acceptance documenting the binary precondition.
  - If unanswered: Leave the acceptance string as-is and note in the Phase 2 changes section that the native runx binary must be built first.
- [low/advisory] `harden-2` stale_doc_wording - docs/demos.md still labels examples/governed-spend/verify.mjs as a "compatibility wrapper" with no deletion date
  - Status: resolved
  - Grounded in: code:oss/docs/demos.md:101
  - Evidence: The current docs/demos.md ends with: "examples/governed-spend/verify.mjs remains as a compatibility wrapper for older demo instructions." The Acceptance section of this spec says: "No stale 'compatibility wrapper' or obsolete demo wording remains in featured docs unless the wrapper is intentionally retained with a deletion date." Either the docs text needs to be removed in Phase 2, or the wrapper needs an explicit retention rationale + deletion date.
  - Recommendation: Track this in Phase 2 changes explicitly: decide whether examples/governed-spend/verify.mjs is being kept (and add a deletion-date comment in docs/demos.md) or being removed (and update all callers, including the github-mcp-hero run.sh which invokes it at line 58).
  - Question: Is examples/governed-spend/verify.mjs being deleted in Phase 2, or kept with a documented retention rationale?
  - Recommended answer: Keep it with a one-line deletion-date marker, because examples/github-mcp-hero/run.sh and several scripts still call into it; flipping them all is out of scope for a pure classification spec.
  - If unanswered: Default to keeping the wrapper and adding a `// retained until <date>` comment plus a matching note in docs/demos.md.


## Planning Log

- none
