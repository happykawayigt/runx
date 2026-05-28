//! Pull-request mutation command + observation + readback validation. The
//! mutation command translates the execution plan into a create-or-reuse
//! intent; the readback validation enforces every invariant the adapter is
//! required to honour before the revision act can seal.

use runx_contracts::TargetRepoRunnerDedupeLookupExecution;
use runx_contracts::TargetRepoRunnerExecutionPlan;
use runx_contracts::TargetRepoRunnerPullRequestDisposition;

use super::adapter::TargetRepoRunnerRuntimeError;
use super::commands::{
    TargetRepoRunnerGitMutationObservation, TargetRepoRunnerGovernedRunnerObservation,
    TargetRepoRunnerPullRequestCreateCommand, TargetRepoRunnerPullRequestMutation,
    TargetRepoRunnerPullRequestMutationCommand, TargetRepoRunnerPullRequestObservation,
    TargetRepoRunnerPullRequestObservationRequest, TargetRepoRunnerPullRequestReuseCommand,
};
use super::provider::{github_pull_request_number, github_repository};
use super::target_repo_runner_branch_name;

pub(super) fn target_repo_runner_pull_request_observation_request(
    execution_plan: &TargetRepoRunnerExecutionPlan,
    dedupe_execution: &TargetRepoRunnerDedupeLookupExecution,
    disposition: TargetRepoRunnerPullRequestDisposition,
    runner_observation: Option<TargetRepoRunnerGovernedRunnerObservation>,
    git_mutation_observation: Option<&TargetRepoRunnerGitMutationObservation>,
) -> Result<TargetRepoRunnerPullRequestObservationRequest, TargetRepoRunnerRuntimeError> {
    let command = target_repo_runner_pull_request_mutation_command(
        execution_plan,
        dedupe_execution,
        disposition,
        runner_observation.as_ref(),
        git_mutation_observation,
    )?;
    Ok(TargetRepoRunnerPullRequestObservationRequest {
        command,
        disposition,
        target_repo: execution_plan.checkout.target_repo.clone(),
        dedupe_key: execution_plan.provider_lookup.key.clone(),
        existing_pull_request: dedupe_execution.existing_pull_request.clone(),
        runner_observation,
    })
}

// rust-style-allow: long-function - assembles the pull-request mutation command from the execution
// plan in one pass so every field of the command is mapped in a single reviewable place.
fn target_repo_runner_pull_request_mutation_command(
    execution_plan: &TargetRepoRunnerExecutionPlan,
    dedupe_execution: &TargetRepoRunnerDedupeLookupExecution,
    disposition: TargetRepoRunnerPullRequestDisposition,
    runner_observation: Option<&TargetRepoRunnerGovernedRunnerObservation>,
    git_mutation_observation: Option<&TargetRepoRunnerGitMutationObservation>,
) -> Result<TargetRepoRunnerPullRequestMutationCommand, TargetRepoRunnerRuntimeError> {
    let repository = github_repository(&execution_plan.checkout.target_repo, "pull_request")?;
    let mutation = match disposition {
        TargetRepoRunnerPullRequestDisposition::Create => {
            let observation = runner_observation.ok_or_else(|| {
                TargetRepoRunnerRuntimeError::CommandValidation {
                    operation: "pull_request",
                    message: "runner observation is required before creating a pull request"
                        .to_owned(),
                }
            })?;
            let git_observation = git_mutation_observation.ok_or_else(|| {
                TargetRepoRunnerRuntimeError::CommandValidation {
                    operation: "pull_request",
                    message: "git mutation readback is required before creating a pull request"
                        .to_owned(),
                }
            })?;
            validate_pull_request_git_readback(execution_plan, dedupe_execution, git_observation)?;
            TargetRepoRunnerPullRequestMutation::Create(TargetRepoRunnerPullRequestCreateCommand {
                title: pull_request_create_title(execution_plan),
                body: pull_request_create_body(
                    execution_plan,
                    dedupe_execution,
                    observation,
                    git_observation,
                ),
                head_branch: git_observation.branch.clone(),
                head_sha: git_observation.head_sha.clone(),
                runner_id: observation.runner_id.clone(),
                runner_summary: observation.summary.clone(),
                runner_revision_refs: observation.revision_refs.clone(),
                git_revision_refs: git_observation.revision_refs.clone(),
                artifact_refs: observation.artifact_refs.clone(),
                verification_refs: observation.verification_refs.clone(),
                git_verification_refs: git_observation.verification_refs.clone(),
            })
        }
        TargetRepoRunnerPullRequestDisposition::Reuse => {
            if git_mutation_observation.is_some() {
                return Err(TargetRepoRunnerRuntimeError::CommandValidation {
                    operation: "pull_request",
                    message:
                        "git mutation readback must not be supplied when reusing a pull request"
                            .to_owned(),
                });
            }
            let existing_pull_request =
                dedupe_execution
                    .existing_pull_request
                    .clone()
                    .ok_or_else(|| TargetRepoRunnerRuntimeError::CommandValidation {
                        operation: "pull_request",
                        message: "existing pull request is required for reuse".to_owned(),
                    })?;
            TargetRepoRunnerPullRequestMutation::Reuse(TargetRepoRunnerPullRequestReuseCommand {
                existing_pull_request,
                reason: "Provider dedupe returned a matching open pull request.".to_owned(),
            })
        }
    };

    Ok(TargetRepoRunnerPullRequestMutationCommand {
        provider: execution_plan.provider_lookup.provider,
        disposition,
        target_repo: execution_plan.checkout.target_repo.clone(),
        repository,
        target_repo_ref: execution_plan.target_repo_ref.clone(),
        base_branch: execution_plan.checkout.base_branch.clone(),
        dedupe_key: execution_plan.provider_lookup.key.clone(),
        source_issue_ref: execution_plan.source_issue_ref.clone(),
        source_thread_ref: execution_plan.source_thread_ref.clone(),
        mutation,
        human_merge_gate_required: true,
        local_path_hidden: true,
    })
}

