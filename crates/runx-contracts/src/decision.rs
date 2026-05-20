//! Decision contracts: choices, justifications, and closure dispositions.
use serde::{Deserialize, Serialize};

use crate::{Intent, Reference};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionChoice {
    Open,
    Continue,
    SpawnChild,
    Escalate,
    Defer,
    Close,
    Decline,
    Monitor,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DecisionInputs {
    #[serde(default)]
    pub signal_refs: Vec<Reference>,
    pub target_ref: Option<Reference>,
    #[serde(default)]
    pub opportunity_refs: Vec<Reference>,
    pub selection_ref: Option<Reference>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DecisionJustification {
    pub summary: String,
    #[serde(default)]
    pub evidence_refs: Vec<Reference>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClosureDisposition {
    Closed,
    Deferred,
    Superseded,
    Declined,
    Blocked,
    Failed,
    Killed,
    TimedOut,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Closure {
    pub disposition: ClosureDisposition,
    pub reason_code: String,
    pub summary: String,
    pub closed_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Decision {
    pub decision_id: String,
    pub choice: DecisionChoice,
    pub inputs: DecisionInputs,
    pub proposed_intent: Intent,
    pub selected_act_id: Option<String>,
    pub selected_harness_ref: Option<Reference>,
    pub justification: DecisionJustification,
    pub closure: Option<Closure>,
    #[serde(default)]
    pub artifact_refs: Vec<Reference>,
}
