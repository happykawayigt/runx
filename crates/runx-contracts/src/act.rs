//! Act algebra: intents, governed acts, success criteria, and verification details.
use serde::{Deserialize, Serialize};

use crate::{ActRef, Closure, Reference, Verification};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SuccessCriterion {
    pub criterion_id: String,
    pub statement: String,
    pub required: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Intent {
    pub purpose: String,
    pub legitimacy: String,
    #[serde(default)]
    pub success_criteria: Vec<SuccessCriterion>,
    #[serde(default)]
    pub constraints: Vec<String>,
    #[serde(default)]
    pub derived_from: Vec<Reference>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CriterionStatus {
    Verified,
    Failed,
    Pending,
    NotApplicable,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActForm {
    Revision,
    Reply,
    Review,
    Observation,
    Verification,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CriterionBinding {
    pub criterion_id: String,
    pub status: CriterionStatus,
    #[serde(default)]
    pub evidence_refs: Vec<Reference>,
    #[serde(default)]
    pub verification_refs: Vec<Reference>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TargetSurface {
    pub surface_ref: Reference,
    pub mutating: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rationale: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChangeRequest {
    pub request_id: String,
    pub summary: String,
    #[serde(default)]
    pub target_surfaces: Vec<TargetSurface>,
    #[serde(default)]
    pub success_criteria: Vec<SuccessCriterion>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChangePlan {
    pub plan_id: String,
    pub summary: String,
    #[serde(default)]
    pub steps: Vec<String>,
    #[serde(default)]
    pub risks: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RevisionDetails {
    pub change_request: ChangeRequest,
    pub change_plan: ChangePlan,
    #[serde(default)]
    pub target_surfaces: Vec<TargetSurface>,
    #[serde(default)]
    pub invariants: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verification: Option<Verification>,
    #[serde(default)]
    pub handoff_refs: Vec<Reference>,
    #[serde(default)]
    pub revision_refs: Vec<Reference>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerificationDetails {
    pub criterion_ids: Vec<String>,
    pub verification: Verification,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deployment_ref: Option<Reference>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Act {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    pub act_id: String,
    pub form: ActForm,
    pub intent: Intent,
    pub summary: String,
    pub closure: Closure,
    #[serde(default)]
    pub criterion_bindings: Vec<CriterionBinding>,
    #[serde(default)]
    pub source_refs: Vec<Reference>,
    #[serde(default)]
    pub target_refs: Vec<Reference>,
    #[serde(default)]
    pub surface_refs: Vec<Reference>,
    #[serde(default)]
    pub artifact_refs: Vec<Reference>,
    #[serde(default)]
    pub verification_refs: Vec<Reference>,
    #[serde(default)]
    pub harness_refs: Vec<Reference>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revision: Option<RevisionDetails>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verification: Option<VerificationDetails>,
    pub performed_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GovernedActRef {
    pub act_ref: ActRef,
}
