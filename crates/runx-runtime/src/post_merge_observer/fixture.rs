use serde::Deserialize;

use runx_contracts::{
    PostMergeProvider, PostMergePullRequestObservation, PostMergeVerificationObservation,
    Reference, ReferenceType,
};

use super::{
    PostMergeObserverAdapter, PostMergeObserverAdapterError,
    PostMergeObserverPullRequestObservationRequest,
    PostMergeObserverVerificationObservationRequest, same_reference_identity,
};
use crate::reference_match::same_reference;

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
    if fixture.pull_request.provider != PostMergeProvider::Github
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
