---
spec_version: '2.0'
task_id: x402-pay-stripe-spt-dogfood-v1
created: '2026-05-21T00:46:25Z'
updated: '2026-05-21T00:46:25Z'
status: draft
harden_status: not_run
size: large
risk_level: high
---

# x402-pay Stripe SPT dogfood v1

## Current State

Status: draft
Current phase: planning
Next: harden
Reason: Follow-up for the deferred Phase 2 scenarios from `x402-pay-dogfood-v1`.
Blockers: none
Allowed follow-up command: `scafld harden x402-pay-stripe-spt-dogfood-v1`
Latest runner update: none
Review gate: not_started

## Summary

Dogfood `stripe-pay` and its `pay-fulfill-rail` `stripe-spt` rail id in
Stripe test mode only. This spec covers test-card success, decline, timeout,
webhook ordering, crash/recover, rate limit, and reconnect behavior without
introducing live-money behavior.

## Scope And Touchpoints

In scope:

- `tests/x402-pay-stripe-spt-dogfood.test.ts`
- `fixtures/harness/stripe-spt/**`
- `fixtures/graphs/payment/stripe-spt-*.yaml`
- `scripts/dogfood-stripe-spt.mjs`
- `skills/stripe-pay/SKILL.md`
- `skills/stripe-pay/X.yaml`
- `skills/pay-fulfill-rail/SKILL.md`
- `skills/pay-fulfill-rail/X.yaml`
- Existing payment profile validation tests if new fixture metadata needs
  validation

Out of scope:

- Stripe live mode.
- Persisting real card data, API keys, webhook secrets, or raw credentials.
- Additional payment skill renames or alias compatibility paths.
- Refund, reversal, and dispute flows.
- Native `runx x402-pay`, `runx receipts`, or `runx ledger` commands.

## Planned Phases

Phase 1: offline Stripe test fixtures.
: Add deterministic fixtures for success, decline, timeout, retry, and webhook
ordering using recorded/sanitized test-mode shapes with no secrets.

Phase 2: gated Stripe test-mode dogfood.
: Add a script that runs only when explicit Stripe test-mode env vars are
present and refuses live keys.

Phase 3: recovery eventualities.
: Prove crash/recover and reconnect behavior against the same idempotency key.

## Acceptance

Profile: strict

Definition of done:
- [ ] `dod1` Offline fixtures cover all P2.1-P2.7 eventualities from
  `x402-pay-dogfood-v1`.
- [ ] `dod2` Test-mode dogfood refuses live Stripe keys and never commits
  secret material.
- [ ] `dod3` Recovery uses idempotency-preserving queries and never issues a
  second spend with a new key.

Validation:
- [ ] `v1` test - Offline Stripe SPT dogfood tests pass.
  - Command: `pnpm exec vitest run tests/x402-pay-stripe-spt-dogfood.test.ts`
  - Expected kind: `exit_code_zero`
  - Timeout seconds: none
  - Result: none
  - Status: pending
  - Evidence: none
  - Source event: none
  - Last attempt: none
  - Checked at: none
- [ ] `v2` dogfood - Stripe test-mode script passes when test env is present.
  - Command: `node scripts/dogfood-stripe-spt.mjs`
  - Expected kind: `exit_code_zero`
  - Timeout seconds: none
  - Result: none
  - Status: pending
  - Evidence: If env is absent, this spec cannot complete; skip is not a pass.
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
- `git checkout HEAD -- tests/x402-pay-stripe-spt-dogfood.test.ts fixtures/harness/stripe-spt fixtures/graphs/payment scripts/dogfood-stripe-spt.mjs skills/stripe-pay skills/pay-fulfill-rail tests/payment-skill-profile-validation.test.ts`

## Harden Rounds

- none

## Planning Log

- 2026-05-21T00:46:25Z: Filed from deferred Phase 2 `stripe-spt` scenarios in
  the completed mock-only dogfood spec.
