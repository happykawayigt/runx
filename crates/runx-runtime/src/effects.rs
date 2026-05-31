use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

use runx_contracts::{
    EffectSettlementPhase, EffectSettlementReceipt, EffectSettlementReceiptSchema, JsonNumber,
    JsonObject, JsonValue, ProofKind, Reference, ReferenceType,
};
use thiserror::Error;

use crate::payment::supervisor::{PaymentSupervisorError, PaymentSupervisorSettlementEvidence};

pub mod state;

pub(crate) use crate::effects::state::{
    EffectIdempotencyEntry, EffectIdempotencyKey, EffectMutationStatus, EffectRecoveryState,
    EffectStepStateInput, consumed_spend_capability_recorded, escalate_effect_mutation,
    lookup_effect_idempotency_entry, lookup_effect_mutation, persist_effect_step_state,
};
pub(crate) use crate::payment::packets::{PaymentRailProof, read_payment_rail_packet};
pub(crate) use crate::payment::supervisor::{
    PAYMENT_RAIL_SUPERVISOR_EVIDENCE_METADATA, PaymentSupervisorProof, PaymentSupervisorProofMatch,
    PaymentSupervisorSettlementRequest, PaymentSupervisorVerificationInput,
    insert_payment_supervisor_proof_metadata as insert_effect_supervisor_proof_metadata,
    payment_supervisor_evidence_from_effect_record,
    payment_supervisor_evidence_metadata_value as effect_evidence_metadata_value,
    payment_supervisor_settlement_request_payload,
    validate_payment_supervisor_proof as validate_effect_supervisor_proof,
    verify_payment_rail_supervisor_proof as verify_effect_supervisor_proof,
};

pub const PAYMENT_EFFECT_FAMILY: &str = "payment";

pub trait EffectSupervisor: Send + Sync {
    fn settlement_evidence(
        &self,
        request: EffectSettlementRequest<'_>,
    ) -> Result<EffectSettlementEvidence, EffectSupervisorError>;
}

#[derive(Clone, Debug)]
pub struct EffectSettlementRequest<'a> {
    pub family: &'a str,
    pub proof_kind: ProofKind,
    pub proof_ref: &'a str,
    pub idempotency_key: Option<&'a str>,
    pub payload: EffectSettlementPayload,
}

#[derive(Clone, Debug)]
pub enum EffectSettlementPayload {
    Json(JsonObject),
}

impl<'a> EffectSettlementRequest<'a> {
    pub fn payment_rail(
        &self,
    ) -> Result<PaymentSupervisorSettlementRequest<'_>, EffectSupervisorError> {
        if self.family != PAYMENT_EFFECT_FAMILY || self.proof_kind != ProofKind::PaymentRail {
            return Err(EffectSupervisorError::InvalidEvidence {
                family: self.family.to_owned(),
                message: format!(
                    "expected payment effect with payment rail proof, got family {} proof {:?}",
                    self.family, self.proof_kind
                ),
            });
        }
        let EffectSettlementPayload::Json(payload) = &self.payload;
        let idempotency_key =
            self.idempotency_key
                .ok_or_else(|| EffectSupervisorError::InvalidEvidence {
                    family: self.family.to_owned(),
                    message: "payment effect idempotency_key is required".to_owned(),
                })?;
        let request = PaymentSupervisorSettlementRequest {
            rail: required_payload_str(payload, self.family, "rail")?,
            counterparty: required_payload_str(payload, self.family, "counterparty")?,
            amount_minor: required_payload_u64(payload, self.family, "amount_minor")?,
            currency: required_payload_str(payload, self.family, "currency")?,
            idempotency_key,
            proof_ref: self.proof_ref,
            skill_settlement_status: optional_payload_str(payload, "skill_settlement_status"),
        };
        match self.idempotency_key {
            Some(idempotency_key) if idempotency_key == request.idempotency_key => Ok(request),
            Some(idempotency_key) => Err(EffectSupervisorError::InvalidEvidence {
                family: self.family.to_owned(),
                message: format!(
                    "payment effect idempotency_key mismatch: expected {idempotency_key}, got {}",
                    request.idempotency_key
                ),
            }),
            None => Err(EffectSupervisorError::InvalidEvidence {
                family: self.family.to_owned(),
                message: "payment effect idempotency_key is required".to_owned(),
            }),
        }
    }
}

