//! Adapter trait and error types for target-repo runner execution. The trait
//! is implemented by fixture-backed and host/provider-backed lanes; the error
//! enum is the runtime's single exit channel for execution errors.

use std::fmt;

use runx_contracts::{
    TargetRepoRunnerDedupeLookupObservation, TargetRepoRunnerPlanError,
    TargetRepoRunnerReadinessObservation,
};

use super::commands::{
    TargetRepoRunnerCheckoutCommand, TargetRepoRunnerGitMutationCommand,
    TargetRepoRunnerGitMutationObservation, TargetRepoRunnerGovernedRunnerInvocation,
    TargetRepoRunnerGovernedRunnerObservation, TargetRepoRunnerProviderDedupeLookupCommand,
    TargetRepoRunnerPullRequestObservation, TargetRepoRunnerPullRequestObservationRequest,
    TargetRepoRunnerSourcePublicationObservation, TargetRepoRunnerSourcePublicationRequest,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TargetRepoRunnerAdapterError {
    pub operation: &'static str,
    pub message: String,
}

impl TargetRepoRunnerAdapterError {
    pub fn new(operation: &'static str, message: impl Into<String>) -> Self {
        Self {
            operation,
            message: message.into(),
        }
    }
}

impl fmt::Display for TargetRepoRunnerAdapterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} failed: {}", self.operation, self.message)
    }
}

impl std::error::Error for TargetRepoRunnerAdapterError {}

pub trait TargetRepoRunnerAdapter {
    fn checkout_readiness(
        &mut self,
        command: &TargetRepoRunnerCheckoutCommand,
    ) -> Result<TargetRepoRunnerReadinessObservation, TargetRepoRunnerAdapterError>;

    fn provider_dedupe_lookup(
        &mut self,
        command: &TargetRepoRunnerProviderDedupeLookupCommand,
    ) -> Result<TargetRepoRunnerDedupeLookupObservation, TargetRepoRunnerAdapterError>;

    fn invoke_governed_runner(
        &mut self,
        invocation: &TargetRepoRunnerGovernedRunnerInvocation,
    ) -> Result<TargetRepoRunnerGovernedRunnerObservation, TargetRepoRunnerAdapterError>;

    fn apply_git_mutation(
        &mut self,
        _command: &TargetRepoRunnerGitMutationCommand,
    ) -> Result<TargetRepoRunnerGitMutationObservation, TargetRepoRunnerAdapterError> {
        Err(TargetRepoRunnerAdapterError::new(
            "git_mutation",
            "adapter does not implement target git mutation readback",
        ))
    }

    fn observe_pull_request(
        &mut self,
        request: &TargetRepoRunnerPullRequestObservationRequest,
    ) -> Result<TargetRepoRunnerPullRequestObservation, TargetRepoRunnerAdapterError>;

    fn publish_source_update(
        &mut self,
        _request: &TargetRepoRunnerSourcePublicationRequest,
    ) -> Result<TargetRepoRunnerSourcePublicationObservation, TargetRepoRunnerAdapterError> {
        Err(TargetRepoRunnerAdapterError::new(
            "source_publication",
            "adapter does not implement source publication readback",
        ))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum TargetRepoRunnerRuntimeError {
    Plan(TargetRepoRunnerPlanError),
    Adapter(TargetRepoRunnerAdapterError),
    CommandValidation {
        operation: &'static str,
        message: String,
    },
    Receipt(String),
    ReceiptProjection(String),
    SourcePublicationMismatch(String),
    ReadinessMismatch(String),
    CheckoutNotScafldReady {
        target_repo: String,
    },
    CreatedPullRequestRequired {
        target_repo: String,
    },
}

impl fmt::Display for TargetRepoRunnerRuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Plan(error) => write!(formatter, "{error}"),
            Self::Adapter(error) => write!(formatter, "{error}"),
            Self::CommandValidation { operation, message } => {
                write!(
                    formatter,
                    "target repo runner {operation} command is invalid: {message}"
                )
            }
            Self::Receipt(message) => {
                write!(formatter, "target repo runner receipt failed: {message}")
            }
            Self::ReceiptProjection(message) => {
                write!(
                    formatter,
                    "target repo runner receipt projection failed: {message}"
                )
            }
            Self::SourcePublicationMismatch(message) => {
                write!(
                    formatter,
                    "target repo runner source publication failed: {message}"
                )
            }
            Self::ReadinessMismatch(message) => formatter.write_str(message),
            Self::CheckoutNotScafldReady { target_repo } => write!(
                formatter,
                "target repo runner fixture requires scafld-ready checkout for '{target_repo}'"
            ),
            Self::CreatedPullRequestRequired { target_repo } => write!(
                formatter,
                "target repo runner fixture needs a created pull request for '{target_repo}'"
            ),
        }
    }
}

impl std::error::Error for TargetRepoRunnerRuntimeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Plan(error) => Some(error),
            Self::Adapter(error) => Some(error),
            Self::CommandValidation { .. }
            | Self::Receipt(_)
            | Self::ReceiptProjection(_)
            | Self::SourcePublicationMismatch(_)
            | Self::ReadinessMismatch(_)
            | Self::CheckoutNotScafldReady { .. }
            | Self::CreatedPullRequestRequired { .. } => None,
        }
    }
}

impl From<TargetRepoRunnerPlanError> for TargetRepoRunnerRuntimeError {
    fn from(error: TargetRepoRunnerPlanError) -> Self {
        Self::Plan(error)
    }
}

impl From<TargetRepoRunnerAdapterError> for TargetRepoRunnerRuntimeError {
    fn from(error: TargetRepoRunnerAdapterError) -> Self {
        Self::Adapter(error)
    }
}
