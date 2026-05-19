---
spec_version: '2.0'
task_id: rust-kernel-port-orchestration
created: '2026-05-17T00:00:00Z'
updated: '2026-05-20T00:00:00Z'
status: draft
harden_status: not_run
size: large
risk_level: high
---

# Rust kernel port orchestration

## Current State

Status: draft
Current phase: refresh only
Next: do not approve as written; either backfill audited receipts or split the
current codebase into new executable kernel/core slices
Reason: current repo already contains the Rust crate graph and `runx-core`
state-machine/policy code, but this draft still describes a pre-implementation
orchestration and has no Phase Receipts or external review evidence
Blockers: Phase Receipts remain empty; orchestrated sub-spec completion was not
verified in this refresh; `rust-kernel-blocking-promotion` is still blocked by
advisory-soak evidence
Allowed follow-up command: none during this refresh; do not run
`scafld harden rust-kernel-port-orchestration`.
Latest runner update: none
Review gate: not_started

## Summary

Drive the Rust pure-kernel port end-to-end across multiple sessions without
data loss. This is the umbrella spec that orchestrates the pre-kernel
contracts bootstrap plus the four kernel parity sub-specs
(`rust-contracts-bootstrap`, `rust-kernel-parity-fixtures`,
`rust-state-machine-parity`, `rust-policy-parity`,
`rust-parity-ci-governance`) through their scafld
lifecycles (harden, approve, execute, review, complete) in the correct
order, with mandatory artifact handoff verification between phases.

The orchestration spec does not write code directly. It enforces the
discipline that any agent executing the port must follow. The phases are
gates: each gate verifies the previous sub-spec produced its checked-in
artifacts and passed its own review before the next sub-spec is approved.

This spec leverages scafld's strict execution profile, mandatory review
gate, harden-before-approve interrogation, per-phase checkpointing, and
rollback-on-fail to make agent drift, scope creep, and silent-skip
failures visible across a multi-week port. It does not claim to complete the
runtime, MCP, adapter, parser, receipt, or CLI cutover work. Those follow the
kernel port and have their own specs.

Refresh note, 2026-05-20: the codebase has advanced beyond the original draft.
`crates/runx-core` exists at version `0.0.2` and currently exports
state-machine and policy parity surfaces, including authority-proof,
public-work, sandbox, connected-auth, graph-scope, retry, local admission, and
payment-authority subset decisions. The Rust workspace also contains
`runx-contracts`, `runx-parser`, `runx-receipts`, `runx-runtime`, `runx-sdk`,
and `runx-cli`. This spec remains `draft` because those observations do not
prove the scafld lifecycle completed; the Phase Receipts, review gate, and
completion evidence are still empty.

This spec depends on the architecture decisions in
`oss/docs/rust-kernel-architecture.md` (sections 2 pure-kernel-scope, 3
crate-graph ordering, and 12 dual-tree maintenance policy) and assumes
all five orchestrated sub-specs are in `status: draft`.

## Context

CWD: `.`

Packages:
- `@runxhq/core`
- `crates/runx-core` (implemented parity crate, not a placeholder)
- `crates/runx-contracts`
- `crates/runx-parser`
- `crates/runx-receipts`
- `crates/runx-runtime`
- `crates/runx-sdk`
- `crates/runx-cli`

Files impacted:
- `.scafld/specs/drafts/rust-kernel-parity-fixtures.md` (Phase 1
  discipline lock)
- `.scafld/specs/drafts/rust-contracts-bootstrap.md` (Phase 1 discipline
  lock; Phase 2 pre-kernel gate)
- `.scafld/specs/drafts/rust-state-machine-parity.md` (Phase 1)
- `.scafld/specs/drafts/rust-policy-parity.md` (Phase 1)
- `.scafld/specs/drafts/rust-parity-ci-governance.md` (Phase 1)
- `docs/rust-kernel-architecture.md` (reference; not modified)
- This spec (`rust-kernel-port-orchestration.md`) for Current State and Phase
  Receipts updates as phases complete.

Invariants:
- Sub-specs are approved one at a time. No parallel approval; no skipping.
- Every sub-spec runs `scafld harden` before `scafld approve`. No exceptions.
- Every sub-spec runs `scafld review` (external provider, not local) before
  status flips to completed.
- No workspace edits during `scafld review`. The mutation guard discards the
  packet; an edit kills the review.
- No deferral of in-scope sub-spec work by relabeling it "out of scope mid
  execution." Use the deviations mechanism with explicit user approval.
- No threshold lowering when tests fail. Fix the test or fix the code; the
  goalpost does not move.
- No `scafld complete` on this orchestration spec while any sub-spec is
  still in_progress.
- Phase Receipts are written at the moment a phase completes. A phase is
  not complete without a Receipt.
- Cross-session resume reads Current State first; never re-derives port
  progress from git log or guesswork.
- Refreshes to this draft must not mark draft work complete based on current
  files alone. Completion still requires scafld status, external review, and
  populated receipts.
- `runx-core` remains a pure decision crate: no filesystem, network,
  subprocess, MCP, adapter, runtime execution, or CLI presentation behavior.
- TypeScript remains authoritative for trusted-kernel behavior until an
  explicit cutover spec changes a consumer.
- CI's `Advisory Rust kernel parity` step is advisory until
  `rust-kernel-blocking-promotion` removes `continue-on-error: true` after the
  five-clean-PR evidence gate.

