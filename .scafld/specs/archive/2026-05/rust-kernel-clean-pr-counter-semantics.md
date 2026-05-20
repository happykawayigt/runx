---
spec_version: '2.0'
task_id: rust-kernel-clean-pr-counter-semantics
created: '2026-05-20T00:00:00Z'
updated: '2026-05-20T00:56:23Z'
status: completed
harden_status: not_run
size: small
risk_level: medium
---

# Rust kernel clean-PR counter semantics

## Current State

Status: completed
Current phase: final
Next: done
Reason: task completed
Blockers: none
Allowed follow-up command: `none`
Latest runner update: 2026-05-20T00:56:23Z
Review gate: pass

## Summary

Lock the semantics of the clean-kernel PR counter before its output is used as
promotion evidence. This is a focused pre-promotion audit slice. It may adjust
the counter, fixture, and tests so the rules are explicit and fail closed, but
it must not remove `continue-on-error: true` or declare Phase B active.

## Context

Grounded current facts:
- `scripts/count-clean-kernel-prs.ts` exists.
- `tests/count-clean-kernel-prs.test.ts` covers fixture-mode counting, rust-only
  and parser-only exclusion, missing evidence, advisory-start requirements, and
  minimum-count failure.
- `tests/fixtures/clean-kernel-prs.json` supplies local audited evidence.
- `rust-kernel-blocking-promotion` still owns live advisory-start evidence,
  five qualifying post-advisory PRs, and the CI flip.

## Scope

In scope:
- Precisely define which PR file changes count toward the five-clean-PR gate.
- Preserve fail-closed behavior when advisory-start evidence is missing.
- Require explicit passing evidence for counting PRs.
- Keep Rust-only, parser-only, missing-evidence, and outside-scope PRs
  non-counting.
- Add fixture tests for ambiguous cases that promotion reviewers are likely to
  challenge, such as mixed TypeScript kernel plus deliberate fixture refreshes.
- Record the final semantics in this spec for handoff to
  `rust-kernel-blocking-promotion`.

Out of scope:
- Live GitHub API integration unless harden explicitly narrows it to read-only
  evidence collection.
- Removing `continue-on-error: true` from CI.
- Changing `rust-kernel-blocking-promotion` evidence thresholds.
- Runtime, parser, receipt, SDK, or CLI cutover.

## Semantics To Lock

- Advisory start must be explicit from CLI input or audited fixture data; never
  infer it from file timestamps or git history.
- A TypeScript kernel PR counts only when every changed file is under
  `packages/core/src/state-machine/` or `packages/core/src/policy/` and ends in
  `.ts`.
- A deliberate kernel fixture refresh counts only when it is explicitly marked
  with `deliberate_kernel_fixture_refresh`, `deliberateKernelFixtureRefresh`,
  `kind: kernel_fixture_refresh`, or `classification: kernel_fixture_refresh`,
  touches at least one `fixtures/kernel/` file, and every changed file is either
  a kernel fixture file or an authoritative TypeScript kernel file.
- Rust-only maintenance PRs remain advisory evidence but do not count toward the
  five-PR trigger.
- Parser-only PRs do not count toward the current five-PR trigger.
- Missing, skipped, failed, renamed, or ambiguous parity evidence makes the PR
  non-counting unless audited fixture data supplies explicit operator evidence
  via `passing_evidence: true` or `passingEvidence: true`.
- Evidence object pass tokens are accepted only from
  `status`, `verdict`, `conclusion`, or `result`. When required checks are
  present, every check must have a passing token, and a top-level evidence pass
  token cannot override a skipped, failed, renamed, or ambiguous check.
- Mixed TypeScript kernel plus deliberate fixture refresh PRs count as
  `kernel_fixture_refresh` only when the explicit deliberate-refresh marker is
  present. The same mixed file set without the marker is
  `outside_kernel_promotion_scope`.

## Acceptance

Profile: strict

Validation:
- [x] `v1` command - counter tests pass.
  - Command: `pnpm exec vitest run --config vitest.config.ts tests/count-clean-kernel-prs.test.ts`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-12
- [x] `v2` command - fixture-mode counter still passes at the audited local
  - Command: `pnpm exec tsx scripts/count-clean-kernel-prs.ts --fixture tests/fixtures/clean-kernel-prs.json --min 3`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-13
- [x] `v3` command - missing advisory-start evidence remains rejected.
  - Command: `pnpm exec tsx -e "import { analyzeCleanKernelPrs } from './scripts/count-clean-kernel-prs.ts'; try { analyzeCleanKernelPrs({ prs: [] }); process.exit(1); } catch (error) { process.exit(String(error instanceof Error ? error.message : error).includes('missing advisory start evidence') ? 0 : 1); }"`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-14
