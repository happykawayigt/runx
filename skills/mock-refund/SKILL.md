---
name: mock-refund
description: Model a same-family mock refund against a sealed charge receipt.
---

# Mock Refund

Compose refund quote, refund reserve, optional approval, and deterministic mock
refund settlement against a linked sealed charge receipt.

This graph profile is for local harnesses, demos, and contract tests. It does
not perform a live rail mutation or claim runtime refund enforcement.

## Quality Profile

- Purpose: show the provider-initiated refund graph for the mock family.
- Audience: operators, registry tooling, and future refund runtime
  implementers.
- Artifact contract: `refund_quote_packet`, `refund_reservation_packet`,
  `refund_approval`, and `refund_rail_packet`.
- Evidence bar: every step carries the original receipt ref and same settlement
  family.
- Strategic bar: refuse cross-family refund shape in profile examples.
- Stop conditions: stop before settlement when original receipt link,
  reservation, approval, or idempotency is missing.

## Inputs

- `original_receipt_ref` (required): linked sealed charge receipt reference.
- `original_receipt` (required): redacted original charge receipt summary.
- `refund_request` (required): requested amount and reason.
- `parent_payment_authority` (required): parent payment authority term or ref.
- `approval_context` (optional): prior approval evidence.
- `idempotency_seed` (optional): stable refund idempotency seed.
