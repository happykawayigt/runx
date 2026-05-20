//! Host protocol contracts: execution events, resolution requests, host-run lifecycle.
use serde::{Deserialize, Serialize};

use crate::{JsonObject, JsonValue};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum ExecutionEvent {
    SkillLoaded {
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        data: Option<JsonValue>,
    },
    InputsResolved {
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        data: Option<JsonValue>,
    },
    AuthResolved {
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        data: Option<JsonValue>,
    },
    ResolutionRequested {
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        data: Option<JsonValue>,
    },
    ResolutionResolved {
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        data: Option<JsonValue>,
    },
    Admitted {
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        data: Option<JsonValue>,
    },
    Executing {
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        data: Option<JsonValue>,
    },
    StepStarted {
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        data: Option<JsonValue>,
    },
    StepWaitingResolution {
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        data: Option<JsonValue>,
    },
    StepCompleted {
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        data: Option<JsonValue>,
    },
    Warning {
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        data: Option<JsonValue>,
    },
    Completed {
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        data: Option<JsonValue>,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ResolutionRequest {
    Input {
        id: String,
        questions: Vec<Question>,
    },
    Approval {
        id: String,
        gate: ApprovalGate,
    },
    AgentAct {
        id: String,
        invocation: AgentActInvocation,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Question {
    pub id: String,
    pub prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub required: bool,
    #[serde(rename = "type")]
    pub question_type: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApprovalGate {
    pub id: String,
    pub reason: String,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub gate_type: Option<String>,
    /// Shallow summary payload. `rust-resolution-payload-parity` owns any
    /// future deep typing for approval summaries.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<JsonObject>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentActInvocation {
    pub id: String,
    pub source_type: AgentActSourceType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task: Option<String>,
    /// Intentionally opaque agent-act envelope.
    ///
    /// `AgentActInvocation` is typed at the protocol edge, but the envelope's
    /// internal payload model is deferred to `rust-resolution-payload-parity`.
    pub envelope: JsonValue,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AgentActSourceType {
    Agent,
    AgentStep,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResolutionResponse {
    pub actor: ResolutionResponseActor,
    /// Shallow response payload. `rust-resolution-payload-parity` owns any
    /// future deep typing for resolution responses.
    pub payload: JsonValue,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolutionResponseActor {
    Human,
    Agent,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApprovalDecision {
    pub gate: ApprovalGate,
    pub approved: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "status",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum HostRunResult {
    NeedsAgent {
        skill_name: String,
        run_id: String,
        requests: Vec<ResolutionRequest>,
        #[serde(skip_serializing_if = "Option::is_none")]
        step_ids: Option<Vec<String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        step_labels: Option<Vec<String>>,
        events: Vec<ExecutionEvent>,
    },
    Completed {
        skill_name: String,
        receipt_id: String,
        output: String,
        events: Vec<ExecutionEvent>,
    },
    Failed {
        skill_name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        receipt_id: Option<String>,
        error: String,
        events: Vec<ExecutionEvent>,
    },
    Escalated {
        skill_name: String,
        receipt_id: String,
        error: String,
        events: Vec<ExecutionEvent>,
    },
    Denied {
        skill_name: String,
        reasons: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        receipt_id: Option<String>,
        events: Vec<ExecutionEvent>,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum HostRunState {
    NeedsAgent(HostNeedsAgentState),
    Completed(HostTerminalState),
    Failed(HostTerminalState),
    Escalated(HostTerminalState),
    Denied(HostTerminalState),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HostNeedsAgentState {
    pub skill_name: String,
    pub run_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requested_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_runner: Option<String>,
    pub requests: Vec<ResolutionRequest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step_labels: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lineage: Option<HostRunLineage>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HostTerminalState {
    pub kind: HostRunKind,
    pub skill_name: String,
    pub run_id: String,
    pub receipt_id: String,
    pub verification: HostRunVerification,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disposition: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outcome_state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actors: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact_types: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runner_provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approval: Option<HostRunApproval>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lineage: Option<HostRunLineage>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostRunKind {
    Harness,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostRunVerification {
    pub status: HostRunVerificationStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostRunVerificationStatus {
    Verified,
    Unverified,
    Invalid,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HostRunLineage {
    pub kind: HostRunLineageKind,
    pub source_run_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_receipt_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostRunLineageKind {
    Rerun,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HostRunApproval {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gate_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gate_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decision: Option<HostRunApprovalDecision>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostRunApprovalDecision {
    Approved,
    Denied,
}
