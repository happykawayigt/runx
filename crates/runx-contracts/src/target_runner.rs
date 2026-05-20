//! Target repo runner planning contracts.
//
// Type definitions live here; planning, dedupe-lookup, and receipt-metadata
// helpers live in the private `plan` submodule.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::{
    JsonObject, OperationalPolicyAction, OperationalPolicyAdmission,
    OperationalPolicyDedupeStrategy, OperationalPolicyDuplicateBehavior, OperationalPolicyError,
    OperationalPolicyPublishMode, OperationalPolicyRunnerKind, OperationalPolicyRunnerRule,
    OperationalPolicySourceIssueClosureMode, OperationalPolicySourceProvider,
    OperationalPolicySourceRule, OperationalPolicyTargetRule, Reference,
};

mod plan;

pub use plan::{
    apply_target_repo_runner_dedupe_lookup_execution, execute_target_repo_runner_dedupe_lookup,
    plan_target_repo_runner, plan_target_repo_runner_dedupe_lookup,
    plan_target_repo_runner_execution, plan_target_repo_runner_pull_request_receipt,
    plan_target_repo_runner_source_publication_receipt,
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TargetRepoRunnerPlanRequest {
    pub source_id: Option<String>,
    pub target_repo: String,
    pub action: OperationalPolicyAction,
    pub runner_id: Option<String>,
    pub source: TargetRepoRunnerSourceContext,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signal_fingerprint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub existing_pull_request: Option<TargetRepoRunnerExistingPullRequest>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TargetRepoRunnerSourceContext {
    pub provider: OperationalPolicySourceProvider,
    pub locator: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread_locator: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread_ts: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issue_url: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TargetRepoRunnerExistingPullRequest {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub number: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct TargetRepoRunnerPlan {
    pub policy_id: String,
    pub action: OperationalPolicyAction,
    pub source: TargetRepoRunnerSourcePlan,
    pub source_thread: TargetRepoRunnerSourceThreadPlan,
    pub target: TargetRepoRunnerTargetPlan,
    pub runner: TargetRepoRunnerRunnerPlan,
    pub owner: TargetRepoRunnerOwnerPlan,
    pub dedupe: TargetRepoRunnerDedupePlan,
    pub source_issue_closure_mode: OperationalPolicySourceIssueClosureMode,
    pub mutate_target_repo: bool,
    pub require_human_merge_gate: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct TargetRepoRunnerSourcePlan {
    pub source_id: String,
    pub provider: OperationalPolicySourceProvider,
    pub locator: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issue_url: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct TargetRepoRunnerSourceThreadPlan {
    pub required: bool,
    pub publish_mode: OperationalPolicyPublishMode,
    pub locator: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct TargetRepoRunnerTargetPlan {
    pub repo: String,
    pub scafld_required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_branch: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct TargetRepoRunnerRunnerPlan {
    pub runner_id: String,
    pub kind: OperationalPolicyRunnerKind,
    pub scafld_required: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct TargetRepoRunnerOwnerPlan {
    pub route_id: String,
    pub owners: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct TargetRepoRunnerDedupePlan {
    pub strategy: OperationalPolicyDedupeStrategy,
    pub key: String,
    pub key_fields: Vec<String>,
    pub components: Vec<TargetRepoRunnerDedupeComponent>,
    pub on_duplicate: OperationalPolicyDuplicateBehavior,
    pub result: TargetRepoRunnerDedupeResult,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub existing_pull_request: Option<TargetRepoRunnerExistingPullRequest>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct TargetRepoRunnerDedupeLookupPlan {
    pub provider: TargetRepoRunnerProvider,
    pub target_repo: String,
    pub key: String,
    pub strategy: OperationalPolicyDedupeStrategy,
    pub query: TargetRepoRunnerDedupeLookupQuery,
    pub components: Vec<TargetRepoRunnerDedupeComponent>,
    pub source_issue_ref: Option<Reference>,
    pub source_thread_ref: Reference,
    pub result: TargetRepoRunnerDedupeResult,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub existing_pull_request: Option<TargetRepoRunnerExistingPullRequest>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct TargetRepoRunnerDedupeLookupQuery {
    pub markers: Vec<String>,
    pub required_refs: Vec<Reference>,
    pub result_limit: u16,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct TargetRepoRunnerExecutionPlan {
    pub checkout: TargetRepoRunnerCheckoutPlan,
    pub readiness: TargetRepoRunnerReadinessPlan,
    pub provider_lookup: TargetRepoRunnerDedupeLookupPlan,
    pub source_issue_ref: Option<Reference>,
    pub source_thread_ref: Reference,
    pub target_repo_ref: Reference,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct TargetRepoRunnerCheckoutPlan {
    pub target_repo: String,
    pub public_repo_ref: Reference,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_branch: Option<String>,
    pub scafld_required: bool,
    pub local_path_hidden: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct TargetRepoRunnerReadinessPlan {
    pub runner_id: String,
    pub runner_kind: OperationalPolicyRunnerKind,
    pub runner_scafld_required: bool,
    pub target_scafld_required: bool,
    pub scafld_ready: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TargetRepoRunnerReadinessObservation {
    pub target_repo: String,
    pub runner_id: String,
    pub scafld_ready: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TargetRepoRunnerProviderPullRequest {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub number: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    pub open: bool,
    #[serde(default)]
    pub markers: Vec<String>,
    #[serde(default)]
    pub refs: Vec<Reference>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TargetRepoRunnerDedupeLookupObservation {
    pub provider: TargetRepoRunnerProvider,
    pub target_repo: String,
    pub key: String,
    #[serde(default)]
    pub pull_requests: Vec<TargetRepoRunnerProviderPullRequest>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct TargetRepoRunnerDedupeLookupExecution {
    pub provider: TargetRepoRunnerProvider,
    pub target_repo: String,
    pub key: String,
    pub result: TargetRepoRunnerDedupeResult,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub existing_pull_request: Option<TargetRepoRunnerExistingPullRequest>,
    pub matched_required_refs: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetRepoRunnerPullRequestDisposition {
    Create,
    Reuse,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct TargetRepoRunnerPullRequestReceiptPlan {
    pub act_form: crate::ActForm,
    pub disposition: TargetRepoRunnerPullRequestDisposition,
    pub target_repo_ref: Reference,
    pub source_issue_ref: Option<Reference>,
    pub source_thread_ref: Reference,
    pub pull_request_ref: Option<Reference>,
    pub metadata: JsonObject,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct TargetRepoRunnerSourcePublicationReceiptPlan {
    pub source_issue_ref: Option<Reference>,
    pub source_thread_ref: Reference,
    pub pull_request_ref: Reference,
    pub metadata: JsonObject,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetRepoRunnerProvider {
    Github,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct TargetRepoRunnerDedupeComponent {
    pub field: String,
    pub value: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetRepoRunnerDedupeResult {
    LookupRequired,
    Reused,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TargetRepoRunnerPlanError {
    Policy(OperationalPolicyError),
    AdmissionDenied(Box<OperationalPolicyAdmission>),
    MissingDedupeField(String),
    InconsistentAdmission(String),
    NotScafldReady { target_repo: String },
    ReadinessMismatch(String),
    ProviderLookupMismatch(String),
    PullRequestRequired,
}

struct TargetRepoRunnerAdmissionValues {
    source_id: String,
    target_repo: String,
    runner_id: String,
    owner_route_id: String,
    thread_locator: String,
}

struct TargetRepoRunnerPolicyContext<'a> {
    source: &'a OperationalPolicySourceRule,
    target: &'a OperationalPolicyTargetRule,
    runner: &'a OperationalPolicyRunnerRule,
}

impl fmt::Display for TargetRepoRunnerPlanError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Policy(error) => write!(formatter, "operational policy error: {error}"),
            Self::AdmissionDenied(admission) => {
                let codes = admission
                    .findings
                    .iter()
                    .map(|finding| finding.code.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(formatter, "target repo runner admission denied: {codes}")
            }
            Self::MissingDedupeField(field) => {
                write!(
                    formatter,
                    "target repo runner dedupe field '{field}' is missing"
                )
            }
            Self::InconsistentAdmission(message) => formatter.write_str(message),
            Self::NotScafldReady { target_repo } => {
                write!(
                    formatter,
                    "target repo runner requires scafld-ready target repo '{target_repo}'"
                )
            }
            Self::ReadinessMismatch(message) | Self::ProviderLookupMismatch(message) => {
                formatter.write_str(message)
            }
            Self::PullRequestRequired => {
                formatter.write_str("target repo runner receipt planning requires a pull request")
            }
        }
    }
}

impl std::error::Error for TargetRepoRunnerPlanError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Policy(error) => Some(error),
            Self::AdmissionDenied(_)
            | Self::MissingDedupeField(_)
            | Self::InconsistentAdmission(_)
            | Self::NotScafldReady { .. }
            | Self::ReadinessMismatch(_)
            | Self::ProviderLookupMismatch(_)
            | Self::PullRequestRequired => None,
        }
    }
}
