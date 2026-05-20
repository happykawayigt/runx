//! Execution semantics contracts: governed disposition, outcome state, and receipt outcome.
use serde::{Deserialize, Serialize};

use crate::{JsonObject, JsonValue};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GovernedDisposition {
    Completed,
    NeedsAgent,
    PolicyDenied,
    ApprovalRequired,
    Observing,
    Escalated,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutcomeState {
    Pending,
    Complete,
    Expired,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReceiptOutcome {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observed_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<JsonObject>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReceiptSurfaceRef {
    #[serde(rename = "type")]
    pub surface_type: String,
    pub uri: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputContextCapture {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capture: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot: Option<JsonValue>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionSemantics {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disposition: Option<GovernedDisposition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outcome_state: Option<OutcomeState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outcome: Option<ReceiptOutcome>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_context: Option<InputContextCapture>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub surface_refs: Option<Vec<ReceiptSurfaceRef>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence_refs: Option<Vec<ReceiptSurfaceRef>>,
}
