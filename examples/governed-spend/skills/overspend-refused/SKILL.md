---
name: overspend-refused
description: A prompt-injected agent runs the x402 payment graph and tries to overspend; runx refuses at the rail gate, before any provider call.
runx:
  category: payments
---

# overspend-refused (demo)

This is the jaw-drop case of this demo. It is the `x402-pay`
graph (quote, reserve, approve, fulfill), unchanged except for one number.

The reserve step grants the agent a bounded child authority capped at **100
minor (1.00) per call** on the `x402` rail to one counterparty. A malicious tool
response (the demo's stand-in for the kind of prompt injection that has drained
real agent wallets) makes the agent try to fulfill a **125-minor (1.25) spend**.

The fulfill step is where runx's payment-rail admission lives. It recomputes the
spend binding against the reserved child cap, sees `125 > 100`, and **blocks the
graph before the x402 rail is ever called**:

> authority Spend denied graph step 'fulfill': payment spend capability binding
> does not match the child harness act

The seeded `pay-fulfill-rail` output in `X.yaml` is what the compromised agent
*wants* to happen (a fulfilled 1.25 spend). Admission never lets it run. runx
holds no wallet and no spend credential and calls no rail. It does hold a
receipt-signing key and signs its receipts, which is what makes them verifiable.

The rail here is `x402`, but the refusal is rail-agnostic: the same comparator
governs `mpp` and `stripe`. The rail is a field; the governance is the product.
