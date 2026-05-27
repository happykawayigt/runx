---
spec_version: '2.0'
task_id: runx-rust-95-release-readiness
created: '2026-05-25T22:08:22Z'
updated: '2026-05-26T00:29:13Z'
status: active
harden_status: passed
size: large
risk_level: high
---

# Runx Rust 9.5 release readiness

## Current State

Status: active
Current phase: phase1
Next: build
Reason: phase phase1 opened
Blockers: none
Allowed follow-up command: `scafld handoff runx-rust-95-release-readiness`
Latest runner update: 2026-05-26T00:29:13Z
Review gate: not_started

## Summary

Bring runx's new Rust implementation from "promising and mostly proven" to a
9.5/10 release shape: green mandatory gates, one native live issue-to-PR route,
fail-closed security defaults for mutating/credentialed work, durable
source-thread/publication outcomes, clean public vocabulary, reviewer-grade
story projections, and maintainable runtime module boundaries.

This is an umbrella convergence spec. It does not replace the narrower specs
already in flight; it coordinates them and defines the bar that must be true
before we call the Rust implementation release-ready. When a narrower spec owns
a slice, execute there and record the evidence here.

## Objectives

- Restore and preserve all required CI/release gates for Rust and graph work.
- Make live issue-to-PR create/observe a first-class native runx path rather
  than a stale or script-blocked dogfood lane.
- Keep the human merge gate intact while giving reviewers complete state at the
  issue, PR, and originating thread.
- Fail closed for sandbox, credential, network, and publication paths that can
  mutate repos, Slack/GitHub threads, payment state, or external systems.
- Remove stale public contract vocabulary and compatibility aliases from active
  source once the canonical names are chosen.
- Move reusable Nitrosend-proven issue/PR/thread story shape into runx core
  contracts, leaving product repos as thin policy/routing layers.
- Reduce large-module complexity where it now hides ownership boundaries.
- Prove the final shape with automated tests plus controlled live dogfood.

## Scope

- In scope:
  - Rust workspace gates under `crates/`.
  - Graph-backed `runx skill` execution, issue-to-PR dogfood command routing,
    and target-repo runner integration.
  - Thread outbox provider process supervision and publication idempotency.
  - Post-merge observer durable dedupe and outcome publication.
  - Runtime HTTP egress checks for configurable transports.
  - Sandbox enforcement defaults for mutating/credentialed profiles.
  - Public source-type vocabulary and generated fixtures/contracts.
  - Target-runner, issue, PR, and source-thread story projections.
  - Focused decomposition of oversized Rust runtime/parser modules touched by
    this work.
  - Documentation needed to explain the 9.5 runx issue-to-PR story.
- Out of scope:
  - Automatic PR merge. Humans remain the merge gate.
  - Product-specific Nitrosend Slack/Sentry routing, except as live dogfood
    evidence for the generic runx contracts.
  - Cloud/Aster UI work beyond any core policy/config contracts required by OSS
    runtime behavior.
  - Broad TypeScript rewrites unrelated to current Rust parity or contract
    cleanup.
  - New payment rail provider integrations beyond the minimum needed to keep
    native fixtures and supervisor contracts coherent.

## Dependencies

- `runx-graph-skill-issue-to-pr` owns the active native graph issue-to-PR slice.
- `runx-security-hardening-v1` owns the completed/reviewed omnibus security
  context; this spec only pulls forward remaining hardening that affects 9.5.
- `runx-target-repo-runners` owns generic target-repo runner design details.
- `runx-post-merge-closure-observer` owns post-merge outcome closure details.
- `payment-rail-supervisor-proof-v1` owns payment supervisor proof coherence.
- `process-credential-delivery-hardening-v1` owns residual process credential
  delivery hardening.
- `monolith-decomposition-v1` owns broad large-file retirement; this spec only
  demands decomposition where release work would otherwise deepen god-files.
- Local Rust toolchain with clippy/rustfmt and Node/pnpm workspace installed.
- GitHub/Slack live credentials are available only through the approved
  dogfood/prod paths, not assumed from ambient local shell state.

## Assumptions

- "9.5" means release-ready engineering shape, not perfection. The target bar:
  all mandatory gates green, security claims enforced or explicitly named as
  unavailable, core flows native and dogfooded, and reviewers have enough
  contextual evidence to approve or reject without prompting the bot.
