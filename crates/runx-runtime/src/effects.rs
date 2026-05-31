use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

use runx_contracts::{JsonObject, ProofKind};
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
    payment_supervisor_evidence_metadata_value as effect_evidence_metadata_value,
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
    pub payload: EffectSettlementPayload<'a>,
}

#[derive(Clone, Debug)]
pub enum EffectSettlementPayload<'a> {
    PaymentRail(PaymentSupervisorSettlementRequest<'a>),
    Json(JsonObject),
}

impl<'a> EffectSettlementRequest<'a> {
    pub fn payment_rail(
        &self,
    ) -> Result<PaymentSupervisorSettlementRequest<'a>, EffectSupervisorError> {
        if self.family != PAYMENT_EFFECT_FAMILY || self.proof_kind != ProofKind::PaymentRail {
            return Err(EffectSupervisorError::InvalidEvidence {
                family: self.family.to_owned(),
                message: format!(
                    "expected payment effect with payment rail proof, got family {} proof {:?}",
                    self.family, self.proof_kind
                ),
            });
        }
        let request = match &self.payload {
            EffectSettlementPayload::PaymentRail(request) => *request,
            EffectSettlementPayload::Json(_) => {
                return Err(EffectSupervisorError::InvalidEvidence {
                    family: self.family.to_owned(),
                    message: "payment effect requires typed payment rail payload".to_owned(),
                });
            }
        };
        if request.proof_ref != self.proof_ref {
            return Err(EffectSupervisorError::InvalidEvidence {
                family: self.family.to_owned(),
                message: format!(
                    "payment effect proof_ref mismatch: expected {}, got {}",
                    self.proof_ref, request.proof_ref
                ),
            });
        }
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

#[derive(Clone, Debug, PartialEq)]
pub enum EffectSettlementEvidence {
    PaymentRail(PaymentSupervisorSettlementEvidence),
    Generic(EffectSettlementRecord),
}

#[derive(Clone, Debug, PartialEq)]
pub struct EffectSettlementRecord {
    pub verifier_id: String,
    pub proof_ref: String,
    pub provider_event_ref: Option<String>,
    pub status: Option<String>,
    pub idempotency_key: Option<String>,
    pub payload: JsonObject,
}

impl EffectSettlementEvidence {
    pub fn from_payment_rail(evidence: PaymentSupervisorSettlementEvidence) -> Self {
        Self::PaymentRail(evidence)
    }

    pub fn generic(record: EffectSettlementRecord) -> Self {
        Self::Generic(record)
    }

    pub fn verifier_id(&self) -> &str {
        match self {
            Self::PaymentRail(evidence) => &evidence.verifier_id,
            Self::Generic(record) => &record.verifier_id,
        }
    }

    pub fn proof_ref(&self) -> &str {
        match self {
            Self::PaymentRail(evidence) => &evidence.proof_ref,
            Self::Generic(record) => &record.proof_ref,
        }
    }

    pub fn status(&self) -> Option<&str> {
        match self {
            Self::PaymentRail(evidence) => evidence.settlement_status.as_deref(),
            Self::Generic(record) => record.status.as_deref(),
        }
    }

    pub fn idempotency_key(&self) -> Option<&str> {
        match self {
            Self::PaymentRail(evidence) => Some(&evidence.idempotency_key),
            Self::Generic(record) => record.idempotency_key.as_deref(),
        }
    }

    fn into_payment_rail(
        self,
    ) -> Result<PaymentSupervisorSettlementEvidence, PaymentSupervisorError> {
        match self {
            Self::PaymentRail(evidence) => Ok(evidence),
            Self::Generic(_) => Err(PaymentSupervisorError::InvalidSupervisorEvidence {
                message: "payment effect returned non-payment evidence".to_owned(),
            }),
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
        evidence.into_payment_rail()
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
        payload: EffectSettlementPayload::PaymentRail(request),
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
                provider_event_ref: Some("mock:event:1".to_owned()),
                status: Some("sealed".to_owned()),
                idempotency_key: request.idempotency_key.map(str::to_owned),
                payload: match request.payload {
                    EffectSettlementPayload::Json(payload) => payload,
                    EffectSettlementPayload::PaymentRail(_) => JsonObject::new(),
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
                proof_kind: ProofKind::PaymentRail,
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
}
