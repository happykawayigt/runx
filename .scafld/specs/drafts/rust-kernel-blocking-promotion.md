---
spec_version: '2.0'
task_id: rust-kernel-blocking-promotion
created: '2026-05-17T00:30:00Z'
updated: '2026-05-22T00:30:13+10:00'
status: draft
harden_status: not_run
size: medium
risk_level: high
---

# Rust kernel blocking promotion

## Current State

Status: draft
Current phase: planning/spec-only reconciliation; evidence script landed; soak
evidence pending
Next: continue soak verification by running the clean-kernel counter against
live GitHub metadata or an audited operator fixture
Reason: CI still marks Rust kernel parity advisory. The obsolete umbrella
orchestration spec has been superseded by narrow slices, and clean-PR counter
semantics are now locked by `rust-kernel-clean-pr-counter-semantics`.
Conservative advisory-start evidence is recorded from the archived completed
`rust-parity-ci-governance` spec, but five qualifying post-advisory PRs are
still missing.
Blockers: 5 clean kernel-touching PRs landed while Rust kernel parity checks
are advisory. Current full local `node scripts/check-rust-kernel-parity.mjs`
passes, but that only clears the local parity prerequisite; it does not satisfy
the five-PR soak gate or authorize the CI flip.
Execution lock: CI blocking promotion is not executable until the clean-kernel
counter returns at least 5 qualifying post-advisory PRs from live GitHub
metadata or audited operator evidence. While the counter remains below 5,
Phase 3 and Phase 4 stay planning-only and no workflow or docs flip may be
made under this draft.
Allowed follow-up command: run the evidence script against audited evidence; do
not run `scafld harden rust-kernel-blocking-promotion`.
Counter update: 2026-05-20 clean-kernel counter live-GitHub mode now
requires parseable advisory-start timestamps for timestamped PR metadata,
requires live GitHub records to include post-advisory merge times, and requires
the Rust kernel parity check itself to pass. A read-only live probe from
`2026-05-20T00:00:00Z` found zero qualifying kernel PRs; the checked-in fixture
still has four qualifying records, so the CI promotion remains blocked.
Latest local evidence update: 2026-05-21T22:14:35+10:00 reran the full Rust
kernel parity gate after aligning the credential-envelope fixtures, tightening
the `mcp-rmcp` crate-graph guard, clearing Rust style findings, and refreshing
the stale `runx-core` public API snapshot. `node
scripts/check-rust-kernel-parity.mjs` passed. This evidence does not satisfy
the live five-PR soak gate and does not authorize the CI flip.
Latest soak evidence update: 2026-05-22T00:30:13+10:00 reran the clean-kernel
counter tests and fixture/live count commands. Counter tests passed, fixture
mode still counts four qualifying records, and live GitHub mode against
`runxhq/runx` still counts zero qualifying PRs after the conservative advisory
start. The five-PR soak gate remains blocked.
Safe evidence/planning update: 2026-05-21T03:19:54Z recorded a conservative
advisory-start timestamp of `2026-05-19T03:33:01Z` from the completed archived
`rust-parity-ci-governance` spec. Fixture mode still counts 4 qualifying PRs;
live GitHub mode against `runxhq/runx` still counts 0 qualifying PRs after
that start.
Prior local parity update: 2026-05-21T04:08:33Z re-ran the full local wrapper
and failed on a rustfmt check. That evidence is superseded by the
2026-05-21T22:14:35+10:00 passing wrapper run, but the five-PR soak gate still
blocks CI promotion.
Review gate: not_started

## Summary

Promote Rust kernel parity from advisory CI signal to blocking CI gate after
the advisory soak proves the dual-tree workflow is sustainable. This spec is
deliberately separate from `rust-parity-ci-governance` because scafld specs are
completed once; the advisory-to-blocking flip is a later operational decision,
not a phase that can be reopened inside the completed governance spec.

The trigger is not calendar time. The trigger is 5 clean kernel-touching PRs
merged after advisory CI lands. A clean kernel-touching PR touches
the still-dual TypeScript state-machine/policy oracle surface or a deliberate
kernel fixture refresh tied to that surface, runs the Rust parity checks, and
either passes them directly or includes an intentional fixture refresh that
makes both the relevant TypeScript oracle and Rust pass. This counting rule is
soak evidence only; it does not make TypeScript broadly authoritative for graph
execution or any already-cut-over local runtime bridge.