- Public contract cleanup should be a hard cutover with no aliases or legacy
  compatibility paths, matching the prior no-compat direction.
- Runx core should own generic story/projection/idempotency contracts. Product
  repos should own only routing, labels, ownership policy, channel selection,
  and product-specific verification.
- Live dogfood may create branches, GitHub issue comments, Slack replies, and
  draft PRs only against explicit allowlisted proving-ground repos/issues.
- Human merge remains out of automation scope; observe mode reports terminal
  outcomes after a human merge or close.

## Touchpoints

- `crates/runx-core/src/policy/payment_authority.rs`
- `crates/runx-contracts/tests/schema_wire_compat.rs`
- `crates/runx-cli/tests/native_no_ts.rs`
- `crates/runx-runtime/src/execution/skill_run.rs`
- `crates/runx-runtime/src/execution/target_runner.rs`
- `crates/runx-runtime/src/post_merge_observer.rs`
- `crates/runx-runtime/src/outbox_provider.rs`
- `crates/runx-runtime/src/runtime_http.rs`
- `crates/runx-runtime/src/sandbox.rs`
- `crates/runx-parser/src/skill.rs`
- `crates/runx-core/src/policy/local.rs`
- `packages/core/src/parser/index.ts`
- `packages/core/src/policy/index.ts`
- `packages/adapters/src/agent/**`
- `packages/contracts/src/**`
- `skills/issue-to-pr/X.yaml`
- `skills/issue-intake/X.yaml`
- `skills/issue-triage/X.yaml`
- other first-party `skills/**/X.yaml` using affected public vocabulary
- `scripts/dogfood-github-issue-to-pr.mjs`
- `scripts/check-rust-kernel-parity.mjs`
- `tests/replay-run.test.ts`
- `tests/sourcey-preflight.test.ts`
- `tests/issue-to-pr-graph.test.ts`
- `.github/workflows/ci.yml`
- `.github/workflows/release.yml`
- `docs/**` or `README.md` where the live issue-to-PR story is documented

## Risks

- Security regression: making sandbox behavior "friendlier" could silently
  weaken isolation for mutating/credentialed work. Default to fail closed.
- Duplicate external effects: retrying issue-to-PR, outbox, or post-merge
  publication can spam Slack/GitHub or create duplicate PRs. Durable idempotency
  is required before live promotion.
- Contract churn: removing `agent-step` or other stale names touches parser,
  contracts, fixtures, adapters, docs, and skills. Do not leave aliases.
- Over-scoping: trying to solve Aster/cloud/admin UI inside this OSS spec will
  slow the core cutover. Keep core contracts clean and leave hosted surfaces to
  follow-on specs.
- Live dogfood can mutate production channels. Use explicit allowlists and
  controlled test issues; never fire broad Slack/Sentry listeners from local
  guesses.
- Module splits can obscure audit trails if done before behavior is pinned.
  Fix gates and tests first, then decompose around stable boundaries.

## Acceptance

Profile: strict

Validation:
- `pnpm verify:fast`
- `pnpm test:heavy:graph`
- `cd crates && cargo fmt --all --check`
- `cd crates && cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
- `cd crates && cargo test --workspace --locked`
- `cd crates && cargo test --workspace --all-features`
- `cd crates && cargo package -p runx-cli`
- `node scripts/check-rust-kernel-parity.mjs`
- `pnpm live:issue-to-pr -- --mode preflight ...` against an explicit
  allowlisted proving-ground issue
- `pnpm dogfood:github-issue-to-pr -- --mode create ...` against the same
  issue, proving a PR is created/updated by the native runx route
- `pnpm dogfood:github-issue-to-pr -- --mode observe ...` after human merge or
  close, proving exactly one terminal source-thread outcome update

Completion bar:
- Mandatory CI/release commands pass locally and in GitHub Actions.
- The live issue-to-PR create path no longer reports a Rust route unavailable
  blocker.
- Mutating/credentialed execution fails closed when sandbox enforcement,
  credential delivery, network egress, or publication idempotency cannot be
  proven.
- Active public source vocabulary has no stale names or aliases outside archived
  specs/history.
- Issue, PR, and source-thread projections contain rich but bounded context:
  source summary, triage reasoning, decision, scope, changed files, verification,
  risks, reviewer action, and terminal outcome.
- No new broad `rust-style-allow` waiver is added without a focused follow-up
  decomposition phase.

## Phase 1: Repair Mandatory Release Gates

Status: active
Dependencies: none

Objective: Make the current workspace green before changing product behavior.

Changes:
- Remove strict clippy violations in Rust production and test code.
- Fix the native no-TypeScript smoke test by either configuring the payment supervisor fixture path correctly or switching the smoke case to a non-payment native fixture while preserving separate payment supervisor tests.
- Update stale heavy graph cutover expectations to the current native graph behavior, or fix the native exit-code behavior if the old expectation is the desired contract.
- Add a short comment or test name update where expectations changed so future reviewers understand why the cutover assertion moved.

Acceptance:
- [ ] `p1_ac1` command - Rust formatting
  - Command: `cd crates && cargo fmt --all --check`
  - Expected kind: `exit_code_zero`
  - Status: pending
- [ ] `p1_ac2` command - Rust clippy
  - Command: `cd crates && cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
  - Expected kind: `exit_code_zero`
  - Status: pending
