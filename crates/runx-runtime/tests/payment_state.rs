use runx_runtime::payment_state::{
    FileBackedPaymentStateStore, MockRailMutation, MockRailMutationStatus, PaymentIdempotencyEntry,
    PaymentIdempotencyKey, PaymentRecoveryState, SpendCapabilityConsumption,
};

#[test]
fn persists_payment_state_across_fresh_store() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let path = temp.path().join("payment-state.json");
    let idempotency_key =
        PaymentIdempotencyKey::new("mock", "merchant:paid-echo", "payment:paid-echo-001");

    {
        let mut store = FileBackedPaymentStateStore::open(&path)?;
        store.record_idempotency(PaymentIdempotencyEntry {
            idempotency_key: idempotency_key.clone(),
            receipt_ref: "receipt:paid-echo:first".to_owned(),
            rail_proof_ref: "receipt-proof:mock:paid-echo-001".to_owned(),
            amount_minor: 125,
            currency: "USD".to_owned(),
        })?;
        store.record_mock_rail_mutation(MockRailMutation {
            idempotency_key: idempotency_key.clone(),
            rail: "mock".to_owned(),
            amount_minor: 125,
            currency: "USD".to_owned(),
            counterparty: "merchant:paid-echo".to_owned(),
            status: MockRailMutationStatus::Fulfilled,
            proof_ref: Some("receipt-proof:mock:paid-echo-001".to_owned()),
            recovery_state: PaymentRecoveryState::Sealed,
        })?;
    }

    let store = FileBackedPaymentStateStore::open(&path)?;
    let entry = store
        .lookup_idempotency(&idempotency_key)
        .ok_or("idempotency entry should survive fresh store open")?;
    assert_eq!(entry.receipt_ref, "receipt:paid-echo:first");
    assert_eq!(entry.rail_proof_ref, "receipt-proof:mock:paid-echo-001");

    let mutation = store
        .lookup_mock_rail_mutation(&idempotency_key)
        .ok_or("mock rail mutation should survive fresh store open")?;
    assert_eq!(mutation.status, MockRailMutationStatus::Fulfilled);
    assert_eq!(mutation.recovery_state, PaymentRecoveryState::Sealed);

    Ok(())
}

#[test]
fn records_consumed_spend_capability_for_reuse_lookup() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let path = temp.path().join("nested").join("payment-state.json");
    let idempotency_key =
        PaymentIdempotencyKey::new("mock", "merchant:paid-echo", "payment:paid-echo-001");
    let capability_ref = "runx:payment-capability:paid-echo-spend-1";

    let mut store = FileBackedPaymentStateStore::open(&path)?;
    store.consume_spend_capability(SpendCapabilityConsumption {
        capability_ref: capability_ref.to_owned(),
        idempotency_key: idempotency_key.clone(),
        receipt_ref: Some("receipt:paid-echo:first".to_owned()),
        recovery_state: Some(PaymentRecoveryState::Sealed),
    })?;

    let store = FileBackedPaymentStateStore::open(&path)?;
    let consumed = store
        .lookup_consumed_spend_capability(capability_ref)
        .ok_or("consumed spend capability should be persisted")?;
    assert_eq!(consumed.idempotency_key, idempotency_key);
    assert_eq!(
        consumed.receipt_ref.as_deref(),
        Some("receipt:paid-echo:first")
    );

    Ok(())
}

#[test]
fn rejects_duplicate_spend_capability_consumption() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let path = temp.path().join("payment-state.json");
    let idempotency_key =
        PaymentIdempotencyKey::new("mock", "merchant:paid-echo", "payment:paid-echo-001");
    let capability_ref = "runx:payment-capability:paid-echo-spend-1";
    let consumption = SpendCapabilityConsumption {
        capability_ref: capability_ref.to_owned(),
        idempotency_key,
        receipt_ref: Some("receipt:paid-echo:first".to_owned()),
        recovery_state: Some(PaymentRecoveryState::Sealed),
    };

    let mut store = FileBackedPaymentStateStore::open(&path)?;
    store.consume_spend_capability(consumption.clone())?;

    let error = store
        .consume_spend_capability(consumption)
        .err()
        .ok_or("duplicate spend capability should be rejected")?;
    assert_eq!(
        error.to_string(),
        "spend capability runx:payment-capability:paid-echo-spend-1 was already consumed"
    );

    Ok(())
}

#[test]
fn rejects_duplicate_idempotency_and_rail_mutation_without_overwrite()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let path = temp.path().join("payment-state.json");
    let idempotency_key =
        PaymentIdempotencyKey::new("mock", "merchant:paid-echo", "payment:paid-echo-001");
    let mut store = FileBackedPaymentStateStore::open(&path)?;

    store.record_idempotency(PaymentIdempotencyEntry {
        idempotency_key: idempotency_key.clone(),
        receipt_ref: "receipt:first".to_owned(),
        rail_proof_ref: "proof:first".to_owned(),
        amount_minor: 125,
        currency: "USD".to_owned(),
    })?;
    let idempotency_error = store
        .record_idempotency(PaymentIdempotencyEntry {
            idempotency_key: idempotency_key.clone(),
            receipt_ref: "receipt:second".to_owned(),
            rail_proof_ref: "proof:second".to_owned(),
            amount_minor: 250,
            currency: "USD".to_owned(),
        })
        .err()
        .ok_or("duplicate idempotency record should be rejected")?;
    assert_eq!(
        idempotency_error.to_string(),
        "idempotency key mock\u{1f}merchant:paid-echo\u{1f}payment:paid-echo-001 was already recorded"
    );

    let stored = store
        .lookup_idempotency(&idempotency_key)
        .ok_or("original idempotency entry should remain stored")?;
    assert_eq!(stored.receipt_ref, "receipt:first");

    store.record_mock_rail_mutation(MockRailMutation {
        idempotency_key: idempotency_key.clone(),
        rail: "mock".to_owned(),
        amount_minor: 125,
        currency: "USD".to_owned(),
        counterparty: "merchant:paid-echo".to_owned(),
        status: MockRailMutationStatus::Partial,
        proof_ref: None,
        recovery_state: PaymentRecoveryState::InFlight,
    })?;
    let mutation_error = store
        .record_mock_rail_mutation(MockRailMutation {
            idempotency_key: idempotency_key.clone(),
            rail: "mock".to_owned(),
            amount_minor: 250,
            currency: "USD".to_owned(),
            counterparty: "merchant:paid-echo".to_owned(),
            status: MockRailMutationStatus::Fulfilled,
            proof_ref: Some("proof:second".to_owned()),
            recovery_state: PaymentRecoveryState::Sealed,
        })
        .err()
        .ok_or("duplicate mock rail mutation should be rejected")?;
    assert_eq!(
        mutation_error.to_string(),
        "mock rail mutation for idempotency key mock\u{1f}merchant:paid-echo\u{1f}payment:paid-echo-001 was already recorded"
    );

    let mutation = store
        .lookup_mock_rail_mutation(&idempotency_key)
        .ok_or("original rail mutation should remain stored")?;
    assert_eq!(mutation.status, MockRailMutationStatus::Partial);
    assert_eq!(mutation.recovery_state, PaymentRecoveryState::InFlight);

    Ok(())
}