Refresh note, 2026-05-20: current CI still contains
`continue-on-error: true` on the `Advisory Rust kernel parity` step, so Phase A
is still advisory. `scripts/count-clean-kernel-prs.ts`, fixture data, tests,
and counter semantics are present and pass against local fixtures, but the
Evidence section below is still intentionally unfilled for live post-advisory
PRs. This draft must not be treated as ready for CI promotion.

Bridge-reality reconciliation, 2026-05-22: current architecture docs state that
trusted local CLI/runtime behavior, including graph execution, has moved to
Rust-owned paths and surviving TypeScript packages may only wrap or bridge
those paths without a hidden runtime-local fallback. This draft therefore talks
about TypeScript only as a remaining state-machine/policy oracle and soak
counter input for the original kernel parity gate. It must not be read as a
claim that TypeScript is broadly authoritative for graph execution.

## Context

CWD: `.`

Packages:
- `@runxhq/core`
- `crates/runx-core`
- `crates/runx-contracts`
- `crates/runx-parser`

Files impacted:
- `.github/workflows/ci.yml`
- `scripts/count-clean-kernel-prs.ts`
- `docs/rust-kernel-architecture.md`
- `docs/trusted-kernel-package-truth.md`
- `CONTRIBUTING.md`

Invariants:
- For still-dual kernel parity surfaces, the relevant TypeScript
  state-machine/policy oracle remains the comparison input until a later
  sunset/cutover spec changes that specific consumer.
- TypeScript is not broadly authoritative for graph execution, runtime-local
  orchestration, harness execution, CLI execution, or already-cut-over local
  bridge paths. This promotion does not reopen those cutover decisions.
- Rust kernel parity checks do not become blocking until 5 clean
  kernel-touching PRs are evidenced.
- CI blocking promotion is not executable while the clean-kernel counter is
  below 5, even if local Rust parity passes.
- The promotion only affects Rust kernel parity checks added by
  `rust-parity-ci-governance`. It must not remove existing Rust launcher CI
  checks or change npm CLI release behavior.
- The CLI feature-parity matrix remains required for any runtime or CLI
  cutover. Blocking kernel parity is not a runtime cutover.
- If the 5-PR evidence is missing or ambiguous, this spec blocks.
- Promotion does not make Rust authoritative for still-dual state-machine or
  policy oracle surfaces, and it does not make TypeScript authoritative for
  graph execution or bridge-owned runtime behavior.
- Parser-only PRs are not counted toward the original 5-PR trigger unless this
  spec is explicitly broadened and hardened again. The current trigger is
  state-machine/policy soak evidence.

Related docs:
- `docs/rust-kernel-architecture.md`
- `docs/trusted-kernel-package-truth.md`
- `.scafld/specs/drafts/rust-kernel-port-orchestration.md`
- `.scafld/specs/drafts/rust-kernel-clean-pr-counter-semantics.md`
- `.scafld/specs/drafts/rust-parity-ci-governance.md`

## Objectives

- Add a deterministic way to count clean kernel-touching PRs.
- Verify at least 5 qualifying PRs landed while Rust kernel checks were
  advisory.
- Remove `continue-on-error: true` from the Rust kernel parity checks added by
  `rust-parity-ci-governance`.
- Update docs to state Phase B is active and kernel parity is blocking.
- Keep runtime, MCP, adapter, parser, receipt, and CLI cutover work out of
  scope.

## Scope

In scope:
- Evidence collection for the 5 clean kernel-touching PR trigger.
- CI promotion from advisory to blocking for Rust kernel parity checks, but
  only after the counter reaches 5.
- Documentation updates that mark Phase B active, but only after the counter
  reaches 5 and the CI flip is actually made.

Out of scope:
- Any Rust implementation changes.
- Any TypeScript kernel behavior changes.
- Any parser, receipt, runtime-local, MCP, adapter, or CLI cutover.
- Any claim that TypeScript owns graph execution or local runtime bridge
  behavior.
- Publishing Cargo crates.

## Dependencies

