use serde::{Deserialize, Serialize};
use url::Url;

use runx_contracts::{
    TargetRepoRunnerDedupeLookupObservation, TargetRepoRunnerProvider,
    TargetRepoRunnerProviderPullRequest,
};

use super::{
    TargetRepoRunnerProviderDedupeLookupCommand, TargetRepoRunnerRuntimeError,
    target_repo_runner_provider_dedupe_observation_from_pull_requests,
};
use crate::runtime_http::strip_one_trailing_slash;
pub use crate::runtime_http::{
    HttpMethod as TargetRepoRunnerHttpMethod,
    ReqwestHttpTransport as TargetRepoRunnerDefaultHttpTransport,
    RuntimeHttpError as TargetRepoRunnerHttpError, RuntimeHttpHeader as TargetRepoRunnerHttpHeader,
    RuntimeHttpRequest as TargetRepoRunnerHttpRequest,
    RuntimeHttpResponse as TargetRepoRunnerHttpResponse,
    RuntimeHttpTransport as TargetRepoRunnerHttpTransport,
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct TargetRepoRunnerGithubRepository {
    pub owner: String,
    pub name: String,
    pub full_name: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct TargetRepoRunnerGithubPullRequestSearchCommand {
    pub repo: String,
    pub state: TargetRepoRunnerGithubPullRequestSearchState,
    pub query: String,
    pub terms: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetRepoRunnerGithubPullRequestSearchState {
    Open,
}

#[derive(Clone, Debug)]
pub struct TargetRepoRunnerGithubApiClient<T = TargetRepoRunnerDefaultHttpTransport> {
    base_url: String,
    transport: T,
    token: Option<String>,
}

#[cfg(feature = "async-http")]
impl TargetRepoRunnerGithubApiClient<TargetRepoRunnerDefaultHttpTransport> {
    pub fn new(token: Option<String>) -> Result<Self, TargetRepoRunnerRuntimeError> {
        Self::with_transport(
            "https://api.github.com",
            TargetRepoRunnerDefaultHttpTransport::new().map_err(|error| {
                TargetRepoRunnerRuntimeError::CommandValidation {
                    operation: "provider_api_lookup",
                    message: error.to_string(),
                }
            })?,
            token,
        )
    }
}

impl<T: TargetRepoRunnerHttpTransport> TargetRepoRunnerGithubApiClient<T> {
    pub fn with_transport(
        base_url: impl AsRef<str>,
        transport: T,
        token: Option<String>,
    ) -> Result<Self, TargetRepoRunnerRuntimeError> {
        let base_url = strip_one_trailing_slash(base_url.as_ref());
        let url = Url::parse(&base_url).map_err(|error| {
            TargetRepoRunnerRuntimeError::CommandValidation {
                operation: "provider_api_lookup",
                message: format!("invalid github api base url: {error}"),
            }
        })?;
        if !matches!(url.scheme(), "http" | "https") {
            return Err(TargetRepoRunnerRuntimeError::CommandValidation {
                operation: "provider_api_lookup",
                message: "github api base url must use http or https".to_owned(),
            });
        }
        Ok(Self {
            base_url,
            transport,
            token: token.filter(|value| !value.trim().is_empty()),
        })
    }

    pub fn provider_dedupe_lookup(
        &self,
        command: &TargetRepoRunnerProviderDedupeLookupCommand,
    ) -> Result<TargetRepoRunnerDedupeLookupObservation, TargetRepoRunnerRuntimeError> {
        self.require_github_command(command)?;
        let url = self.github_search_url(command)?;
        let response = self
            .transport
            .send(TargetRepoRunnerHttpRequest {
                method: TargetRepoRunnerHttpMethod::Get,
                url,
                headers: self.github_headers(),
                body: None,
            })
            .map_err(|error| TargetRepoRunnerRuntimeError::CommandValidation {
                operation: "provider_api_lookup",
                message: error.to_string(),
            })?;
        if !(200..=299).contains(&response.status) {
            return Err(TargetRepoRunnerRuntimeError::CommandValidation {
                operation: "provider_api_lookup",
                message: format!("github search API returned HTTP {}", response.status),
            });
        }
        let payload: GithubIssueSearchResponse =
            serde_json::from_str(&response.body).map_err(|error| {
                TargetRepoRunnerRuntimeError::CommandValidation {
                    operation: "provider_api_lookup",
                    message: format!("github search API returned invalid JSON: {error}"),
                }
            })?;
        let pull_requests = payload
            .items
            .into_iter()
            .filter_map(|item| github_search_item_to_pull_request(command, item))
            .collect();
        target_repo_runner_provider_dedupe_observation_from_pull_requests(command, pull_requests)
    }

    fn require_github_command(
        &self,
        command: &TargetRepoRunnerProviderDedupeLookupCommand,
    ) -> Result<(), TargetRepoRunnerRuntimeError> {
        if command.provider == TargetRepoRunnerProvider::Github {
            return Ok(());
        }
        Err(TargetRepoRunnerRuntimeError::CommandValidation {
            operation: "provider_api_lookup",
            message: "github provider lookup client only supports github commands".to_owned(),
        })
    }

    fn github_search_url(
        &self,
        command: &TargetRepoRunnerProviderDedupeLookupCommand,
    ) -> Result<String, TargetRepoRunnerRuntimeError> {
        let mut url = Url::parse(&format!("{}/search/issues", self.base_url)).map_err(|error| {
            TargetRepoRunnerRuntimeError::CommandValidation {
                operation: "provider_api_lookup",
                message: format!("invalid github search url: {error}"),
            }
        })?;
        {
            let mut pairs = url.query_pairs_mut();
            pairs.append_pair("q", &command.query.query);
            pairs.append_pair("per_page", &command.result_limit.to_string());
        }
        Ok(url.to_string())
    }

    fn github_headers(&self) -> Vec<TargetRepoRunnerHttpHeader> {
        let mut headers = vec![
            TargetRepoRunnerHttpHeader::new("accept", "application/vnd.github+json"),
            TargetRepoRunnerHttpHeader::new("user-agent", "runx-target-repo-runner"),
            TargetRepoRunnerHttpHeader::new("x-github-api-version", "2022-11-28"),
        ];
        if let Some(token) = &self.token {
            headers.push(TargetRepoRunnerHttpHeader::new(
                "authorization",
                format!("Bearer {token}"),
            ));
        }
        headers
    }
}

#[derive(Clone, Debug, Deserialize)]
struct GithubIssueSearchResponse {
    #[serde(default)]
    items: Vec<GithubIssueSearchItem>,
}

#[derive(Clone, Debug, Deserialize)]
struct GithubIssueSearchItem {
    html_url: String,
    #[serde(default)]
    number: Option<u64>,
    #[serde(default)]
    state: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    pull_request: Option<GithubPullRequestMarker>,
}

#[derive(Clone, Debug, Deserialize)]
struct GithubPullRequestMarker {}

pub(super) fn github_pull_request_number(
    repo: &str,
    url: &str,
) -> Result<u64, TargetRepoRunnerRuntimeError> {
    let prefix = format!("https://github.com/{repo}/pull/");
    let Some(number) = url.strip_prefix(&prefix) else {
        return Err(TargetRepoRunnerRuntimeError::CommandValidation {
            operation: "pull_request",
            message: "pull request readback URL must belong to the target repo".to_owned(),
        });
    };
    let number = number.strip_suffix('/').unwrap_or(number);
    if number.is_empty() || !number.chars().all(|character| character.is_ascii_digit()) {
        return Err(TargetRepoRunnerRuntimeError::CommandValidation {
            operation: "pull_request",
            message: "pull request readback URL must end with a pull request number".to_owned(),
        });
    }
    number
        .parse::<u64>()
        .map_err(|error| TargetRepoRunnerRuntimeError::CommandValidation {
            operation: "pull_request",
            message: format!("pull request readback number is invalid: {error}"),
        })
}

pub(super) fn github_repository(
    repo: &str,
    operation: &'static str,
) -> Result<TargetRepoRunnerGithubRepository, TargetRepoRunnerRuntimeError> {
    let mut parts = repo.split('/');
    let owner = parts.next().unwrap_or_default();
    let name = parts.next().unwrap_or_default();
    if owner.is_empty() || name.is_empty() || parts.next().is_some() {
        return Err(invalid_github_repo(operation));
    }
    if !valid_github_owner(owner) || !valid_github_repo_name(name) {
        return Err(invalid_github_repo(operation));
    }
    Ok(TargetRepoRunnerGithubRepository {
        owner: owner.to_owned(),
        name: name.to_owned(),
        full_name: format!("{owner}/{name}"),
    })
}

fn invalid_github_repo(operation: &'static str) -> TargetRepoRunnerRuntimeError {
    TargetRepoRunnerRuntimeError::CommandValidation {
        operation,
        message: "target repo must be a github owner/repo with safe path segments".to_owned(),
    }
}

fn valid_github_owner(owner: &str) -> bool {
    !owner.is_empty()
        && owner.len() <= 39
        && !owner.starts_with('-')
        && !owner.ends_with('-')
        && owner
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '-')
}

fn valid_github_repo_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 100
        && name != "."
        && name != ".."
        && name.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-')
        })
}

