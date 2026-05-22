use runx_contracts::{JsonObject, ProofKind, ReferenceType};
use runx_runtime::receipts::step_receipt;
use runx_runtime::{InvocationStatus, SkillOutput};

const CREATED_AT: &str = "2026-05-18T00:00:00Z";

#[test]
fn payment_rail_receipts_carry_proof_and_scoped_credential_refs()
-> Result<(), Box<dyn std::error::Error>> {
    let output = SkillOutput {
        status: InvocationStatus::Success,
        stdout: r#"{"payment_rail_packet":{"data":{"rail_result":{"status":"fulfilled","rail":"mock","amount_minor":125,"currency":"USD"},"rail_proof":{"proof_ref":"receipt-proof:mock:demo-search-001","idempotency_key":"payment:demo-search-001"},"credential_envelope":{"form":"paid_tool_credential","credential_ref":"credential:mock:demo-search-001"}}}}"#.to_owned(),
        stderr: String::new(),
        exit_code: Some(0),
        duration_ms: 10,
        metadata: JsonObject::new(),
    };

    let receipt = step_receipt("payment_execute", "fulfill", 1, &output, CREATED_AT)?;
    let act = &receipt.acts[0];

    let verification_refs: Vec<_> = act
        .criteria
        .iter()
        .flat_map(|criterion| criterion.verification_refs.iter())
        .collect();
    assert!(verification_refs.iter().any(|reference| {
        reference.reference_type == ReferenceType::Verification
            && reference.uri == "receipt-proof:mock:demo-search-001"
            && reference.locator.as_deref() == Some("payment:demo-search-001")
            && reference.proof_kind.as_ref() == Some(&ProofKind::PaymentRail)
    }));
    // The scoped credential ref rides on the criterion evidence refs.
    let evidence_refs: Vec<_> = act
        .criteria
        .iter()
        .flat_map(|criterion| criterion.evidence_refs.iter())
        .collect();
    assert!(evidence_refs.iter().any(|reference| {
        reference.reference_type == ReferenceType::Credential
            && reference.uri == "credential:mock:demo-search-001"
    }));
    Ok(())
}