- `rust-kernel-port-orchestration` marked obsolete as written and superseded
  by fresh executable slices.
- `rust-kernel-clean-pr-counter-semantics` completed locally.
- `rust-parity-ci-governance` completed with advisory CI checks present.
- GitHub CLI or equivalent CI metadata access is available for counting merged
  PRs. If not available, the operator must provide audited PR evidence and the
  script must support a checked-in fixture mode.

## Assumptions

- The advisory start timestamp should be recorded in a replacement audited
  evidence block. The obsolete `rust-kernel-port-orchestration` file has no
  valid Phase Receipts and must not be backfilled. If audited evidence does not
  exist, the script must refuse to infer it from file timestamps or git history
  alone.
- The CI workflow names for Rust kernel parity checks are stable enough for
  `scripts/count-clean-kernel-prs.ts` to identify them.
- Some qualifying PRs may include fixture refreshes. That is acceptable only if
  the PR also shows both TypeScript and Rust parity passing after refresh.

## Risks

- High: promoting too early can block normal TypeScript kernel work. Mitigated
  by the 5 clean PR evidence gate.
- High: PR counting can be wrong if CI check names change. Mitigated by a
  script with explicit check-name configuration and fixture tests.
- Medium: advisory checks may have been bypassed manually. Mitigated by
  requiring PR-level evidence, not just current branch green status.
- Medium: CI promotion can accidentally affect existing launcher checks.
  Mitigated by limiting the workflow diff to the parity checks added by
  `rust-parity-ci-governance`.

## 2026-05-20 Refresh Findings

Observed current state:
- `.github/workflows/ci.yml` has a blocking `Rust checks` step for cargo
  fmt/clippy/test/package, followed by `Advisory Rust kernel parity` with
  `continue-on-error: true`.
- `package.json` maps `pnpm rust:check` to
  `node scripts/check-rust-kernel-parity.mjs`.
- `scripts/check-rust-kernel-parity.mjs` runs Cargo fmt, clippy, workspace
  tests, crate-graph/style guards, cargo-deny, and the `runx-core` public API
  snapshot unless `--api-only` is used.
- `docs/trusted-kernel-package-truth.md` still says CI is advisory during
  Phase A and becomes blocking only through this spec after five clean
  kernel-touching PRs.
- `scripts/count-clean-kernel-prs.ts` exists with fixture-mode and live GitHub
  metadata modes, and fails closed without advisory-start evidence. Timestamped
  PR metadata also requires a parseable advisory-start timestamp so live
  counting cannot infer the soak window from prose.
- Clean PR evidence remains `<to be filled at exec time>`.
- 2026-05-20 local API evidence: `node scripts/check-rust-kernel-parity.mjs
  --api-only` initially failed because `crates/runx-core/api-snapshot.txt` was
  stale after the kernel JSON bridge and payment authority subset API were
  exported. The snapshot was regenerated with the command printed by the gate,
  and the API-only parity check now passes.
- 2026-05-21 local full-gate evidence: `node
  scripts/check-rust-kernel-parity.mjs` passes after regenerating the
  `runx-core` public API snapshot. The run covered cargo fmt, clippy, workspace
  tests, crate-graph/style guards, cargo-deny, and the API snapshot gate.

## Gate Classification

Blocking before this spec may promote CI:
- The conservative advisory start point has been recorded as explicit
  evidence, but any earlier start point must come from audited operator
  evidence before it replaces this timestamp.
- `scripts/count-clean-kernel-prs.ts` must verify at least five qualifying
  post-advisory PRs from live metadata or audited evidence.
- `node scripts/check-rust-kernel-parity.mjs` must pass locally before the CI
  `continue-on-error` line is removed.
- Until the clean-kernel counter reaches 5, Phase 3 CI promotion is not an
  executable slice. Operators may only update planning/evidence text or rerun
  read-only counter verification.

Advisory until those blockers clear:
- Existing CI `Advisory Rust kernel parity`.
- Cargo-deny and public API snapshot enforcement inside CI, because they are
  currently inside the advisory parity step.
- Rust-only maintenance PRs that do not exercise TypeScript kernel drift.

