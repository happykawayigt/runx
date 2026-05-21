---
name: stripe-charge
description: Model provider-side charge verification through the Stripe settlement family.
runx:
  category: payments
---

# Stripe Charge

Compose provider-side charge pricing, challenge emission, credential
verification, receipt sealing, and modeled forwarding for Stripe-style
credential verification.

This graph profile is registry documentation and harness shape. It does not
perform live Stripe calls, read merchant credentials, or enable runtime
forwarding.

## Quality Profile

- Purpose: show how provider-side Stripe charge verification fits the governed
  charge graph.
- Audience: operators, registry tooling, and future Stripe adapter
  implementers.
- Artifact contract: `charge_price_packet`, `charge_challenge_packet`,
  `charge_verification_packet`, `charge_seal`, and `forwarded_result`.
- Evidence bar: success requires priced bounds, challenge idempotency,
  Stripe-family proof ref, receipt ref, and modeled forward gate.
- Strategic bar: keep Stripe credential material behind references.
- Stop conditions: stop before modeled forwarding when verification lacks a
  sealed receipt ref.

## Inputs

- `mcp_tool_call` (required): inbound MCP operation request.
- `provider_policy` (required): provider price and family policy.
- `returned_credential` (required): Stripe credential envelope or reference.
- `parent_payment_authority` (optional): parent payment authority term or ref.
- `verify_capability_ref` (required): single-use verification capability ref.
- `idempotency_seed` (optional): stable challenge idempotency seed.
