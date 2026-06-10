---
spec_version: '2.0'
task_id: runx-release-readiness-v1
created: '2026-06-05T03:25:35Z'
updated: '2026-06-10T11:54:25Z'
status: completed
harden_status: passed
size: medium
risk_level: high
---

# runx-release-readiness-v1

## Current State

Status: completed
Current phase: complete
Next: done
Reason: finalization receipt passed
Blockers: none
Allowed follow-up command: `none`
Latest runner update: 2026-06-10T11:51:39Z
Review gate: not_started

## Summary

Prove that a fresh checkout and packaged artifacts are release-ready. This is the
boring but decisive lane: install, build, help, first skill, receipts, package
exports, release archives, docs/status links, and no stale scafld status
references. It does not add new product behavior.

## Objectives

- Fresh checkout path works exactly as README says.
- Published package shape contains only intended files and exports.
- Release archive smoke works for the native binary.
- Demo verifier and receipt docs are coherent.
- Stale scafld status/doc references are fixed or guarded, with no
  draft-prerequisite banner left in public docs.

## Scope

In scope:
- README, docs/getting-started, docs/demos, docs/api-surface, package manifests.
- `scripts/check-cli-package-contract.mjs`,
  `scripts/make-signature-manifest.ts`, `scripts/package-rust-cli.ts`,
  `scripts/release-rust-cli.ts`,
  `scripts/check-rust-cli-release-artifacts.ts`.
- Fresh checkout scripts, docs/status checks, and scafld duplicate
  active/draft-status guards.
- The release workflow is reference material for the local package/archive smoke;
  this spec does not exercise hosted release ops.

Out of scope:
- Hosted ops, live-funded rails, new demos.

## Dependencies

- Gate hardening and demo prune should define the canonical gate/demo list.
- Existing release workflow and package contract checks.
- The readiness structural guard owns stale active/draft spec detection; release
  readiness wires that guard into the final gate rather than adding another
  checker.

## Assumptions

- Native Rust CLI remains the trusted path; npm wrapper is a distribution/UX shim.
- Release readiness should be reproducible without private credentials.
- Local release smoke only packages the current host platform. The CI release
  matrix remains responsible for all platform targets.

## Risks

- **Local-only success, package failure.** Mitigation: test packed artifacts and
  archives, not only workspace commands.
- **Artifact check without artifacts.** Mitigation: create the local release
  state directory, then generate the host-platform signature manifest and
  package tree immediately before checking it.
- **Docs drift.** Mitigation: add link/command guards where cheap.

## Acceptance

Profile: strict

Validation:
- Fresh checkout path in README works.
- `runx --help`, first skill, harness, receipt verify, package contracts, and
  release archive smoke pass.
- Readiness structural guard rejects stale active/draft spec confusion.
- Public docs no longer carry stale draft-prerequisite status banners.

## Phase 1: Fresh checkout smoke

Status: pass
Dependencies: runx-readiness-gate-hardening-v1

Objective: prove README commands.

Changes:
- none

Acceptance:
- [x] `p1_ac1` command - fresh checkout build and first skill
  - Command: `pnpm install --frozen-lockfile && pnpm build && cargo build --manifest-path crates/Cargo.toml -p runx-cli && RUNX_RECEIPT_DIR="$(mktemp -d)" RUNX_RECEIPT_SIGN_KID=runx-demo-key RUNX_RECEIPT_SIGN_ED25519_SEED_BASE64=QkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkI= RUNX_RECEIPT_SIGN_ISSUER_TYPE=hosted crates/target/debug/runx skill examples/hello-world --message "release smoke" --non-interactive --json`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-10

## Phase 2: Package and archive smoke

Status: pass
Dependencies: Phase 1

Objective: prove the shipped artifact shape.

Changes:
- none

Acceptance:
- [x] `p2_ac1` command - package contracts pass
  - Command: `node scripts/check-cli-package-contract.mjs && pnpm authoring:check-package-contract`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-11
- [x] `p2_ac2` command - release artifact check passes
  - Command: `rm -rf .runx/rust-cli-artifacts .runx/release-readiness-signatures.json && mkdir -p .runx && PLATFORM="$(node -p "process.platform + '-' + process.arch")" && pnpm exec tsx scripts/make-signature-manifest.ts --binary crates/target/debug/runx --platform "$PLATFORM" --out .runx/release-readiness-signatures.json --identity local-release-readiness && pnpm exec tsx scripts/release-rust-cli.ts --binary crates/target/debug/runx --platform "$PLATFORM" --artifact-dir .runx/rust-cli-artifacts --signature-manifest .runx/release-readiness-signatures.json`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-12