- [ ] `p1_ac3` command - Rust locked tests
  - Command: `cd crates && cargo test --workspace --locked`
  - Expected kind: `exit_code_zero`
  - Status: pending
- [ ] `p1_ac4` command - Heavy graph tests
  - Command: `pnpm test:heavy:graph`
  - Expected kind: `exit_code_zero`
  - Status: pending

## Phase 2: Native Issue-To-PR Product Route

Status: pending
Dependencies: Phase 1; coordinate with `runx-graph-skill-issue-to-pr` and
`runx-target-repo-runners`

Objective: Make upstream runx own a real native issue-to-PR create/observe
route that can be dogfooded without Nitrosend wrapper magic.

Changes:
- Replace the stale create blocker in `scripts/dogfood-github-issue-to-pr.mjs`
  with an actual native `runx skill` / target-runner invocation path.
- Ensure dynamic issue/thread inputs, caller answers, target workspace,
  allowlisted repo policy, and run receipts are passed through explicit
  contracts.
- Wire concrete target-runner adapters for checkout readiness, governed runner
  invocation, git mutation, PR observation, and source update publication.
- Keep human merge outside automation; create/update draft PRs only.
- Preserve dedupe before branch/PR creation.
- Emit a machine-readable dogfood result containing issue URL, PR URL, branch,
  run id, receipt refs, source-thread publication refs, and next human gate.

Acceptance:
- [ ] `p2_ac1` command - Issue-to-PR graph test
  - Command: `pnpm test -- tests/issue-to-pr-graph.test.ts`
  - Expected kind: `exit_code_zero`
  - Status: pending
- [ ] `p2_ac2` command - Native route preflight
  - Command: `pnpm live:issue-to-pr -- --mode preflight --allow-repo <owner/repo> --repo <owner/repo> --issue <number> --workspace <path>`
  - Expected kind: `exit_code_zero`
  - Status: pending
- [ ] `p2_ac3` manual - Native create dogfood
  - Command: `pnpm dogfood:github-issue-to-pr -- --mode create --prepare-branch --allow-repo <owner/repo> --repo <owner/repo> --issue <number> --workspace <path>`
  - Expected kind: `exit_code_zero`
  - Status: pending
- [ ] `p2_ac4` manual - GitHub/Slack evidence
  - Command: `gh issue view <number> --repo <owner/repo> --comments --json url,comments && gh pr list --repo <owner/repo> --head <branch> --json url,isDraft,state`
  - Expected kind: `exit_code_zero`
  - Status: pending

## Phase 3: Fail-Closed Runtime Security

Status: pending
Dependencies: Phase 1; coordinate with `runx-security-hardening-v1` and
`process-credential-delivery-hardening-v1`

Objective: Make the Rust runtime's security claims enforceable by default on
mutating or credentialed paths.

Changes:
- Require enforced sandbox backends for mutating or credentialed local execution
  unless an explicit development-only override is supplied.
- Make declared-policy-only execution visible in receipts and reject it for repo
  mutation, external publication, payment, or credentialed subprocesses.
- Canonicalize and validate existing cwd/writable paths before execution; reject
  symlink escape or non-canonical workspace roots where enforcement depends on
  paths.
- Extend configurable HTTP transport validation to resolve DNS and reject
  private/reserved/link-local/metadata addresses before connect; keep redirects
  disabled or revalidated.
