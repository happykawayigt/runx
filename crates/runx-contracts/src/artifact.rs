//! Artifact contract: emitted artifacts and their producer attribution.
use serde::{Deserialize, Serialize};

use crate::{ActRef, HashCommitment, JsonObject, Reference};

pub const ARTIFACT_SCHEMA: &str = "runx.artifact.v1";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArtifactSchema {
    #[serde(rename = "runx.artifact.v1")]
    V1,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactProducedBy {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receipt_ref: Option<Reference>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub harness_ref: Option<Reference>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub act_ref: Option<ActRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decision_ref: Option<Reference>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signal_ref: Option<Reference>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Artifact {
    pub schema: ArtifactSchema,
    pub artifact_id: String,
    pub artifact_ref: Reference,
    pub produced_by: ArtifactProducedBy,
    pub media_type: String,
    pub created_at: String,
    pub size_bytes: u64,
    pub hash: HashCommitment,
    #[serde(default)]
    pub redaction_refs: Vec<Reference>,
    #[serde(default)]
    pub source_refs: Vec<Reference>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_ref: Option<Reference>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<JsonObject>,
}
