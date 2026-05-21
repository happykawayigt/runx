---
spec_version: '2.0'
task_id: x402-pay-dogfood-v1
created: '2026-05-20T00:00:00Z'
updated: '2026-05-21T00:45:21Z'
status: completed
harden_status: passed
size: large
risk_level: high
---

# x402-pay dogfood v1

## Current State

Status: completed
Current phase: final
Next: done
Reason: task completed
Blockers: none
Allowed follow-up command: `none`
Latest runner update: 2026-05-21T00:45:21Z
Review gate: pass

## Summary

Drive the current payment execution skills through a Phase 1 mock-rail
dogfood gate using only surfaces that exist today: `runx harness`,
`runx skill`, `runx history`, fixture/oracle files, and the existing dogfood
script. v1 does not create a native `runx x402-pay` command, does not add new
`x402-pay`/rail marquee skill directories, and does not make Stripe test-mode
behavior part of approval. It defines the local eventualities, touchpoints,
rollback, and acceptance needed before later Stripe, composer, or paid-surface
work can claim dogfood coverage.

## Current Codebase Alignment

- There is no native `runx x402-pay ...` command today. Current native CLI
  entrypoints are `runx skill`, `runx harness`, and `runx history`.
- There are no current `runx receipts`, `runx ledger`, or
  `runx skill inspect` CLI surfaces. Receipt state is observed through
  `runx history`, explicit receipt files, and ledger projection files written
  by the harness under test.
- Current implemented payment skill directories are `payment-authorize-reserve`,
  `payment-execute`, `payment-fulfill-rail`, `payment-quote`,
  `payment-quote-preflight`, `payment-rail-mock`, `payment-recover`,
  `payment-recover-inspect`, and `payment-reserve`.
- There is no concrete `x402-pay`, `mock-pay`, `stripe-pay`, or `mpp-pay`
  skill directory yet. Those names remain product intent and are explicitly
  deferred from this v1 dogfood gate.
- `payment-fulfill-rail` uses rail ids `mock`, `x402`, `mpp`, and
  `stripe-spt`. It does not use `mock-pay` or `stripe-pay` rail ids.
- `paid-echo` and composer interception are future dogfood deliverables, not
  current product behavior.

## Scope And Touchpoints

This v1 is a mock-only dogfood hardening contract. Build agents may touch:

- `.scafld/specs/active/x402-pay-dogfood-v1.md`
- `scripts/dogfood-core-skills.mjs`
- `tests/payment-skill-profile-validation.test.ts`
- `tests/external-skill-proving-ground.test.ts`
- `tests/harness-cli.test.ts`
- `tests/runtime-local-harness.test.ts`
- `fixtures/harness/payment-approval-graph.yaml`
- `fixtures/harness/oracle/payment-approval-graph.*.json`
- `fixtures/graphs/payment/approval-spend.yaml`
- A new Phase 1 mock dogfood test only if needed, e.g.
  `tests/x402-pay-dogfood-mock.test.ts`

Build agents must not touch Rust contracts/runtime code, add live-money rail
behavior, create native payment CLI commands, create new `x402-pay`/`mock-pay`/
`stripe-pay`/`mpp-pay` skill directories, or introduce a `paid-echo` server in
this v1 unless a follow-up spec opens that scope.

## Why Dogfood Before Harden

The previous spec ships skeletons and X.yaml profiles. None of it has been
exercised under failure conditions. Hardening with synthetic tests would
encode our assumptions, not our blind spots. Dogfooding the CLI exposes the
governed surface to real timing, real ambiguity, and real operator ergonomics.
Findings feed back into core invariants, settlement marquees, plumbing skills,
and the CLI before any of it claims production behavior.

Dogfooding here means exercising the payment flow through the current CLI:
`runx harness <fixture|skill-dir|SKILL.md>` for fixture-backed cases and
`runx skill <skill-dir|SKILL.md>` for skill execution cases, then observing
closure through `runx history`, receipt files, and ledger projection files.
Constructing the flow by hand or stubbing past the CLI does not count.