Related docs:
- `docs/rust-kernel-architecture.md` (prerequisite reading)
- `docs/trusted-kernel-package-truth.md`
- `oss/.scafld/config.yaml` (the validation/review/harden machinery this
  spec leans on)
- `AGENTS.md`

## Objectives

- Sequence the pre-kernel and kernel sub-specs in dependency order: contracts
  bootstrap, fixtures, state-machine, policy, CI governance.
- Bake `profile: strict`, `self_eval_threshold >= 8`, mandatory external
  review, and mandatory harden-before-approve into each sub-spec via
  Phase 1.
- Define the artifact handoff contract between phases (what files must
  exist after phase N before phase N+1 can begin).
- Define the Phase Receipts format (extension to standard spec layout) so
  evidence of each phase's completion survives across sessions.
- Define the cross-session resume protocol so a fresh agent in week 6 can
  pick up exactly where week 1 left off.
- Define the rollback semantics for the whole port (per-sub-spec rollback
  is the unit; this orchestration tracks which phases need rolling).
- Leave the Phase A advisory soak and promotion-to-blocking gate to
  `rust-kernel-blocking-promotion`, which runs only after this kernel port
  completes and 5 clean kernel-touching PRs have landed.
- Record the refreshed state of the current Rust kernel/core surface without
  converting observed files into completion evidence.
- Identify small follow-up slices that can be executed safely from the current
  codebase.

## Scope

In scope:
- Orchestration of `rust-contracts-bootstrap`, `rust-kernel-parity-fixtures`,
  `rust-state-machine-parity`, `rust-policy-parity`, and
  `rust-parity-ci-governance` through their scafld lifecycles.
- Modification of the four sub-specs' acceptance frontmatter to lock the
  discipline standards in Phase 1.
- Phase Receipts tracking per phase.
- Handoff to the follow-up `rust-kernel-blocking-promotion` spec.

Out of scope:
- `rust-cli-feature-parity-matrix` and `rust-runx-cli-placeholder`.
  Adjacent specs; their own orchestration spec if needed.
- `rust-sdk-surface-parity`. Adjacent SDK work; it may ship as a CLI-backed
  client, but it is not part of the pure-kernel port.
- Full `rust-contracts-parity`. Only `rust-contracts-bootstrap` is part of
  this kernel orchestration; complete contract parity is SDK/runtime work.
- Blocking promotion after the advisory soak. That is owned by
  `rust-kernel-blocking-promotion`.
- Any parser, receipts, runtime, or CLI cutover work. Those are follow-up
  orchestration specs (`rust-impure-domain-orchestration`,
  `rust-runtime-orchestration`, `rust-cli-cutover-orchestration` once they
  exist).
- Runtime payment execution, payment rails, and payment receipt work. Parent is
  actively editing runtime payment files; this spec only notes that
  `runx-core` has a pure payment-authority subset helper.
- Treating authority-proof, public-work, or payment-authority code as completed
  scafld work without an explicit audit. Current code contains these surfaces,
  but the orchestration receipts here do not prove how they landed.
- Direct code writing. This spec coordinates; the sub-specs write.

## Dependencies

- `docs/rust-kernel-architecture.md` exists and is approved.
- The five orchestrated sub-specs exist in `.scafld/specs/drafts/` at
  status `draft`.
- Scafld config at `oss/.scafld/config.yaml` has `strict_spec_adherence:
  true`, `checkpoint_frequency: per_phase`, `rollback_on_fail: true`,
  `self_review: mandatory` (it does, as of 2026-05-17).
- An external review provider is configured (Claude). Local
  review cannot satisfy complete.

## Assumptions

- An agent executing this spec spans many sessions. Persistent state lives
  in the spec file on disk, not in any single session's context.
- The advisory soak is calendar-independent and intentionally not part of
  this orchestration. If the kernel does not see 5 PRs that touch
  state-machine or policy after advisory CI lands, `rust-kernel-blocking-
  promotion` blocks indefinitely. This is correct behavior, not a bug.
- Sub-specs may surface deviations during their own execution. Deviations
  flow up: this orchestration spec's Phase Receipts record sub-spec
  deviations so the cumulative deviation budget is visible at the port
  level, not buried in individual sub-specs.
- The Phase 1 discipline lock is allowed to modify sub-specs even though
  they are status `draft`. After Phase 1, sub-specs are still draft but
  their acceptance frontmatter is hardened.
- Self-eval threshold 9 is intentional and aggressive. This forces a
  second pass on anything 8 or below. Default of 7 is too permissive for
  a trust-boundary port.
- `docs/trusted-kernel-package-truth.md` is the current local authority for the
  advisory/blocking distinction: Phase A remains advisory in CI, TypeScript is
  authoritative, and `pnpm rust:check` is the local Rust parity wrapper.
- The package-boundary guardrail is executable through
  `scripts/check-rust-crate-graph.mjs`; the purity/style guardrail is
  executable through `scripts/check-rust-core-style.mjs`.

## Touchpoints

- Scafld lifecycle commands: `harden`, `approve`, `status`, `review`,
  `complete`, `fail`, and `cancel`.
- Five sub-spec files in `.scafld/specs/drafts/`.
- The architecture doc (read-only reference).
- Validation pipeline profiles (strict profile across the port).
- External review providers (Claude per scafld
  config).

## Risks

- High: agent context loss across sessions. Mitigated by Current State as
  the source of truth, Phase Receipts as the audit log, and an explicit
  resume protocol.
