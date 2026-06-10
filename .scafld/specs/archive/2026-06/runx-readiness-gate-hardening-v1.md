---
spec_version: '2.0'
task_id: runx-readiness-gate-hardening-v1
created: '2026-06-05T03:25:35Z'
updated: '2026-06-10T11:45:55Z'
status: completed
harden_status: passed
size: medium
risk_level: high
---

# runx-readiness-gate-hardening-v1

## Current State

Status: completed
Current phase: complete
Next: done
Reason: finalization receipt passed
Blockers: none
Allowed follow-up command: `none`
Latest runner update: 2026-06-10T11:42:06Z
Review gate: not_started

## Summary

Make the current clean shape hard to regress. The core CI path is strong, but the
last advisory lane is still `continue-on-error`, several useful checks live as
manual scripts, and there is no explicit guard for the cleanup quality the project
now expects: no orphan examples, no untracked committed build output, no empty
leftover directories, no resurrected compatibility packages, and no stale demo
surface.

This spec is intentionally first. It does not add product scope. It turns the
existing readiness posture into enforceable gates.

## Objectives

- Remove `continue-on-error` from the Rust parity/advisory lane, or split the
  lane into required checks and explicitly documented non-blocking diagnostics.
- Wire the high-signal local gates into `verify:fast` or CI: demo receipts,
  release smoke, package boundary, orphan example/package checks, empty-dir checks,
  committed-dist policy, and anti-retired-package imports.
- Keep guard ownership explicit: `check-runtime-cutover-legacy.mjs` owns
  domain-free runtime/contract scans, `check-boundaries.mjs` owns license/import
  boundaries, and `check-readiness-structural.mjs` owns cleanup debris such as
  retired packages, committed build output, empty dirs, and duplicate active specs.
- Keep CI runtime practical; slow live-funded lanes stay opt-in and preflight-only.

## Scope

In scope:
- `.github/workflows/ci.yml`, `.github/workflows/release.yml`.
- Existing wired scripts: `scripts/verify-fast.mjs`,
  `scripts/check-verify-fast-plan.mjs`, `scripts/check-boundaries.mjs`, and
  `scripts/check-runtime-cutover-legacy.mjs`.
- New focused guard scripts for readiness debris and demo inventory if needed.
- Guard coverage for examples, package manifests, committed dist policy, empty
  dirs, orphan package dirs, retired `@runxhq/core` imports, and live-lane
  preflight commands.

Out of scope:
- New demos, new fronts, new product runtime behavior, hosted live settlement.
- Approval-gate schema tightening or behavior changes; that is a contract change,
  not a readiness-gate change.

## Dependencies

- Existing gates: `pnpm verify:fast`, `pnpm demos:check`,
  `pnpm x402:dogfood:local`, `cargo fmt`, `cargo clippy`, `cargo nextest`,
  `cargo test --doc`, license boundary checks, and `scripts/check-rust-kernel-parity.mjs`.
- Focused coverage already exists for policy, payment authority, and contract
  schema validation. This spec may add source-level guard wiring for those
  commands, but it does not change approval-gate runtime/schema semantics.
- The demo-prune spec should run after this or in parallel, but this spec owns the
  guard shape that prevents demo cruft from returning.

## Assumptions

- The target is a readiness gate, not a broad review. Any failing guard must point
  to a small concrete fix.
- Live-funded payment lanes stay preflight-only in default CI; they must never
  require private keys or funded wallets.

## Risks

- **CI becomes too slow.** Mitigation: keep heavy Rust gates in the existing Rust
  job, keep live settlement opt-in, and run cheap structural guards in
  `verify:fast`.
- **False-positive cleanup guards block useful generated output.** Mitigation:
  encode explicit allowlists with rationale, not broad regex exceptions.
- **Advisory checks are flaky because tools are missing.** Mitigation: install
  required tools in CI or split install checks into loud preflight failures.
- **Public API snapshot drift becomes a blocking PR failure.** Mitigation:
  `scripts/check-rust-kernel-parity.mjs` prints the snapshot refresh command when
  drift is intentional; keep the refresh runbook in that script and treat
  rustdoc-toolchain drift as a gate incident, not as a silent advisory.

## Acceptance

Profile: strict

Validation:
- `pnpm verify:fast` includes the new structural readiness guards and fails if
  demo/package/orphan/retired-surface rules are violated.
- CI has no `continue-on-error` for gates that are described as required.
- `pnpm x402:dogfood:local` remains zero-funded and green.
- No default gate requires private credentials, funded testnet wallets, or network
  settlement.

## Phase 1: Turn advisory gates into required gates

Status: pass
Dependencies: none

Objective: remove ambiguity from CI.

