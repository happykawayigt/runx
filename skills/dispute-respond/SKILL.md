---
name: dispute-respond
description: Prepare a governed dispute response artifact from a linked charge receipt.
runx:
  category: payments
---

# Dispute Respond

Prepare a profile-local dispute response artifact for a counterparty-initiated
payment dispute.

This skill attaches the linked sealed charge receipt, prior refund receipts,
and provider evidence, then selects a response posture. It does not settle the
dispute or produce a rail closure receipt.

## Quality Profile

- Purpose: keep disputes separate from silent refunds.
- Audience: provider operators, reviewers, registry tooling, and future dispute
  runtime enforcement.
- Artifact contract: `dispute_response`, `dispute_evidence`,
  `linked_receipts`, `posture`, and `open_questions`.
- Evidence bar: every response must cite the original charge receipt and any
  prior refund receipts.
- Strategic bar: never mask an open dispute by recommending an untracked refund.
- Stop conditions: return `needs_agent` when dispute id, linked receipt, prior
  refunds, or evidence posture is ambiguous.

## Inputs

- `dispute_event` (required): provider-initiated dispute or chargeback event.
- `original_receipt_ref` (required): linked sealed charge receipt reference.
- `prior_refund_receipt_refs` (optional): prior refund receipts.
- `evidence_refs` (optional): provider evidence references.
- `operator_posture` (optional): requested response posture.