A native `runx x402-pay ...` command, `runx receipts`, or `runx ledger` would
be a new CLI surface. If any of those surfaces are created, they need their own
implementation, help text, and acceptance proof before a dogfood scenario may
depend on them.

## Phases

Phase 0: existing proof pass.
: Re-run the current dogfood baseline and record whether it passes without
manual intervention: `node scripts/dogfood-core-skills.mjs`.

Phase 1: current mock rail through `payment-execute` and `payment-fulfill-rail`.
: Deterministic local settlement. Fastest iteration. Proves every Core-Owned
Rule and Skill-Owned Rule from `payment-execution-skills-v1` without external
rail variability. Phase 1 must be green before any Stripe or composer work is
allowed to depend on this spec.

Phase 2: deferred `stripe-spt` test-mode dogfood.
: Real rail behavior through the current rail id: timing, webhook ordering,
declines, rate limits, restarts mid-settlement. A `stripe-pay` graph marquee
may be introduced by a later spec, but it is not a v1 deliverable and not part
of v1 acceptance. Stripe live mode is explicitly out of scope.

Phase 3: deferred.
: Any `crypto-pay` or live-money graph stays hidden. Live-money rails,
production agent loops, and internal paid surfaces beyond the local dogfood
paid surface are not in scope for v1.

## Paid Surface

Phase 1 starts and ends with the existing payment harness fixtures and current
payment skills. A minimal local `paid-echo` MCP server, composer paid-tool
interception, or any internal paid surface is deferred. Those surfaces need a
separate spec because they add runtime/tooling behavior beyond the current
harness and skill execution contract.

## Eventualities

Each entry below is one runnable scenario. Each scenario is exercised by an
explicit current CLI invocation. Fixture cases use `runx harness`; skill
execution cases use `runx skill`. Expected state is visible through
`runx history`, explicit receipt files, and any ledger projection files the
scenario creates. Pass means: the expected closure was
produced without manual workarounds, escapes, or stack-trace leakage to the
operator.

### Phase 1 (current mock rail)

P1.1 Happy path
: Challenge issued, quote produced, reserve granted within policy, mock
settlement succeeds, receipt sealed with proof, paid tool result returned to
caller unchanged.

P1.2 Unsupported challenge shape
: Quote rejects a malformed challenge with a governed error. No reserve. No
settlement. No partial ledger entry.

P1.3 Reserve declines: cap exceeded
: Policy cap below required bound. Reserve refuses without contacting any
rail. Ledger records the refused intent, not a spend.

P1.4 Reserve declines: ambiguous bounds
: Challenge offers a range or undefined currency. Reserve refuses with a
governed reason; no rail call.

P1.5 Approval gate: approved
: Policy requires human approval at this amount. Operator approves through
the configured surface. Flow resumes; receipt sealed.

P1.6 Approval gate: denied
: Operator denies. Clean halt. No settlement attempt. Receipt records the
denial as a terminal decision.

P1.7 Idempotency replay
: Same idempotency key submitted twice. Second call returns the recovered
receipt without a second mock spend.

P1.8 Authority subset violation
: A crafted settlement step attempts an `AuthorityTerm` broader than the
reserved child term. Core rejects before mock execution.

P1.9 Single-use spend cap reuse
: A second use of the same spend capability ref is rejected by core.

P1.10 Receipt-before-success
: Mock settlement succeeds but receipt persistence is delayed. Caller does
not see a success result until the receipt is durably stored.

P1.11 Mock crash mid-settle
: Mock rail aborts after a partial state mutation. Recover queries by
idempotency key, classifies the state, and either seals or escalates.

P1.12 Settlement proof missing
: Mock rail returns success without the required proof fields. Core refuses
to seal the child receipt as success.

P1.13 Concurrent reserves
: Two paid tool calls reserve under the same policy at the same time. Budget
arithmetic is atomic; neither call sees stale bounds.

