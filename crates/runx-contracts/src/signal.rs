//! Signal contracts: trust-tagged events that enter the act lifecycle.
use serde::{Deserialize, Serialize};

use crate::{Fingerprint, JsonObject, Links, Reference};

pub const SIGNAL_SCHEMA: &str = "runx.signal.v1";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignalSchema {
    #[serde(rename = "runx.signal.v1")]
    V1,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignalType {
    IssueOpened,
    IssueComment,
    PullRequestEvent,
    ReviewEvent,
    ChatMessage,
    Alert,
    DeploymentEvent,
    PaymentRequired,
    ScheduleTick,
    OperatorNote,
    SystemEvent,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignalTrustLevel {
    Unverified,
    Observed,
    VerifiedDelivery,
    VerifiedSignature,
    OperatorAttested,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignalAuthenticity {
    pub host_ref: Reference,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub principal_ref: Option<Reference>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verified_by_ref: Option<Reference>,
    pub trust_level: SignalTrustLevel,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verified_at: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub signature_refs: Vec<Reference>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence_refs: Vec<Reference>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Signal {
    pub schema: SignalSchema,
    pub signal_id: String,
    pub source_ref: Reference,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authenticity: Option<SignalAuthenticity>,
    pub signal_type: SignalType,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_preview: Option<String>,
    pub observed_at: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence_refs: Vec<Reference>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fingerprint: Option<Fingerprint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub links: Option<Links>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<JsonObject>,
}