- High: silent scope drift. Mitigated by `strict_spec_adherence: true`,
  the `scope_drift` adversarial review pass, and explicit anti-pattern
  invariants.
- High: "good enough" false completion. Mitigated by mandatory external
  review per sub-spec, self_eval_threshold 9, and the requirement that no
  phase completes without a Receipt.
- Medium: dual-tree maintenance fatigue mid-port causing scope reduction.
  Mitigated by the cost estimate baked into the arch doc section 12 and
  the explicit advisory soak phase.
- Medium: blocking promotion may wait indefinitely if kernel churn stops
  before 5 clean kernel-touching PRs. This is delegated to
  `rust-kernel-blocking-promotion` so the kernel port itself can complete
  with advisory checks enabled.
- Medium: review provider unavailability stalls phases. Mitigated by
  scafld config's `fallback_policy: warn` and dual provider support.
- Low: cross-spec dependency confusion if sub-spec files are reordered or
  renamed during execution. Mitigated by final coherence verification.

## 2026-05-20 Refresh Findings

Observed current code and guardrails:
- Rust workspace members are `runx-cli`, `runx-contracts`, `runx-core`,
  `runx-parser`, `runx-receipts`, `runx-runtime`, and `runx-sdk`.
- `runx-core` is no longer a placeholder. It exports state-machine decisions
  and policy decisions from `crates/runx-core/src/lib.rs`.
- Current policy modules include authority proof, connected auth, graph scope,
  interpreter normalization, local admission, payment authority subset,
  public-work policy, retry admission, sandbox admission, and scope helpers.
- Kernel fixtures currently include 14 state-machine fixtures, 37 policy
  fixtures, and 1 runner-ingestion fixture under `fixtures/kernel/`.
- `package.json` exposes `pnpm rust:check`,
  `pnpm rust:crate-graph`, `pnpm rust:style`, kernel fixture validate/check/key
  scripts, and `pnpm verify:fast`.
- `.github/workflows/ci.yml` runs general Rust checks as blocking CI, then runs
  `Advisory Rust kernel parity` with `continue-on-error: true`.
- `scripts/check-boundaries.mjs` still defines TypeScript pure core domains as
  `parser`, `policy`, and `state-machine`.

Stale assumptions corrected by this refresh:
- The draft must not say `runx-core` is a future placeholder. It is an
  implemented parity crate.
- The draft must not claim authority-proof and public-work policy exports are
  still absent from Rust. They are present in `runx-core`, though their scafld
  lifecycle evidence is not recorded in this file.
- Parser is no longer merely future architecture. `crates/runx-parser` exists
  and is included in the Rust workspace/style guardrails, but parser cutover
  remains outside this kernel orchestration.
- Payment authority is present only as a pure subset decision in `runx-core`.
  Runtime payment execution remains out of scope for this spec.

## Gate Classification

Blocking gates for completing this orchestration spec:
- Each orchestrated sub-spec must have scafld status `completed` and external
  review verdict `pass`.
- Phase Receipts must be filled with commit SHA, review verdict, reviewer
  model, self-eval score, artifacts, and deviations.
- `pnpm verify:fast` must pass for the repo-level fast gate.
- `pnpm rust:crate-graph`, `pnpm rust:style`, kernel fixture validation/checks,
  and relevant Cargo tests must pass for kernel/core changes.

Advisory gates until promotion:
- CI `Advisory Rust kernel parity` remains advisory while
  `continue-on-error: true` is present.
- Cargo deny and `cargo public-api` are required by the local `pnpm rust:check`
  wrapper, but their dedicated CI effect is advisory until
  `rust-kernel-blocking-promotion` executes.
- The five-clean-PR soak is an advisory-promotion gate, not evidence that this
  orchestration spec completed.

## Package Boundary And Purity

Confirmed assumptions:
- `runx-core` may depend on `runx-contracts`, serde, deterministic JSON/hash
  support, and test-only proptest. It must not own filesystem, network,
  subprocess, MCP, adapter, runtime execution, or CLI presentation behavior.
- `runx-parser` may depend on `runx-contracts` and `runx-core` for typed
  boundaries and sandbox normalization. Parser parity is pure, but parser
  cutover is separate from this kernel orchestration.
- `runx-runtime` is the impure boundary for filesystem, subprocess, network,
  adapters, MCP, sandbox enforcement, and payment execution wiring.
- `runx-sdk` v0 remains a CLI-backed client over `runx-contracts`; it must not
  depend on `runx-core` before a separate SDK/runtime cutover changes that.
- TypeScript `@runxhq/core` remains authoritative for state-machine, policy,
  and parser behavior until explicit cutover specs change consumers.

## Next Executable Kernel/Core Slices

Use these as new or refreshed specs rather than marking this draft complete:
- Authority-proof/public-work closure audit: reconcile the current Rust exports
  and fixtures with stale docs/spec language, prove TS/Rust oracle parity, and
  update only the docs/specs that still say those re-exports are deferred.
- Payment-authority fixture parity: add TypeScript oracle generation and kernel
  fixture JSON for `is_payment_authority_subset` before treating the Rust helper
  as more than proptest/unit-test covered pure logic. Do not touch runtime
  payment execution in this slice.
- Parser pure-core parity hardening: because `parser` is in
  `pureCoreDomains` and `runx-parser` now exists, run a parser-only acceptance
  pass over fixture generation/checks, Rust parser fixtures, crate graph, and
  style coverage without changing runtime consumers.