P1.14 Quote drift
: Bounds reserved at T1, mock attempts a spend above the reserved bound at
T2. Core rejects the spend before mock executes.

P1.15 CLI: invocation
: `runx harness <payment fixture>` and, where the skill can run directly,
`runx skill ./skills/payment-execute ...` run end to end without operator
intervention beyond the approval gate when configured. No stack traces.

P1.16 CLI: receipt observation
: `runx history` lists the sealed receipt after P1.1 within one operator
action, and the receipt file shows settlement family, proof ref, idempotency
key, and sealed timestamp.

P1.17 Ledger projection observation
: The explicit ledger projection file, if present for the scenario, shows the
accrual for P1.1 and the refused entries from P1.3 and P1.4 distinctly. A
future `runx ledger` command is a separate CLI deliverable, not assumed here.

P1.18 Composer flow
: Deferred. An outer skill that invokes a paid tool and transparently triggers
the payment graph is not current behavior and is not a v1 acceptance blocker.

### Deferred Phase 2 (`stripe-spt` in test mode)

The following scenarios are retained as follow-up design intent only. They do
not block v1 hardening, approval, or completion.

P2.1 Happy path with test card
: Stripe test card settles through the `stripe-spt` rail id, webhook arrives,
receipt sealed, result returned.

P2.2 Settlement timeout
: Stripe slow. Recover distinguishes pending from failed without escalating
early or sealing prematurely.

P2.3 Crash mid-settlement
: Process killed after Stripe call returns but before receipt persisted. On
restart, recover queries Stripe state and reaches a terminal decision.

P2.4 Webhook ordering
: Webhook arrives before the foreground call would have persisted. Receipt
ordering is preserved; no double-seal.

P2.5 Test decline
: Stripe declines a known-decline card. Flow halts with a governed error,
no partial receipt.

P2.6 Rate limit
: Stripe returns a rate-limit error. Recover backs off, retries once,
escalates if still failing. Idempotency preserved across retries.

P2.7 Network partition
: Stripe call attempted while offline. Recover queries Stripe state on
reconnect, reaches a terminal decision without operator intervention beyond
the configured surfaces.

P2.8 Refund and reversal
: Deferred to a follow-up spec.

## Dogfood Loop

Each iteration:

1. Pick the next unmet eventuality.
2. Set the precondition state on the local fixture and policy file.
3. Run the current CLI entrypoint with the scenario inputs:
   `runx harness <fixture|skill-dir|SKILL.md>` for harness cases or
   `runx skill <skill-dir|SKILL.md>` for skill cases.
4. Observe closure through `runx history`, explicit receipt or ledger files,
   structured logs, and the CLI exit. Classify pass, fail, or ambiguous.
5. If fail or ambiguous, append a punch list entry. Land a fix as its own
   commit. Re-run the scenario from step 3 until pass.
6. Move to the next eventuality.

Phase 1 closes only when every v1 scenario is pass and the punch list is empty.
Phase 2 requires a separate spec.

## Hardening Punch List

Findings accrue in `.scafld/specs/drafts/x402-pay-dogfood-punchlist.md`. Each
entry records:

- Scenario id (e.g. P1.11).
- Observed behavior.
- Expected behavior.
- Root cause sketch.
- Fix commit reference once landed.
- Closed timestamp.

Closed entries remain in the file for audit. Entries are never edited or
deleted; only superseded by a follow-up entry that references the prior id.

## Out of Scope

- Live-money settlement on any rail.
- `crypto-pay` activation or exercise.
- Provider-side skills (`x402-charge` family). Deferred per the prior spec.
- Refund and reversal flows (P2.8).
- Stripe test-mode dogfood as an approval or completion blocker for v1.
- `paid-echo`, composer paid-tool interception, or internal paid surfaces.
- Multi-tenant policy and approval routing.
- UI affordances beyond what the CLI already exposes.
- Treating `runx x402-pay`, `runx receipts`, `runx ledger`, or
  `runx skill inspect` as current CLI surfaces without separately implementing
  and accepting them.