Non-goals for this promotion:
- Runtime, parser, receipt, adapter, MCP, SDK, or CLI cutover.
- Making Rust policy authoritative for runtime-local execution.
- Reasserting TypeScript authority over graph execution or local runtime bridge
  behavior that current bridge docs assign to Rust-owned paths.
- Changing runtime payment code or payment rails.

## Clean PR Evidence Rules

The counting script must fail closed:
- Qualifying PRs must merge after the recorded advisory start.
- A qualifying PR must touch the still-dual TypeScript state-machine or policy
  oracle surface, or a deliberate kernel fixture/oracle refresh tied to that
  surface.
- A Rust-only `crates/runx-core` maintenance PR can be recorded as advisory
  evidence, but it does not count toward the five-PR promotion trigger unless
  this spec is explicitly broadened.
- Fixture refresh PRs qualify only when both the TypeScript oracle and Rust
  parity pass after refresh.
- Missing, renamed, skipped, or failed parity checks make the PR non-qualifying
  unless audited evidence is checked into the fixture mode.
- Parser-only PRs remain out of the five-PR count for this draft, even though
  parser is a pure TypeScript core domain and `runx-parser` now exists.

## Next Executable Slices

Execute these in order:
- Advisory-start evidence slice: record the exact advisory start point from an
  audited source before live PR counting is allowed.
- Soak verification slice: run the script against live GitHub metadata or an
  operator-provided audited fixture and fill the Evidence section only after
  the minimum is met.
- CI flip slice: after the clean-kernel counter reaches 5 and all blockers
  pass, remove only the `continue-on-error: true` on the Rust kernel parity
  step.
- Docs coherence slice: update docs to say Phase B is active only after the CI
  flip is made. Until then, docs must keep Phase A/advisory language.

## Acceptance

Profile: strict
Self-eval threshold: 8
Review provider: external Claude; local review does not satisfy
complete.
Harden required before approve: yes

Definition of done:
- [x] `dod1` `scripts/count-clean-kernel-prs.ts` exists and has fixture tests.
- [ ] `dod2` The script verifies at least 5 clean kernel-touching PRs.
- [ ] `dod3` Rust kernel parity checks are blocking in CI.
- [ ] `dod4` Docs state Phase B is active only after the CI flip, preserve
  still-dual state-machine/policy oracle rules, and avoid claiming TypeScript
  is broadly authoritative for graph execution.
- [ ] `dod5` Runtime/CLI cutover language still points to the CLI
  feature-parity matrix.

Validation:
- [ ] `v1` command - Rust kernel parity still passes.
  - Command: `node scripts/check-rust-kernel-parity.mjs`
  - Expected kind: `exit_code_zero`
  - Status: failed 2026-05-21T04:08:33Z because `cargo fmt --check` reported
    an unformatted diff in existing untracked
    `crates/runx-cli/tests/locality.rs`.
- [ ] `v2` command - 5 clean kernel-touching PRs are evidenced.
  - Command: `pnpm exec tsx scripts/count-clean-kernel-prs.ts --min 5`
  - Expected kind: `exit_code_zero`
  - Timeout seconds: 60
  - Status: failed 2026-05-22T00:30:13+10:00 with 4 qualifying fixture records,
    below the required 5.
- [ ] `v3` command - Rust kernel parity checks are no longer advisory.
  - Command: `! rg -n 'continue-on-error: true' .github/workflows/ci.yml | rg -qE 'cargo-deny|cargo public-api|check-rust-kernel-parity'`
  - Expected kind: `exit_code_zero`
  - Status: pending
- [ ] `v4` command - docs state blocking phase is active.
  - Command: `rg -n 'Phase B.*active|blocking.*kernel parity|kernel parity.*blocking' docs CONTRIBUTING.md`
  - Expected kind: `exit_code_zero`
  - Status: pending

## Phase 1: Evidence script

Goal: Add a script that identifies clean kernel-touching PRs and can be tested
without live GitHub access.

Status: completed
Dependencies: none

Changes:
- `scripts/count-clean-kernel-prs.ts` (all, exclusive) - Count merged PRs that
  touch `packages/core/src/state-machine/` or `packages/core/src/policy/` and
  have passing Rust kernel parity checks, from either audited fixture evidence
  or live GitHub PR metadata.
