// rust-style-allow: large-file because post-merge closure projection keeps the
// local publication ledger, live adapter boundary, and receipt projection in
// one slice until the live webhook/scheduler adapter lands.
//! Runtime support for post-merge observer publication.

use std::collections::BTreeSet;

use runx_contracts::post_merge_observer::{
    PostMergeObserverCommand, PostMergeObserverCommandRequest,
    normalize_post_merge_observer_command,
};
use runx_contracts::{
    HarnessReceipt, OperationalPolicy, PostMergeObserverPlan, PostMergeObserverPlanError,
    PostMergeObserverPlanRequest, PostMergeObserverPublicationProjection,
    PostMergeObserverRuntimeDecision, PostMergeObserverRuntimeDedupePlan,
    PostMergeObserverSignalSource, PostMergeProvider, PostMergePullRequestObservation,
    PostMergePullRequestState, PostMergeSourceIssueDisposition, PostMergeVerificationObservation,
    Reference, ReferenceType, plan_post_merge_observer_closure,
    project_post_merge_observer_publication_from_receipt,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use url::Url;

pub use crate::runtime_http::{
    HostedHttpError as PostMergeObserverHttpError, HostedHttpHeader as PostMergeObserverHttpHeader,
    HostedHttpRequest as PostMergeObserverHttpRequest,
    HostedHttpResponse as PostMergeObserverHttpResponse,
    HostedTransport as PostMergeObserverHttpTransport, HttpMethod as PostMergeObserverHttpMethod,
    ReqwestHttpTransport as PostMergeObserverDefaultHttpTransport,
};

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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signal_ref: Option<Reference>,
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
    pub command: PostMergeObserverCommand,
    pub pull_request: PostMergePullRequestObservation,
    pub verification: PostMergeVerificationObservation,
    pub closure_plan: PostMergeObserverPlan,
    pub dedupe: PostMergeObserverRuntimeDedupePlan,
    pub publication: PostMergeObserverPublicationRuntime,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PostMergeObserverLivePublishedPublication {
    pub command: PostMergeObserverCommand,
    pub pull_request: PostMergePullRequestObservation,
    pub verification: PostMergeVerificationObservation,
    pub closure_plan: PostMergeObserverPlan,
    pub dedupe: PostMergeObserverRuntimeDedupePlan,
    pub publication: PostMergeObserverPublicationRuntime,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_publication: Option<PostMergeObserverSourcePublicationReadback>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PostMergeObserverSourcePublicationRequest {
    pub publication_key: String,
    pub receipt_ref: Reference,
    pub source_issue_ref: Reference,
    pub source_thread_ref: Reference,
    pub pull_request_ref: Reference,
    pub reason_code: String,
    pub close_source_issue: bool,
    pub commands: Vec<PostMergeObserverPublicationCommand>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PostMergeObserverSourcePublicationObservation {
    pub source_issue_ref: Reference,
    pub source_thread_ref: Reference,
    pub pull_request_ref: Reference,
    pub receipt_ref: Reference,
    pub published_refs: Vec<Reference>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub closed_ref: Option<Reference>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PostMergeObserverSourcePublicationReadback {
    pub request: PostMergeObserverSourcePublicationRequest,
    pub observation: PostMergeObserverSourcePublicationObservation,
    pub proof_refs: Vec<Reference>,
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

pub trait PostMergeObserverPublicationAdapter {
    fn publish_source_update(
        &mut self,
        request: &PostMergeObserverSourcePublicationRequest,
    ) -> Result<PostMergeObserverSourcePublicationObservation, PostMergeObserverAdapterError>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct GithubPullRequestIdentity {
    repo: String,
    number: u64,
}

#[derive(Clone, Debug)]
pub struct GithubPostMergePullRequestObserverAdapter<T = PostMergeObserverDefaultHttpTransport> {
    base_url: String,
    transport: T,
    token: Option<String>,
}

#[cfg(feature = "async-http")]
impl GithubPostMergePullRequestObserverAdapter<PostMergeObserverDefaultHttpTransport> {
    pub fn new(token: Option<String>) -> Result<Self, PostMergeObserverAdapterError> {
        Self::with_transport(
            "https://api.github.com",
            PostMergeObserverDefaultHttpTransport::new().map_err(|error| {
                PostMergeObserverAdapterError::new(
                    "configure_github_pr_observer",
                    error.to_string(),
                )
            })?,
            token,
        )
    }
}

impl<T: PostMergeObserverHttpTransport> GithubPostMergePullRequestObserverAdapter<T> {
    pub fn with_transport(
        base_url: impl AsRef<str>,
        transport: T,
        token: Option<String>,
    ) -> Result<Self, PostMergeObserverAdapterError> {
        let base_url = strip_one_trailing_slash(base_url.as_ref().trim());
        let url = Url::parse(&base_url).map_err(|error| {
            PostMergeObserverAdapterError::new(
                "configure_github_pr_observer",
                format!("invalid github api base url: {error}"),
            )
        })?;
        if !matches!(url.scheme(), "http" | "https") {
            return Err(PostMergeObserverAdapterError::new(
                "configure_github_pr_observer",
                "github api base url must use http or https",
            ));
        }
        Ok(Self {
            base_url,
            transport,
            token: token.filter(|value| !value.trim().is_empty()),
        })
    }

    fn github_pull_request_url(
        &self,
        identity: &GithubPullRequestIdentity,
    ) -> Result<String, PostMergeObserverAdapterError> {
        let (owner, repo) = github_repo_parts(&identity.repo)?;
        let mut url = Url::parse(&self.base_url).map_err(|error| {
            PostMergeObserverAdapterError::new(
                "observe_pull_request_github",
                format!("invalid github api base url: {error}"),
            )
        })?;
        {
            let mut segments = url.path_segments_mut().map_err(|_| {
                PostMergeObserverAdapterError::new(
                    "observe_pull_request_github",
                    "github api base url cannot be a base for path segments",
                )
            })?;
            let number = identity.number.to_string();
            segments.pop_if_empty();
            segments.extend(["repos", owner, repo, "pulls", number.as_str()]);
        }
        Ok(url.to_string())
    }

    fn github_headers(&self) -> Vec<PostMergeObserverHttpHeader> {
        let mut headers = vec![
            PostMergeObserverHttpHeader::new("accept", "application/vnd.github+json"),
            PostMergeObserverHttpHeader::new("user-agent", "runx-post-merge-observer"),
            PostMergeObserverHttpHeader::new("x-github-api-version", "2022-11-28"),
        ];
        if let Some(token) = &self.token {
            headers.push(PostMergeObserverHttpHeader::new(
                "authorization",
                format!("Bearer {token}"),
            ));
        }
        headers
    }
}

impl<T: PostMergeObserverHttpTransport> PostMergeObserverAdapter
    for GithubPostMergePullRequestObserverAdapter<T>
{
    fn observe_pull_request(
        &mut self,
        request: &PostMergeObserverPullRequestObservationRequest,
    ) -> Result<PostMergePullRequestObservation, PostMergeObserverAdapterError> {
        require_github_source_issue_request(request)?;
        let identity = github_pull_request_identity(&request.pull_request_ref)?;
        let response = self
            .transport
            .send(PostMergeObserverHttpRequest {
                method: PostMergeObserverHttpMethod::Get,
                url: self.github_pull_request_url(&identity)?,
                headers: self.github_headers(),
                body: None,
            })
            .map_err(|error: PostMergeObserverHttpError| {
                PostMergeObserverAdapterError::new("observe_pull_request_github", error.to_string())
            })?;
        if !(200..=299).contains(&response.status) {
            return Err(PostMergeObserverAdapterError::new(
                "observe_pull_request_github",
                format!("github pull request API returned HTTP {}", response.status),
            ));
        }
        let payload: GithubPullRequestApiResponse =
            serde_json::from_str(&response.body).map_err(|error| {
                PostMergeObserverAdapterError::new(
                    "observe_pull_request_github",
                    format!("github pull request API returned invalid JSON: {error}"),
                )
            })?;
        github_pull_request_observation_from_response(request, &identity, payload)
    }

    fn observe_verification(
        &mut self,
        _request: &PostMergeObserverVerificationObservationRequest,
    ) -> Result<PostMergeVerificationObservation, PostMergeObserverAdapterError> {
        Err(PostMergeObserverAdapterError::new(
            "observe_verification_github",
            "verification readback adapter is not configured",
        ))
    }
}

#[derive(Clone, Debug, Deserialize)]
struct GithubPullRequestApiResponse {
    number: u64,
    state: String,
    merged: bool,
    #[serde(default)]
    merge_commit_sha: Option<String>,
    updated_at: String,
    #[serde(default)]
    closed_at: Option<String>,
    #[serde(default)]
    merged_at: Option<String>,
    #[serde(default)]
    user: Option<GithubApiUser>,
    #[serde(default)]
    merged_by: Option<GithubApiUser>,
    base: GithubPullRequestBase,
}

#[derive(Clone, Debug, Deserialize)]
struct GithubPullRequestBase {
    repo: GithubRepository,
}

#[derive(Clone, Debug, Deserialize)]
struct GithubRepository {
    full_name: String,
}

#[derive(Clone, Debug, Deserialize)]
struct GithubApiUser {
    login: String,
}

fn require_github_source_issue_request(
    request: &PostMergeObserverPullRequestObservationRequest,
) -> Result<(), PostMergeObserverAdapterError> {
    if request.source_issue_ref.reference_type != ReferenceType::GithubIssue
        || request.source_issue_ref.provider.as_deref() != Some("github")
        || non_empty_option(request.source_issue_ref.locator.as_deref()).is_none()
    {
        return Err(PostMergeObserverAdapterError::new(
            "observe_pull_request_github",
            "source issue ref must be a GitHub issue with provider and locator metadata",
        ));
    }
    if let Some(source_thread_ref) = &request.source_thread_ref {
        if source_thread_ref.reference_type != ReferenceType::SlackThread
            || source_thread_ref.provider.as_deref() != Some("slack")
            || non_empty_option(source_thread_ref.locator.as_deref()).is_none()
        {
            return Err(PostMergeObserverAdapterError::new(
                "observe_pull_request_github",
                "source thread ref must be a Slack thread with provider and locator metadata",
            ));
        }
    }
    Ok(())
}

fn github_pull_request_identity(
    reference: &Reference,
) -> Result<GithubPullRequestIdentity, PostMergeObserverAdapterError> {
    if reference.reference_type != ReferenceType::GithubPullRequest
        || reference.provider.as_deref() != Some("github")
    {
        return Err(PostMergeObserverAdapterError::new(
            "observe_pull_request_github",
            "pull request ref must be a GitHub pull request with provider and locator metadata",
        ));
    }
    let locator = reference
        .locator
        .as_deref()
        .and_then(non_empty_str)
        .ok_or_else(|| {
            PostMergeObserverAdapterError::new(
                "observe_pull_request_github",
                "pull request ref must be a GitHub pull request with provider and locator metadata",
            )
        })?;
    let (repo, number) = locator.rsplit_once('#').ok_or_else(|| {
        PostMergeObserverAdapterError::new(
            "observe_pull_request_github",
            "pull request locator must use owner/repo#number",
        )
    })?;
    github_repo_parts(repo)?;
    let number = number.parse::<u64>().map_err(|error| {
        PostMergeObserverAdapterError::new(
            "observe_pull_request_github",
            format!("pull request locator number must be a positive integer: {error}"),
        )
    })?;
    if number == 0 {
        return Err(PostMergeObserverAdapterError::new(
            "observe_pull_request_github",
            "pull request locator number must be a positive integer",
        ));
    }
    Ok(GithubPullRequestIdentity {
        repo: repo.to_owned(),
        number,
    })
}

fn github_repo_parts(repo: &str) -> Result<(&str, &str), PostMergeObserverAdapterError> {
    let (owner, name) = repo.split_once('/').ok_or_else(|| {
        PostMergeObserverAdapterError::new(
            "observe_pull_request_github",
            "pull request locator must use owner/repo#number",
        )
    })?;
    if non_empty_option(Some(owner)).is_none()
        || non_empty_option(Some(name)).is_none()
        || name.contains('/')
    {
        return Err(PostMergeObserverAdapterError::new(
            "observe_pull_request_github",
            "pull request locator must use owner/repo#number",
        ));
    }
    Ok((owner, name))
}

fn github_pull_request_observation_from_response(
    request: &PostMergeObserverPullRequestObservationRequest,
    identity: &GithubPullRequestIdentity,
    payload: GithubPullRequestApiResponse,
) -> Result<PostMergePullRequestObservation, PostMergeObserverAdapterError> {
    if payload.number != identity.number || payload.base.repo.full_name != identity.repo {
        return Err(PostMergeObserverAdapterError::new(
            "observe_pull_request_github",
            "github pull request readback does not match requested pull request",
        ));
    }
    let state = match payload.state.as_str() {
        "open" => PostMergePullRequestState::Open,
        "closed" => PostMergePullRequestState::Closed,
        other => {
            return Err(PostMergeObserverAdapterError::new(
                "observe_pull_request_github",
                format!("github pull request API returned unsupported state '{other}'"),
            ));
        }
    };
    let updated_at = non_empty_owned(payload.updated_at).ok_or_else(|| {
        PostMergeObserverAdapterError::new(
            "observe_pull_request_github",
            "github pull request readback is missing updated_at",
        )
    })?;
    let closed_at = payload.closed_at.and_then(non_empty_owned);
    let merged_at = payload.merged_at.and_then(non_empty_owned);
    let observed_at = merged_at
        .clone()
        .or_else(|| closed_at.clone())
        .unwrap_or_else(|| updated_at.clone());
    let merge_sha = payload.merge_commit_sha.and_then(non_empty_owned);
    if payload.merged && merge_sha.is_none() {
        return Err(PostMergeObserverAdapterError::new(
            "observe_pull_request_github",
            "merged pull request readback requires merge_commit_sha",
        ));
    }
    let actor = payload
        .merged_by
        .or(payload.user)
        .and_then(|user| non_empty_owned(user.login))
        .map(|login| format!("github:user:{login}"));

    Ok(PostMergePullRequestObservation {
        provider: PostMergeProvider::Github,
        repo: identity.repo.clone(),
        number: identity.number,
        uri: request.pull_request_ref.uri.clone(),
        state,
        merged: payload.merged,
        merge_sha,
        observed_at,
        closed_at,
        actor,
    })
}

fn non_empty_owned(value: String) -> Option<String> {
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}

fn non_empty_option(value: Option<&str>) -> Option<&str> {
    value.and_then(non_empty_str)
}

fn non_empty_str(value: &str) -> Option<&str> {
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}

fn strip_one_trailing_slash(value: &str) -> String {
    value
        .strip_suffix('/')
        .map_or_else(|| value.to_owned(), str::to_owned)
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixtureBackedGitHubPostMergeObservation {
    pub source_issue_ref: Reference,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_thread_ref: Option<Reference>,
    pub pull_request_ref: Reference,
    pub pull_request: PostMergePullRequestObservation,
    pub verification: PostMergeVerificationObservation,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FixtureBackedGitHubPostMergeObserverAdapter {
    observation: FixtureBackedGitHubPostMergeObservation,
}

impl FixtureBackedGitHubPostMergeObserverAdapter {
    pub fn from_json_str(source: &str) -> Result<Self, PostMergeObserverAdapterError> {
        serde_json::from_str::<FixtureBackedGitHubPostMergeObservation>(source)
            .map(|observation| Self { observation })
            .map_err(|error| {
                PostMergeObserverAdapterError::new(
                    "load_fixture_github_post_merge_observation",
                    error.to_string(),
                )
            })
    }
}

impl PostMergeObserverAdapter for FixtureBackedGitHubPostMergeObserverAdapter {
    fn observe_pull_request(
        &mut self,
        request: &PostMergeObserverPullRequestObservationRequest,
    ) -> Result<PostMergePullRequestObservation, PostMergeObserverAdapterError> {
        require_github_fixture_request(request, &self.observation)?;
        Ok(self.observation.pull_request.clone())
    }

    fn observe_verification(
        &mut self,
        request: &PostMergeObserverVerificationObservationRequest,
    ) -> Result<PostMergeVerificationObservation, PostMergeObserverAdapterError> {
        if !same_reference(
            &request.source_issue_ref,
            &self.observation.source_issue_ref,
        ) {
            return Err(PostMergeObserverAdapterError::new(
                "observe_verification_fixture",
                "source issue ref does not match fixture readback",
            ));
        }
        if request.source_thread_ref != self.observation.source_thread_ref {
            return Err(PostMergeObserverAdapterError::new(
                "observe_verification_fixture",
                "source thread ref does not match fixture readback",
            ));
        }
        if request.pull_request != self.observation.pull_request {
            return Err(PostMergeObserverAdapterError::new(
                "observe_verification_fixture",
                "pull request observation does not match fixture readback",
            ));
        }
        Ok(self.observation.verification.clone())
    }
}

fn require_github_fixture_request(
    request: &PostMergeObserverPullRequestObservationRequest,
    fixture: &FixtureBackedGitHubPostMergeObservation,
) -> Result<(), PostMergeObserverAdapterError> {
    if fixture.pull_request.provider != runx_contracts::PostMergeProvider::Github
        || fixture.pull_request_ref.reference_type != ReferenceType::GithubPullRequest
        || fixture.pull_request_ref.provider.as_deref() != Some("github")
    {
        return Err(PostMergeObserverAdapterError::new(
            "observe_pull_request_fixture",
            "fixture must describe a GitHub pull request",
        ));
    }
    if request.source_issue_ref != fixture.source_issue_ref {
        return Err(PostMergeObserverAdapterError::new(
            "observe_pull_request_fixture",
            "source issue ref does not match fixture readback",
        ));
    }
    if request.source_thread_ref != fixture.source_thread_ref {
        return Err(PostMergeObserverAdapterError::new(
            "observe_pull_request_fixture",
            "source thread ref does not match fixture readback",
        ));
    }
    if !same_reference_identity(&request.pull_request_ref, &fixture.pull_request_ref) {
        return Err(PostMergeObserverAdapterError::new(
            "observe_pull_request_fixture",
            "pull request ref does not match fixture readback",
        ));
    }
    if request.pull_request_ref.uri != fixture.pull_request.uri {
        return Err(PostMergeObserverAdapterError::new(
            "observe_pull_request_fixture",
            "pull request observation URI does not match requested pull request",
        ));
    }
    Ok(())
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
    #[error("post-merge source publication readback mismatch: {0}")]
    SourcePublicationMismatch(String),
}

pub fn execute_post_merge_observer_with_adapter<A: PostMergeObserverAdapter>(
    policy: &OperationalPolicy,
    request: &PostMergeObserverLivePublicationRequest,
    sealed_receipt: &HarnessReceipt,
    adapter: &mut A,
    ledger: &mut PostMergeObserverPublicationLedger,
) -> Result<PostMergeObserverLivePublication, PostMergeObserverRuntimeError> {
    let command = normalize_post_merge_observer_command(
        policy,
        &PostMergeObserverCommandRequest {
            source_id: request.source_id.clone(),
            source_issue_ref: request.source_issue_ref.clone(),
            source_thread_ref: request.source_thread_ref.clone(),
            pull_request_ref: request.pull_request_ref.clone(),
            signal_source: request.signal_source,
            signal_ref: request.signal_ref.clone(),
        },
    )?;
    let observed = observe_post_merge_closure(policy, &command, sealed_receipt, adapter)?;

    let dedupe = sealed_receipt_dedupe_plan(sealed_receipt, request.signal_source);
    let publication =
        project_post_merge_observer_publication_commands(&dedupe, sealed_receipt, ledger)?;

    Ok(PostMergeObserverLivePublication {
        command,
        pull_request: observed.pull_request,
        verification: observed.verification,
        closure_plan: observed.closure_plan,
        dedupe,
        publication,
    })
}

pub fn execute_post_merge_observer_with_publication_adapter<
    A: PostMergeObserverAdapter,
    P: PostMergeObserverPublicationAdapter,
>(
    policy: &OperationalPolicy,
    request: &PostMergeObserverLivePublicationRequest,
    sealed_receipt: &HarnessReceipt,
    adapter: &mut A,
    publisher: &mut P,
    ledger: &mut PostMergeObserverPublicationLedger,
) -> Result<PostMergeObserverLivePublishedPublication, PostMergeObserverRuntimeError> {
    let command = normalize_post_merge_observer_command(
        policy,
        &PostMergeObserverCommandRequest {
            source_id: request.source_id.clone(),
            source_issue_ref: request.source_issue_ref.clone(),
            source_thread_ref: request.source_thread_ref.clone(),
            pull_request_ref: request.pull_request_ref.clone(),
            signal_source: request.signal_source,
            signal_ref: request.signal_ref.clone(),
        },
    )?;
    let observed = observe_post_merge_closure(policy, &command, sealed_receipt, adapter)?;

    let dedupe = sealed_receipt_dedupe_plan(sealed_receipt, request.signal_source);
    let (publication, projection) =
        plan_post_merge_observer_publication_commands(&dedupe, sealed_receipt, ledger)?;
    let source_publication =
        if publication.decision == PostMergeObserverPublicationRuntimeDecision::Publish {
            let publication_request = source_publication_request(&publication, &projection)?;
            let observation = publisher.publish_source_update(&publication_request)?;
            validate_source_publication_observation(&publication_request, &observation)?;
            ledger.mark_published(&dedupe.publication_key);
            Some(PostMergeObserverSourcePublicationReadback {
                proof_refs: source_publication_proof_refs(&publication_request, &observation),
                request: publication_request,
                observation,
            })
        } else {
            None
        };

    Ok(PostMergeObserverLivePublishedPublication {
        command,
        pull_request: observed.pull_request,
        verification: observed.verification,
        closure_plan: observed.closure_plan,
        dedupe,
        publication,
        source_publication,
    })
}

struct ObservedPostMergeClosure {
    pull_request: PostMergePullRequestObservation,
    verification: PostMergeVerificationObservation,
    closure_plan: PostMergeObserverPlan,
}

fn observe_post_merge_closure<A: PostMergeObserverAdapter>(
    policy: &OperationalPolicy,
    command: &PostMergeObserverCommand,
    sealed_receipt: &HarnessReceipt,
    adapter: &mut A,
) -> Result<ObservedPostMergeClosure, PostMergeObserverRuntimeError> {
    let pull_request =
        adapter.observe_pull_request(&PostMergeObserverPullRequestObservationRequest {
            source_id: Some(command.source_id.clone()),
            source_issue_ref: command.source_issue_ref.clone(),
            source_thread_ref: command.source_thread_ref.clone(),
            pull_request_ref: command.pull_request_ref.clone(),
        })?;
    let verification =
        adapter.observe_verification(&PostMergeObserverVerificationObservationRequest {
            source_id: Some(command.source_id.clone()),
            source_issue_ref: command.source_issue_ref.clone(),
            source_thread_ref: command.source_thread_ref.clone(),
            pull_request: pull_request.clone(),
        })?;
    let closure_plan = plan_post_merge_observer_closure(
        policy,
        &PostMergeObserverPlanRequest {
            source_id: Some(command.source_id.clone()),
            source_issue_ref: command.source_issue_ref.clone(),
            source_thread_ref: command.source_thread_ref.clone(),
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
    Ok(ObservedPostMergeClosure {
        pull_request,
        verification,
        closure_plan,
    })
}

pub fn project_post_merge_observer_publication_commands(
    dedupe: &PostMergeObserverRuntimeDedupePlan,
    sealed_receipt: &HarnessReceipt,
    ledger: &mut PostMergeObserverPublicationLedger,
) -> Result<PostMergeObserverPublicationRuntime, PostMergeObserverRuntimeError> {
    let (runtime, _) =
        plan_post_merge_observer_publication_commands(dedupe, sealed_receipt, ledger)?;
    if runtime.decision == PostMergeObserverPublicationRuntimeDecision::Publish {
        ledger.mark_published(&runtime.publication_key);
    }
    Ok(runtime)
}

fn plan_post_merge_observer_publication_commands(
    dedupe: &PostMergeObserverRuntimeDedupePlan,
    sealed_receipt: &HarnessReceipt,
    ledger: &PostMergeObserverPublicationLedger,
) -> Result<
    (
        PostMergeObserverPublicationRuntime,
        PostMergeObserverPublicationProjection,
    ),
    PostMergeObserverRuntimeError,
> {
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
        return Ok((
            PostMergeObserverPublicationRuntime {
                decision: PostMergeObserverPublicationRuntimeDecision::AlreadyPublished,
                publication_key: dedupe.publication_key.clone(),
                receipt_ref: projection.harness_receipt_ref.clone(),
                commands: Vec::new(),
            },
            projection,
        ));
    }

    let commands = publication_commands(&dedupe.publication_key, &projection)?;

    Ok((
        PostMergeObserverPublicationRuntime {
            decision: PostMergeObserverPublicationRuntimeDecision::Publish,
            publication_key: dedupe.publication_key.clone(),
            receipt_ref: projection.harness_receipt_ref.clone(),
            commands,
        },
        projection,
    ))
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

fn source_publication_request(
    publication: &PostMergeObserverPublicationRuntime,
    projection: &PostMergeObserverPublicationProjection,
) -> Result<PostMergeObserverSourcePublicationRequest, PostMergeObserverRuntimeError> {
    let source_thread_ref = projection
        .source_thread_ref
        .as_ref()
        .ok_or(PostMergeObserverRuntimeError::MissingSourceThreadTarget)?;
    require_source_thread_metadata(source_thread_ref)?;

    Ok(PostMergeObserverSourcePublicationRequest {
        publication_key: publication.publication_key.clone(),
        receipt_ref: publication.receipt_ref.clone(),
        source_issue_ref: projection.source_issue_ref.clone(),
        source_thread_ref: source_thread_ref.clone(),
        pull_request_ref: projection.pull_request_ref.clone(),
        reason_code: projection.reason_code.clone(),
        close_source_issue: projection.close_authorized
            && projection.source_issue_disposition == PostMergeSourceIssueDisposition::Close,
        commands: publication.commands.clone(),
    })
}

fn validate_source_publication_observation(
    request: &PostMergeObserverSourcePublicationRequest,
    observation: &PostMergeObserverSourcePublicationObservation,
) -> Result<(), PostMergeObserverRuntimeError> {
    validate_source_publication_identity(request, observation)?;
    validate_source_publication_proofs(request, observation)?;
    validate_source_publication_required_commands(request, observation)?;
    validate_source_publication_close_readback(request, observation)
}

fn validate_source_publication_identity(
    request: &PostMergeObserverSourcePublicationRequest,
    observation: &PostMergeObserverSourcePublicationObservation,
) -> Result<(), PostMergeObserverRuntimeError> {
    require_matching_reference_identity(
        &observation.source_issue_ref,
        &request.source_issue_ref,
        "source issue readback does not match publication request",
    )?;
    require_matching_reference_identity(
        &observation.source_thread_ref,
        &request.source_thread_ref,
        "source thread readback does not match publication request",
    )?;
    require_matching_reference_identity(
        &observation.pull_request_ref,
        &request.pull_request_ref,
        "target pull request readback does not match publication request",
    )?;
    require_matching_reference(
        &observation.receipt_ref,
        &request.receipt_ref,
        "receipt readback does not match publication request",
    )
}

fn validate_source_publication_proofs(
    request: &PostMergeObserverSourcePublicationRequest,
    observation: &PostMergeObserverSourcePublicationObservation,
) -> Result<(), PostMergeObserverRuntimeError> {
    for reference in &observation.published_refs {
        require_readback_reference_metadata(reference, "published ref")?;
    }
    if let Some(reference) = &observation.closed_ref {
        require_readback_reference_metadata(reference, "source issue close readback")?;
    }

    let proof_ref_count =
        observation.published_refs.len() + usize::from(observation.closed_ref.is_some());
    if proof_ref_count < request.commands.len() {
        return Err(PostMergeObserverRuntimeError::SourcePublicationMismatch(
            "publication readback did not return a proof ref for every source command".to_owned(),
        ));
    }
    Ok(())
}

fn validate_source_publication_required_commands(
    request: &PostMergeObserverSourcePublicationRequest,
    observation: &PostMergeObserverSourcePublicationObservation,
) -> Result<(), PostMergeObserverRuntimeError> {
    if source_issue_comment_required(request)
        && !has_provider_ref(&observation.published_refs, "github")
    {
        return Err(PostMergeObserverRuntimeError::SourcePublicationMismatch(
            "source issue comment readback is required".to_owned(),
        ));
    }
    if source_thread_reply_required(request)
        && !has_typed_provider_ref(
            &observation.published_refs,
            ReferenceType::SlackThread,
            "slack",
        )
    {
        return Err(PostMergeObserverRuntimeError::SourcePublicationMismatch(
            "source thread reply readback is required".to_owned(),
        ));
    }
    Ok(())
}

fn validate_source_publication_close_readback(
    request: &PostMergeObserverSourcePublicationRequest,
    observation: &PostMergeObserverSourcePublicationObservation,
) -> Result<(), PostMergeObserverRuntimeError> {
    let close_required = source_issue_close_required(request);
    match (close_required, &observation.closed_ref) {
        (true, Some(reference))
            if same_reference_identity(reference, &request.source_issue_ref) => {}
        (true, Some(_)) => {
            return Err(PostMergeObserverRuntimeError::SourcePublicationMismatch(
                "source issue close readback does not match publication request".to_owned(),
            ));
        }
        (true, None) => {
            return Err(PostMergeObserverRuntimeError::SourcePublicationMismatch(
                "source issue close readback is required".to_owned(),
            ));
        }
        (false, Some(_)) => {
            return Err(PostMergeObserverRuntimeError::SourcePublicationMismatch(
                "source issue close readback was returned when close was not planned".to_owned(),
            ));
        }
        (false, None) => {}
    }

    Ok(())
}

fn require_matching_reference_identity(
    observed: &Reference,
    expected: &Reference,
    message: &'static str,
) -> Result<(), PostMergeObserverRuntimeError> {
    if !same_reference_identity(observed, expected) {
        return Err(PostMergeObserverRuntimeError::SourcePublicationMismatch(
            message.to_owned(),
        ));
    }
    Ok(())
}

fn require_matching_reference(
    observed: &Reference,
    expected: &Reference,
    message: &'static str,
) -> Result<(), PostMergeObserverRuntimeError> {
    if !same_reference(observed, expected) {
        return Err(PostMergeObserverRuntimeError::SourcePublicationMismatch(
            message.to_owned(),
        ));
    }
    Ok(())
}

fn source_issue_comment_required(request: &PostMergeObserverSourcePublicationRequest) -> bool {
    request.commands.iter().any(|command| {
        matches!(
            command,
            PostMergeObserverPublicationCommand::SourceIssueComment { .. }
        )
    })
}

fn source_thread_reply_required(request: &PostMergeObserverSourcePublicationRequest) -> bool {
    request.commands.iter().any(|command| {
        matches!(
            command,
            PostMergeObserverPublicationCommand::SourceThreadReply { .. }
        )
    })
}

fn source_issue_close_required(request: &PostMergeObserverSourcePublicationRequest) -> bool {
    request.commands.iter().any(|command| {
        matches!(
            command,
            PostMergeObserverPublicationCommand::SourceIssueClose { .. }
        )
    })
}

fn source_publication_proof_refs(
    request: &PostMergeObserverSourcePublicationRequest,
    observation: &PostMergeObserverSourcePublicationObservation,
) -> Vec<Reference> {
    let mut proof_refs = observation.published_refs.clone();
    if let Some(reference) = &observation.closed_ref {
        proof_refs.push(reference.clone());
    }
    proof_refs.push(request.receipt_ref.clone());
    proof_refs
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

fn require_readback_reference_metadata(
    reference: &Reference,
    label: &'static str,
) -> Result<(), PostMergeObserverRuntimeError> {
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
        return Err(PostMergeObserverRuntimeError::SourcePublicationMismatch(
            format!("{label} requires provider and locator metadata"),
        ));
    }
    Ok(())
}

fn same_reference(left: &Reference, right: &Reference) -> bool {
    left.reference_type == right.reference_type && left.uri == right.uri
}

fn same_reference_identity(left: &Reference, right: &Reference) -> bool {
    same_reference(left, right)
        && left.provider.as_deref() == right.provider.as_deref()
        && left.locator.as_deref() == right.locator.as_deref()
}

fn has_provider_ref(references: &[Reference], provider: &str) -> bool {
    references
        .iter()
        .any(|reference| reference.provider.as_deref() == Some(provider))
}

fn has_typed_provider_ref(
    references: &[Reference],
    reference_type: ReferenceType,
    provider: &str,
) -> bool {
    references.iter().any(|reference| {
        reference.reference_type == reference_type
            && reference.provider.as_deref() == Some(provider)
    })
}

fn public_reply_body(projection: &PostMergeObserverPublicationProjection) -> String {
    sanitize_public_text(&format!(
        "Post-merge observer: {}. Source issue: {}. Target PR: {}. Merge: {}. Review gate: external_human. Closure: {}. Verification: {}. Verification summary: {}. Proof: {}. Next: {}. Receipt: {}.",
        projection.summary,
        projection.source_issue_ref.uri,
        projection.pull_request_ref.uri,
        projection.merge_sha.as_deref().unwrap_or("not_available"),
        projection.reason_code,
        projection
            .verification_criterion_id
            .as_deref()
            .unwrap_or("not_required"),
        projection
            .verification_summary
            .as_deref()
            .unwrap_or("not_required"),
        projection.proof_criterion_id,
        next_human_action(projection),
        projection.harness_receipt_ref.uri
    ))
}

fn next_human_action(projection: &PostMergeObserverPublicationProjection) -> &'static str {
    if projection.close_authorized
        && projection.source_issue_disposition == PostMergeSourceIssueDisposition::Close
    {
        return "none";
    }
    match projection.reason_code.as_str() {
        "failed_verification" => "review_failed_verification",
        "merged_pending_verification" => "wait_for_verification",
        "closed_unmerged" => "review_source_issue",
        _ => "review_source_issue",
    }
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