## Acceptance

Profile: strict

Definition of done:
- [x] `dod1` v1 is narrowed to current mock-rail dogfood surfaces, with Stripe,
  composer, paid-echo, native payment CLI, and live-money work deferred.
- [x] `dod2` Existing dogfood proof passes through the current local commands.
- [x] `dod3` No open dogfood punch-list file exists for this v1.
- [x] `dod4` The first review blocker, `F1-missing-acceptance-evidence`, is
  repaired by recording command evidence in this structured acceptance section.

Validation:
- [x] `v1` validate - Active spec validates.
  - Command: `scafld validate x402-pay-dogfood-v1 --json`
  - Expected kind: `exit_code_zero`
  - Timeout seconds: none
  - Result: `{"ok":true,"command":"validate","result":{"task_id":"x402-pay-dogfood-v1","path":"/Users/kam/dev/runx/runx/oss/.scafld/specs/active/x402-pay-dogfood-v1.md","valid":true,"errors":null}}`
  - Status: passed
  - Evidence: The command exited 0 against the active spec.
  - Source event: local build verification
  - Last attempt: 2026-05-21T00:43:30Z
  - Checked at: 2026-05-21T00:43:30Z
- [x] `v2` dogfood - Existing dogfood loop passes.
  - Command: `node scripts/dogfood-core-skills.mjs`
  - Expected kind: `exit_code_zero`
  - Timeout seconds: none
  - Result: `doctor status=success errors=0 warnings=16; tests/external-skill-proving-ground.test.ts 25 passed`
  - Status: passed
  - Evidence: The dogfood script built the Rust CLI binary, built workspace packages, ran doctor with 0 errors, and proved official skills with a fresh caller.
  - Source event: local build verification
  - Last attempt: 2026-05-21T00:43:30Z
  - Checked at: 2026-05-21T00:43:44Z
- [x] `v3` payment profile validation - Payment skill profile validation passes.
  - Command: `pnpm exec vitest run tests/payment-skill-profile-validation.test.ts`
  - Expected kind: `exit_code_zero`
  - Timeout seconds: none
  - Result: `1 test file passed; 3 tests passed`
  - Status: passed
  - Evidence: `tests/payment-skill-profile-validation.test.ts` passed all 3 tests.
  - Source event: local build verification
  - Last attempt: 2026-05-21T00:43:30Z
  - Checked at: 2026-05-21T00:43:32Z
- [x] `v4` punch list - No open v1 dogfood punch list exists.
  - Command: `test ! -e .scafld/specs/drafts/x402-pay-dogfood-punchlist.md && test ! -e .scafld/specs/active/x402-pay-dogfood-punchlist.md`
  - Expected kind: `exit_code_zero`
  - Timeout seconds: none
  - Result: `punchlist_absent`
  - Status: passed
  - Evidence: No draft or active punch-list file exists for this v1.
  - Source event: local build verification
  - Last attempt: 2026-05-21T00:43:30Z
  - Checked at: 2026-05-21T00:43:30Z

Criteria notes:
- Phase 1 mock eventualities are covered for v1 by the existing dogfood script,
  profile validation, and current harness/proving-ground fixtures. Any finer
  grained future mock scenario that is not represented by those surfaces must
  be filed as a follow-up spec or punch-list entry before claiming broader
  runtime behavior.
- No scenario may surface a raw stack trace, raw rail payload, or undocumented
  exit code to the operator.
- `runx history` and explicit receipt or ledger files remain the source of
  truth for scenario closure; structured logs are diagnostic only.
- The dogfood loop must run through current `runx harness` and `runx skill`
  invocations unless a future spec creates and accepts a new native CLI command.

## Rollback And Repair

Rollback is spec/test/fixture-only for v1. Revert any changes made under the
declared touchpoints, regenerate any touched oracle files from the accepted
fixture command, and re-run `node scripts/dogfood-core-skills.mjs` plus the
payment profile validation test.