- `tests/count-clean-kernel-prs.test.ts` (all, exclusive) - Fixture tests for
  clean PRs, non-kernel PRs, missing CI checks, failed CI checks, fixture
  refresh PRs, and live GitHub response normalization.

Acceptance:
- [x] `ac1_1` command - script tests pass.
  - Command: `pnpm exec vitest run --config vitest.config.ts tests/count-clean-kernel-prs.test.ts`
  - Expected kind: `exit_code_zero`
  - Status: passed 2026-05-20, 10 tests
- [x] `ac1_2` command - script exposes fixture mode.
  - Command: `pnpm exec tsx scripts/count-clean-kernel-prs.ts --fixture tests/fixtures/clean-kernel-prs.json --min 1`
  - Expected kind: `exit_code_zero`
  - Status: passed 2026-05-20 by local fixture run with `--min 1`; fixture
    currently counts 4 qualifying local evidence records.

## Phase 2: Soak verification

Goal: Verify 5 clean kernel-touching PRs using live metadata or audited
operator-provided fixture evidence.

Status: pending
Dependencies: Phase 1

Acceptance:
- [ ] `ac2_1` command - 5 qualifying PRs found.
  - Command: `pnpm exec tsx scripts/count-clean-kernel-prs.ts --min 5`
  - Expected kind: `exit_code_zero`
  - Timeout seconds: 60
  - Status: failed 2026-05-22T00:30:13+10:00 with 4 qualifying fixture records,
    below the required 5.
- [ ] `ac2_2` command - evidence is recorded in this spec.
  - Command: `rg -n 'Clean PR evidence: filled' .scafld/specs/drafts/rust-kernel-blocking-promotion.md`
  - Expected kind: `exit_code_zero`
  - Status: pending

## Phase 3: CI promotion

Goal: Remove advisory markers from the Rust kernel parity checks only.

Status: pending
Dependencies: Phase 2
Execution lock: not executable until Phase 2 verifies at least 5 qualifying
post-advisory PRs. A passing local parity wrapper is necessary but not
sufficient.

Changes:
- `.github/workflows/ci.yml` (partial, shared) - Remove `continue-on-error:
  true` from Rust kernel parity checks added by `rust-parity-ci-governance`.
  Do not alter pre-existing Rust launcher checks.

Acceptance:
- [ ] `ac3_1` command - parity checks are blocking.
  - Command: `! rg -n 'continue-on-error: true' .github/workflows/ci.yml | rg -qE 'cargo-deny|cargo public-api|check-rust-kernel-parity'`
  - Expected kind: `exit_code_zero`
  - Status: pending
- [ ] `ac3_2` command - parity wrapper still passes.
  - Command: `node scripts/check-rust-kernel-parity.mjs`
  - Expected kind: `exit_code_zero`
  - Status: pending

## Phase 4: Docs

Goal: Mark Phase B active while preserving the still-dual
state-machine/policy oracle rules, current Rust-owned graph/runtime bridge
boundaries, and CLI feature-parity cutover rules.

Status: pending
Dependencies: Phase 3

Changes:
- `docs/rust-kernel-architecture.md` (partial, shared) - Mark Phase B active.
- `docs/trusted-kernel-package-truth.md` (partial, shared) - State that Rust
  kernel parity blocks state-machine/policy parity drift but does not authorize
  runtime/CLI cutover or reassign graph execution authority to TypeScript.
- `CONTRIBUTING.md` (partial, shared) - Document the blocking local check.

Acceptance:
- [ ] `ac4_1` command - docs mention Phase B active.
  - Command: `rg -n 'Phase B.*active|blocking.*kernel parity|kernel parity.*blocking' docs CONTRIBUTING.md`
  - Expected kind: `exit_code_zero`
  - Status: pending
- [ ] `ac4_2` command - docs still preserve CLI cutover gate.
  - Command: `rg -n 'cli-feature-parity|feature-parity matrix|runtime.*cutover|CLI.*cutover' docs CONTRIBUTING.md`
  - Expected kind: `exit_code_zero`
  - Status: pending

## Rollback

Strategy: per_phase

