---
name: payment-fulfill
description: Deterministically fulfill an approved payment through the fixture rail.
source:
  type: cli-tool
  command: node
  args:
    - -e
    - "process.stdout.write(JSON.stringify({payment_rail_packet:{data:{rail_result:{status:'fulfilled',rail:'mock',amount_minor:125,currency:'USD'},rail_proof:{proof_ref:'receipt-proof:mock:payment-execution-001',idempotency_key:'payment:payment-execution-001',rail_session_material_ref:'rail-session-material:mock:payment-execution-001'},credential_envelope:{form:'paid_tool_credential',credential_ref:'credential:mock:payment-execution-001'}}}}))"
  timeout_seconds: 10
  sandbox:
    profile: readonly
    cwd_policy: skill-directory
inputs: {}
---

Use this fixture only to prove the payment approval graph seals with rail proof.