## Phase 3: Docs and final gates

Status: pass
Dependencies: Phase 2

Objective: release docs and gates agree.

Changes:
- none

Acceptance:
- [x] `p3_ac1` command - full release readiness gate
  - Command: `pnpm verify:fast && node scripts/check-readiness-structural.mjs && pnpm demos:check && pnpm x402:dogfood:local`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-13
- [x] `p3_ac2` command - no stale draft-prerequisite banner in public docs
  - Command: `! rg -n "Status: draft, prerequisite" docs README.md`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-14

## Rollback

- Revert docs/check changes together. Do not leave release-only scripts unwired.

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

Status: needs_revision
Started: 2026-06-05T03:46:41Z
Ended: 2026-06-05T03:46:41Z
Verdict: needs_revision
Provider: claude
Model: claude-opus-4-7
Output format: claude.mcp_submit_harden
Summary: Spec correctly targets the boring-but-decisive release lane and most paths/commands resolve, but Phase 2 acceptance `p2_ac2` is non-executable in a fresh checkout: `scripts/check-rust-cli-release-artifacts.ts` defaults to `.runx/rust-cli-artifacts`, a directory that only `scripts/release-rust-cli.ts` (or the release workflow) populates. As written it exits 1 with `artifact_dir_missing`. Objective 5 ("stale docs/spec references fixed or archived") also has no enforcement gate among the acceptance commands. Both must be addressed before approval.

Checks:
- path audit
  - Grounded in: code:oss/scripts/check-cli-package-contract.mjs:1, code:oss/scripts/check-rust-cli-release-artifacts.ts:1, code:oss/examples/hello-world/SKILL.md:1, code:oss/scripts/check-demos.mjs:1, code:oss/scripts/verify-fast.mjs:1
  - Result: passed
  - Evidence: All paths named in scope and phases exist: check-cli-package-contract.mjs, check-rust-cli-release-artifacts.ts, examples/hello-world (SKILL.md, X.yaml, run.mjs), check-demos.mjs (mapped to `demos:check`), verify-fast.mjs, and `x402:dogfood:local` resolves to scripts/x402-local-dogfood.mjs in oss/package.json line 73.
- command audit
  - Grounded in: code:oss/scripts/check-rust-cli-release-artifacts.ts:42-60, code:oss/scripts/check-rust-cli-release-artifacts.ts:62-92, code:oss/.github/workflows/release.yml:282-294
  - Result: failed
  - Evidence: `pnpm exec tsx scripts/check-rust-cli-release-artifacts.ts` (p2_ac2) defaults `artifactDir` to `.runx/rust-cli-artifacts` (parseArgs line 63). That directory only exists after `scripts/release-rust-cli.ts` runs, which the release workflow invokes with `--out-dir .runx/rust-cli-artifacts` (release.yml:145). In a fresh checkout the script reports `artifact_dir_missing` and exits 1, so the acceptance command cannot pass as written.
- scope/migration audit
  - Grounded in: spec_gap:objectives, code:oss/scripts/verify-fast.mjs:41-48
  - Result: failed
  - Evidence: Objective 5 (`Stale docs/spec references are either fixed or archived with clear status`) is unmoored: no acceptance command in any phase enforces it, no concrete list of stale files is named, and no new guard is added. Either name the files/guard or drop the objective. Scope also lists `release workflow smoke checks` but no phase exercises the release workflow shape; acceptance is implicit only.
- acceptance timing audit
  - Grounded in: code:oss/scripts/check-rust-cli-release-artifacts.ts:42-60, code:oss/scripts/release-rust-cli.ts:77
  - Result: failed
  - Evidence: Phase 2 lacks a step that populates `.runx/rust-cli-artifacts` before p2_ac2 inspects it. Phase 1 only builds the dev binary at `crates/target/debug/runx`; nothing produces the packaged per-platform tree the release-artifact check requires. p2_ac2 should be preceded by `pnpm exec tsx scripts/release-rust-cli.ts --binary crates/target/debug/runx --platform <host> ...` or invoke the check with an explicit `--artifact-dir` pointed at a freshly built tree.
