//! Fingerprint contracts: content hashing identifiers.
use serde::{Deserialize, Serialize};

use crate::Reference;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FingerprintAlgorithm {
    Sha256,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fingerprint {
    pub algorithm: FingerprintAlgorithm,
    pub canonicalization: String,
    pub value: String,
    pub derived_from: Vec<Reference>,
}