- Keep raw credentials out of process env for generic providers unless a
  narrowed, audited delivery contract explicitly permits it.

Acceptance:
- [ ] `p3_ac1` command - Sandbox/security focused tests
  - Command: `cd crates && cargo test --workspace --all-features sandbox runtime_http local_credential credential`
  - Expected kind: `exit_code_zero`
  - Status: pending
- [ ] `p3_ac2` command - Runtime auth security tests
  - Command: `pnpm test -- tests/runtime-local-auth-security.test.ts`
  - Expected kind: `exit_code_zero`
  - Status: pending
- [ ] `p3_ac3` command - Full clippy after security changes
  - Command: `cd crates && cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
  - Expected kind: `exit_code_zero`
  - Status: pending

## Phase 4: Durable Publication And Outcome Idempotency

Status: pending
Dependencies: Phase 2; coordinate with `runx-post-merge-closure-observer`

Objective: Ensure retries and process restarts never duplicate source-thread,
issue, PR, or post-merge outcome publication.

Changes:
- Replace in-memory-only post-merge publication ledger decisions with a durable
  receipt-backed or provider-readback idempotency store.
- Require publication requests to carry stable idempotency keys and validate
  provider observations against them.
- Reuse concurrent bounded stdout/stderr drains for thread outbox providers,
  matching the safer `cli-tool` adapter behavior.
- Add retry tests for duplicate create, duplicate observe, provider timeout,
  provider oversized output, and restart-after-publish.
- Keep Slack/GitHub source updates as replies/comments in the originating
  thread; never root-post milestone updates when thread metadata exists.

Acceptance:
- [ ] `p4_ac1` command - Thread outbox provider contract tests
  - Command: `cd crates && cargo test --workspace --all-features outbox_provider thread_outbox`
  - Expected kind: `exit_code_zero`
  - Status: pending
- [ ] `p4_ac2` command - Post-merge observer tests
  - Command: `cd crates && cargo test --workspace --all-features post_merge_observer`
  - Expected kind: `exit_code_zero`
  - Status: pending
- [ ] `p4_ac3` manual - Observe retry dogfood
  - Command: `pnpm dogfood:github-issue-to-pr -- --mode observe --allow-repo <owner/repo> --repo <owner/repo> --issue <number> --workspace <path>` twice
  - Expected kind: `exit_code_zero`
  - Status: pending

## Phase 5: Clean Public Contract Vocabulary

Status: pending
Dependencies: Phase 1; must be sequenced carefully because this is a hard
cutover

Objective: Finish the naming cleanup with no aliases, no compatibility shims,
and no stale public names in active source.

Changes:
- Decide and document the canonical replacement for `agent-step` before code
  edits. Use the prior friendly-contract direction if still correct; otherwise
  record the new decision in this spec before execution.
- Update parser, contracts, generated schemas, adapters, policies, skills,
  fixtures, tests, docs, and lockfiles in one hard cutover.
- Regenerate all affected contract/schema/fixture artifacts.
- Remove active `agent-step`, `agent_step`, `request-triage`, and `first-send`
  references outside archived specs/history. Do not leave aliases.

Acceptance:
- [ ] `p5_ac1` command - Stale vocabulary sweep
  - Command: `rg -n 'agent-step|agent_step|request-triage|request_triage|first-send|first_send' README.md package.json packages skills tests fixtures scripts crates docs --glob '!crates/target/**'`
  - Expected kind: `no_matches`
  - Status: pending
- [ ] `p5_ac2` command - Contract/schema regeneration check
  - Command: `pnpm verify:fast`
  - Expected kind: `exit_code_zero`
  - Status: pending
- [ ] `p5_ac3` command - Rust parser/contracts
  - Command: `cd crates && cargo test --workspace --all-features runx-parser runx-contracts`
  - Expected kind: `exit_code_zero`
  - Status: pending

## Phase 6: Reviewer-Grade Story Projections

Status: pending
Dependencies: Phase 2 and Phase 4

Objective: Move the useful Nitrosend issue/PR/thread storytelling shape into
runx core so reviewers get rich context without noisy spam.

Changes:
- Define a core projection contract for issue intake, triage result, PR
  creation, human merge gate, verification, and terminal outcome.
- Update target-runner PR body generation to include source summary, triage
  reasoning, scope, changed files, validation, risk/invariants, reviewer action,
  source issue/thread links, and receipt refs.
- Keep detail bounded: comprehensive enough for review, but no full local paths,
  raw secrets, raw Sentry packets, or noisy command dumps.
- Add redaction tests for local paths, env names likely to contain secrets,
  tokens, and oversized provider payloads.
- Ensure product wrappers can supply routing/ownership labels without replacing
  the core projection.

Acceptance:
- [ ] `p6_ac1` command - Projection tests
  - Command: `cd crates && cargo test --workspace --all-features target_runner post_merge_observer`
  - Expected kind: `exit_code_zero`
  - Status: pending
- [ ] `p6_ac2` command - Snapshot/fixture check
  - Command: `pnpm verify:fast`
  - Expected kind: `exit_code_zero`
  - Status: pending
- [ ] `p6_ac3` manual - Reviewer context audit
  - Command: `gh issue view <number> --repo <owner/repo> --comments --json comments && gh pr view <pr> --repo <owner/repo> --json body,comments`
  - Expected kind: `exit_code_zero`
  - Status: pending

## Phase 7: Focused Runtime Decomposition

Status: pending
Dependencies: Phases 1-6 stable enough that behavior is pinned

Objective: Reduce large-module risk without turning the release hardening into
an unrelated rewrite.

Changes:
- Split touched oversized modules only along real boundaries:
  - target-runner planning, adapter invocation, git/PR mutation, source
    publication, and projections
  - post-merge observation, dedupe planning, publication, and receipts
  - sandbox policy validation, backend selection, process command construction,
    and receipt metadata
  - parser source-kind vocabulary and source-specific validation
- Remove `rust-style-allow` waivers where the split makes them unnecessary.
- Keep public APIs stable unless explicitly changed by earlier phases.
- Avoid moving code that is not touched by this spec unless a style guard blocks
  completion.

Acceptance:
- [ ] `p7_ac1` command - Rust style guard
  - Command: `pnpm rust:style`
  - Expected kind: `exit_code_zero`
  - Status: pending
- [ ] `p7_ac2` command - Rust crate graph
  - Command: `pnpm rust:crate-graph`
  - Expected kind: `exit_code_zero`
  - Status: pending
- [ ] `p7_ac3` command - Full Rust tests
  - Command: `cd crates && cargo test --workspace --all-features`
  - Expected kind: `exit_code_zero`
  - Status: pending

## Phase 8: Full Release And Live Dogfood Proof

Status: pending
Dependencies: Phases 1-7

Objective: Prove the final runx shape locally, in CI, and through a controlled
live issue-to-PR lifecycle.

Changes:
- Run the full local validation suite.
- Open or reuse an explicit proving-ground GitHub issue with source-thread
  metadata.
- Execute preflight, create, and observe through the native route.
- Have a human merge or close the PR; observe posts exactly one terminal outcome
  to the issue/source thread.
- Record all evidence in this spec before review.

Acceptance:
- [ ] `p8_ac1` command - Fast workspace verification
  - Command: `pnpm verify:fast`
  - Expected kind: `exit_code_zero`
  - Status: pending
- [ ] `p8_ac2` command - Heavy graph verification
  - Command: `pnpm test:heavy:graph`
  - Expected kind: `exit_code_zero`
  - Status: pending
- [ ] `p8_ac3` command - Rust release verification
  - Command: `cd crates && cargo fmt --all --check && cargo clippy --workspace --all-targets --all-features --locked -- -D warnings && cargo test --workspace --all-features && cargo package -p runx-cli`
  - Expected kind: `exit_code_zero`
  - Status: pending
- [ ] `p8_ac4` command - Kernel/advisory parity
  - Command: `node scripts/check-rust-kernel-parity.mjs`
  - Expected kind: `exit_code_zero`
  - Status: pending
- [ ] `p8_ac5` manual - End-to-end live dogfood
  - Command: `pnpm live:issue-to-pr -- --mode preflight ... && pnpm dogfood:github-issue-to-pr -- --mode create ... && pnpm dogfood:github-issue-to-pr -- --mode observe ...`
  - Expected kind: `exit_code_zero`
  - Status: pending
- [ ] `p8_ac6` manual - GitHub Actions
  - Command: `gh run list --repo runxhq/runx --branch <branch> --limit 5`
  - Expected kind: `exit_code_zero`
  - Status: pending

## Rollback

- Keep each phase in its own logical commit or tightly grouped commit series.
- If Phase 1 fails after changes, revert the release-gate patch and restore the
  previously observed failing evidence before attempting behavior work.
- If Phase 2 live create mutates an incorrect branch/PR/thread, stop, document
  the issue URL/PR URL/thread URL, close the PR without merge, and revert the
  dogfood route patch before retry.
- If Phase 3 sandbox/security changes break legitimate local development, keep
  the fail-closed production behavior and add an explicit dev-only policy knob
  rather than weakening defaults.
- If Phase 4 publication idempotency misfires, disable live create/observe
  publication through the dogfood allowlist until durable dedupe is repaired.
- If Phase 5 vocabulary cutover is too large to land safely, split generated
  artifacts from manual contract edits but do not reintroduce aliases.
- If Phase 7 decomposition destabilizes behavior, revert decomposition only and
  keep the earlier behavioral/security fixes.

## Review

Status: not_started
Verdict: none

Findings:
- none

## Self Eval

- Target score before review: 9.5/10.
- Minimum score to request review: 9.0/10 with all mandatory gates green and
  explicit residual risks documented.
- Any known duplicate-publication, sandbox fail-open, stale contract alias, or
  live create blocker caps the score at 8.0 until fixed.

## Deviations

- none

## Metadata

- created_by: scafld

## Origin

Created by: scafld
Source: plan

## Harden Rounds

### round-1

Status: passed
Started: 2026-05-26T00:28:06Z
Ended: 2026-05-26T00:29:03Z

Checks:
- path audit
  - Grounded in: code:scripts/dogfood-github-issue-to-pr.mjs:430
  - Result: passed
  - Evidence: The spec names the real release-readiness paths: the stale
- command audit
  - Grounded in: spec_gap:acceptance
  - Result: passed
  - Evidence: The spec records the exact failing gates from the local review
- scope/migration audit
  - Grounded in: code:crates/runx-parser/src/skill.rs:76
  - Result: passed
  - Evidence: The spec explicitly treats public vocabulary cleanup as a hard
- acceptance timing audit
  - Grounded in: code:crates/runx-runtime/src/execution/skill_run.rs:690
  - Result: passed
  - Evidence: Phase ordering starts with red gate repair, then native
- rollback/repair audit
  - Grounded in: spec_gap:rollback
  - Result: passed
  - Evidence: Rollback is phase-specific: revert release-gate patches, close
- design challenge
  - Grounded in: code:crates/runx-runtime/src/post_merge_observer.rs:33
  - Result: passed
  - Evidence: The plan attacks the weak product assumptions directly:

Issues:
- none


## Planning Log

- 2026-05-26T08:00+10:00: Reviewed current runx OSS state at
  `/Users/kam/dev/runx/runx/oss`; workspace was clean before creating this
  draft.
- 2026-05-26T08:02+10:00: Verified `pnpm verify:fast` passes.
- 2026-05-26T08:03+10:00: Observed Rust clippy failure under required
  `-D warnings` on `payment_authority.rs` and `schema_wire_compat.rs`.
- 2026-05-26T08:04+10:00: Observed `cargo test --workspace --locked` failing
  `native_cli_smoke_runs_without_node_or_typescript_env` because the payment
  approval harness lacks a configured rail supervisor.
- 2026-05-26T08:05+10:00: Observed `pnpm test:heavy:graph` failing two stale
  native graph cutover expectations while the issue-to-PR graph tests passed.
- 2026-05-26T08:06+10:00: Confirmed `scripts/dogfood-github-issue-to-pr.mjs`
  still blocks create mode as `dogfood_create_rust_route_unavailable` even
  though native graph skill execution now supports graph runners.
- 2026-05-26T08:08+10:00: Drafted this umbrella spec to coordinate existing
  narrow specs into one release-readiness bar.
- 2026-05-27: Replaced the stale dogfood create blocker with the existing
  native `runx skill skills/issue-to-pr` graph route. The harness now hydrates
  the GitHub issue thread, passes explicit workspace/branch/scafld/policy/repo
  snapshot inputs, returns `needs_agent` continuation instructions with
  `run_id` and `--answers`, and reports sealed PR/source-thread evidence from
  the existing `thread.push_outbox` boundary. No new adapter path was added.
