---
name: x402-charge
description: Model unpinned provider-side charge verification selected from the inbound credential family.
runx:
  category: payments
---

# X402 Charge

Compose provider-side charge pricing, challenge emission, credential
verification, receipt sealing, and modeled forwarding while leaving settlement
family selection to the inbound credential and provider policy.

This graph profile documents the future unpinned provider charge surface. In
v1 it is static registry shape only: no dynamic runtime dispatch, no live
settlement, and no upstream forwarding.

## Quality Profile

- Purpose: show the unpinned provider-side charge graph without hiding pricing,
  challenge, verify, seal, or forward gates.
- Audience: operators, registry tooling, and future runtime implementers.
- Artifact contract: `charge_price_packet`, `charge_challenge_packet`,
  `charge_verification_packet`, `charge_seal`, and `forwarded_result`.
- Evidence bar: success requires priced bounds, accepted families, credential
  family, verification proof, receipt ref, and modeled forward gate.
- Strategic bar: keep dynamic dispatch as metadata until runtime owns it.
- Stop conditions: stop before modeled forwarding when credential family is not
  accepted or verification lacks a sealed receipt ref.

## Inputs

- `mcp_tool_call` (required): inbound MCP operation request.
- `provider_policy` (required): provider price and family policy.
- `returned_credential` (required): payment credential envelope or reference.
- `parent_payment_authority` (optional): parent payment authority term or ref.
- `verify_capability_ref` (required): single-use verification capability ref.
- `idempotency_seed` (optional): stable challenge idempotency seed.