- Kernel docs coherence: align `crates/runx-core/README.md`,
  `fixtures/kernel/README.md`, and architecture/truth docs with the current
  implemented Rust surfaces while preserving "TypeScript authoritative" and
  "CI advisory until promotion" language.
- Public API snapshot maintenance: any public `runx-core` API change must
  regenerate `crates/runx-core/api-snapshot.txt` and pass
  `node scripts/check-rust-kernel-parity.mjs --api-only`.

## Acceptance

Profile: strict

Definition of done:
- [ ] `dod1` All five orchestrated sub-specs at status `completed` with
  review gate `pass`.
- [ ] `dod2` Phase Receipts populated for every phase with sub-spec id,
  commit SHA, review verdict, reviewer model, self-eval score, artifact
  paths, and any deviations.
- [ ] `dod3` `runx-core` crate exists and exports state-machine + policy
  per the parity specs.
- [ ] `dod4` JSON fixtures exist, validate against schema, and pass on
  both TypeScript and Rust sides.
- [ ] `dod5` CI runs the new Rust kernel checks in advisory mode and docs
  point blocking promotion to `rust-kernel-blocking-promotion`.
- [ ] `dod6` Cross-spec coherence verification passes: every cross-spec
  reference resolves, no orphaned artifacts, no broken handoffs.
- [ ] `dod7` Self-eval score >= 9 across the orchestration.

Validation:
- [ ] `v1` command - all orchestrated sub-specs completed.
  - Command: `for s in rust-contracts-bootstrap rust-kernel-parity-fixtures rust-state-machine-parity rust-policy-parity rust-parity-ci-governance; do scafld status --json "$s" | jq -e '.result.status == "completed"' >/dev/null || exit 1; p="$(find .scafld/specs -name "$s.md" -print -quit)"; test -n "$p" || exit 1; rg -n 'Review gate: pass|Verdict: pass' "$p" >/dev/null || exit 1; done`
  - Expected kind: `exit_code_zero`
  - Timeout seconds: 60
  - Result: none
  - Status: pending
  - Evidence: none
  - Source event: none
  - Last attempt: none
  - Checked at: none
- [ ] `v2` command - kernel artifacts exist.
  - Command: `test -d fixtures/kernel/schema && test -d fixtures/kernel/state-machine && test -d fixtures/kernel/policy && test -d crates/runx-core/src && test -f scripts/check-rust-core-style.mjs && test -f scripts/check-rust-kernel-parity.mjs`
  - Expected kind: `exit_code_zero`
  - Timeout seconds: none
  - Result: none
  - Status: pending
  - Evidence: none
  - Source event: none
  - Last attempt: none
  - Checked at: none
- [ ] `v3` command - kernel parity passes on both sides.
  - Command: `pnpm verify:fast && node scripts/check-rust-kernel-parity.mjs`
  - Expected kind: `exit_code_zero`
  - Timeout seconds: none
  - Result: none
  - Status: pending
  - Evidence: none
  - Source event: none
  - Last attempt: none
  - Checked at: none
- [ ] `v4` command - CI is in advisory mode for the new Rust checks.
  - Command: `rg -n 'continue-on-error: true' .github/workflows/ci.yml | rg -qE 'cargo-deny|cargo public-api|check-rust-kernel-parity' && rg -n 'rust-kernel-blocking-promotion' docs CONTRIBUTING.md README.md .scafld/specs/drafts/rust-kernel-blocking-promotion.md`
  - Expected kind: `exit_code_zero`
  - Timeout seconds: none
  - Result: none
  - Status: pending
  - Evidence: none
  - Source event: none
  - Last attempt: none
  - Checked at: none
- [ ] `v5` command - Phase Receipts are populated.
  - Command: `! rg -nE '<to be filled at exec time>|commit_sha: <to be filled' .scafld/specs/drafts/rust-kernel-port-orchestration.md`
  - Expected kind: `exit_code_zero`
  - Timeout seconds: none
  - Result: none
  - Status: pending
  - Evidence: none
  - Source event: none
  - Last attempt: none
  - Checked at: none

## Phase 1: Discipline lock on sub-specs

Goal: Bake the strict discipline standards into each kernel parity sub-spec
before any of them is approved. This is the only phase that modifies
sub-spec files. After this phase, sub-spec acceptance frontmatter is
frozen.

Status: pending
Dependencies: none

Changes:
- `.scafld/specs/drafts/rust-kernel-parity-fixtures.md` (partial, shared) -
  Set `Profile: strict` in Acceptance (already set; verify), add explicit
  `Self-eval threshold: 8` line in Acceptance, add explicit
  `Review provider: external Claude; local review does not
  satisfy complete.` line, add explicit `Harden required before approve:
  yes` line.
- `.scafld/specs/drafts/rust-contracts-bootstrap.md` (partial, shared) -
  same four additions.
- `.scafld/specs/drafts/rust-state-machine-parity.md` (partial, shared) -
  same four additions.
- `.scafld/specs/drafts/rust-policy-parity.md` (partial, shared) - same
  four additions.
- `.scafld/specs/drafts/rust-parity-ci-governance.md` (partial, shared) -
  same four additions, plus raise `Profile:` to `strict` if not already.

