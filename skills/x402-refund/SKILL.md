---
name: x402-refund
description: Model unpinned same-family refund dispatch from the linked original receipt.
---

# X402 Refund

Compose refund quote, refund reserve, optional approval, and a modeled
same-family refund settlement where the family is selected from the linked
original charge receipt.

This graph profile documents the future unpinned refund surface. In v1 it is
static registry shape only: no dynamic runtime dispatch and no live rail
mutation.

## Quality Profile

- Purpose: show the unpinned provider-side refund graph without hiding receipt
  linkage or same-family refusal.
- Audience: operators, registry tooling, and future runtime implementers.
- Artifact contract: `refund_quote_packet`, `refund_reservation_packet`,
  `refund_approval`, and `refund_rail_packet`.
- Evidence bar: every step carries the original receipt ref, selected family,
  refund idempotency key, and proof ref.
- Strategic bar: keep dynamic dispatch as metadata until runtime owns it.
- Stop conditions: stop before settlement when the requested family does not
  match the original receipt family.

## Inputs

- `original_receipt_ref` (required): linked sealed charge receipt reference.
- `original_receipt` (required): redacted original charge receipt summary.
- `refund_request` (required): requested amount and reason.
- `parent_payment_authority` (required): parent payment authority term or ref.
- `approval_context` (optional): prior approval evidence.
- `idempotency_seed` (optional): stable refund idempotency seed.
