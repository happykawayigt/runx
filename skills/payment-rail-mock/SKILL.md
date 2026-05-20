---
name: payment-rail-mock
description: Produce deterministic mock rail proof under reserved runx payment authority.
---

# Payment Rail Mock

Fulfill one deterministic local rail operation below the runx spend gate.

This skill exists for harnesses, demos, and contract tests. It models the shape
of a rail mutation without claiming live provider behavior. It can run only
after the parent harness has selected a payment Decision, reserved budget by
idempotency key, and passed an attenuated `payment` authority term plus a
single-use spend capability reference into the child harness.

The skill receives references, never raw funding material. It returns a stable
mock proof payload/ref suitable for sealing into the child harness receipt.

## Quality Profile

- Purpose: demonstrate payment rail receipt discipline with deterministic local
  proof material.
- Audience: runtime harness, receipt verifier, rail implementer, and operator.
- Artifact contract: `rail_result`, `rail_proof`, `credential_envelope`,
  `redactions`, and `recovery_hint`.
- Evidence bar: include rail response refs, idempotency key, challenge id, and
  proof hash/ref. Redact sensitive payload fields.
- Voice bar: operational status only: fulfilled, declined, retryable,
  recovered, or ambiguous.
- Strategic bar: keep provider behavior out of this mock; keep governance in
  core.
- Stop conditions: return `needs_agent` or `ambiguous` when the result cannot
  be tied to the idempotency key and reserved authority.

## Output

- `rail_result`: mock rail status, amount, currency, counterparty, and
  operation.
- `rail_proof`: redacted proof payload or proof ref for the child harness
  receipt.
- `credential_envelope`: paid-tool credential reference, with sensitive fields
  redacted or referenced.
- `redactions`: fields withheld from receipts and logs.
- `recovery_hint`: idempotency/retry guidance for `payment-recover`.

## Inputs

- `payment_challenge` (required): protocol/provider challenge to fulfill.
- `reserved_payment_authority` (required): child payment authority term.
- `spend_capability_ref` (required): scoped single-use spend capability ref.
- `rail_profile_ref` (required): configured rail profile reference.
- `idempotency` (required): reservation key and recovery fields.
- `quote_packet` (optional): source quote packet for evidence continuity.
