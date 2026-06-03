use serde::Serialize;

pub(super) use super::{github_pull_request_number, github_repository};

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