Commands:
- Phase 1: remove `scripts/count-clean-kernel-prs.ts` and its tests.
- Phase 2: clear the recorded PR evidence from this spec.
- Phase 3: re-add `continue-on-error: true` to the Rust kernel parity checks.
- Phase 4: revert docs from Phase B active back to advisory language.

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

## Deviations

- none

## Evidence

Advisory-start evidence: conservative timestamp
`2026-05-19T03:33:01Z`, sourced from the archived completed
`.scafld/specs/archive/2026-05/rust-parity-ci-governance.md` `updated`
frontmatter. The same archived spec records advisory CI integration as
completed and hands off blocking promotion to this draft. This is safe to use
for live non-promoting probes because it may under-count, not over-count,
post-advisory PRs. Replace only with stronger audited operator evidence.

Clean PR evidence: blocked; minimum 5 qualifying post-advisory PRs not met.

Local non-promoting evidence, 2026-05-20:
- `pnpm exec vitest run --config vitest.config.ts tests/count-clean-kernel-prs.test.ts`
  passed, 6 tests.
- `pnpm exec tsx scripts/count-clean-kernel-prs.ts --fixture tests/fixtures/clean-kernel-prs.json --min 1`
  passed with 4 fixture-qualified records. This is fixture evidence only, not
  live five-PR soak evidence.
- `node scripts/check-rust-kernel-parity.mjs --api-only` passed after
  regenerating `crates/runx-core/api-snapshot.txt`.
- `pnpm exec vitest run --config vitest.config.ts tests/count-clean-kernel-prs.test.ts`
  passed, 7 tests, after adding live GitHub response normalization coverage.
- `pnpm exec vitest run --config vitest.config.ts tests/count-clean-kernel-prs.test.ts`
  passed, 10 tests, after adding advisory-window enforcement and live parity
  check selection coverage.
- `pnpm exec tsx scripts/count-clean-kernel-prs.ts --fixture tests/fixtures/clean-kernel-prs.json --min 5`
  failed closed with 4 qualifying fixture records, below the required 5.
- `pnpm exec tsx scripts/count-clean-kernel-prs.ts --from-github --repo runxhq/runx --advisory-start 2026-05-20T00:00:00Z --min 5 --limit 20`
  failed closed with 0 qualifying records in the probed live merged-PR window.
  The live records were outside the post-advisory window for this probe. This
  is live non-promoting evidence; the CI flip remains blocked.

Local non-promoting evidence, 2026-05-21T01:11:28Z:
- `pnpm exec vitest run --config vitest.config.ts tests/count-clean-kernel-prs.test.ts`
  passed, 10 tests.
- `pnpm exec tsx scripts/count-clean-kernel-prs.ts --fixture tests/fixtures/clean-kernel-prs.json --min 1`
  passed with 4 fixture-qualified records: PRs 101, 102, 103, and 108.
- `pnpm exec tsx scripts/count-clean-kernel-prs.ts --min 5` failed closed
  with 4 qualifying fixture records, below the required 5.
- `pnpm exec tsx scripts/count-clean-kernel-prs.ts --from-github --repo runxhq/runx --advisory-start 2026-05-20T00:00:00Z --min 5 --limit 100`
  failed closed with 0 qualifying live records. The latest returned merged PR
  was PR 36, merged at 2026-05-14T14:17:35Z, so returned live records were
  outside the probed post-advisory window. This is not audited advisory-start
  evidence and does not authorize the CI flip.
- `rg -n -C 3 'Advisory Rust kernel parity|continue-on-error|check-rust-kernel-parity' .github/workflows/ci.yml`
  confirmed the Rust kernel parity step still has `continue-on-error: true`.

Local non-promoting evidence, 2026-05-21T03:19:54Z:
- `scafld status rust-kernel-blocking-promotion --json` reported the spec is
  still `draft`, with follow-up limited to running the evidence script against
  audited evidence.
- `scafld validate rust-kernel-blocking-promotion --json` passed.
- `pnpm exec vitest run --config vitest.config.ts tests/count-clean-kernel-prs.test.ts`
  passed, 10 tests.
- `pnpm exec tsx scripts/count-clean-kernel-prs.ts --fixture tests/fixtures/clean-kernel-prs.json --min 5`
  failed closed with 4 qualifying fixture records: PRs 101, 102, 103, and 108.
