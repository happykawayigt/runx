//! Runtime support for post-merge observer publication.

use std::collections::BTreeSet;

use runx_contracts::{
    HarnessReceipt, OperationalPolicy, PostMergeObserverPlan, PostMergeObserverPlanError,
    PostMergeObserverPlanRequest, PostMergeObserverPublicationProjection,
    PostMergeObserverRuntimeDecision, PostMergeObserverRuntimeDedupePlan,
    PostMergeObserverSignalSource, PostMergePullRequestObservation,
    PostMergeSourceIssueDisposition, PostMergeVerificationObservation, Reference, ReferenceType,
    plan_post_merge_observer_closure, project_post_merge_observer_publication_from_receipt,
};
use serde::Serialize;
use thiserror::Error;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PostMergeObserverPublicationLedger {
    published_keys: BTreeSet<String>,
}

impl PostMergeObserverPublicationLedger {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn contains(&self, publication_key: &str) -> bool {
        self.published_keys.contains(publication_key)
    }

    fn mark_published(&mut self, publication_key: &str) {
        self.published_keys.insert(publication_key.to_owned());
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PostMergeObserverPublicationRuntimeDecision {
    Publish,
    AlreadyPublished,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PostMergeObserverPublicationRuntime {
    pub decision: PostMergeObserverPublicationRuntimeDecision,
    pub publication_key: String,
    pub receipt_ref: Reference,
    pub commands: Vec<PostMergeObserverPublicationCommand>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PostMergeObserverLivePublicationRequest {
    pub source_id: Option<String>,
    pub source_issue_ref: Reference,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_thread_ref: Option<Reference>,
    pub pull_request_ref: Reference,
    pub signal_source: PostMergeObserverSignalSource,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PostMergeObserverPullRequestObservationRequest {
    pub source_id: Option<String>,
    pub source_issue_ref: Reference,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_thread_ref: Option<Reference>,
    pub pull_request_ref: Reference,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PostMergeObserverVerificationObservationRequest {
    pub source_id: Option<String>,
    pub source_issue_ref: Reference,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_thread_ref: Option<Reference>,
    pub pull_request: PostMergePullRequestObservation,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PostMergeObserverLivePublication {
    pub pull_request: PostMergePullRequestObservation,
    pub verification: PostMergeVerificationObservation,
    pub closure_plan: PostMergeObserverPlan,
    pub dedupe: PostMergeObserverRuntimeDedupePlan,
    pub publication: PostMergeObserverPublicationRuntime,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PostMergeObserverPublicationCommand {
    SourceIssueComment {
        publication_key: String,
        target: Reference,
        receipt_ref: Reference,
        body: String,
    },
    SourceThreadReply {
        publication_key: String,
        target: Reference,
        receipt_ref: Reference,
        body: String,
    },
    SourceIssueClose {
        publication_key: String,
        target: Reference,
        receipt_ref: Reference,
        reason_code: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PostMergeObserverAdapterError {
    pub operation: &'static str,
    pub message: String,
}

impl PostMergeObserverAdapterError {
    pub fn new(operation: &'static str, message: impl Into<String>) -> Self {
        Self {
            operation,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for PostMergeObserverAdapterError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{} failed: {}", self.operation, self.message)
    }
}

impl std::error::Error for PostMergeObserverAdapterError {}

pub trait PostMergeObserverAdapter {
    fn observe_pull_request(
        &mut self,
        request: &PostMergeObserverPullRequestObservationRequest,
    ) -> Result<PostMergePullRequestObservation, PostMergeObserverAdapterError>;

    fn observe_verification(
        &mut self,
        request: &PostMergeObserverVerificationObservationRequest,
    ) -> Result<PostMergeVerificationObservation, PostMergeObserverAdapterError>;
}

#[derive(Debug, Error)]
pub enum PostMergeObserverRuntimeError {
    #[error("{0}")]
    Adapter(#[from] PostMergeObserverAdapterError),
    #[error("post-merge observer planning or projection failed: {0}")]
    Projection(#[from] PostMergeObserverPlanError),
    #[error(
        "observed closure reason '{observed_reason_code}' does not match sealed receipt reason '{receipt_reason_code}'"
    )]
    ObservedClosureMismatch {
        observed_reason_code: String,
        receipt_reason_code: String,
    },
    #[error(
        "dedupe plan receipt id '{dedupe_receipt_id}' does not match sealed receipt '{receipt_id}'"
    )]
    ReceiptMismatch {
        dedupe_receipt_id: String,
        receipt_id: String,
    },
    #[error(
        "dedupe plan receipt ref '{dedupe_receipt_ref}' does not match sealed receipt ref '{receipt_ref}'"
    )]
    ReceiptRefMismatch {
        dedupe_receipt_ref: String,
        receipt_ref: String,
    },
    #[error("post-merge source-thread publication requires a thread target")]
    MissingSourceThreadTarget,
    #[error("post-merge source-thread publication requires provider and locator metadata")]
    MissingSourceThreadMetadata,
}

pub fn execute_post_merge_observer_with_adapter<A: PostMergeObserverAdapter>(
    policy: &OperationalPolicy,
    request: &PostMergeObserverLivePublicationRequest,
    sealed_receipt: &HarnessReceipt,
    adapter: &mut A,
    ledger: &mut PostMergeObserverPublicationLedger,
) -> Result<PostMergeObserverLivePublication, PostMergeObserverRuntimeError> {
    let pull_request =
        adapter.observe_pull_request(&PostMergeObserverPullRequestObservationRequest {
            source_id: request.source_id.clone(),
            source_issue_ref: request.source_issue_ref.clone(),
            source_thread_ref: request.source_thread_ref.clone(),
            pull_request_ref: request.pull_request_ref.clone(),
        })?;
    let verification =
        adapter.observe_verification(&PostMergeObserverVerificationObservationRequest {
            source_id: request.source_id.clone(),
            source_issue_ref: request.source_issue_ref.clone(),
            source_thread_ref: request.source_thread_ref.clone(),
            pull_request: pull_request.clone(),
        })?;
    let closure_plan = plan_post_merge_observer_closure(
        policy,
        &PostMergeObserverPlanRequest {
            source_id: request.source_id.clone(),
            source_issue_ref: request.source_issue_ref.clone(),
            source_thread_ref: request.source_thread_ref.clone(),
            pull_request: pull_request.clone(),
            verification: verification.clone(),
        },
    )?;
    if closure_plan.reason_code != sealed_receipt.seal.reason_code {
        return Err(PostMergeObserverRuntimeError::ObservedClosureMismatch {
            observed_reason_code: closure_plan.reason_code,
            receipt_reason_code: sealed_receipt.seal.reason_code.clone(),
        });
    }

    let dedupe = sealed_receipt_dedupe_plan(sealed_receipt, request.signal_source);
    let publication =
        project_post_merge_observer_publication_commands(&dedupe, sealed_receipt, ledger)?;

    Ok(PostMergeObserverLivePublication {
        pull_request,
        verification,
        closure_plan,
        dedupe,
        publication,
    })
}

pub fn project_post_merge_observer_publication_commands(
    dedupe: &PostMergeObserverRuntimeDedupePlan,
    sealed_receipt: &HarnessReceipt,
    ledger: &mut PostMergeObserverPublicationLedger,
) -> Result<PostMergeObserverPublicationRuntime, PostMergeObserverRuntimeError> {
    if dedupe.receipt_id != sealed_receipt.id {
        return Err(PostMergeObserverRuntimeError::ReceiptMismatch {
            dedupe_receipt_id: dedupe.receipt_id.clone(),
            receipt_id: sealed_receipt.id.clone(),
        });
    }

    let projection = project_post_merge_observer_publication_from_receipt(sealed_receipt)?;
    if dedupe.receipt_ref.uri != projection.harness_receipt_ref.uri {
        return Err(PostMergeObserverRuntimeError::ReceiptRefMismatch {
            dedupe_receipt_ref: dedupe.receipt_ref.uri.clone(),
            receipt_ref: projection.harness_receipt_ref.uri.clone(),
        });
    }

    if dedupe.decision == PostMergeObserverRuntimeDecision::AlreadyPublished
        || ledger.contains(&dedupe.publication_key)
    {
        return Ok(PostMergeObserverPublicationRuntime {
            decision: PostMergeObserverPublicationRuntimeDecision::AlreadyPublished,
            publication_key: dedupe.publication_key.clone(),
            receipt_ref: projection.harness_receipt_ref,
            commands: Vec::new(),
        });
    }

    let commands = publication_commands(&dedupe.publication_key, &projection)?;
    ledger.mark_published(&dedupe.publication_key);

    Ok(PostMergeObserverPublicationRuntime {
        decision: PostMergeObserverPublicationRuntimeDecision::Publish,
        publication_key: dedupe.publication_key.clone(),
        receipt_ref: projection.harness_receipt_ref,
        commands,
    })
}

fn sealed_receipt_dedupe_plan(
    sealed_receipt: &HarnessReceipt,
    signal_source: PostMergeObserverSignalSource,
) -> PostMergeObserverRuntimeDedupePlan {
    PostMergeObserverRuntimeDedupePlan {
        decision: PostMergeObserverRuntimeDecision::SealAndPublish,
        signal_source,
        lock_key: format!(
            "post-merge-observer:{}",
            sealed_receipt.harness.idempotency.content_hash
        ),
        receipt_id: sealed_receipt.id.clone(),
        receipt_ref: Reference {
            reference_type: ReferenceType::HarnessReceipt,
            uri: format!("runx:harness_receipt:{}", sealed_receipt.id),
            provider: None,
            locator: Some(sealed_receipt.seal.digest.clone()),
            label: Some("post-merge observer harness receipt".to_owned()),
            observed_at: Some(sealed_receipt.seal.closed_at.clone()),
            proof_kind: None,
        },
        publication_key: format!(
            "post-merge-publication:{}:{}",
            sealed_receipt.harness.idempotency.intent_key,
            sealed_receipt.harness.idempotency.content_hash
        ),
        content_hash: sealed_receipt.harness.idempotency.content_hash.clone(),
    }
}

fn publication_commands(
    publication_key: &str,
    projection: &PostMergeObserverPublicationProjection,
) -> Result<Vec<PostMergeObserverPublicationCommand>, PostMergeObserverRuntimeError> {
    let source_thread_ref = projection
        .source_thread_ref
        .as_ref()
        .ok_or(PostMergeObserverRuntimeError::MissingSourceThreadTarget)?;
    require_source_thread_metadata(source_thread_ref)?;

    let body = public_reply_body(projection);
    let mut commands = vec![
        PostMergeObserverPublicationCommand::SourceIssueComment {
            publication_key: publication_key.to_owned(),
            target: projection.source_issue_ref.clone(),
            receipt_ref: projection.harness_receipt_ref.clone(),
            body: body.clone(),
        },
        PostMergeObserverPublicationCommand::SourceThreadReply {
            publication_key: publication_key.to_owned(),
            target: source_thread_ref.clone(),
            receipt_ref: projection.harness_receipt_ref.clone(),
            body,
        },
    ];

    if projection.close_authorized
        && projection.source_issue_disposition == PostMergeSourceIssueDisposition::Close
    {
        commands.push(PostMergeObserverPublicationCommand::SourceIssueClose {
            publication_key: publication_key.to_owned(),
            target: projection.source_issue_ref.clone(),
            receipt_ref: projection.harness_receipt_ref.clone(),
            reason_code: projection.reason_code.clone(),
        });
    }

    Ok(commands)
}

fn require_source_thread_metadata(
    reference: &Reference,
) -> Result<(), PostMergeObserverRuntimeError> {
    if reference.reference_type != ReferenceType::SlackThread {
        return Err(PostMergeObserverRuntimeError::MissingSourceThreadTarget);
    }
    if reference
        .provider
        .as_deref()
        .unwrap_or_default()
        .trim()
        .is_empty()
        || reference
            .locator
            .as_deref()
            .unwrap_or_default()
            .trim()
            .is_empty()
    {
        return Err(PostMergeObserverRuntimeError::MissingSourceThreadMetadata);
    }
    Ok(())
}

fn public_reply_body(projection: &PostMergeObserverPublicationProjection) -> String {
    sanitize_public_text(&format!(
        "Post-merge observer: {}. Closure: {}. Verification: {}. Proof: {}. Receipt: {}.",
        projection.summary,
        projection.reason_code,
        projection
            .verification_criterion_id
            .as_deref()
            .unwrap_or("not_required"),
        projection.proof_criterion_id,
        projection.harness_receipt_ref.uri
    ))
}

fn sanitize_public_text(text: &str) -> String {
    text.split_whitespace()
        .map(sanitize_token)
        .collect::<Vec<_>>()
        .join(" ")
}

fn sanitize_token(token: &str) -> String {
    let trimmed = token.trim_matches(|character: char| {
        matches!(
            character,
            '.' | ',' | ';' | ':' | ')' | '(' | '"' | '\'' | '[' | ']'
        )
    });
    let upper = trimmed.to_ascii_uppercase();
    if trimmed.starts_with("/Users/")
        || trimmed.starts_with("/home/")
        || trimmed.starts_with("/var/folders/")
        || trimmed.starts_with("/private/")
        || upper.starts_with("TOKEN=")
        || upper.starts_with("SECRET=")
        || upper.starts_with("PASSWORD=")
        || upper.starts_with("API_KEY=")
        || upper.starts_with("OPENAI_API_KEY=")
    {
        "[redacted]".to_owned()
    } else {
        token.to_owned()
    }
}
