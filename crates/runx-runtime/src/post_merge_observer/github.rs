// rust-style-allow: large-file because the GitHub observer adapter keeps URL
// parsing, REST readback, and response normalization together so provider
// validation stays auditable.
use serde::Deserialize;
use url::Url;

use runx_contracts::{
    PostMergeProvider, PostMergePullRequestObservation, PostMergePullRequestState,
    PostMergeVerificationObservation, Reference, ReferenceType,
};

use super::{
    PostMergeObserverAdapter, PostMergeObserverAdapterError,
    PostMergeObserverPullRequestObservationRequest,
    PostMergeObserverVerificationObservationRequest,
};
use crate::runtime_http::strip_one_trailing_slash;
pub use crate::runtime_http::{
    HttpMethod as PostMergeObserverHttpMethod,
    ReqwestHttpTransport as PostMergeObserverDefaultHttpTransport,
    RuntimeHttpError as PostMergeObserverHttpError,
    RuntimeHttpHeader as PostMergeObserverHttpHeader,
    RuntimeHttpRequest as PostMergeObserverHttpRequest,
    RuntimeHttpResponse as PostMergeObserverHttpResponse,
    RuntimeHttpTransport as PostMergeObserverHttpTransport,
};

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
        uri: request.pull_request_ref.uri.clone().into_string(),
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
