//! Post-merge observer closure planning contracts.
//
// Type definitions live here; planning logic (closure plan, runtime dedupe,
// publication projection) lives in the private `plan` submodule.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::{
    ActForm, ClosureDisposition, CriterionStatus, OperationalPolicyError,
    OperationalPolicyPublishMode, Reference,
};

mod plan;

pub use plan::{
    plan_post_merge_observer_closure, plan_post_merge_observer_runtime_dedupe,
    project_post_merge_observer_publication_from_receipt,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PostMergeProvider {
    Github,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PostMergePullRequestState {
    Open,
    Closed,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PostMergePullRequestObservation {
    pub provider: PostMergeProvider,
    pub repo: String,
    pub number: u64,
    pub uri: String,
    pub state: PostMergePullRequestState,
    pub merged: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub merge_sha: Option<String>,
    pub observed_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub closed_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actor: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PostMergeVerificationStatus {
    Passed,
    Failed,
    Pending,
    NotRequired,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PostMergeVerificationObservation {
    pub status: PostMergeVerificationStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verification_ref: Option<Reference>,
    #[serde(default)]
    pub evidence_refs: Vec<Reference>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verified_at: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PostMergeObserverPlanRequest {
    pub source_id: Option<String>,
    pub source_issue_ref: Reference,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_thread_ref: Option<Reference>,
    pub pull_request: PostMergePullRequestObservation,
    pub verification: PostMergeVerificationObservation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PostMergeObserverClosureState {
    MergedVerified,
    FailedVerification,
    MergedPendingVerification,
    ClosedUnmerged,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PostMergeSourceIssueDisposition {
    KeepOpen,
    Close,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PostMergeObserverPlan {
    pub policy_id: String,
    pub source_id: String,
    pub final_state: PostMergeObserverClosureState,
    pub reason_code: String,
    pub seal_disposition: ClosureDisposition,
    pub summary: String,
    pub closure_key: String,
    pub observed_at: String,
    pub provider: PostMergeObserverProviderPlan,
    pub verification: PostMergeObserverVerificationPlan,
    pub publication: PostMergeObserverPublicationPlan,
    pub source_issue: PostMergeObserverSourceIssuePlan,
    pub act_forms: Vec<ActForm>,
    pub seal_criteria: Vec<PostMergeObserverCriterionPlan>,
    pub idempotency: PostMergeObserverIdempotencyPlan,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PostMergeObserverProviderPlan {
    pub provider: PostMergeProvider,
    pub pull_request_ref: Reference,
    pub merged: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub merge_sha: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PostMergeObserverVerificationPlan {
    pub required: bool,
    pub status: PostMergeVerificationStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub criterion_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verification_ref: Option<Reference>,
    #[serde(default)]
    pub evidence_refs: Vec<Reference>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PostMergeObserverPublicationPlan {
    pub final_source_thread_update: bool,
    pub source_issue_comment_required: bool,
    pub publish_mode: OperationalPolicyPublishMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_thread_ref: Option<Reference>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PostMergeObserverSourceIssuePlan {
    pub disposition: PostMergeSourceIssueDisposition,
    pub reason: String,
    pub target_ref: Reference,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PostMergeObserverCriterionPlan {
    pub criterion_id: String,
    pub status: CriterionStatus,
    pub required: bool,
    pub summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub act_form: Option<ActForm>,
    #[serde(default)]
    pub evidence_refs: Vec<Reference>,
    #[serde(default)]
    pub verification_refs: Vec<Reference>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PostMergeObserverIdempotencyPlan {
    pub closure_key: String,
    pub act_forms: Vec<ActForm>,
    pub intent_key: String,
    pub trigger_fingerprint: String,
    pub content_hash: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PostMergeObserverSignalSource {
    Webhook,
    Scheduler,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PostMergeObserverRuntimeDecision {
    SealAndPublish,
    AlreadyPublished,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PostMergeObserverRuntimeDedupePlan {
    pub decision: PostMergeObserverRuntimeDecision,
    pub signal_source: PostMergeObserverSignalSource,
    pub lock_key: String,
    pub receipt_id: String,
    pub receipt_ref: Reference,
    pub publication_key: String,
    pub content_hash: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PostMergeObserverPublicationProjection {
    pub harness_receipt_ref: Reference,
    pub source_issue_ref: Reference,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_thread_ref: Option<Reference>,
    pub reason_code: String,
    pub summary: String,
    pub proof_criterion_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verification_criterion_id: Option<String>,
    pub source_issue_disposition: PostMergeSourceIssueDisposition,
    pub close_authorized: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PostMergeObserverPlanError {
    Policy(OperationalPolicyError),
    ProviderObservationDisabled,
    SourceRequired,
    UnknownSource(String),
    ProviderStateNotTerminal,
    MissingSourceThread { source_id: String },
    VerificationRequired,
    InconsistentObservation(String),
    ReceiptNotSealed,
    ReceiptNotPostMergeObserver,
    MissingReceiptCriterion(String),
    MissingReceiptReference(&'static str),
    ReceiptPublicationNotAuthorized(String),
}

impl fmt::Display for PostMergeObserverPlanError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Policy(error) => write!(formatter, "operational policy error: {error}"),
            Self::ProviderObservationDisabled => {
                formatter.write_str("post-merge observer planning requires observe_provider=true")
            }
            Self::SourceRequired => {
                formatter.write_str("post-merge observer planning requires a source_id")
            }
            Self::UnknownSource(source_id) => {
                write!(
                    formatter,
                    "post-merge observer planning references unknown source '{source_id}'"
                )
            }
            Self::ProviderStateNotTerminal => {
                formatter.write_str("post-merge observer planning requires terminal PR state")
            }
            Self::MissingSourceThread { source_id } => {
                write!(
                    formatter,
                    "source '{source_id}' requires a source-thread target before final publication"
                )
            }
            Self::VerificationRequired => {
                formatter.write_str("merged post-merge observer planning requires verification")
            }
            Self::InconsistentObservation(message) => formatter.write_str(message),
            Self::ReceiptNotSealed => {
                formatter.write_str("post-merge publication requires a sealed harness receipt")
            }
            Self::ReceiptNotPostMergeObserver => {
                formatter.write_str("sealed harness receipt is not a post-merge observer closure")
            }
            Self::MissingReceiptCriterion(criterion_id) => {
                write!(
                    formatter,
                    "sealed post-merge receipt is missing required criterion '{criterion_id}'"
                )
            }
            Self::MissingReceiptReference(kind) => {
                write!(
                    formatter,
                    "sealed post-merge receipt is missing required {kind} reference"
                )
            }
            Self::ReceiptPublicationNotAuthorized(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for PostMergeObserverPlanError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Policy(error) => Some(error),
            Self::ProviderObservationDisabled
            | Self::SourceRequired
            | Self::UnknownSource(_)
            | Self::ProviderStateNotTerminal
            | Self::MissingSourceThread { .. }
            | Self::VerificationRequired
            | Self::InconsistentObservation(_)
            | Self::ReceiptNotSealed
            | Self::ReceiptNotPostMergeObserver
            | Self::MissingReceiptCriterion(_)
            | Self::MissingReceiptReference(_)
            | Self::ReceiptPublicationNotAuthorized(_) => None,
        }
    }
}
