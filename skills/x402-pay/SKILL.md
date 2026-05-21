---
name: x402-pay
description: Govern an x402 payment-required signal through quote, reserve, settlement, and receipt sealing.
runx:
  category: payments
---

# X402 Pay

Run the x402 payment graph.

The graph turns a payment-required signal into a quote, selects and reserves a
payment decision, routes approval when required, fulfills the x402 rail under
attenuated authority, and leaves recovery evidence if the rail result is
ambiguous.

This is the unpinned x402 marquee: the paid-tool challenge supplies the payment
surface, while core still owns quote admission, reservation, single-use spend
authority, recovery, and receipt-before-success.

## Quality Profile

- Purpose: execute a paid action through runx authority without hiding the
  payment governance path.
- Audience: agent hosts, operators, approval reviewers, and receipt verifiers.
- Artifact contract: `payment_execution`, `payment_quote_packet`,
  `payment_reservation_packet`, `payment_rail_packet`, and `recovery_packet`
  when needed.
- Evidence bar: every successful execution carries a quote, selected decision,
  reserved child authority, idempotency key, rail proof ref, and receipt seal
  requirement.
- Voice bar: operator-grade execution record; avoid wallet/product marketing.
- Strategic bar: keep rails pluggable while core owns payment authority.
- Stop conditions: stop before rail execution when quote, approval, parent
  authority, reservation, idempotency, or spend capability is missing.

## Output

- `payment_execution`: overall status and receipt/proof refs.
- `payment_quote_packet`: normalized quote output.
- `payment_reservation_packet`: selected reservation decision and child
  authority term.
- `payment_rail_packet`: rail proof and credential envelope.
- `recovery_packet`: recovery assessment when a rail result is ambiguous.

## Inputs

- `payment_signal` (required): payment-required signal or challenge.
- `parent_payment_authority` (required): parent payment authority term or ref.
- `rail_profile_ref` (required): configured rail profile reference.
- `realm` (optional): authority realm.
- `spend_policy` (optional): policy limits and approval thresholds.
- `approval_context` (optional): prior approval evidence.
- `idempotency_seed` (optional): stable idempotency material.