Acceptance:
- [ ] `ac1_1` command - every sub-spec has strict profile.
  - Command: `for s in rust-contracts-bootstrap rust-kernel-parity-fixtures rust-state-machine-parity rust-policy-parity rust-parity-ci-governance; do rg -n 'Profile: strict' ".scafld/specs/drafts/$s.md" >/dev/null || exit 1; done`
  - Expected kind: `exit_code_zero`
  - Status: pending
- [ ] `ac1_2` command - every sub-spec names self-eval threshold >= 8.
  - Command: `for s in rust-contracts-bootstrap rust-kernel-parity-fixtures rust-state-machine-parity rust-policy-parity rust-parity-ci-governance; do rg -n 'Self-eval threshold: [89]' ".scafld/specs/drafts/$s.md" >/dev/null || exit 1; done`
  - Expected kind: `exit_code_zero`
  - Status: pending
- [ ] `ac1_3` command - every sub-spec requires external review.
  - Command: `for s in rust-contracts-bootstrap rust-kernel-parity-fixtures rust-state-machine-parity rust-policy-parity rust-parity-ci-governance; do rg -n 'Review provider: external' ".scafld/specs/drafts/$s.md" >/dev/null || exit 1; done`
  - Expected kind: `exit_code_zero`
  - Status: pending
- [ ] `ac1_4` command - every sub-spec requires harden before approve.
  - Command: `for s in rust-contracts-bootstrap rust-kernel-parity-fixtures rust-state-machine-parity rust-policy-parity rust-parity-ci-governance; do rg -n 'Harden required before approve: yes' ".scafld/specs/drafts/$s.md" >/dev/null || exit 1; done`
  - Expected kind: `exit_code_zero`
  - Status: pending

## Phase 2: Execute contracts bootstrap and rust-kernel-parity-fixtures

Goal: Drive the pre-kernel contracts bootstrap through its full lifecycle, then
drive the fixtures sub-spec through its full lifecycle. This keeps contracts
as a stable dependency before any `runx-core` implementation begins without
requiring full `rust-contracts-parity` to block the kernel.

Status: pending
Dependencies: Phase 1

Steps (the agent runs these in order; this is procedural, not declarative):
1. `scafld harden rust-contracts-bootstrap`. Answer every question.
   Do not skip the harden round.
2. `scafld approve rust-contracts-bootstrap`.
3. Execute the bootstrap sub-spec phases. Run validation per phase.
4. `scafld review rust-contracts-bootstrap`. No workspace edits during
   review. Poll `scafld status --json` for review progress.
5. If review verdict is `fail` or `conditional`, address findings and
   re-review. Do not run `scafld complete` until verdict is `pass`.
6. `scafld complete rust-contracts-bootstrap`.
7. `scafld harden rust-kernel-parity-fixtures`. Answer every question.
8. `scafld approve rust-kernel-parity-fixtures`.
9. Execute the fixtures sub-spec phases. Run validation per phase.
10. `scafld review rust-kernel-parity-fixtures`. No workspace edits during
    review.
11. If review verdict is `fail` or `conditional`, address findings and
    re-review. Do not run `scafld complete` until verdict is `pass`.
12. `scafld complete rust-kernel-parity-fixtures`.
13. Update Phase Receipts (Phase 2 entry) in this orchestration spec.

Acceptance:
- [ ] `ac2_1` command - contracts bootstrap is completed with review gate pass.
  - Command: `scafld status --json rust-contracts-bootstrap | jq -e '.result.status == "completed"' >/dev/null && p="$(find .scafld/specs -name rust-contracts-bootstrap.md -print -quit)" && test -n "$p" && rg -n 'Review gate: pass|Verdict: pass' "$p" >/dev/null`
  - Expected kind: `exit_code_zero`
  - Status: pending
- [ ] `ac2_2` command - crate graph guardrails pass.
  - Command: `node scripts/check-rust-crate-graph.mjs && node scripts/check-rust-core-style.mjs`
  - Expected kind: `exit_code_zero`
  - Status: pending
- [ ] `ac2_3` command - fixture sub-spec is completed with review gate pass.
  - Command: `scafld status --json rust-kernel-parity-fixtures | jq -e '.result.status == "completed"' >/dev/null && p="$(find .scafld/specs -name rust-kernel-parity-fixtures.md -print -quit)" && test -n "$p" && rg -n 'Review gate: pass|Verdict: pass' "$p" >/dev/null`
  - Expected kind: `exit_code_zero`
  - Status: pending
- [ ] `ac2_4` command - fixture artifacts exist.
  - Command: `test -d fixtures/kernel/schema && test -d fixtures/kernel/state-machine && test -d fixtures/kernel/policy && test -f scripts/generate-kernel-parity-fixtures.ts && test -f scripts/validate-kernel-fixture-schemas.ts && test -f tests/kernel-parity-fixtures.test.ts && test -f packages/core/src/policy/posix-basename.ts`
  - Expected kind: `exit_code_zero`
  - Status: pending
- [ ] `ac2_5` command - node:path is gone from policy.
  - Command: `! rg -n "from ['\"]node:path['\"]" packages/core/src/policy`
  - Expected kind: `exit_code_zero`
  - Status: pending
- [ ] `ac2_6` command - Phase Receipts populated for Phase 2.
  - Command: `rg -n 'Phase 2 receipt: filled' .scafld/specs/drafts/rust-kernel-port-orchestration.md`
  - Expected kind: `exit_code_zero`
  - Status: pending

## Phase 3: Execute rust-state-machine-parity

Goal: Drive the state-machine parity sub-spec through its full lifecycle.

Status: pending
Dependencies: Phase 2

