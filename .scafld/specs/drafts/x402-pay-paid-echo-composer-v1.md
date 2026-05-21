---
spec_version: '2.0'
task_id: x402-pay-paid-echo-composer-v1
created: '2026-05-21T00:46:25Z'
updated: '2026-05-21T00:46:25Z'
status: draft
harden_status: not_run
size: large
risk_level: high
---

# x402-pay paid-echo composer dogfood v1

## Current State

Status: draft
Current phase: planning
Next: harden
Reason: Follow-up for the paid-surface and composer deferrals from `x402-pay-dogfood-v1`.
Blockers: none
Allowed follow-up command: `scafld harden x402-pay-paid-echo-composer-v1`
Latest runner update: none
Review gate: not_started

## Summary

Introduce a local-only `paid-echo` dogfood surface and prove that composer
paid-tool interception can trigger the existing payment graph without leaking
raw rail artifacts. The goal is an end-to-end local paid-tool loop:
`payment_required` signal, quote, reserve, mock settlement, sealed receipt,
and returned echo result.

## Scope And Touchpoints

In scope:

- `fixtures/paid-echo/**`
- `tests/x402-pay-paid-echo-composer.test.ts`
- `scripts/dogfood-paid-echo-composer.mjs`
- Composer/runtime-local code only if hardening confirms the current composer
  has no extension point for paid-tool interception
- `scripts/dogfood-core-skills.mjs` only to add the new dogfood script after
  it is deterministic

Out of scope:

- Live-money rails and Stripe test mode.
- Internal paid surfaces.
- Additional payment skill renames or alias compatibility paths.
- Native `runx x402-pay`, `runx receipts`, or `runx ledger` commands.
- Provider-side charge forwarding.

## Planned Phases

Phase 1: local paid-echo fixture.
: Add a deterministic local server/tool fixture that emits a
`payment_required` signal for one tool and accepts only a fulfilled credential
for that same tool.

Phase 2: composer interception.
: Route the local signal through the existing payment graph using mock rail
settlement and return the paid tool result only after the receipt is sealed.

Phase 3: negative paths.
: Prove unsupported challenge, denied approval, idempotency replay, and raw
rail artifact suppression.

## Acceptance

Profile: strict

Definition of done:
- [ ] `dod1` Local paid-echo success returns the echo result only after a sealed
  payment receipt exists.
- [ ] `dod2` Composer sees governed success or governed error only; no raw rail
  payload is exposed.
- [ ] `dod3` Negative paths cover denial, malformed challenge, and idempotency
  replay.

Validation:
- [ ] `v1` test - Paid-echo composer dogfood test passes.
  - Command: `pnpm exec vitest run tests/x402-pay-paid-echo-composer.test.ts`
  - Expected kind: `exit_code_zero`
  - Timeout seconds: none
  - Result: none
  - Status: pending
  - Evidence: none
  - Source event: none
  - Last attempt: none
  - Checked at: none
- [ ] `v2` dogfood - Paid-echo dogfood script passes.
  - Command: `node scripts/dogfood-paid-echo-composer.mjs`
  - Expected kind: `exit_code_zero`
  - Timeout seconds: none
  - Result: none
  - Status: pending
  - Evidence: none
  - Source event: none
  - Last attempt: none
  - Checked at: none
- [ ] `v3` regression - Core dogfood remains green.
  - Command: `node scripts/dogfood-core-skills.mjs`
  - Expected kind: `exit_code_zero`
  - Timeout seconds: none
  - Result: none
  - Status: pending
  - Evidence: none
  - Source event: none
  - Last attempt: none
  - Checked at: none

## Rollback

Strategy: per_phase

Commands:
- `git checkout HEAD -- fixtures/paid-echo tests/x402-pay-paid-echo-composer.test.ts scripts/dogfood-paid-echo-composer.mjs scripts/dogfood-core-skills.mjs`

## Harden Rounds

- none

## Planning Log

- 2026-05-21T00:46:25Z: Filed from the paid-echo and composer deferrals in the
  completed mock-only dogfood spec.