fn pull_request_create_title(execution_plan: &TargetRepoRunnerExecutionPlan) -> String {
    format!(
        "Runx target update for {}",
        execution_plan.checkout.target_repo
    )
}

fn pull_request_create_body(
    execution_plan: &TargetRepoRunnerExecutionPlan,
    dedupe_execution: &TargetRepoRunnerDedupeLookupExecution,
    runner_observation: &TargetRepoRunnerGovernedRunnerObservation,
    git_observation: &TargetRepoRunnerGitMutationObservation,
) -> String {
    let source_issue = execution_plan
        .source_issue_ref
        .as_ref()
        .map(|reference| reference.uri.as_str())
        .unwrap_or("none");
    let markers = execution_plan
        .provider_lookup
        .query
        .markers
        .iter()
        .map(|marker| format!("- {marker}"))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "Runx target runner prepared this pull request for human review.\n\nTarget repo: {}\nHead branch: {}\nHead commit: {}\nSource thread: {}\nSource issue: {source_issue}\nDedupe key: {}\n\nDedupe markers:\n{markers}\n\nRunner: {}\n{}\n\nHuman review remains the merge gate.",
        execution_plan.checkout.target_repo,
        git_observation.branch,
        git_observation.head_sha,
        execution_plan.source_thread_ref.uri,
        dedupe_execution.key,
        runner_observation.runner_id,
        runner_observation.summary
    )
}

fn validate_pull_request_git_readback(
    execution_plan: &TargetRepoRunnerExecutionPlan,
    dedupe_execution: &TargetRepoRunnerDedupeLookupExecution,
    git_observation: &TargetRepoRunnerGitMutationObservation,
) -> Result<(), TargetRepoRunnerRuntimeError> {
    if git_observation.target_repo != execution_plan.checkout.target_repo {
        return Err(TargetRepoRunnerRuntimeError::CommandValidation {
            operation: "pull_request",
            message: "git mutation readback target repo does not match execution target".to_owned(),
        });
    }
    let expected_branch = target_repo_runner_branch_name(execution_plan, dedupe_execution);
    if git_observation.branch != expected_branch {
        return Err(TargetRepoRunnerRuntimeError::CommandValidation {
            operation: "pull_request",
            message: "git mutation readback branch does not match the dedupe branch".to_owned(),
        });
    }
    validate_branch_for_operation(&git_observation.branch, "pull_request")?;
    validate_head_sha(&git_observation.head_sha, "pull_request")
}