Steps: same procedural pattern as Phase 2 with task id
`rust-state-machine-parity`.

Acceptance:
- [ ] `ac3_1` command - sub-spec completed with review gate pass.
  - Command: `scafld status --json rust-state-machine-parity | jq -e '.result.status == "completed"' >/dev/null && p="$(find .scafld/specs -name rust-state-machine-parity.md -print -quit)" && test -n "$p" && rg -n 'Review gate: pass|Verdict: pass' "$p" >/dev/null`
  - Expected kind: `exit_code_zero`
  - Status: pending
- [ ] `ac3_2` command - runx-core state-machine exists and tests pass.
  - Command: `cargo test -p runx-core --test state_machine_fixtures && cargo test -p runx-core --test state_machine_proptest && node scripts/check-rust-core-style.mjs`
  - Expected kind: `exit_code_zero`
  - Timeout seconds: 120
  - Status: pending
- [ ] `ac3_3` command - cargo-deny passes.
  - Command: `cargo deny --manifest-path crates/Cargo.toml check bans licenses sources`
  - Expected kind: `exit_code_zero`
  - Status: pending
- [ ] `ac3_4` command - Phase Receipts populated for Phase 3.
  - Command: `rg -n 'Phase 3 receipt: filled' .scafld/specs/drafts/rust-kernel-port-orchestration.md`
  - Expected kind: `exit_code_zero`
  - Status: pending

## Phase 4: Execute rust-policy-parity

Goal: Drive the policy parity sub-spec through its full lifecycle.

Status: pending
Dependencies: Phase 3

Steps: same procedural pattern as Phase 2 with task id
`rust-policy-parity`.

Acceptance:
- [ ] `ac4_1` command - sub-spec completed with review gate pass.
  - Command: `scafld status --json rust-policy-parity | jq -e '.result.status == "completed"' >/dev/null && p="$(find .scafld/specs -name rust-policy-parity.md -print -quit)" && test -n "$p" && rg -n 'Review gate: pass|Verdict: pass' "$p" >/dev/null`
  - Expected kind: `exit_code_zero`
  - Status: pending
- [ ] `ac4_2` command - policy fixtures pass on both sides.
  - Command: `cargo test -p runx-core --test policy_fixtures && cargo test -p runx-core --test policy_proptest && node scripts/check-rust-core-style.mjs && pnpm exec vitest run --config vitest.config.ts tests/kernel-parity-fixtures.test.ts`
  - Expected kind: `exit_code_zero`
  - Timeout seconds: 180
  - Status: pending
- [ ] `ac4_3` command - every rejection variant has at least one fixture.
  - Command: `pnpm exec tsx scripts/check-rejection-variant-coverage.ts`
  - Expected kind: `exit_code_zero`
  - Status: pending
- [ ] `ac4_4` command - Phase Receipts populated for Phase 4.
  - Command: `rg -n 'Phase 4 receipt: filled' .scafld/specs/drafts/rust-kernel-port-orchestration.md`
  - Expected kind: `exit_code_zero`
  - Status: pending

## Phase 5: Execute rust-parity-ci-governance

Goal: Drive the advisory CI governance sub-spec through its full lifecycle.
Blocking promotion is not part of this spec; it is owned by
`rust-kernel-blocking-promotion` after 5 clean kernel-touching PRs land.

Status: pending
Dependencies: Phase 4

Steps: same procedural pattern as Phase 2 with task id
`rust-parity-ci-governance`.

Acceptance:
- [ ] `ac5_1` command - sub-spec completed with review gate pass.
  - Command: `scafld status --json rust-parity-ci-governance | jq -e '.result.status == "completed"' >/dev/null && p="$(find .scafld/specs -name rust-parity-ci-governance.md -print -quit)" && test -n "$p" && rg -n 'Review gate: pass|Verdict: pass' "$p" >/dev/null`
  - Expected kind: `exit_code_zero`
  - Status: pending
- [ ] `ac5_2` command - wrapper script exists.
  - Command: `test -f scripts/check-rust-core-style.mjs && test -f scripts/check-rust-kernel-parity.mjs && test -f crates/runx-core/api-snapshot.txt`
  - Expected kind: `exit_code_zero`
  - Status: pending
- [ ] `ac5_3` command - new Rust CI checks are advisory.
  - Command: `rg -n 'continue-on-error: true' .github/workflows/ci.yml | rg -qE 'cargo-deny|cargo public-api|check-rust-kernel-parity'`
  - Expected kind: `exit_code_zero`
  - Status: pending
- [ ] `ac5_4` command - Phase Receipts populated for Phase 5.
  - Command: `rg -n 'Phase 5 receipt: filled' .scafld/specs/drafts/rust-kernel-port-orchestration.md`
  - Expected kind: `exit_code_zero`
  - Status: pending

## Phase 6: Coherence verification

Goal: Final check that all artifacts produced by phases 2-5 are still
present, validate against schema, and that the cross-spec dependency graph
is intact.

Status: pending
Dependencies: Phase 5

Changes:
- `.scafld/specs/drafts/rust-kernel-port-orchestration.md` (partial, exclusive) -
  Write the final Phase Receipts summary and update Current State to
  reflect the orchestration as ready for `scafld complete`.

Acceptance:
- [ ] `ac6_1` command - all kernel fixtures still validate.
  - Command: `pnpm exec tsx scripts/validate-kernel-fixture-schemas.ts`
  - Expected kind: `exit_code_zero`
  - Status: pending