fn required_payload_str<'a>(
    payload: &'a JsonObject,
    family: &str,
    field: &'static str,
) -> Result<&'a str, EffectSupervisorError> {
    payload
        .get(field)
        .and_then(JsonValue::as_str)
        .ok_or_else(|| EffectSupervisorError::InvalidEvidence {
            family: family.to_owned(),
            message: format!("effect payload requires string field {field}"),
        })
}

fn optional_payload_str<'a>(payload: &'a JsonObject, field: &'static str) -> Option<&'a str> {
    payload.get(field).and_then(JsonValue::as_str)
}

fn required_payload_u64(
    payload: &JsonObject,
    family: &str,
    field: &'static str,
) -> Result<u64, EffectSupervisorError> {
    match payload.get(field) {
        Some(JsonValue::Number(JsonNumber::U64(value))) => Ok(*value),
        Some(JsonValue::Number(JsonNumber::I64(value))) => {
            u64::try_from(*value).map_err(|_| EffectSupervisorError::InvalidEvidence {
                family: family.to_owned(),
                message: format!("effect payload field {field} must be a u64"),
            })
        }
        Some(JsonValue::Number(JsonNumber::F64(value)))
            if value.is_finite() && value.fract() == 0.0 && *value >= 0.0 =>
        {
            Ok(*value as u64)
        }
        _ => Err(EffectSupervisorError::InvalidEvidence {
            family: family.to_owned(),
            message: format!("effect payload requires u64 field {field}"),
        }),
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum EffectSettlementEvidence {
    Generic(EffectSettlementRecord),
}

#[derive(Clone, Debug, PartialEq)]
pub struct EffectSettlementRecord {
    pub verifier_id: String,
    pub proof_ref: String,
    pub phase: EffectSettlementPhase,
    pub provider_event_ref: Option<String>,
    pub status: Option<String>,
    pub idempotency_key: Option<String>,
    pub payload: JsonObject,
}

impl EffectSettlementRecord {
    #[must_use]
    pub fn settlement_receipt(
        &self,
        id: impl Into<String>,
        created_at: impl Into<String>,
        family: impl Into<String>,
        original_receipt_ref: Reference,
        criterion_id: impl Into<String>,
    ) -> EffectSettlementReceipt {
        EffectSettlementReceipt {
            schema: EffectSettlementReceiptSchema::V1,
            id: id.into().into(),
            created_at: created_at.into().into(),
            family: family.into().into(),
            phase: self.phase.clone(),
            original_receipt_ref,
            criterion_id: criterion_id.into().into(),
            proof_ref: Some(Reference::with_uri(
                ReferenceType::Verification,
                self.proof_ref.clone(),
            )),
            evidence_refs: self
                .provider_event_ref
                .as_ref()
                .map(|event_ref| {
                    vec![Reference::with_uri(
                        ReferenceType::ProviderEvent,
                        event_ref.clone(),
                    )]
                })
                .unwrap_or_default(),
            payload: self.payload.clone(),
        }
    }
}

impl EffectSettlementEvidence {
    pub fn generic(record: EffectSettlementRecord) -> Self {
        Self::Generic(record)
    }

    pub fn verifier_id(&self) -> &str {
        match self {
            Self::Generic(record) => &record.verifier_id,
        }
    }

    pub fn proof_ref(&self) -> &str {
        match self {
            Self::Generic(record) => &record.proof_ref,
        }
    }

    pub fn status(&self) -> Option<&str> {
        match self {
            Self::Generic(record) => record.status.as_deref(),
        }
    }

    pub fn idempotency_key(&self) -> Option<&str> {
        match self {
            Self::Generic(record) => record.idempotency_key.as_deref(),
        }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum EffectSupervisorError {
    #[error("effect supervisor for family {family} is not configured")]
    SupervisorUnavailable { family: String },
    #[error("effect supervisor for family {family} is already configured")]
    DuplicateSupervisor { family: String },
    #[error("effect supervisor evidence for family {family} is invalid: {message}")]
    InvalidEvidence { family: String, message: String },
}

impl From<PaymentSupervisorError> for EffectSupervisorError {
    fn from(error: PaymentSupervisorError) -> Self {
        match error {
            PaymentSupervisorError::SupervisorUnavailable => Self::SupervisorUnavailable {
                family: PAYMENT_EFFECT_FAMILY.to_owned(),
            },
            other => Self::InvalidEvidence {
                family: PAYMENT_EFFECT_FAMILY.to_owned(),
                message: other.to_string(),
            },
        }
    }
}

#[derive(Clone)]
pub struct RuntimeEffectRegistry {
    supervisors: BTreeMap<&'static str, Arc<dyn EffectSupervisor>>,
}

impl RuntimeEffectRegistry {
    #[must_use]
    pub fn empty() -> Self {
        Self {
            supervisors: BTreeMap::new(),
        }
    }

    #[must_use]
    pub fn with_effect<T>(family: &'static str, supervisor: T) -> Self
    where
        T: EffectSupervisor + 'static,
    {
        let mut registry = Self::empty();
        registry.supervisors.insert(family, Arc::new(supervisor));
        registry
    }

    #[must_use]
    pub fn with_payment_effect<T>(supervisor: T) -> Self
    where
        T: EffectSupervisor + 'static,
    {
        Self::with_effect(PAYMENT_EFFECT_FAMILY, supervisor)
    }

    pub fn register_effect<T>(
        &mut self,
        family: &'static str,
        supervisor: T,
    ) -> Result<(), EffectSupervisorError>
    where
        T: EffectSupervisor + 'static,
    {
        if self.supervisors.contains_key(family) {
            return Err(EffectSupervisorError::DuplicateSupervisor {
                family: family.to_owned(),
            });
        }
        self.supervisors.insert(family, Arc::new(supervisor));
        Ok(())
    }

    pub fn settlement_evidence(
        &self,
        request: EffectSettlementRequest<'_>,
    ) -> Result<EffectSettlementEvidence, EffectSupervisorError> {
        let Some(supervisor) = self.supervisors.get(request.family) else {
            return Err(EffectSupervisorError::SupervisorUnavailable {
                family: request.family.to_owned(),
            });
        };
        supervisor.settlement_evidence(request)
    }

    pub fn payment_rail_settlement_evidence(
        &self,
        request: PaymentSupervisorSettlementRequest<'_>,
    ) -> Result<PaymentSupervisorSettlementEvidence, PaymentSupervisorError> {
        let evidence = self
            .settlement_evidence(payment_rail_effect_request(request))
            .map_err(payment_effect_error)?;
        let EffectSettlementEvidence::Generic(record) = evidence;
        payment_supervisor_evidence_from_effect_record(record)
    }
}

impl Default for RuntimeEffectRegistry {
    fn default() -> Self {
        Self::empty()
    }
}

impl fmt::Debug for RuntimeEffectRegistry {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let families = self.supervisors.keys().copied().collect::<Vec<_>>();
        formatter
            .debug_struct("RuntimeEffectRegistry")
            .field("families", &families)
            .finish()
    }
}

fn payment_rail_effect_request(
    request: PaymentSupervisorSettlementRequest<'_>,
) -> EffectSettlementRequest<'_> {
    EffectSettlementRequest {
        family: PAYMENT_EFFECT_FAMILY,
        proof_kind: ProofKind::PaymentRail,
        proof_ref: request.proof_ref,
        idempotency_key: Some(request.idempotency_key),
        payload: EffectSettlementPayload::Json(payment_supervisor_settlement_request_payload(
            request,
        )),
    }
}

fn payment_effect_error(error: EffectSupervisorError) -> PaymentSupervisorError {
    match error {
        EffectSupervisorError::SupervisorUnavailable { .. } => {
            PaymentSupervisorError::SupervisorUnavailable
        }
        EffectSupervisorError::DuplicateSupervisor { family } => {
            PaymentSupervisorError::InvalidSupervisorEvidence {
                message: format!("duplicate effect supervisor for family {family}"),
            }
        }
        EffectSupervisorError::InvalidEvidence { message, .. } => {
            PaymentSupervisorError::InvalidSupervisorEvidence { message }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use runx_contracts::JsonValue;

    #[derive(Debug)]
    struct MockSupervisor;

    impl EffectSupervisor for MockSupervisor {
        fn settlement_evidence(
            &self,
            request: EffectSettlementRequest<'_>,
        ) -> Result<EffectSettlementEvidence, EffectSupervisorError> {
            Ok(EffectSettlementEvidence::generic(EffectSettlementRecord {
                verifier_id: "runx.mock_effect.local.v1".to_owned(),
                proof_ref: request.proof_ref.to_owned(),
                phase: EffectSettlementPhase::Sealed,
                provider_event_ref: Some("mock:event:1".to_owned()),
                status: Some("sealed".to_owned()),
                idempotency_key: request.idempotency_key.map(str::to_owned),
                payload: match request.payload {
                    EffectSettlementPayload::Json(payload) => payload,
                },
            }))
        }
    }

    #[test]
    fn registry_dispatches_non_payment_effect_family() {
        let mut payload = JsonObject::new();
        payload.insert(
            "message_id".to_owned(),
            JsonValue::String("msg_1".to_owned()),
        );
        let registry = RuntimeEffectRegistry::with_effect("message", MockSupervisor);

        let evidence = registry
            .settlement_evidence(EffectSettlementRequest {
                family: "message",
                proof_kind: ProofKind::EffectSettlement,
                proof_ref: "proof:message:1",
                idempotency_key: Some("message:1"),
                payload: EffectSettlementPayload::Json(payload),
            })
            .expect("mock effect should settle");

        assert_eq!(evidence.verifier_id(), "runx.mock_effect.local.v1");
        assert_eq!(evidence.proof_ref(), "proof:message:1");
        assert_eq!(evidence.status(), Some("sealed"));
        assert_eq!(evidence.idempotency_key(), Some("message:1"));
    }

    #[test]
    fn duplicate_effect_family_registration_fails_closed() {
        let mut registry = RuntimeEffectRegistry::with_effect("message", MockSupervisor);

        let error = registry
            .register_effect("message", MockSupervisor)
            .expect_err("duplicate effect family should fail");

        assert_eq!(
            error,
            EffectSupervisorError::DuplicateSupervisor {
                family: "message".to_owned()
            }
        );
    }

    #[test]
    fn generic_effect_record_projects_follow_on_settlement_receipt() {
        let mut payload = JsonObject::new();
        payload.insert(
            "provider_status".to_owned(),
            JsonValue::String("ok".to_owned()),
        );
        let record = EffectSettlementRecord {
            verifier_id: "runx.mock_effect.local.v1".to_owned(),
            proof_ref: "runx:proof:deploy-1".to_owned(),
            phase: EffectSettlementPhase::InFlight,
            provider_event_ref: Some("provider:event:deploy-1".to_owned()),
            status: Some("pending".to_owned()),
            idempotency_key: Some("deploy:prod:1".to_owned()),
            payload,
        };

        let receipt = record.settlement_receipt(
            "runx:effect-settlement:deploy-1",
            "2026-05-31T00:00:00Z",
            "deployment",
            Reference::with_uri(ReferenceType::Receipt, "runx:receipt:original"),
            "criterion_deploy_settled",
        );

        assert_eq!(receipt.schema, EffectSettlementReceiptSchema::V1);
        assert_eq!(receipt.phase, EffectSettlementPhase::InFlight);
        assert_eq!(receipt.family, "deployment");
        assert_eq!(receipt.criterion_id, "criterion_deploy_settled");
        assert_eq!(
            receipt
                .proof_ref
                .as_ref()
                .map(|reference| reference.uri.as_str()),
            Some("runx:proof:deploy-1")
        );
        assert_eq!(receipt.evidence_refs.len(), 1);
    }
}