- rollback/repair audit
  - Grounded in: spec_gap:rollback
  - Result: passed
  - Evidence: Spec adds no new product behavior and only wires checks/docs. The rollback note (`Revert docs/check changes together. Do not leave release-only scripts unwired.`) is sufficient given the limited blast radius. A failed Phase 2 leaves no persistent state beyond a missing/empty `.runx/rust-cli-artifacts`, which is gitignored.
- design challenge
  - Grounded in: code:oss/scripts/verify-fast.mjs:41-90, code:oss/README.md:14-21
  - Result: passed
  - Evidence: Architecturally sound: explicitly proves the fresh-checkout README path, the packaged artifact shape, and the doc/demo gates separately from inner-loop fast checks. The boring lane is the right architectural move for a release gate. Caveats: Phase 3 `pnpm verify:fast` rebuilds the same Rust binary Phase 1 already built (wasted minutes), and Phase 2's `pnpm authoring:check-package-contract` overlaps `verify:fast` (verify-fast.mjs:90). Acceptable duplication for a hard release boundary.

Issues:
- [critical/blocks approval] `issue-p2-artifact-dir-missing` command_non_executable - p2_ac2 cannot pass in a fresh checkout: `.runx/rust-cli-artifacts` is unpopulated.
  - Status: open
  - Grounded in: code:oss/scripts/check-rust-cli-release-artifacts.ts:42-92, code:oss/.github/workflows/release.yml:145-294
  - Evidence: `scripts/check-rust-cli-release-artifacts.ts` defaults `--artifact-dir` to `.runx/rust-cli-artifacts` and fails with `artifact_dir_missing` when absent. That directory is only created by `scripts/release-rust-cli.ts` (invoked in release.yml:145 with `--out-dir .runx/rust-cli-artifacts`). Phase 2 has no step that produces it.
  - Recommendation: Either (a) prepend a step that runs `pnpm exec tsx scripts/release-rust-cli.ts --binary crates/target/debug/runx --platform <host>` (and any required signature manifest) before p2_ac2, or (b) change p2_ac2 to pass an explicit `--artifact-dir` pointing at a freshly built packed-tree. Mirror the release.yml invocation (`--no-js-delegation --verify-signatures`) so the gate matches the actual release contract.
  - Question: Should release readiness exercise the full release-rust-cli.ts → check-rust-cli-release-artifacts.ts chain locally, or only assert the check script behavior against a host-platform subset?
  - Recommended answer: Run release-rust-cli.ts for the host platform only, then run the check with `--artifact-dir .runx/rust-cli-artifacts --no-js-delegation` (skip `--verify-signatures` locally because release signature manifests are CI-only).
  - If unanswered: Default to running release-rust-cli.ts for the host platform and invoking the check with `--no-js-delegation` (no signature verification) before approval.
- [medium/blocks approval] `issue-stale-refs-unenforced` objective_unenforced - Objective 5 on stale docs/spec references has no acceptance gate or named guard.
  - Status: open
  - Grounded in: spec_gap:objectives, code:oss/scripts/verify-fast.mjs:41-48
  - Evidence: Objectives include `Stale docs/spec references are either fixed or archived with clear status` and validation echoes `No stale active/draft/archive status confusion in public docs`, but no acceptance command in Phase 1, 2, or 3 enforces this. There is no docs link checker, no archive-vs-active guard call, and no enumerated file list.
  - Recommendation: Either (a) add a concrete guard (e.g., a `scripts/check-docs-links.mjs` invocation, or extend `check-readiness-structural.mjs` to detect stale active/draft/archive duplicates) and wire it into Phase 3, or (b) enumerate the specific docs files that must be fixed/archived as part of this spec and add a grep-based guard. As written the objective is unfalsifiable.
  - Question: Is there an existing docs-coherence guard the spec should call, or should this spec add one (and if so, which script)?
  - Recommended answer: Extend `check-readiness-structural.mjs` with a duplicate-active/draft/archive detector and add `node scripts/check-docs-links.mjs` to Phase 3; fail closed on any drift.
  - If unanswered: Drop objective 5 from this spec and file a separate spec for stale-reference cleanup, so this lane stays falsifiable.
