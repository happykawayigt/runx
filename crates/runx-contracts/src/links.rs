//! Receipt link contracts: duplicate candidates and reference linking.
use serde::{Deserialize, Serialize};

use crate::{JsonNumber, Reference};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DuplicateCandidate {
    pub candidate_ref: Reference,
    pub confidence: JsonNumber,
    pub observed_at: String,
    #[serde(default)]
    pub evidence_refs: Vec<Reference>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reviewer_refs: Vec<Reference>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Links {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duplicate_of: Option<Reference>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub duplicate_candidates: Vec<DuplicateCandidate>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub supersedes: Vec<Reference>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub superseded_by: Vec<Reference>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub related: Vec<Reference>,
}