- [ ] `ac6_2` command - generator check mode is clean.
  - Command: `pnpm exec tsx scripts/generate-kernel-parity-fixtures.ts --check`
  - Expected kind: `exit_code_zero`
  - Status: pending
- [ ] `ac6_3` command - Rust kernel parity is clean.
  - Command: `node scripts/check-rust-kernel-parity.mjs`
  - Expected kind: `exit_code_zero`
  - Status: pending
- [ ] `ac6_4` command - no cross-spec orphaned references.
  - Command: `for s in rust-contracts-bootstrap rust-kernel-parity-fixtures rust-state-machine-parity rust-policy-parity rust-parity-ci-governance rust-kernel-blocking-promotion; do scafld validate --json "$s" | jq -e '.result.valid == true' >/dev/null || exit 1; done`
  - Expected kind: `exit_code_zero`
  - Status: pending
- [ ] `ac6_5` command - all Phase Receipts populated.
  - Command: `! rg -n '<to be filled at exec time>' .scafld/specs/drafts/rust-kernel-port-orchestration.md`
  - Expected kind: `exit_code_zero`
  - Status: pending

## Cross-session resume protocol

When a fresh agent picks up this spec mid-port:

1. Read `## Current State` in this file. Identify the in-progress phase.
2. Read `## Phase Receipts` and confirm which phases are filled vs not.
3. For the in-progress phase's sub-spec, run
   `scafld status --json <sub-spec-id>`.
4. Read the sub-spec's `## Current State` block.
5. Continue from the next pending action in the in-progress phase's Steps
   block.

Do not re-derive port progress from `git log`, file timestamps, or
guesswork. The Current State and Phase Receipts are the only authoritative
sources.

## Anti-patterns

The agent must not:

- Approve any sub-spec without running `scafld harden` first.
- Declare any phase complete without `scafld review` verdict `pass`.
- Edit any workspace file while `scafld review` is running. The mutation
  guard discards the packet; an edit kills the review.