- [low/advisory] `issue-release-workflow-scope-loose` scope_gap - `release workflow smoke checks` is in scope but no phase exercises the workflow shape.
  - Status: open
  - Grounded in: spec_gap:scope
  - Evidence: Scope line 45 lists `release workflow smoke checks`. No acceptance command invokes `.github/workflows/release.yml`, runs `act`, or asserts on its structure. The phases only run the scripts that the workflow happens to invoke.
  - Recommendation: Either name a smoke gate (e.g., a YAML lint or a structural assertion that release.yml still wires `release-rust-cli.ts` and `check-rust-cli-release-artifacts.ts`) or drop `release workflow smoke checks` from scope to avoid wishful framing.
- [low/advisory] `issue-phase3-duplication` duplication_advisory - Phase 3 `verify:fast` overlaps Phase 1 cargo build and Phase 2 authoring contract check.
  - Status: open
  - Grounded in: code:oss/scripts/verify-fast.mjs:41-90
  - Evidence: `verify-fast.mjs` builds the native runx binary (lines 56-60) and runs `authoring package contract` (line 90). Phase 1 already builds the binary and Phase 2 already runs `pnpm authoring:check-package-contract`. The release lane will rebuild and re-check.
  - Recommendation: Acceptable for a release gate (better redundant than missing), but note in the spec that Phase 3 intentionally re-runs the binary build so reviewers do not try to optimize it away.

### round-2

Status: passed
Started: 2026-06-05T04:16:36Z
Ended: 2026-06-05T04:16:36Z
Verdict: pass
Provider: claude
Model: claude-opus-4-7
Output format: claude.mcp_submit_harden
Summary: Round 2 spec resolves both round-1 blockers. Phase 2 acceptance p2_ac2 is now executable in a fresh checkout: it clears `.runx/rust-cli-artifacts` and `.runx/release-readiness-signatures.json`, generates a host-platform signature manifest via `scripts/make-signature-manifest.ts`, then runs `scripts/release-rust-cli.ts` which packages the artifact and transitively invokes `scripts/check-rust-cli-release-artifacts.ts --no-js-delegation --verify-signatures` (release-rust-cli.ts:38-46). Phase 3 enforces objective 5 with `scripts/check-readiness-structural.mjs` (whose `checkDuplicateActiveAndDraftSpecs` at lines 107-137 detects active/draft confusion) plus a `! rg -n "Status: draft, prerequisite" docs README.md` guard for the public-docs banner. Scope was tightened to mark the release workflow as reference material only, so the loose round-1 framing is gone. The Phase 1 dependency on `runx-readiness-gate-hardening-v1` is honored since that spec is now in active and ships the structural-guard duplicate detector this spec relies on. Phase 3 duplicates Phase 1's binary build and Phase 2's authoring contract check; redundancy is intentional and acceptable for a release boundary. No new product behavior is introduced; the rollback note (revert docs/checks together) is sufficient given limited blast radius. Verdict: pass.

Checks:
- path audit
  - Grounded in: code:oss/scripts/check-cli-package-contract.mjs:1, code:oss/scripts/make-signature-manifest.ts:1, code:oss/scripts/release-rust-cli.ts:1, code:oss/scripts/package-rust-cli.ts:1, code:oss/scripts/check-rust-cli-release-artifacts.ts:1, code:oss/scripts/check-readiness-structural.mjs:1, code:oss/scripts/verify-fast.mjs:1, code:oss/scripts/x402-local-dogfood.mjs:1, code:oss/examples/hello-world/SKILL.md:1, code:oss/package.json:72-77
  - Result: passed
  - Evidence: All paths named in scope and acceptance commands exist and behave as the spec expects. `check-cli-package-contract.mjs`, `make-signature-manifest.ts`, `release-rust-cli.ts`, `package-rust-cli.ts`, `check-rust-cli-release-artifacts.ts`, `check-readiness-structural.mjs`, `verify-fast.mjs`, and `x402-local-dogfood.mjs` are all present. `examples/hello-world/SKILL.md` declares a `cli-tool` source running `node run.mjs` with `--message` as a required input — matches the p1_ac1 invocation. `pnpm demos:check`, `pnpm verify:fast`, `pnpm x402:dogfood:local`, and `pnpm authoring:check-package-contract` resolve to existing scripts in oss/package.json. `.runx/` is gitignored (.gitignore:47), so the rm/mkdir scaffolding in p2_ac2 does not pollute git.