- [x] `v4` command - CI remains advisory after this slice.
  - Command: `rg -n 'Advisory Rust kernel parity|continue-on-error: true' .github/workflows/ci.yml`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-15

## Phase 1: Semantics Audit

Status: completed
Dependencies: none

Objective: Complete this phase.

Changes:
- none

Acceptance:
- none

## Phase 2: Promotion Handoff

Status: completed
Dependencies: Phase 1

Objective: Complete this phase.

Changes:
- none

Acceptance:
- none

## Review

Status: completed
Verdict: pass
Mode: discover
Provider: claude:claude-opus-4-7
Output: claude.mcp_submit_review
Summary: Discover-mode review of rust-kernel-clean-pr-counter-semantics. The counter (scripts/count-clean-kernel-prs.ts), fixture, and tests implement the spec's locked semantics consistently: TS-kernel detection requires every file to match packages/core/src/(state-machine|policy)/.+\.ts; kernel_fixture_refresh requires an explicit marker plus at least one fixtures/kernel/ file and every file being kernel-fixture or TS-kernel; parser-only and rust-only PRs are non-counting; missing advisory start throws; explicit pass tokens are limited to status/verdict/conclusion/result; checks-array gating prevents a direct pass token from overriding skipped/failed/renamed/ambiguous checks; operator override via passing_evidence: true is honored; min/--min defaults fail closed below the requested minimum. Acceptance v1-v4 are recorded as passing. CI workflow still preserves `continue-on-error: true` on Advisory Rust kernel parity, matching the out-of-scope guard. No completion blockers found.

Attack log:
- `scripts/count-clean-kernel-prs.ts classifyPullRequest`: Verify ordering parser_only -> rust_only -> missing_passing_evidence -> ts_kernel -> kernel_fixture_refresh -> outside against spec semantics -> clean (Order is fail-closed: missing evidence intercepts before kernel classifications; parser_only first prevents kernel-misclassification of pure parser PRs.)
- `scripts/count-clean-kernel-prs.ts evidenceContainsPass`: Try direct status='passed' with a skipped or renamed check in the checks array (spec says top-level token must not override ambiguous check) -> clean (allChecksPass=false because skipped/renamed are not in the passing token list; function returns false. Test 'does not let evidence-object pass tokens override skipped or ambiguous required checks' covers this.)
- `scripts/count-clean-kernel-prs.ts isTsKernelFile / isDeliberateKernelFixtureRefresh`: Mixed TS kernel + kernel fixture file with and without deliberate-refresh marker -> clean (Without marker -> outside_kernel_promotion_scope (PR 109). With marker and only kernel-fixture+TS-kernel files -> kernel_fixture_refresh (PR 108). Non-kernel non-fixture file with marker fails the every() guard.)
- `scripts/count-clean-kernel-prs.ts resolveAdvisoryStart`: Missing advisory_start in both CLI and fixture; conflicting snake/camel keys; empty string -> clean (Missing -> throws 'missing advisory start evidence' (covered by v3 and test). Conflicting snake/camel -> throws via JSON.stringify comparison. Empty string treated as not explicit.)
- `scripts/count-clean-kernel-prs.ts parseCliArgs --min`: Non-integer, negative, missing value, value beginning with -- -> clean (Non-integer/negative throw; missing value throws via requiredArgValue; flag-looking values rejected as missing.)
- `tests/fixtures/clean-kernel-prs.json + tests/count-clean-kernel-prs.test.ts`: Confirm test assertions exercise spec semantics, including mixed/ambiguous and fail-closed minimum -> clean (Tests assert specific counting numbers [101,102,103,108], non-counting set including 104,105,106,107,110, 109 outside scope without marker, and meets_minimum=false at --min 5.)
- `.github/workflows/ci.yml`: Verify continue-on-error: true preserved on Advisory Rust kernel parity, matching scope exclusion -> clean (Workflow line 82-83 retains 'Advisory Rust kernel parity' with continue-on-error: true; spec acceptance v4 grep matches.)
- `task scope vs workspace diff`: Look for ambient drift or undeclared changes outside scope -> clean (Workspace baseline reports no task-scoped or ambient changes since approval; deleted draft specs noted in git status are outside this task scope and not produced by it.)

Findings:
- none

## Metadata

Estimated effort hours: 3
Actual effort hours: none
AI model: none
React cycles: none

Tags:
- rust
- trusted-kernel
- ci
- advisory
- promotion-evidence

## Origin

Source:
- split from obsolete `rust-kernel-port-orchestration` after observing that the
  clean-kernel counter exists but promotion evidence semantics still need to be
  trusted independently.

Repo:
- runxhq/runx

Git:
- none

Sync:
- none

Supersession:
- follows: rust-kernel-port-orchestration
- hands_off_to: rust-kernel-blocking-promotion

## Harden Rounds

- none
