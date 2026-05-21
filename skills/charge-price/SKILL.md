---
name: charge-price
description: Price an inbound provider-side paid tool call without collecting payment.
runx:
  category: payments
---

# Charge Price

Turn an inbound MCP operation plus provider policy into a charge price packet
and requested provider-side payment authority.

This skill is the first read-only step in a provider charge flow. It classifies
the requested operation, selects the smallest acceptable price and settlement
family set, and records the policy evidence that a challenge step can expose to
the caller. It does not issue a challenge, verify a credential, forward the
upstream tool call, or receive rail credentials.

## Quality Profile

- Purpose: make provider-side price and authority bounds explicit before any
  `payment_required` signal is emitted.
- Audience: provider harnesses, operators, registry tooling, and future charge
  runtime enforcement.
- Artifact contract: `charge_price`, `requested_payment_authority`,
  `price_evidence`, `policy_metadata`, and `open_questions`.
- Evidence bar: every amount, currency, operation, counterparty, and settlement
  family must come from provider policy, the inbound operation, or a named
  inference.
- Strategic bar: preserve the narrowest authority shape that can satisfy the
  operation; never widen rails or caps for convenience.
- Stop conditions: return `needs_agent` when price, currency, operation,
  counterparty, or settlement families are ambiguous.

## Output

- `charge_price`: normalized provider-side price, accepted settlement
  families, operation, counterparty, and expiry.
- `requested_payment_authority`: requested `payment` authority bounds for the
  later challenge and verify steps.
- `price_evidence`: source refs and redacted policy facts used to set price.
- `policy_metadata`: provider-facing policy labels and routing hints.
- `open_questions`: missing data that blocks challenge emission.

## Inputs

- `mcp_tool_call` (required): inbound MCP operation request.
- `provider_policy` (required): pricing policy and settlement family allowlist.
- `parent_payment_authority` (optional): parent payment authority term or ref.
- `realm` (optional): provider realm such as `local`, `test`, or `prod`.
- `idempotency_seed` (optional): stable material for challenge idempotency.