Changes:
- Audit `.github/workflows/ci.yml` and `scripts/check-rust-kernel-parity.mjs`.
- Either make the advisory Rust parity lane fail CI or split it into required subchecks plus explicitly non-blocking diagnostics.
- Update docs and `verify:fast` plan guard so the intended required gates are machine-checked.
- Add a short runbook for refreshing Rust public API snapshots if public API drift intentionally changes.

Acceptance:
- [x] `p1_ac1` command - no required gate is continue-on-error
  - Command: `! rg -n "continue-on-error:\\s*true" .github/workflows/ci.yml`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-46
- [x] `p1_ac2` command - verify-fast plan still matches required ordering
  - Command: `pnpm verify:fast:plan-check && node scripts/check-readiness-structural.mjs && node scripts/check-demo-inventory.mjs`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-47

## Phase 2: Add structural cleanup guards

Status: pass
Dependencies: Phase 1

Objective: prevent the debris ring from reappearing.

Changes:
- Add guards for orphan examples, empty leftover directories, retired package imports, committed dist policy, stale archived/draft duplicate specs, and untracked demo runners.
- Add a guard that proves any named focused checks are either wired into a required gate or explicitly documented as manual deep checks.
- Wire the guards into `verify:fast` and CI.

Acceptance:
- [x] `p2_ac1` command - structural guards pass
  - Command: `pnpm verify:fast`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-48
- [x] `p2_ac2` command - demo/local payment dogfood remains green
  - Command: `pnpm demos:check && pnpm x402:dogfood:local`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-49
- [x] `p2_ac3` command - focused-check wiring is machine-checked
  - Command: `node scripts/check-readiness-structural.mjs && pnpm verify:fast:plan-check`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-50

## Phase 3: Final gate proof

Status: pass
Dependencies: Phase 2

Objective: prove the readiness gate set from a clean checkout.

Changes:
- none

Acceptance:
- [x] `p3_ac1` command - full local readiness gate
  - Command: `pnpm verify:fast && (cd crates && cargo fmt --all --check && cargo clippy --workspace --all-targets --all-features -- -D warnings && cargo nextest run --workspace --all-features && cargo test --workspace --all-features --doc)`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-51

## Rollback

- Revert the guard scripts and CI changes together. Do not leave dead scripts
  unwired.

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
Started: 2026-06-05T03:37:21Z
Ended: 2026-06-05T03:48:34Z
Verdict: passed
Provider: claude
Model: claude-opus-4-7
Output format: claude.mcp_submit_harden
Summary: The structural shape is sound and every declared path/command resolves. The original harden pass found three approval blockers and three advisories; the draft has been revised to keep this a pure readiness-gate spec, add concrete machine checks, and name the public API snapshot drift runbook. The revised implementation removes the Rust parity `continue-on-error`, wires structural/demo guards into `verify:fast`, and keeps runtime/domain/license boundaries under their existing owners.

Checks:
- path audit
  - Grounded in: code:.github/workflows/ci.yml:97
  - Result: passed
  - Evidence: CI still has a required `Rust kernel parity` step, and the verify-fast plan guard now checks for both readiness and demo guard markers.
- command audit
  - Grounded in: code:scripts/verify-fast.mjs:41
  - Result: passed
  - Evidence: Required source checks are run through `verify:fast`; package scripts already expose `boundary:check`, `demos:check`, `x402:dogfood:local`, `verify:fast`, and `verify:fast:plan-check`.
- scope/migration audit
  - Grounded in: spec_gap:objectives
  - Result: passed
  - Evidence: The runtime approval-gate schema objective was removed; the out-of-scope section now explicitly excludes approval-gate schema behavior changes.
- acceptance timing audit
  - Grounded in: code:scripts/check-readiness-structural.mjs:17
  - Result: passed
  - Evidence: p1_ac2 and p2_ac3 now run the structural readiness guard and plan guard; the demo inventory guard is also wired into `verify:fast`.
- rollback/repair audit
  - Grounded in: spec_gap:rollback
  - Result: passed
  - Evidence: Rollback section instructs reverting guard scripts and CI changes together with 'do not leave dead scripts unwired'. Acceptable for a guards-only spec, where reverting CI+script changes restores the prior advisory posture. A finer-grained per-phase rollback would help (e.g., Phase 1 could land without Phase 2 if structural guards uncover unexpected debris), but the present rollback is coherent and recoverable.
- design challenge
  - Grounded in: code:.github/workflows/ci.yml:62
  - Result: passed
  - Evidence: Turning advisory readiness signals into required gates is sound because CI installs the Rust advisory tools before running the now-required Rust kernel parity step. Live-funded payment lanes remain opt-in.

Issues:
- none


## Planning Log

- none
