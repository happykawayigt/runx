use runx_contracts::{
    EffectSettlementPhase, EffectSettlementReceipt, EffectSettlementReceiptSchema, JsonObject,
    JsonValue, ProofKind, Reference, ReferenceType,
};

const STRIPE_SPT_RAIL: &str = "stripe-spt";

#[test]
fn payment_finality_deferred_chain_reaches_sealed_at_threshold() {
    let original = Reference::runx(ReferenceType::Receipt, "receipt_payment_original");
    let proof = x402_tx_proof_reference(
        "0x1111111111111111111111111111111111111111111111111111111111111111",
    );

    let provisional = effect_settlement_receipt(EffectSettlementReceiptInput {
        id: "payment-finality-provisional",
        created_at: "2026-06-01T00:00:00Z".to_owned(),
        phase: EffectSettlementPhase::Provisional,
        original_receipt_ref: original.clone(),
        criterion_id: "criterion_payment_finality".to_owned(),
        proof_ref: None,
        evidence_refs: Vec::new(),
        confirmation_depth: None,
        payload: finality_payload("mpp-tempo", "submitted"),
    });
    let in_flight_1 = effect_settlement_receipt(EffectSettlementReceiptInput {
        id: "payment-finality-in-flight-1",
        created_at: "2026-06-01T00:00:10Z".to_owned(),
        phase: EffectSettlementPhase::InFlight,
        original_receipt_ref: original.clone(),
        criterion_id: "criterion_payment_finality".to_owned(),
        proof_ref: Some(proof.clone()),
        evidence_refs: vec![Reference::runx(ReferenceType::Artifact, &provisional.id)],
        confirmation_depth: Some(1),
        payload: finality_payload("mpp-tempo", "confirming"),
    });
    let in_flight_2 = effect_settlement_receipt(EffectSettlementReceiptInput {
        id: "payment-finality-in-flight-2",
        created_at: "2026-06-01T00:00:20Z".to_owned(),
        phase: EffectSettlementPhase::InFlight,
        original_receipt_ref: original.clone(),
        criterion_id: "criterion_payment_finality".to_owned(),
        proof_ref: Some(proof.clone()),
        evidence_refs: vec![Reference::runx(ReferenceType::Artifact, &in_flight_1.id)],
        confirmation_depth: Some(2),
        payload: finality_payload("mpp-tempo", "confirming"),
    });
    let sealed = effect_settlement_receipt(EffectSettlementReceiptInput {
        id: "payment-finality-sealed",
        created_at: "2026-06-01T00:00:30Z".to_owned(),
        phase: EffectSettlementPhase::Sealed,
        original_receipt_ref: original.clone(),
        criterion_id: "criterion_payment_finality".to_owned(),
        proof_ref: Some(proof.clone()),
        evidence_refs: vec![Reference::runx(ReferenceType::Artifact, &in_flight_2.id)],
        confirmation_depth: Some(3),
        payload: finality_payload("mpp-tempo", "sealed"),
    });

    assert_eq!(provisional.phase, EffectSettlementPhase::Provisional);
    assert_eq!(provisional.confirmation_depth, None);
    assert_eq!(in_flight_1.phase, EffectSettlementPhase::InFlight);
    assert_eq!(in_flight_1.confirmation_depth, Some(1));
    assert_eq!(in_flight_2.phase, EffectSettlementPhase::InFlight);
    assert_eq!(in_flight_2.confirmation_depth, Some(2));
    assert_eq!(sealed.phase, EffectSettlementPhase::Sealed);
    assert_eq!(sealed.confirmation_depth, Some(3));
    assert_eq!(sealed.proof_ref.as_ref(), Some(&proof));
    assert_eq!(proof.proof_kind, Some(ProofKind::PaymentRail));

    for receipt in [&provisional, &in_flight_1, &in_flight_2, &sealed] {
        assert_eq!(receipt.family.as_ref(), "payment");
        assert_eq!(receipt.original_receipt_ref, original);
    }
    assert_eq!(
        in_flight_1.evidence_refs,
        vec![Reference::runx(ReferenceType::Artifact, &provisional.id)]
    );
    assert_eq!(
        in_flight_2.evidence_refs,
        vec![Reference::runx(ReferenceType::Artifact, &in_flight_1.id)]
    );
    assert_eq!(
        sealed.evidence_refs,
        vec![Reference::runx(ReferenceType::Artifact, &in_flight_2.id)]
    );
    assert_ne!(provisional.id, in_flight_1.id);
    assert_ne!(in_flight_1.id, in_flight_2.id);
    assert_ne!(in_flight_2.id, sealed.id);
}

#[test]
fn payment_finality_provider_event_seals_directly_without_confirmation_depth() {
    let original = Reference::runx(ReferenceType::Receipt, "receipt_stripe_original");
    let mut proof = Reference::with_uri(
        ReferenceType::Verification,
        "mpp-fiat:payment_intent:pi_test",
    );
    proof.proof_kind = Some(ProofKind::PaymentRail);
    proof.provider = Some(STRIPE_SPT_RAIL.into());

    let sealed = effect_settlement_receipt(EffectSettlementReceiptInput {
        id: "payment-finality-provider-sealed",
        created_at: "2026-06-01T00:00:05Z".to_owned(),
        phase: EffectSettlementPhase::Sealed,
        original_receipt_ref: original.clone(),
        criterion_id: "criterion_payment_finality".to_owned(),
        proof_ref: Some(proof.clone()),
        evidence_refs: Vec::new(),
        confirmation_depth: None,
        payload: finality_payload(STRIPE_SPT_RAIL, "provider_event_sealed"),
    });

    assert_eq!(sealed.phase, EffectSettlementPhase::Sealed);
    assert_eq!(sealed.confirmation_depth, None);
    assert_eq!(sealed.original_receipt_ref, original);
    assert_eq!(sealed.proof_ref.as_ref(), Some(&proof));
}

fn finality_payload(rail: &str, status: &str) -> JsonObject {
    JsonObject::from([
        ("rail".to_owned(), JsonValue::String(rail.to_owned())),
        ("status".to_owned(), JsonValue::String(status.to_owned())),
    ])
}

struct EffectSettlementReceiptInput {
    id: &'static str,
    created_at: String,
    phase: EffectSettlementPhase,
    original_receipt_ref: Reference,
    criterion_id: String,
    proof_ref: Option<Reference>,
    evidence_refs: Vec<Reference>,
    confirmation_depth: Option<u64>,
    payload: JsonObject,
}

fn effect_settlement_receipt(input: EffectSettlementReceiptInput) -> EffectSettlementReceipt {
    EffectSettlementReceipt {
        schema: EffectSettlementReceiptSchema::V1,
        id: input.id.into(),
        created_at: input.created_at.into(),
        family: "payment".into(),
        phase: input.phase,
        original_receipt_ref: input.original_receipt_ref,
        criterion_id: input.criterion_id.into(),
        proof_ref: input.proof_ref,
        evidence_refs: input.evidence_refs,
        confirmation_depth: input.confirmation_depth,
        payload: input.payload,
    }
}

fn x402_tx_proof_reference(tx_hash: &str) -> Reference {
    let mut reference = Reference::with_uri(ReferenceType::Verification, tx_hash);
    reference.proof_kind = Some(ProofKind::PaymentRail);
    reference.provider = Some("x402".into());
    reference
}
