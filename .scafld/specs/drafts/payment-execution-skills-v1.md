---
spec_version: '2.0'
task_id: payment-execution-skills-v1
created: '2026-05-20T00:00:00Z'
updated: '2026-05-20T12:55:31Z'
status: completed
harden_status: not_run
size: medium
risk_level: high
---

# Payment execution skills v1

## Current State

Status: completed
Current phase: final
Next: harden
Reason: first-party payment skill skeletons and X.yaml profiles now exercise
the payment authority term through quote, reserve, approval, deterministic mock
rail fulfillment, and recovery inspection without claiming live runtime payment
behavior.
Allowed follow-up command: `scafld harden payment-execution-skills-v1`
Latest runner update: 2026-05-20T12:55:31Z
Review gate: pass

## Summary

Payment execution in runx is a governed graph over the existing spine. The
first-party skills make the flow legible to humans and registry tooling, but
they do not own payment truth. Core owns the spend decision, reservation,
idempotency lookup, authority subset proof, recovery path, and receipt-before-
success invariant. Rail skills run only below that gate with attenuated payment
authority and a single-use spend capability.

## Skill Set

`payment-quote`
: Turns a `payment_required` signal, MCP payment challenge, invoice request, or
operator intent into a quote packet plus requested payment authority bounds.
It is non-mutating and receives no rail secrets.

`payment-reserve`
: Selects or declines the payment intent. The output is a Decision-shaped
reservation packet containing payment bounds, idempotency key, approval status,
and the child authority term that may be passed to a rail harness. It does not
call a rail.

`payment-rail-*`
: Fulfills one protocol or provider family under an already reserved child
payment authority term. Each rail skill receives a challenge, a redacted rail
profile ref, and a single-use spend capability ref. It returns a rail proof
payload/ref suitable for sealing into the child harness receipt.

`payment-recover`
: Reconciles an idempotency key after crash, timeout, retry, or ambiguous rail
state. It must query by idempotency key before any repeat mutation and returns a
recovered proof, a safe retry recommendation, or an escalation.

`payment-execute`
: The graph profile that composes quote, reserve, approval, rail fulfillment,
and recovery handoff. It is the first "seamless agent payments" surface: a
paid tool call can enter as one request and leave as a sealed payment receipt
without hiding the governance steps.

## Spine Mapping

- Challenge input is `SignalType::PaymentRequired`.
- Quote output is evidence and requested authority, not spend.
- Reservation is a selected `Decision` and authority subset proof metadata.
- Rail fulfillment is a child `Harness` with one terminal `Act`.
- Rail proof is receipt payload/reference with sensitive fields redacted.
- Ledger/reporting remains a projection over sealed receipts.

## Core-Owned Rules

- Core compares child and parent `AuthorityTerm` values with the payment
authority partial order before rail execution starts.
- Core reserves budget atomically by idempotency key before the rail skill runs.
- Core derives or validates the single-use spend capability for `spend`.
- Core performs idempotency recovery before retrying a mutating rail call.
- Core refuses success until the child harness receipt carries the rail proof.

## Skill-Owned Rules

- A quote skill may normalize protocol-specific challenge fields and recommend
payment bounds, but it cannot authorize spend.
- A reserve skill may present the human-readable decision record, but it cannot
mint broader authority than core admits.
- A rail skill may adapt one payment protocol/provider, but it cannot set caps,
read raw wallet secrets, or decide approval.
- A recovery skill may inspect idempotency state and rail proof refs, but it
cannot hide an ambiguous spend as success.

## Initial Rail Families

- `payment-rail-mock`: deterministic local rail for harnesses, demos, and
contract tests.
- `payment-rail-x402`: HTTP payment challenge/credential exchange.
- `payment-rail-mpp`: multi-party payment protocol family.
- `payment-rail-stripe-spt`: Stripe session/payment token family.

These names are first-party skill packages, not hardcoded core concepts. Core
only sees payment authority terms, idempotency keys, child harnesses, and
receipt proof refs.

## Acceptance Criteria

- Each payment skill has a human-readable `SKILL.md`.
- Each payment skill has an `X.yaml` profile with concrete inputs, outputs,
artifacts, and harness cases.
- The graph profile makes the authority transition visible:
  quote -> reserve -> optional approval -> rail fulfill.
- Rail profiles declare payment authority metadata under `runx` and never
declare raw secret inputs.
- Existing profile parsing validates all new payment X.yaml files.
- No runtime or CLI payment behavior is claimed until the runtime harness owns
reserve-before-rail and receipt-before-success enforcement.