// rust-style-allow: long-function - pull-request readback validation checks all command invariants
// in one gate; keeping the checks together makes the accept/reject boundary auditable at a glance.
pub(super) fn validate_pull_request_readback(
    command: &TargetRepoRunnerPullRequestMutationCommand,
    observation: &TargetRepoRunnerPullRequestObservation,
) -> Result<(), TargetRepoRunnerRuntimeError> {
    if observation.provider != command.provider {
        return Err(TargetRepoRunnerRuntimeError::CommandValidation {
            operation: "pull_request",
            message: "pull request readback provider does not match command".to_owned(),
        });
    }
    if observation.target_repo != command.target_repo {
        return Err(TargetRepoRunnerRuntimeError::CommandValidation {
            operation: "pull_request",
            message: "pull request readback target repo does not match command".to_owned(),
        });
    }
    let pull_request = &observation.pull_request;
    let url_number = github_pull_request_number(&command.repository.full_name, &pull_request.url)?;
    if let Some(readback_number) = pull_request.number {
        if readback_number != url_number {
            return Err(TargetRepoRunnerRuntimeError::CommandValidation {
                operation: "pull_request",
                message: "pull request readback number does not match its URL".to_owned(),
            });
        }
    }
    if let Some(branch) = &pull_request.branch {
        validate_pull_request_branch(branch)?;
    }

    match &command.mutation {
        TargetRepoRunnerPullRequestMutation::Create(create) => {
            let Some(head_branch) = observation.head_branch.as_deref() else {
                return Err(TargetRepoRunnerRuntimeError::CommandValidation {
                    operation: "pull_request",
                    message: "created pull request readback head branch is required".to_owned(),
                });
            };
            if head_branch != create.head_branch {
                return Err(TargetRepoRunnerRuntimeError::CommandValidation {
                    operation: "pull_request",
                    message: "created pull request readback head branch does not match command"
                        .to_owned(),
                });
            }
            let Some(head_sha) = observation.head_sha.as_deref() else {
                return Err(TargetRepoRunnerRuntimeError::CommandValidation {
                    operation: "pull_request",
                    message: "created pull request readback head sha is required".to_owned(),
                });
            };
            if head_sha != create.head_sha {
                return Err(TargetRepoRunnerRuntimeError::CommandValidation {
                    operation: "pull_request",
                    message: "created pull request readback head sha does not match command"
                        .to_owned(),
                });
            }
            match pull_request.branch.as_deref() {
                Some(branch) if branch == create.head_branch => Ok(()),
                Some(_) => Err(TargetRepoRunnerRuntimeError::CommandValidation {
                    operation: "pull_request",
                    message: "created pull request branch does not match git mutation readback"
                        .to_owned(),
                }),
                None => Err(TargetRepoRunnerRuntimeError::CommandValidation {
                    operation: "pull_request",
                    message: "created pull request branch readback is required".to_owned(),
                }),
            }
        }
        TargetRepoRunnerPullRequestMutation::Reuse(reuse) => {
            if pull_request.url != reuse.existing_pull_request.url {
                return Err(TargetRepoRunnerRuntimeError::CommandValidation {
                    operation: "pull_request",
                    message: "reused pull request readback does not match provider dedupe"
                        .to_owned(),
                });
            }
            if let Some(expected_number) = reuse.existing_pull_request.number {
                if pull_request.number != Some(expected_number) {
                    return Err(TargetRepoRunnerRuntimeError::CommandValidation {
                        operation: "pull_request",
                        message: "reused pull request number does not match provider dedupe"
                            .to_owned(),
                    });
                }
            }
            if let (Some(expected_branch), Some(readback_branch)) =
                (&reuse.existing_pull_request.branch, &pull_request.branch)
            {
                if expected_branch != readback_branch {
                    return Err(TargetRepoRunnerRuntimeError::CommandValidation {
                        operation: "pull_request",
                        message: "reused pull request branch does not match provider dedupe"
                            .to_owned(),
                    });
                }
            }
            Ok(())
        }
    }
}

fn validate_pull_request_branch(branch: &str) -> Result<(), TargetRepoRunnerRuntimeError> {
    validate_branch_for_operation(branch, "pull_request")
}

pub(super) fn validate_branch_for_operation(
    branch: &str,
    operation: &'static str,
) -> Result<(), TargetRepoRunnerRuntimeError> {
    if branch.trim().is_empty()
        || branch.starts_with('/')
        || branch.ends_with('/')
        || branch.contains("..")
        || branch.chars().any(|character| {
            character.is_control() || character.is_whitespace() || character == '\\'
        })
    {
        return Err(TargetRepoRunnerRuntimeError::CommandValidation {
            operation,
            message: "git branch is not a safe branch name".to_owned(),
        });
    }
    Ok(())
}

pub(super) fn validate_head_sha(
    head_sha: &str,
    operation: &'static str,
) -> Result<(), TargetRepoRunnerRuntimeError> {
    if head_sha.len() != 40
        || !head_sha
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    {
        return Err(TargetRepoRunnerRuntimeError::CommandValidation {
            operation,
            message: "head sha must be a 40 character hex commit".to_owned(),
        });
    }
    Ok(())
}