- Modify a sub-spec's Scope, Objectives, or Acceptance after `scafld
  approve`. If scope must change, stop execution, record a deviation in the
  spec, and get explicit user approval before continuing.
- Defer in-scope sub-spec work by relabeling it "out of scope mid
  execution."
- Lower validation thresholds, fixture counts, or acceptance criteria when
  tests fail. Fix the test or the code; do not move the goalpost.
- Run `scafld complete` on this orchestration spec while any sub-spec is
  still in_progress.
- Run sub-specs in parallel. Scafld config has `parallel_execution: false`
  for a reason.
- Skip Phase Receipts updates. A phase is not complete without a Receipt.
- Re-derive port progress from git log or guesswork on resume.
- Claim "good enough" when self-eval falls below threshold 9. Perform the
  second pass.
- Add test-only branches in production code. The repo invariant
  `no_test_logic_in_production: true` applies.

## Rollback

Strategy: per_phase

A rollback unit is one phase of this orchestration. Sub-spec rollback is
delegated to each sub-spec's own rollback section.

Commands per phase:
- Phase 1: revert sub-spec acceptance frontmatter changes.
- Phase 2-5: follow each sub-spec's own rollback section manually, then clear
  that phase's Receipt. Scafld does not provide a rollback command.
- Phase 6: revert documentation finalization changes.

Whole-port rollback is the sequence: Phase 6 -> 5 -> ... -> 1, executed
explicitly. There is no atomic whole-port revert; this is by design.

## Review

Status: not_started
Verdict: none
Timestamp: none
Review rounds: none
Reviewer mode: none
Reviewer session: none
Round status: none
Override applied: none
Override reason: none
Override confirmed at: none
Reviewed head: none
Reviewed dirty: none
Reviewed diff: none
Blocking count: none
Non-blocking count: none

Reviewer requirements specific to this orchestration spec:
- Reviewer model: claude-opus-4-7.
- Reviewer must run the adversarial passes `scope_drift`, `regression_hunt`,
  `convention_check`, `dark_patterns`.
- Reviewer must verify Phase Receipts are populated and consistent.
- Reviewer must verify all five orchestrated sub-specs are at status completed with
  their own review gates passed.
- Local review provider does not satisfy complete.

Findings:
- none

Passes:
- none

## Self Eval

Status: not_started
Completeness: none
Architecture fidelity: none
Spec alignment: none
Validation depth: none
Total: none
Second pass performed: none

Threshold: 9 (raised from default 7; below this triggers second pass).

Notes:
none

Improvements:
- none

## Deviations

- none

## Phase Receipts

Phase 1 receipt: <to be filled at exec time>
- sub_spec: n/a (this phase modifies sub-specs in this workspace)
- commit_sha: <to be filled at exec time>
- review_verdict: <to be filled at exec time>
- reviewer_model: <to be filled at exec time>
- self_eval_score: <to be filled at exec time>
- artifacts:
  - <to be filled at exec time>
- deviations: <to be filled at exec time>

Phase 2 receipt: <to be filled at exec time>
- sub_spec: rust-contracts-bootstrap and rust-kernel-parity-fixtures
- commit_sha: <to be filled at exec time>
- review_verdict: <to be filled at exec time>
- reviewer_model: <to be filled at exec time>
- self_eval_score: <to be filled at exec time>
- artifacts:
  - scripts/check-rust-crate-graph.mjs
  - scripts/check-rust-core-style.mjs
  - package.json (`rust:crate-graph`, `rust:style`)
  - scripts/verify-fast.mjs (Rust guardrails wired)
  - crates/*/Cargo.toml publish/dependency guardrails
  - fixtures/kernel/schema/fixture.schema.json
  - fixtures/kernel/schema/state-machine.schema.json
  - fixtures/kernel/schema/policy.schema.json
  - fixtures/kernel/state-machine/*.json (final count: <to be filled at exec time>)
  - fixtures/kernel/policy/*.json (final count: <to be filled at exec time>)
  - scripts/generate-kernel-parity-fixtures.ts
  - scripts/validate-kernel-fixture-schemas.ts
  - scripts/check-fixture-key-order.ts
  - tests/kernel-parity-fixtures.test.ts
  - packages/core/src/policy/posix-basename.ts
  - packages/core/src/policy/posix-basename.test.ts
- deviations: <to be filled at exec time>

Phase 3 receipt: <to be filled at exec time>
- sub_spec: rust-state-machine-parity
- commit_sha: <to be filled at exec time>
- review_verdict: <to be filled at exec time>
- reviewer_model: <to be filled at exec time>
- self_eval_score: <to be filled at exec time>
- artifacts:
  - crates/runx-core/Cargo.toml
  - crates/runx-core/src/lib.rs
  - crates/runx-core/src/state_machine.rs
  - crates/runx-core/src/state_machine/types.rs
  - crates/runx-core/src/state_machine/single_step.rs
  - crates/runx-core/src/state_machine/sequential_graph.rs
  - crates/runx-core/src/state_machine/fanout.rs
  - crates/runx-core/src/serde_conventions.rs
  - crates/runx-core/tests/state_machine_fixtures.rs
  - crates/runx-core/tests/state_machine_proptest.rs
  - crates/deny.toml
  - scripts/check-rust-core-style.mjs
- deviations: <to be filled at exec time>

Phase 4 receipt: <to be filled at exec time>
- sub_spec: rust-policy-parity
- commit_sha: <to be filled at exec time>
- review_verdict: <to be filled at exec time>
- reviewer_model: <to be filled at exec time>
- self_eval_score: <to be filled at exec time>
- artifacts:
  - crates/runx-core/src/policy.rs
  - crates/runx-core/src/policy/authority_proof.rs
  - crates/runx-core/src/policy/connected_auth.rs
  - crates/runx-core/src/policy/graph_scope.rs
  - crates/runx-core/src/policy/interpreter.rs
  - crates/runx-core/src/policy/local.rs
  - crates/runx-core/src/policy/payment_authority.rs
  - crates/runx-core/src/policy/types.rs
  - crates/runx-core/src/policy/sandbox.rs
  - crates/runx-core/src/policy/scope.rs
  - crates/runx-core/src/policy/posix_basename.rs
  - crates/runx-core/src/policy/public_work.rs
  - crates/runx-core/src/policy/retry.rs
  - crates/runx-core/tests/policy_fixtures.rs
  - crates/runx-core/tests/policy_proptest.rs
  - scripts/check-rejection-variant-coverage.ts
- deviations: <to be filled at exec time>

Phase 5 receipt: <to be filled at exec time>
- sub_spec: rust-parity-ci-governance (Phases 1-3 only; Phase 4 deferred)
- commit_sha: <to be filled at exec time>
- review_verdict: <to be filled at exec time>
- reviewer_model: <to be filled at exec time>
- self_eval_score: <to be filled at exec time>
- artifacts:
  - scripts/check-rust-kernel-parity.mjs
  - crates/runx-core/api-snapshot.txt
  - .github/workflows/ci.yml (advisory checks added)
- deviations: <to be filled at exec time>

Phase 6 receipt: <to be filled at exec time>
- coherence_check_result: <to be filled at exec time>
- final_commit_sha: <to be filled at exec time>
- cumulative_deviations: <to be filled at exec time>

## Metadata

Estimated effort hours: 70 (orchestration only; sub-spec hours separate
in their own metadata blocks)
Actual effort hours: none
AI model: none
React cycles: none

Tags:
- rust
- trusted-kernel
- orchestration
- governance
- meta

## Origin

Source:
- user requested an umbrella spec leveraging scafld features to drive the
  Rust pure-kernel port without data loss across agent sessions.

Repo:
- runxhq/runx

Git:
- none

Sync:
- none

Supersession:
- coordinates: rust-contracts-bootstrap
- coordinates: rust-kernel-parity-fixtures
- coordinates: rust-state-machine-parity
- coordinates: rust-policy-parity
- coordinates: rust-parity-ci-governance

## Harden Rounds

- none

## Planning Log

- 2026-05-17T00:00:00Z: Drafted as umbrella orchestration for the four
  kernel parity sub-specs. Leverages scafld strict execution profile,
  mandatory external review, harden-before-approve, per-phase
  checkpointing, rollback-on-fail, and Phase Receipts (an extension to
  the standard layout) to prevent agent data loss across the multi-week
  port. Self-eval threshold raised from default 7 to 9. CLI parity and
  runtime cutover specs are deliberately out of scope; their own
  orchestration specs come later.
- 2026-05-17T02:10:00Z: Added `rust-contracts-bootstrap` as a pre-kernel
  gate. The full contracts parity port remains separate, but the crate graph,
  placeholder publish-readiness policy, and Rust guardrail scripts must
  complete before fixtures and kernel implementation begin. Reviews are
  Claude-only.
