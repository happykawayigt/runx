---
name: mock-charge
description: Model provider-side charge verification through the deterministic mock settlement family.
runx:
  category: payments
---

# Mock Charge

Compose provider-side charge pricing, challenge emission, credential
verification, receipt sealing, and modeled forwarding for the deterministic
mock settlement family.

This graph profile is for local harnesses, demos, and contract tests. It makes
the authority transition visible without claiming executable provider-side
runtime forwarding.

## Quality Profile

- Purpose: show the provider-side charge graph using deterministic local
  settlement evidence.
- Audience: operators, registry tooling, and future runtime implementers.
- Artifact contract: `charge_price_packet`, `charge_challenge_packet`,
  `charge_verification_packet`, `charge_seal`, and `forwarded_result`.
- Evidence bar: success requires price, challenge, verification proof, sealed
  receipt ref, and a modeled forward gate.
- Strategic bar: keep mock deterministic and avoid raw rail or merchant
  credentials.
- Stop conditions: stop before modeled forwarding when verification lacks a
  sealed receipt ref.

## Output

- `charge_price_packet`: provider-side price and requested authority.
- `charge_challenge_packet`: `payment_required` challenge and idempotency key.
- `charge_verification_packet`: mock settlement proof and receipt ref.
- `charge_seal`: modeled child receipt seal.
- `forwarded_result`: modeled upstream result gated by the seal.

## Inputs

- `mcp_tool_call` (required): inbound MCP operation request.
- `provider_policy` (required): provider price and family policy.
- `returned_credential` (required): mock credential envelope or reference.
- `parent_payment_authority` (optional): parent payment authority term or ref.
- `verify_capability_ref` (required): single-use verification capability ref.
- `idempotency_seed` (optional): stable challenge idempotency seed.