- `pnpm exec tsx scripts/count-clean-kernel-prs.ts --from-github --repo runxhq/runx --advisory-start 2026-05-19T03:33:01Z --min 5 --limit 100`
  failed closed with 0 qualifying live records. The latest returned merged PR
  was PR 36, merged at 2026-05-14T14:17:35Z, which is before the conservative
  advisory start.
- `rg -n -C 3 'Advisory Rust kernel parity|continue-on-error|check-rust-kernel-parity' .github/workflows/ci.yml`
  confirmed the Rust kernel parity step still has `continue-on-error: true`.
- `node scripts/check-rust-kernel-parity.mjs` failed. Cargo fmt/clippy and many
  workspace tests passed first, then `runx-runtime --test dev` failed in
  `dev_runs_deterministic_tool_fixtures_and_skips_excluded_lanes` with
  `left: 3, right: 2` and `dev_marks_workspace_executable_files_executable`
  with `left: 3, right: 1`. This is out of this slice's allowed edit scope and
  blocks any CI promotion.

Local non-promoting evidence, 2026-05-21T03:31:43Z:
- `node scripts/check-rust-crate-graph.mjs` passed after the guard was updated
  to encode the completed async HTTP policy: pure crates still reject
  `tokio`, `reqwest`, `hyper`, `rmcp`, and CLI/protocol frameworks; only
  `runx-runtime` may carry the optional, exact-pinned `async-http` edge with
  `cli-tool = ["async-http"]`.
- `node scripts/check-rust-core-style.mjs` passed after the native dev CLI
  test returned a concrete `serde_json::Error` and large runtime cutover slices
  received explicit style-allow reasons tied to active module-boundary work.
- `cargo fmt --manifest-path crates/Cargo.toml --all -- --check` passed.
- `node scripts/check-rust-kernel-parity.mjs` passed end to end, including
  cargo fmt/check/clippy/workspace tests, crate graph, Rust style,
  cargo-deny, and the public API snapshot gate.
- The CI promotion remains blocked because the five clean post-advisory
  kernel-touching PRs are still not evidenced and `.github/workflows/ci.yml`
  still intentionally keeps the Rust kernel parity step advisory.

Local non-promoting evidence, 2026-05-21T04:08:33Z:
- `scafld status rust-kernel-blocking-promotion --json` reported the spec is
  still `draft`, with follow-up limited to running the evidence script against
  audited evidence.
- `scafld validate rust-kernel-blocking-promotion --json` passed.
- `pnpm exec vitest run --config vitest.config.ts tests/count-clean-kernel-prs.test.ts`
  passed, 10 tests.
- `pnpm exec tsx scripts/count-clean-kernel-prs.ts --fixture tests/fixtures/clean-kernel-prs.json --min 5`
  failed closed with 4 qualifying fixture records: PRs 101, 102, 103, and 108.
- `pnpm exec tsx scripts/count-clean-kernel-prs.ts --min 5` failed closed
  with the same 4 qualifying fixture records, below the required 5.
- `pnpm exec tsx scripts/count-clean-kernel-prs.ts --from-github --repo runxhq/runx --advisory-start 2026-05-19T03:33:01Z --min 5 --limit 100`
  failed closed with 0 qualifying live records. The latest returned merged PR
  was PR 36, merged at 2026-05-14T14:17:35Z, which is before the conservative
  advisory start.
- `node scripts/check-rust-kernel-parity.mjs` failed at the rustfmt check with
  an unformatted diff in existing untracked
  `crates/runx-cli/tests/locality.rs`. This is outside this promotion evidence
  slice's allowed edit scope and blocks any CI promotion.

Local non-promoting evidence, 2026-05-21T22:14:35+10:00:
- `node scripts/check-rust-core-style.mjs` passed after non-semantic style
  guard fixes in the MCP transport, connect, runner step, and payment-ledger
  files.
- `node scripts/check-rust-crate-graph.mjs` passed after the `mcp-rmcp`
  feature contract was narrowed to the exact `rmcp` client dependency and the
  scoped `tokio/process` + `tokio/io-util` features.
- `pnpm contracts:schemas:check` passed after regenerating the credential
  envelope schema.