- command audit
  - Grounded in: code:oss/scripts/release-rust-cli.ts:22-46, code:oss/scripts/release-rust-cli.ts:77, code:oss/scripts/make-signature-manifest.ts:21-46, code:oss/scripts/package-rust-cli.ts:84-110, code:oss/scripts/check-rust-cli-release-artifacts.ts:42-60, code:oss/scripts/check-readiness-structural.mjs:107-137
  - Result: passed
  - Evidence: Round-1 blocker is fixed. p2_ac2 builds the signature manifest from `crates/target/debug/runx` (Phase 1 product), passes `--signature-manifest .runx/release-readiness-signatures.json` to `release-rust-cli.ts` (which fails fast at line 22 if the manifest is missing), and `release-rust-cli.ts` then runs `package-rust-cli.ts` (writes `native/checksums.json` and `native/signatures.json` lines 94-110) and `check-rust-cli-release-artifacts.ts --no-js-delegation --verify-signatures` (lines 38-46) against `.runx/rust-cli-artifacts`. The `package = ${manifest.name}-${platform}` field emitted by make-signature-manifest.ts matches `nativePackageName` in package-rust-cli.ts, so signature/checksum/package-name cross-checks at check-rust-cli-release-artifacts.ts:446-500 align. Phase 3 commands `pnpm verify:fast && node scripts/check-readiness-structural.mjs && pnpm demos:check && pnpm x402:dogfood:local` and `! rg -n "Status: draft, prerequisite" docs README.md` are all executable and the rg guard currently finds zero matches.
- scope/migration audit
  - Grounded in: spec_gap:objectives, code:oss/scripts/check-readiness-structural.mjs:107-137, code:oss/.scafld/specs/active/runx-readiness-gate-hardening-v1.md
  - Result: passed
  - Evidence: Objective 5 (`Stale scafld status/doc references are fixed or guarded, with no draft-prerequisite banner left in public docs`) is now enforced by two acceptance commands in Phase 3: `node scripts/check-readiness-structural.mjs` (which calls `checkDuplicateActiveAndDraftSpecs` at lines 107-137 to detect spec/draft duplication) and `! rg -n "Status: draft, prerequisite" docs README.md` (p3_ac2) for the banner. The Phase 1 dependency `runx-readiness-gate-hardening-v1` exists in `.scafld/specs/active/`, which is the spec that ships the structural-guard duplicate detector. Round-1's scope concern about `release workflow smoke checks` being unenforced is addressed by the new scope text: "The release workflow is reference material for the local package/archive smoke; this spec does not exercise hosted release ops." Hosted ops/live rails are explicitly out of scope.
- acceptance timing audit
  - Grounded in: code:oss/scripts/release-rust-cli.ts:26-46, code:oss/scripts/package-rust-cli.ts:47-110, code:oss/scripts/check-rust-cli-release-artifacts.ts:42-60
  - Result: passed
  - Evidence: Round-1 timing blocker is fixed. Phase 2 acceptance p2_ac2 now explicitly produces `.runx/rust-cli-artifacts` before the check runs: the rm/mkdir step, then make-signature-manifest.ts → release-rust-cli.ts which runs package-rust-cli.ts → check-rust-cli-release-artifacts.ts as one chained invocation. The Phase 1 binary (`crates/target/debug/runx`) is the input to make-signature-manifest.ts (--binary) and package-rust-cli.ts (via release-rust-cli.ts), so the sha256 in `.runx/release-readiness-signatures.json` matches the staged binary written into `.runx/rust-cli-artifacts/<platform>/bin/runx`. Phase ordering also stands: p1 builds the binary, p2_ac1 checks the published-shape contracts, p2_ac2 exercises the per-platform package and signature chain, p3 runs the union release gate.
- rollback/repair audit
  - Grounded in: spec_gap:rollback, code:oss/.gitignore:47
  - Result: passed
  - Evidence: Spec adds no new product behavior — only wires checks and removes stale doc banners. Rollback note (`Revert docs/check changes together. Do not leave release-only scripts unwired.`) is sufficient. A failed Phase 2 leaves residue only inside `.runx/`, which is gitignored (.gitignore:47); the rm at the start of p2_ac2 ensures the next run is hermetic. There is no migration, no data shape change, and no irreversible deletion. Failure surfaces are bounded to docs/checks reverts.
