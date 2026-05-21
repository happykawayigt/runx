use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use runx_contracts::{HarnessReceipt, ProofKind, Reference, ReferenceType};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const PAYMENT_LEDGER_PROJECTION_SCHEMA_VERSION: &str = "runx.payment_ledger_projection.v1";
pub const X402_PAY_PAYMENT_PROFILE: &str = "x402-pay";
pub const PAYMENT_LEDGER_PROJECTED_EVENT_KIND: &str = "payment_ledger_projected";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaymentLedgerProjection {
    pub schema_version: String,
    pub payment_profile: String,
    pub scenario_id: String,
    pub source_receipt_id: String,
    pub disposition: PaymentLedgerDisposition,
    pub accrual: PaymentLedgerAccrual,
    pub refusal: Option<PaymentLedgerRefusal>,
    pub evidence_refs: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaymentLedgerProjectionArtifact {
    pub artifact_id: String,
    pub artifact_type: String,
    pub path: PathBuf,
    pub event_payload: PaymentLedgerProjectedEventPayload,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaymentLedgerProjectedEventPayload {
    pub kind: String,
    pub payment_profile: String,
    pub projection_artifact_id: String,
    pub projection_artifact_path: String,
    pub source_receipt_id: String,
    pub scenario_id: String,
    pub disposition: PaymentLedgerDisposition,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaymentLedgerDisposition {
    Settled,
    Refused,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaymentLedgerAccrual {
    pub amount_minor: u64,
    pub currency: String,
    pub rail: String,
    pub counterparty: String,
    pub operation: String,
    pub idempotency_key: String,
    pub rail_proof_refs: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaymentLedgerRefusal {
    pub reason_code: String,
    pub refused_stage: String,
    pub rail_call_performed: bool,
    pub ledger_spend_recorded: bool,
}

#[derive(Clone, Debug)]
pub struct PaymentLedgerProjectionInput<'a> {
    pub graph_receipt: &'a HarnessReceipt,
    pub scenario_id: &'a str,
    pub evidence: Vec<PaymentLedgerEvidence<'a>>,
}

#[derive(Clone, Debug)]
pub struct PaymentLedgerEvidence<'a> {
    pub receipt: &'a HarnessReceipt,
    pub packet: PaymentLedgerEvidencePacket,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PaymentLedgerEvidencePacket {
    Reservation(PaymentReservationEvidence),
    RailSettlement(PaymentRailSettlementEvidence),
    Refusal(PaymentRefusalEvidence),
    PaidTool(PaidToolEvidence),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PaymentReservationEvidence {
    pub amount_minor: u64,
    pub currency: String,
    pub rail: String,
    pub counterparty: String,
    pub operation: String,
    pub idempotency_key: String,
    pub spend_capability_ref: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PaymentRailSettlementEvidence {
    pub amount_minor: u64,
    pub currency: String,
    pub rail: String,
    pub proof_ref: String,
    pub idempotency_key: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PaymentRefusalEvidence {
    pub reason_code: String,
    pub refused_stage: String,
    pub rail_call_performed: bool,
    pub ledger_spend_recorded: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PaidToolEvidence {
    pub payment_proof_ref: String,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PaymentLedgerProjectionError {
    #[error("payment ledger projection requires at least one reservation evidence packet")]
    MissingReservation,
    #[error("payment ledger projection requires a rail settlement or refusal evidence packet")]
    MissingDispositionEvidence,
    #[error(
        "settlement evidence proof ref {proof_ref} is not present as a typed payment rail proof on receipt {receipt_id}"
    )]
    MissingReceiptRailProof {
        receipt_id: String,
        proof_ref: String,
    },
    #[error("paid tool evidence proof ref {proof_ref} has no matching settlement proof")]
    PaidToolProofMismatch { proof_ref: String },
    #[error(
        "child receipt {child_receipt_id} is not referenced by graph receipt {graph_receipt_id}"
    )]
    ChildReceiptNotReferenced {
        graph_receipt_id: String,
        child_receipt_id: String,
    },
    #[error("settlement evidence does not match reservation evidence")]
    SettlementReservationMismatch,
    #[error(
        "payment ledger projection source receipt id {source_receipt_id} is not a harness receipt ref"
    )]
    InvalidSourceReceiptId { source_receipt_id: String },
    #[error("payment ledger projection artifact already exists with different contents at {path}")]
    ArtifactConflict { path: PathBuf },
    #[error("payment ledger projection artifact I/O failed at {path}: {message}")]
    ArtifactIo { path: PathBuf, message: String },
    #[error("payment ledger projection artifact JSON failed at {path}: {message}")]
    ArtifactJson { path: PathBuf, message: String },
}

pub fn build_payment_ledger_projection(
    input: PaymentLedgerProjectionInput<'_>,
) -> Result<PaymentLedgerProjection, PaymentLedgerProjectionError> {
    validate_child_receipts(input.graph_receipt, &input.evidence)?;

    let reservation = input
        .evidence
        .iter()
        .find_map(|evidence| match &evidence.packet {
            PaymentLedgerEvidencePacket::Reservation(reservation) => Some(reservation),
            _ => None,
        })
        .ok_or(PaymentLedgerProjectionError::MissingReservation)?;

    let refusal = input
        .evidence
        .iter()
        .find_map(|evidence| match &evidence.packet {
            PaymentLedgerEvidencePacket::Refusal(refusal) => Some(refusal),
            _ => None,
        });

    let settlement = input
        .evidence
        .iter()
        .find_map(|evidence| match &evidence.packet {
            PaymentLedgerEvidencePacket::RailSettlement(settlement) => Some((evidence, settlement)),
            _ => None,
        });

    let (disposition, accrual, refusal) = if let Some(refusal) = refusal {
        (
            PaymentLedgerDisposition::Refused,
            refused_accrual(reservation),
            Some(PaymentLedgerRefusal {
                reason_code: refusal.reason_code.clone(),
                refused_stage: refusal.refused_stage.clone(),
                rail_call_performed: refusal.rail_call_performed,
                ledger_spend_recorded: refusal.ledger_spend_recorded,
            }),
        )
    } else if let Some((evidence, settlement)) = settlement {
        validate_settlement_matches_reservation(reservation, settlement)?;
        validate_receipt_rail_proof(evidence.receipt, settlement)?;
        validate_paid_tool_refs(&input.evidence, &settlement.proof_ref)?;
        (
            PaymentLedgerDisposition::Settled,
            PaymentLedgerAccrual {
                amount_minor: settlement.amount_minor,
                currency: settlement.currency.clone(),
                rail: settlement.rail.clone(),
                counterparty: reservation.counterparty.clone(),
                operation: reservation.operation.clone(),
                idempotency_key: settlement.idempotency_key.clone(),
                rail_proof_refs: vec![settlement.proof_ref.clone()],
            },
            None,
        )
    } else {
        return Err(PaymentLedgerProjectionError::MissingDispositionEvidence);
    };

    Ok(PaymentLedgerProjection {
        schema_version: PAYMENT_LEDGER_PROJECTION_SCHEMA_VERSION.to_owned(),
        payment_profile: X402_PAY_PAYMENT_PROFILE.to_owned(),
        scenario_id: input.scenario_id.to_owned(),
        source_receipt_id: receipt_ref(input.graph_receipt),
        disposition,
        accrual,
        refusal,
        evidence_refs: evidence_refs(&input.evidence),
    })
}

pub fn write_payment_ledger_projection_artifact(
    receipt_dir: impl AsRef<Path>,
    projection: &PaymentLedgerProjection,
) -> Result<PaymentLedgerProjectionArtifact, PaymentLedgerProjectionError> {
    let receipt_id = source_receipt_file_stem(&projection.source_receipt_id)?;
    let artifact_id = format!(
        "{}:{}",
        projection.payment_profile, projection.source_receipt_id
    );
    let artifact_dir = receipt_dir
        .as_ref()
        .join("artifacts")
        .join("payment-ledger")
        .join(&projection.payment_profile);
    let artifact_path = artifact_dir.join(format!("{receipt_id}.json"));
    let mut contents = serde_json::to_vec_pretty(projection).map_err(|source| {
        PaymentLedgerProjectionError::ArtifactJson {
            path: artifact_path.clone(),
            message: source.to_string(),
        }
    })?;
    contents.push(b'\n');

    fs::create_dir_all(&artifact_dir).map_err(|source| {
        PaymentLedgerProjectionError::ArtifactIo {
            path: artifact_dir.clone(),
            message: source.to_string(),
        }
    })?;

    if artifact_path.exists() {
        let existing = fs::read(&artifact_path).map_err(|source| {
            PaymentLedgerProjectionError::ArtifactIo {
                path: artifact_path.clone(),
                message: source.to_string(),
            }
        })?;
        if existing != contents {
            return Err(PaymentLedgerProjectionError::ArtifactConflict {
                path: artifact_path,
            });
        }
    } else {
        fs::write(&artifact_path, &contents).map_err(|source| {
            PaymentLedgerProjectionError::ArtifactIo {
                path: artifact_path.clone(),
                message: source.to_string(),
            }
        })?;
    }

    let projection_artifact_path = artifact_path.to_string_lossy().into_owned();
    Ok(PaymentLedgerProjectionArtifact {
        artifact_id: artifact_id.clone(),
        artifact_type: PAYMENT_LEDGER_PROJECTION_SCHEMA_VERSION.to_owned(),
        path: artifact_path,
        event_payload: PaymentLedgerProjectedEventPayload {
            kind: PAYMENT_LEDGER_PROJECTED_EVENT_KIND.to_owned(),
            payment_profile: projection.payment_profile.clone(),
            projection_artifact_id: artifact_id,
            projection_artifact_path,
            source_receipt_id: projection.source_receipt_id.clone(),
            scenario_id: projection.scenario_id.clone(),
            disposition: projection.disposition.clone(),
        },
    })
}

fn refused_accrual(reservation: &PaymentReservationEvidence) -> PaymentLedgerAccrual {
    PaymentLedgerAccrual {
        amount_minor: 0,
        currency: reservation.currency.clone(),
        rail: reservation.rail.clone(),
        counterparty: reservation.counterparty.clone(),
        operation: reservation.operation.clone(),
        idempotency_key: reservation.idempotency_key.clone(),
        rail_proof_refs: Vec::new(),
    }
}

fn validate_child_receipts(
    graph_receipt: &HarnessReceipt,
    evidence: &[PaymentLedgerEvidence<'_>],
) -> Result<(), PaymentLedgerProjectionError> {
    let graph_child_receipts = graph_receipt
        .harness
        .child_harness_receipt_refs
        .iter()
        .map(|reference| reference.uri.as_str())
        .collect::<HashSet<_>>();
    for evidence in evidence {
        let child_ref = receipt_ref(evidence.receipt);
        if !graph_child_receipts.contains(child_ref.as_str()) {
            return Err(PaymentLedgerProjectionError::ChildReceiptNotReferenced {
                graph_receipt_id: graph_receipt.id.clone(),
                child_receipt_id: evidence.receipt.id.clone(),
            });
        }
    }
    Ok(())
}

fn validate_settlement_matches_reservation(
    reservation: &PaymentReservationEvidence,
    settlement: &PaymentRailSettlementEvidence,
) -> Result<(), PaymentLedgerProjectionError> {
    if reservation.amount_minor == settlement.amount_minor
        && reservation.currency == settlement.currency
        && reservation.rail == settlement.rail
        && reservation.idempotency_key == settlement.idempotency_key
    {
        Ok(())
    } else {
        Err(PaymentLedgerProjectionError::SettlementReservationMismatch)
    }
}

fn validate_receipt_rail_proof(
    receipt: &HarnessReceipt,
    settlement: &PaymentRailSettlementEvidence,
) -> Result<(), PaymentLedgerProjectionError> {
    let has_proof = receipt
        .harness
        .acts
        .iter()
        .flat_map(|act| act.verification_refs.iter())
        .any(|reference| is_matching_payment_rail_proof(reference, settlement));
    if has_proof {
        Ok(())
    } else {
        Err(PaymentLedgerProjectionError::MissingReceiptRailProof {
            receipt_id: receipt.id.clone(),
            proof_ref: settlement.proof_ref.clone(),
        })
    }
}

fn is_matching_payment_rail_proof(
    reference: &Reference,
    settlement: &PaymentRailSettlementEvidence,
) -> bool {
    reference.reference_type == ReferenceType::Verification
        && reference.proof_kind.as_ref() == Some(&ProofKind::PaymentRail)
        && reference.uri == settlement.proof_ref
        && reference.locator.as_deref() == Some(settlement.idempotency_key.as_str())
}

fn validate_paid_tool_refs(
    evidence: &[PaymentLedgerEvidence<'_>],
    proof_ref: &str,
) -> Result<(), PaymentLedgerProjectionError> {
    for evidence in evidence {
        if let PaymentLedgerEvidencePacket::PaidTool(paid_tool) = &evidence.packet
            && paid_tool.payment_proof_ref != proof_ref
        {
            return Err(PaymentLedgerProjectionError::PaidToolProofMismatch {
                proof_ref: paid_tool.payment_proof_ref.clone(),
            });
        }
    }
    Ok(())
}

fn evidence_refs(evidence: &[PaymentLedgerEvidence<'_>]) -> Vec<String> {
    let mut refs = Vec::new();
    for evidence in evidence {
        if matches!(
            evidence.packet,
            PaymentLedgerEvidencePacket::RailSettlement(_)
                | PaymentLedgerEvidencePacket::Refusal(_)
        ) {
            push_unique(&mut refs, evidence.receipt.harness.harness_ref.uri.clone());
            push_unique(&mut refs, receipt_ref(evidence.receipt));
        }
    }
    for evidence in evidence {
        if let PaymentLedgerEvidencePacket::Reservation(reservation) = &evidence.packet {
            push_unique(&mut refs, reservation.spend_capability_ref.clone());
        }
    }
    refs
}

fn receipt_ref(receipt: &HarnessReceipt) -> String {
    format!("runx:harness_receipt:{}", receipt.id)
}

fn source_receipt_file_stem(source_receipt_id: &str) -> Result<&str, PaymentLedgerProjectionError> {
    const PREFIX: &str = "runx:harness_receipt:";
    let Some(receipt_id) = source_receipt_id.strip_prefix(PREFIX) else {
        return Err(PaymentLedgerProjectionError::InvalidSourceReceiptId {
            source_receipt_id: source_receipt_id.to_owned(),
        });
    };
    if receipt_id.is_empty()
        || !receipt_id
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-'))
    {
        return Err(PaymentLedgerProjectionError::InvalidSourceReceiptId {
            source_receipt_id: source_receipt_id.to_owned(),
        });
    }
    Ok(receipt_id)
}

fn push_unique(refs: &mut Vec<String>, reference: String) {
    if !refs.contains(&reference) {
        refs.push(reference);
    }
}