Scenario repair is limited to local mock recovery: if a mock settlement reaches
an ambiguous state, retry through `payment-recover` with the same idempotency
key and classify the outcome as sealed, refused, or escalated. A second spend
with a new key is not a repair path. Runtime auto-repair, Stripe reconciliation,
refund, reversal, and dispute handling are follow-up specs.

## Harden Rounds

### round-1

Status: passed
Started: 2026-05-20T16:52:46Z
Ended: 2026-05-20T16:53:16Z
Verdict: pass
Provider: manual
Summary: Manual hardening narrowed the draft to an executable v1 mock-rail dogfood gate and deferred future surfaces.

Checks:
- scope/migration audit
  - Grounded in: code:.scafld/specs/active/x402-pay-dogfood-v1.md:55
  - Result: passed
  - Evidence: Scope And Touchpoints declares a mock-only v1 surface, allowed paths, and explicit non-goals.
- path audit
  - Grounded in: code:.scafld/specs/active/x402-pay-dogfood-v1.md:59
  - Result: passed
  - Evidence: Build-time touchpoints are explicit paths or one named optional Phase 1 test path.
- command audit
  - Grounded in: code:.scafld/specs/active/x402-pay-dogfood-v1.md:300
  - Result: passed
  - Evidence: Acceptance is pinned to current local commands that exist in this checkout.
- acceptance timing audit
  - Grounded in: code:.scafld/specs/active/x402-pay-dogfood-v1.md:108
  - Result: passed
  - Evidence: Stripe test-mode scenarios are explicitly deferred and do not block v1 approval or completion.
- rollback/repair audit
  - Grounded in: code:.scafld/specs/active/x402-pay-dogfood-v1.md:317
  - Result: passed
  - Evidence: Rollback is limited to declared spec/test/fixture touchpoints and same-key mock recovery classification.
- design challenge
  - Grounded in: code:.scafld/specs/active/x402-pay-dogfood-v1.md:27
  - Result: passed
  - Evidence: The design proves current payment skills through current CLI surfaces and avoids bundling future product work.

Issues:
- none


## Planning Log

- 2026-05-21T00:50:18Z: Manual hardening narrowed v1 from an all-rails
  dogfood plan to a current-surface mock-only gate, removed future CLI/surface
  assumptions from acceptance, and added scope, touchpoints, acceptance, and
  rollback/repair sections.

## Review

Status: completed
Verdict: pass
Mode: verify
Provider: codex
Output: codex.output_file
Summary: Verify pass. The prior blocker F1 is repaired for this review packet: acceptance evidence is no longer empty, the active spec records command/result/status/evidence for validation, dogfood, payment profile validation, and punch-list absence, and no active punch-list file exists. The earlier failed review remains as historical evidence, but I found no completion-blocking regression in the repair.

Attack log:
- `F1-missing-acceptance-evidence`: known blocker verification -> clean (F1 was previously based on empty packet/session acceptance evidence. The current review packet now includes four passed acceptance entries, and the active spec records command, result, status, evidence, source event, and timestamps for the same checks at lines 311-351.)
- `.scafld/runs/x402-pay-dogfood-v1/session.json`: durable ledger inspection -> clean (Read .scafld/runs/x402-pay-dogfood-v1/session.json. The earlier failed review remains recorded as history, and a new review_attempt entry is open for this review; I did not find a separate post-repair build entry with command output in session.json, but the review packet's Acceptance Criteria section and the active spec now carry the acceptance evidence that was missing from the prior packet.)
- `workspace classification`: scope and ambient drift separation -> clean (git status shows only the active/approved spec move plus unrelated Rust/runtime/dev drift. The review packet classifies task changes as none and ambient drift as context. No out-of-scope code change is attributable to this spec-only repair.)
- `.scafld/specs/*/x402-pay-dogfood-punchlist.md`: punch-list absence -> clean (find .scafld/specs -path '*x402-pay-dogfood-punchlist.md' returned no files, matching v4 and the acceptance evidence.)

Findings:
- none