pub(super) fn validate_provider_lookup_term(
    value: &str,
    field: &'static str,
) -> Result<(), TargetRepoRunnerRuntimeError> {
    if value.trim().is_empty() || value.chars().any(char::is_control) {
        return Err(TargetRepoRunnerRuntimeError::CommandValidation {
            operation: "provider_dedupe_lookup",
            message: format!("provider lookup {field} must be non-empty text"),
        });
    }
    Ok(())
}

pub(super) fn github_search_exact_term(value: &str) -> String {
    let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}

fn github_search_item_to_pull_request(
    command: &TargetRepoRunnerProviderDedupeLookupCommand,
    item: GithubIssueSearchItem,
) -> Option<TargetRepoRunnerProviderPullRequest> {
    item.pull_request.as_ref()?;
    let expected_prefix = format!("https://github.com/{}/pull/", command.target_repo);
    if !item.html_url.starts_with(&expected_prefix) {
        return None;
    }
    let text = [item.title.as_deref(), item.body.as_deref()]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join("\n");
    let markers = command
        .markers
        .iter()
        .filter(|marker| text.contains(marker.as_str()))
        .cloned()
        .collect();
    let refs = command
        .required_refs
        .iter()
        .filter(|reference| text.contains(reference.uri.as_str()))
        .cloned()
        .collect();
    Some(TargetRepoRunnerProviderPullRequest {
        url: item.html_url,
        number: item.number,
        branch: None,
        open: item
            .state
            .as_deref()
            .is_none_or(|state| state.eq_ignore_ascii_case("open")),
        markers,
        refs,
    })
}
