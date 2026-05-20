//! Verification contracts: checks and statuses for governed verification.
use serde::{Deserialize, Serialize};

use crate::Reference;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationStatus {
    Passed,
    Failed,
    Pending,
    NotApplicable,
    Missing,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerificationCheck {
    pub check_id: String,
    pub criterion_ids: Vec<String>,
    pub status: VerificationStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(default)]
    pub checked_refs: Vec<Reference>,
    #[serde(default)]
    pub evidence_refs: Vec<Reference>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verified_at: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Verification {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verification_id: Option<String>,
    pub status: VerificationStatus,
    #[serde(default)]
    pub checks: Vec<VerificationCheck>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verified_at: Option<String>,
    #[serde(default)]
    pub evidence_refs: Vec<Reference>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReceiptVerificationSummary {
    pub signature_valid: bool,
    pub hash_commitments_valid: bool,
    pub authority_attenuation_valid: bool,
    pub criteria_bound: bool,
    pub redaction_valid: bool,
    pub external_attestations_present: bool,
}
