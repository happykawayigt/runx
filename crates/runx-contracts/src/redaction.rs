//! Redaction contracts: redacted-field markers and hash commitments.
use serde::{Deserialize, Serialize};

use crate::Reference;

pub const REDACTION_SCHEMA: &str = "runx.redaction.v1";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RedactionSchema {
    #[serde(rename = "runx.redaction.v1")]
    V1,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HashAlgorithm {
    Sha256,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HashCommitment {
    pub algorithm: HashAlgorithm,
    pub value: String,
    pub canonicalization: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Redaction {
    pub schema: RedactionSchema,
    pub redaction_id: String,
    pub policy_ref: Reference,
    #[serde(default)]
    pub redacted_fields: Vec<String>,
    #[serde(default)]
    pub hash_commitments: Vec<HashCommitment>,
    pub canonicalization: String,
    pub performed_by_ref: Reference,
    pub performed_at: String,
}
