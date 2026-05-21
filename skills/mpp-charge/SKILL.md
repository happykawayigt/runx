---
name: mpp-charge
description: Model provider-side charge verification through the MPP settlement family.
runx:
  category: payments
---

# MPP Charge

Compose provider-side charge pricing, challenge emission, credential
verification, receipt sealing, and modeled forwarding for the multi-party
payment protocol settlement family.

This graph profile records registry and harness shape only. It does not
perform live settlement, read rail credentials, or enable runtime forwarding.

## Quality Profile

- Purpose: show how MPP provider-side credential verification fits the governed
  charge graph.
- Audience: operators, registry tooling, and future MPP adapter implementers.
- Artifact contract: `charge_price_packet`, `charge_challenge_packet`,
  `charge_verification_packet`, `charge_seal`, and `forwarded_result`.
- Evidence bar: success requires priced bounds, challenge idempotency,
  MPP-family proof ref, receipt ref, and modeled forward gate.
- Strategic bar: keep MPP credential material behind references.
- Stop conditions: stop before modeled forwarding when verification lacks a
  sealed receipt ref.

## Inputs

- `mcp_tool_call` (required): inbound MCP operation request.
- `provider_policy` (required): provider price and family policy.
- `returned_credential` (required): MPP credential envelope or reference.
- `parent_payment_authority` (optional): parent payment authority term or ref.
- `verify_capability_ref` (required): single-use verification capability ref.
- `idempotency_seed` (optional): stable challenge idempotency seed.
