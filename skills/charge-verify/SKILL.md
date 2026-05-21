---
name: charge-verify
description: Verify a returned payment credential and produce provider-side charge receipt evidence.
runx:
  category: payments
---

# Charge Verify

Verify a returned payment credential against a priced provider-side challenge.

This skill models the verification step before a provider forwards a paid MCP
operation. It selects the settlement-family verifier from profile inputs,
checks idempotency evidence, emits a settlement proof reference, and exposes
the receipt evidence that a future runtime must seal before forwarding. It
does not set prices, issue challenges, receive raw merchant credentials, or
forward the upstream tool call.

## Quality Profile

- Purpose: make credential verification evidence explicit before provider-side
  forwarding.
- Audience: provider harnesses, operators, receipt reviewers, and future charge
  runtime enforcement.
- Artifact contract: `verification_result`, `settlement_proof`,
  `sealed_receipt_ref`, `redactions`, and `recovery_hint`.
- Evidence bar: every successful verification must carry settlement family,
  idempotency key, proof ref, and redaction notes.
- Strategic bar: verify under already-priced authority; never widen price,
  settlement family, or forwarding authority.
- Stop conditions: return `escalated` when the credential family, idempotency
  state, or proof fields are ambiguous.

## Output

- `verification_result`: credential verification status and settlement family.
- `settlement_proof`: proof ref or payload with sensitive fields redacted.
- `sealed_receipt_ref`: receipt ref that must exist before future forwarding.
- `redactions`: sensitive fields omitted from the receipt surface.
- `recovery_hint`: modeled recovery state such as `sealed` or
  `reversal_required`.

## Inputs

- `charge_price_packet` (required): output from `charge-price`.
- `charge_challenge_packet` (required): output from `charge-challenge`.
- `returned_credential` (required): caller-returned payment credential.
- `priced_payment_authority` (required): admitted provider-side payment term.
- `verify_capability_ref` (required): single-use verification capability ref.
- `settlement_family` (required): selected settlement family.
- `idempotency` (required): challenge idempotency key and replay policy.