- `pnpm exec tsc -p tsconfig.typecheck.json --noEmit` passed.
- `cargo test --manifest-path crates/Cargo.toml -p runx-runtime --features
  mcp-rmcp --test mcp_adapter -- --nocapture` passed, 11 tests.
- `cargo test --manifest-path crates/Cargo.toml -p runx-runtime --features
  mcp-rmcp --lib rmcp_transport_tests -- --nocapture` passed, 4 tests.
- `node scripts/check-rust-kernel-parity.mjs` passed end to end after
  regenerating `crates/runx-core/api-snapshot.txt`. The wrapper covered cargo
  fmt/check/clippy/workspace tests, crate graph, Rust style, cargo-deny, and
  the public API snapshot gate.
- CI promotion remains blocked because the five clean post-advisory
  kernel-touching PRs are still not evidenced and `.github/workflows/ci.yml`
  still intentionally keeps the Rust kernel parity step advisory.

Local non-promoting evidence, 2026-05-22T00:30:13+10:00:
- `pnpm exec vitest run --config vitest.config.ts tests/count-clean-kernel-prs.test.ts`
  passed, 10 tests.
- `pnpm exec tsx scripts/count-clean-kernel-prs.ts --fixture tests/fixtures/clean-kernel-prs.json --min 5`
  failed closed with 4 qualifying fixture records: PRs 101, 102, 103, and 108.
- `pnpm exec tsx scripts/count-clean-kernel-prs.ts --min 5` failed closed
  with the same 4 qualifying fixture records, below the required 5.
- `pnpm exec tsx scripts/count-clean-kernel-prs.ts --from-github --repo runxhq/runx --advisory-start 2026-05-19T03:33:01Z --min 5 --limit 100`
  reached live GitHub metadata and failed closed with 0 qualifying live records.
  The latest returned merged PR was PR 36, merged at 2026-05-14T14:17:35Z,
  which is before the conservative advisory start. CI promotion remains blocked.

## Metadata

Estimated effort hours: 6
Actual effort hours: none
AI model: none
React cycles: none

Tags:
- rust
- trusted-kernel
- ci
- governance

## Origin

Source:
- Split from `rust-parity-ci-governance` after validating the installed scafld
  CLI and finding no supported command for reopening a completed spec.

Repo:
- runxhq/runx

Git:
- none

Sync:
- none

Supersession:
- follows: rust-kernel-port-orchestration
- follows: rust-parity-ci-governance

## Harden Rounds

- none

## Planning Log

- 2026-05-17T00:30:00Z: Drafted as the separate blocking-promotion spec. This
  keeps `rust-parity-ci-governance` executable as an advisory CI spec and
  avoids the invalid assumption that a completed scafld spec can be reopened
  with a `deviate` command.
- 2026-05-21T03:19:54Z: Executed the safe evidence/planning slice only. The
  advisory start is now recorded conservatively from the completed governance
  spec, but the fixture/live soak counts remain below 5. CI remains advisory.
- 2026-05-21T03:31:43Z: Re-ran the local parity wrapper after the async HTTP
  crate-graph guard update and runtime dev executable-fixture fix; the wrapper
  now passes, but the five-PR soak evidence remains the promotion blocker.
- 2026-05-21T04:08:33Z: Executed the next safe soak-verification slice. Fixture
  mode still counts 4 qualifying PRs, live GitHub mode against `runxhq/runx`
  still counts 0 qualifying PRs after the conservative advisory start, and the
  full parity wrapper now fails on rustfmt for existing untracked
  `crates/runx-cli/tests/locality.rs`. CI remains advisory.
- 2026-05-21T22:14:35+10:00: Re-ran the full local parity wrapper after the
  credential-envelope cutover, MCP feature-gate cleanup, Rust style cleanup,
  and `runx-core` API snapshot regeneration. The wrapper passes end to end.
  CI remains advisory because the five clean post-advisory PR soak evidence is
  still missing.
- 2026-05-22T00:30:13+10:00: Re-ran the clean-kernel counter evidence slice.
  Counter tests passed, fixture/default count mode still found only 4
  qualifying records, and live GitHub mode found 0 qualifying records after
  the conservative advisory start. CI remains advisory.