- design challenge
  - Grounded in: code:oss/scripts/verify-fast.mjs:41-90, code:oss/scripts/release-rust-cli.ts:26-46, code:oss/README.md
  - Result: passed
  - Evidence: Architecturally the right move: the spec proves the fresh-checkout README path, the per-platform packaged shape, and the doc/demo gate as three separable acceptance commands. This is not a bandaid — it codifies the boring-but-decisive release gate that hosted ops will rely on. The choice to drive `release-rust-cli.ts` (which is the same script CI uses) instead of hand-rolling a parallel checker keeps local and CI release contracts aligned. Trade-off: Phase 3 `pnpm verify:fast` rebuilds the binary already produced in Phase 1 and re-runs `authoring package contract` already checked in Phase 2. This is acceptable redundancy for a release boundary (better to over-check than to land a regression in the published shape). The decision to package the debug binary in p2_ac2 rather than a release-mode build is a deliberate scope choice — the spec is about artifact-shape contracts, not optimization parity, and the per-platform release matrix remains owned by CI.

Issues:
- [critical/advisory] `issue-p2-artifact-dir-missing` command_non_executable - Round-1 blocker resolved: p2_ac2 now produces the artifact dir it inspects.
  - Status: fixed
  - Grounded in: code:oss/scripts/release-rust-cli.ts:22-46, code:oss/scripts/make-signature-manifest.ts:21-46, code:oss/scripts/package-rust-cli.ts:84-110
  - Evidence: Acceptance command in p2_ac2 now removes `.runx/rust-cli-artifacts` and `.runx/release-readiness-signatures.json`, regenerates the signature manifest via `make-signature-manifest.ts`, then invokes `release-rust-cli.ts --artifact-dir .runx/rust-cli-artifacts --signature-manifest .runx/release-readiness-signatures.json`. `release-rust-cli.ts` runs `package-rust-cli.ts` (which writes `native/checksums.json` and `native/signatures.json` lines 94-110) and `check-rust-cli-release-artifacts.ts --no-js-delegation --verify-signatures` against the populated tree.
  - Recommendation: Keep this resolution: the chain is self-contained and mirrors the CI release contract. No further change needed.
- [medium/advisory] `issue-stale-refs-unenforced` objective_unenforced - Round-1 blocker resolved: objective 5 is now gated by structural guard + rg banner check.
  - Status: fixed
  - Grounded in: code:oss/scripts/check-readiness-structural.mjs:107-137
  - Evidence: Phase 3 adds two enforcing commands: `node scripts/check-readiness-structural.mjs` (whose `checkDuplicateActiveAndDraftSpecs` at lines 107-137 fails on any active/draft duplicate), and `! rg -n "Status: draft, prerequisite" docs README.md` (p3_ac2) for the public-doc banner. Objective text was narrowed accordingly to refer to the structural guard rather than a vague "archived with clear status" notion.
  - Recommendation: No further change needed. Optional: if more banner patterns surface later, extend `check-readiness-structural.mjs` rather than adding new rg lines to the spec.
- [low/advisory] `issue-release-workflow-scope-loose` scope_gap - Round-1 advisory resolved: scope no longer claims release-workflow smoke checks.
  - Status: fixed
  - Grounded in: spec_gap:scope
  - Evidence: Scope now reads "The release workflow is reference material for the local package/archive smoke; this spec does not exercise hosted release ops." Out-of-scope list still explicitly excludes hosted ops and live-funded rails. The framing is no longer wishful.
  - Recommendation: No further change needed.
- [low/advisory] `issue-phase3-duplication` duplication_advisory - Phase 3 `pnpm verify:fast` rebuilds the binary from Phase 1 and re-runs `authoring package contract` from Phase 2.
  - Status: open
  - Grounded in: code:oss/scripts/verify-fast.mjs:56-90
  - Evidence: `verify-fast.mjs` lines 56-68 build the native runx binary and line 90 runs `check-authoring-package-contract.mjs`. Phase 1 already builds the binary; Phase 2 already runs `pnpm authoring:check-package-contract`. The release lane will rebuild and re-check.
  - Recommendation: Accept the redundancy for a release boundary (intentional belt-and-braces). Optional: add a one-line note in the spec that Phase 3 deliberately re-runs the full gate so a future reviewer does not try to optimize the rebuild away.


## Planning Log

- none
