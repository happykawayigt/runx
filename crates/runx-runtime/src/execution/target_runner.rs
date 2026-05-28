// rust-style-allow: large-file because target-runner orchestration, mutation
// readback, receipt sealing, and public projection still share live execution
// invariants; provider HTTP is split out and the remaining slices should move
// only with receipt parity gates beside them.
//! Runtime support for target-repo runner execution.

mod adapter;
mod commands;
mod projection;
mod provider;
mod pull_request;

use std::fmt::Write as _;

use sha2::{Digest, Sha256};

pub use adapter::{
    TargetRepoRunnerAdapter, TargetRepoRunnerAdapterError, TargetRepoRunnerRuntimeError,
};
pub use commands::{
    TargetRepoRunnerCheckoutCommand, TargetRepoRunnerFixtureExecution,
    TargetRepoRunnerFixtureExecutionInput, TargetRepoRunnerGitMutationCommand,
    TargetRepoRunnerGitMutationObservation, TargetRepoRunnerGovernedRunnerInvocation,
    TargetRepoRunnerGovernedRunnerObservation, TargetRepoRunnerLiveExecution,
    TargetRepoRunnerProviderDedupeLookupCommand, TargetRepoRunnerPullRequestCreateCommand,
    TargetRepoRunnerPullRequestMutation, TargetRepoRunnerPullRequestMutationCommand,
    TargetRepoRunnerPullRequestObservation, TargetRepoRunnerPullRequestObservationRequest,
    TargetRepoRunnerPullRequestReuseCommand, TargetRepoRunnerRevisionReceiptProjection,
    TargetRepoRunnerSourcePublicationCommand, TargetRepoRunnerSourcePublicationObservation,
    TargetRepoRunnerSourcePublicationProjection, TargetRepoRunnerSourcePublicationRequest,
};
pub use projection::{
    project_target_repo_runner_revision_receipt,
    project_target_repo_runner_source_publication_receipt,
};

use runx_contracts::{
    ActForm, AuthorityAttenuation, AuthoritySubsetProof, AuthoritySubsetResult, ChangePlan,
    ChangeRequest, Closure, ClosureDisposition, CriterionBinding, CriterionStatus, Intent,
    JsonNumber, JsonObject, JsonValue, Lineage, RECEIPT_CANONICALIZATION, Receipt, ReceiptAct,
    ReceiptAuthority, ReceiptEnforcement, ReceiptIdempotency, ReceiptSchema, Reference,
    ReferenceType, RevisionDetails, Seal, SignatureAlgorithm, Subject, SuccessCriterion,
    TargetRepoRunnerDedupeLookupExecution, TargetRepoRunnerDedupeLookupObservation,
    TargetRepoRunnerDedupeLookupPlan, TargetRepoRunnerDedupeResult, TargetRepoRunnerExecutionPlan,
    TargetRepoRunnerExistingPullRequest, TargetRepoRunnerPlan, TargetRepoRunnerPlanError,
    TargetRepoRunnerProviderPullRequest, TargetRepoRunnerPullRequestDisposition,
    TargetRepoRunnerReadinessObservation, TargetRepoRunnerSourcePublicationReceiptPlan,
    TargetSurface, apply_target_repo_runner_dedupe_lookup_execution,
    execute_target_repo_runner_dedupe_lookup, plan_target_repo_runner_execution,
    plan_target_repo_runner_pull_request_receipt,
    plan_target_repo_runner_source_publication_receipt,
};
use runx_contracts::{operational_policy_source_provider, receipt_subject_kind};
use runx_receipts::{
    canonical_receipt_body_digest, content_addressed_receipt_id, validate_receipt,
};

use crate::receipts::local_target_runner_issuer;
use crate::reference_match::same_reference;
pub use provider::{
    TargetRepoRunnerDefaultHttpTransport, TargetRepoRunnerGithubApiClient,
    TargetRepoRunnerGithubPullRequestSearchCommand, TargetRepoRunnerGithubPullRequestSearchState,
    TargetRepoRunnerGithubRepository, TargetRepoRunnerHttpError, TargetRepoRunnerHttpHeader,
    TargetRepoRunnerHttpMethod, TargetRepoRunnerHttpRequest, TargetRepoRunnerHttpResponse,
    TargetRepoRunnerHttpTransport,
};
use provider::{github_repository, github_search_exact_term, validate_provider_lookup_term};

pub fn target_repo_runner_checkout_command(
    plan: &TargetRepoRunnerPlan,
) -> Result<TargetRepoRunnerCheckoutCommand, TargetRepoRunnerRuntimeError> {
    let repository = github_repository(&plan.target.repo, "checkout")?;
    if repository.full_name != plan.target.repo {
        return Err(TargetRepoRunnerRuntimeError::CommandValidation {
            operation: "checkout",
            message: "target repo must be a canonical github owner/repo".to_owned(),
        });
    }
    if !plan.mutate_target_repo {
        return Err(TargetRepoRunnerRuntimeError::CommandValidation {
            operation: "checkout",
            message: "target repo mutation must be admitted before checkout".to_owned(),
        });
    }
    if !plan.require_human_merge_gate {
        return Err(TargetRepoRunnerRuntimeError::CommandValidation {
            operation: "checkout",
            message: "target runner requires a human merge gate".to_owned(),
        });
    }

    Ok(TargetRepoRunnerCheckoutCommand {
        target_repo: plan.target.repo.clone(),
        public_repo_ref: Reference {
            reference_type: ReferenceType::GithubRepo,
            uri: format!("https://github.com/{}", plan.target.repo).into(),
            provider: Some(operational_policy_source_provider::GITHUB.into()),
            locator: Some(plan.target.repo.clone().into()),
            label: Some("target repo".to_owned().into()),
            observed_at: None,
            proof_kind: None,
        },
        base_branch: plan.target.base_branch.clone(),
        runner_id: plan.runner.runner_id.clone(),
        runner_kind: plan.runner.kind.clone(),
        target_scafld_required: plan.target.scafld_required,
        runner_scafld_required: plan.runner.scafld_required,
        mutate_target_repo: plan.mutate_target_repo,
        local_path_hidden: true,
    })
}

pub fn target_repo_runner_provider_dedupe_lookup_command(
    lookup: &TargetRepoRunnerDedupeLookupPlan,
) -> Result<TargetRepoRunnerProviderDedupeLookupCommand, TargetRepoRunnerRuntimeError> {
    let repository = github_repository(&lookup.target_repo, "provider_dedupe_lookup")?;
    validate_provider_dedupe_lookup(lookup)?;
    let terms = provider_dedupe_lookup_terms(&repository, lookup);
    let query = terms.join(" ");

    Ok(TargetRepoRunnerProviderDedupeLookupCommand {
        provider: lookup.provider,
        target_repo: lookup.target_repo.clone(),
        repository,
        dedupe_key: lookup.key.clone(),
        result_limit: lookup.query.result_limit,
        query: TargetRepoRunnerGithubPullRequestSearchCommand {
            repo: lookup.target_repo.clone(),
            state: TargetRepoRunnerGithubPullRequestSearchState::Open,
            query,
            terms,
        },
        markers: lookup.query.markers.clone(),
        required_refs: lookup.query.required_refs.clone(),
    })
}

fn validate_provider_dedupe_lookup(
    lookup: &TargetRepoRunnerDedupeLookupPlan,
) -> Result<(), TargetRepoRunnerRuntimeError> {
    if lookup.query.result_limit == 0 {
        return Err(TargetRepoRunnerRuntimeError::CommandValidation {
            operation: "provider_dedupe_lookup",
            message: "provider lookup result limit must be greater than zero".to_owned(),
        });
    }
    if lookup.query.markers.is_empty() {
        return Err(TargetRepoRunnerRuntimeError::CommandValidation {
            operation: "provider_dedupe_lookup",
            message: "provider lookup requires at least one dedupe marker".to_owned(),
        });
    }
    if lookup.query.required_refs.is_empty() {
        return Err(TargetRepoRunnerRuntimeError::CommandValidation {
            operation: "provider_dedupe_lookup",
            message: "provider lookup requires source references".to_owned(),
        });
    }
    for marker in &lookup.query.markers {
        validate_provider_lookup_term(marker, "marker")?;
    }
    for reference in &lookup.query.required_refs {
        validate_provider_lookup_term(&reference.uri, "source reference")?;
    }
    Ok(())
}

fn provider_dedupe_lookup_terms(
    repository: &TargetRepoRunnerGithubRepository,
    lookup: &TargetRepoRunnerDedupeLookupPlan,
) -> Vec<String> {
    let mut terms = vec![
        format!("repo:{}", repository.full_name),
        "is:pr".to_owned(),
        "is:open".to_owned(),
    ];
    terms.extend(
        lookup
            .query
            .markers
            .iter()
            .map(|marker| github_search_exact_term(marker)),
    );
    terms.extend(
        lookup
            .query
            .required_refs
            .iter()
            .map(|reference| github_search_exact_term(&reference.uri)),
    );
    terms
}

pub fn target_repo_runner_provider_dedupe_observation_from_pull_requests(
    command: &TargetRepoRunnerProviderDedupeLookupCommand,
    pull_requests: Vec<TargetRepoRunnerProviderPullRequest>,
) -> Result<TargetRepoRunnerDedupeLookupObservation, TargetRepoRunnerRuntimeError> {
    if pull_requests.len() > usize::from(command.result_limit) {
        return Err(TargetRepoRunnerRuntimeError::CommandValidation {
            operation: "provider_dedupe_lookup",
            message: "provider lookup readback exceeded the command result limit".to_owned(),
        });
    }
    Ok(TargetRepoRunnerDedupeLookupObservation {
        provider: command.provider,
        target_repo: command.target_repo.clone(),
        key: command.dedupe_key.clone(),
        pull_requests,
    })
}

// rust-style-allow: long-function because this is the live target-runner
// orchestration boundary: readiness, dedupe, mutation observation, revision
// seal, and source publication must stay visibly ordered.
pub fn execute_target_repo_runner_with_adapter<A: TargetRepoRunnerAdapter>(
    plan: &TargetRepoRunnerPlan,
    adapter: &mut A,
    created_at: &str,
) -> Result<TargetRepoRunnerLiveExecution, TargetRepoRunnerRuntimeError> {
    let checkout_command = target_repo_runner_checkout_command(plan)?;
    let readiness = adapter.checkout_readiness(&checkout_command)?;
    let execution_plan = plan_target_repo_runner_execution(plan, &readiness)?;
    let provider_lookup_command =
        target_repo_runner_provider_dedupe_lookup_command(&execution_plan.provider_lookup)?;
    let dedupe_observation = adapter.provider_dedupe_lookup(&provider_lookup_command)?;
    validate_provider_dedupe_lookup_observation(&provider_lookup_command, &dedupe_observation)?;
    let dedupe_execution = execute_target_repo_runner_dedupe_lookup(
        &execution_plan.provider_lookup,
        &dedupe_observation,
    )?;
    let deduped_plan = apply_target_repo_runner_dedupe_lookup_execution(plan, &dedupe_execution)?;
    let disposition = if dedupe_execution.result == TargetRepoRunnerDedupeResult::Reused {
        TargetRepoRunnerPullRequestDisposition::Reuse
    } else {
        TargetRepoRunnerPullRequestDisposition::Create
    };
    let runner_observation = if disposition == TargetRepoRunnerPullRequestDisposition::Create {
        Some(
            adapter.invoke_governed_runner(&TargetRepoRunnerGovernedRunnerInvocation {
                execution_plan: execution_plan.clone(),
                deduped_plan: deduped_plan.clone(),
                disposition,
            })?,
        )
    } else {
        None
    };
    let git_mutation_command = runner_observation
        .as_ref()
        .map(|observation| {
            target_repo_runner_git_mutation_command(&execution_plan, &dedupe_execution, observation)
        })
        .transpose()?;
    let git_mutation_observation = git_mutation_command
        .as_ref()
        .map(|command| -> Result<_, TargetRepoRunnerRuntimeError> {
            let observation = adapter.apply_git_mutation(command)?;
            validate_git_mutation_readback(command, &observation)?;
            Ok(observation)
        })
        .transpose()?;
    let pull_request_request = target_repo_runner_pull_request_observation_request(
        &execution_plan,
        &dedupe_execution,
        disposition,
        runner_observation.clone(),
        git_mutation_observation.as_ref(),
    )?;
    let pull_request_observation = adapter.observe_pull_request(&pull_request_request)?;
    validate_pull_request_readback(&pull_request_request.command, &pull_request_observation)?;
    let pull_request = pull_request_observation.pull_request.clone();

    let execution = execute_target_repo_runner_execution_fixture(
        plan,
        &execution_plan,
        &readiness,
        &dedupe_observation,
        Some(&pull_request),
    )?;
    let revision_receipt =
        target_repo_runner_revision_receipt(&execution, runner_observation.as_ref(), created_at)?;
    let revision_projection = project_target_repo_runner_revision_receipt(&revision_receipt)?;
    let source_publication_request = target_repo_runner_source_publication_request(
        &execution,
        &revision_receipt,
        &revision_projection,
    );
    let source_publication_observation =
        adapter.publish_source_update(&source_publication_request)?;
    let source_publication_receipt = target_repo_runner_source_publication_receipt_node(
        &source_publication_request,
        &source_publication_observation,
        created_at,
    )?;
    let source_publication_projection =
        project_target_repo_runner_source_publication_receipt(&source_publication_receipt)?;

    Ok(TargetRepoRunnerLiveExecution {
        checkout_command,
        readiness,
        provider_lookup_command,
        dedupe_observation,
        runner_observation,
        git_mutation_command,
        git_mutation_observation,
        pull_request_request,
        pull_request_observation,
        execution,
        revision_receipt,
        revision_projection,
        source_publication_request,
        source_publication_observation,
        source_publication_receipt,
        source_publication_projection,
    })
}

pub fn execute_target_repo_runner_fixture(
    input: TargetRepoRunnerFixtureExecutionInput,
) -> Result<TargetRepoRunnerFixtureExecution, TargetRepoRunnerRuntimeError> {
    let execution_plan = plan_target_repo_runner_execution(&input.plan, &input.readiness)?;
    execute_target_repo_runner_execution_fixture(
        &input.plan,
        &execution_plan,
        &input.readiness,
        &input.dedupe,
        input.created_pull_request.as_ref(),
    )
}

pub fn execute_target_repo_runner_execution_fixture(
    plan: &TargetRepoRunnerPlan,
    execution_plan: &TargetRepoRunnerExecutionPlan,
    readiness: &TargetRepoRunnerReadinessObservation,
    dedupe_observation: &TargetRepoRunnerDedupeLookupObservation,
    created_pull_request: Option<&TargetRepoRunnerExistingPullRequest>,
) -> Result<TargetRepoRunnerFixtureExecution, TargetRepoRunnerRuntimeError> {
    validate_readiness_boundary(execution_plan, readiness)?;
    if execution_plan.readiness.target_scafld_required && !execution_plan.readiness.scafld_ready {
        return Err(TargetRepoRunnerRuntimeError::CheckoutNotScafldReady {
            target_repo: execution_plan.checkout.target_repo.clone(),
        });
    }
    if execution_plan.readiness.runner_scafld_required && !readiness.scafld_ready {
        return Err(TargetRepoRunnerRuntimeError::CheckoutNotScafldReady {
            target_repo: execution_plan.checkout.target_repo.clone(),
        });
    }

    let dedupe_execution = execute_target_repo_runner_dedupe_lookup(
        &execution_plan.provider_lookup,
        dedupe_observation,
    )?;
    let deduped_plan = apply_target_repo_runner_dedupe_lookup_execution(plan, &dedupe_execution)?;
    let disposition = if dedupe_execution.result == TargetRepoRunnerDedupeResult::Reused {
        TargetRepoRunnerPullRequestDisposition::Reuse
    } else {
        TargetRepoRunnerPullRequestDisposition::Create
    };
    let pull_request = match disposition {
        TargetRepoRunnerPullRequestDisposition::Reuse => {
            dedupe_execution.existing_pull_request.clone().ok_or(
                TargetRepoRunnerRuntimeError::Plan(TargetRepoRunnerPlanError::PullRequestRequired),
            )?
        }
        TargetRepoRunnerPullRequestDisposition::Create => {
            created_pull_request.cloned().ok_or_else(|| {
                TargetRepoRunnerRuntimeError::CreatedPullRequestRequired {
                    target_repo: execution_plan.checkout.target_repo.clone(),
                }
            })?
        }
    };

    let pull_request_receipt =
        plan_target_repo_runner_pull_request_receipt(&deduped_plan, Some(&pull_request))?;
    let source_publication_receipt =
        plan_target_repo_runner_source_publication_receipt(&deduped_plan, &pull_request);

    Ok(TargetRepoRunnerFixtureExecution {
        execution_plan: execution_plan.clone(),
        dedupe_execution,
        deduped_plan,
        disposition,
        pull_request,
        pull_request_receipt,
        source_publication_receipt,
    })
}

fn target_repo_runner_git_mutation_command(
    execution_plan: &TargetRepoRunnerExecutionPlan,
    dedupe_execution: &TargetRepoRunnerDedupeLookupExecution,
    runner_observation: &TargetRepoRunnerGovernedRunnerObservation,
) -> Result<TargetRepoRunnerGitMutationCommand, TargetRepoRunnerRuntimeError> {
    if runner_observation.target_repo != execution_plan.checkout.target_repo {
        return Err(TargetRepoRunnerRuntimeError::CommandValidation {
            operation: "git_mutation",
            message: "runner observation target repo does not match execution target".to_owned(),
        });
    }
    let repository = github_repository(&execution_plan.checkout.target_repo, "git_mutation")?;
    let branch = target_repo_runner_branch_name(execution_plan, dedupe_execution);
    validate_branch_for_operation(&branch, "git_mutation")?;
    Ok(TargetRepoRunnerGitMutationCommand {
        provider: execution_plan.provider_lookup.provider,
        target_repo: execution_plan.checkout.target_repo.clone(),
        repository,
        target_repo_ref: execution_plan.target_repo_ref.clone(),
        base_branch: execution_plan.checkout.base_branch.clone(),
        branch,
        dedupe_key: execution_plan.provider_lookup.key.clone(),
        source_issue_ref: execution_plan.source_issue_ref.clone(),
        source_thread_ref: execution_plan.source_thread_ref.clone(),
        runner_id: runner_observation.runner_id.clone(),
        runner_summary: runner_observation.summary.clone(),
        runner_revision_refs: runner_observation.revision_refs.clone(),
        artifact_refs: runner_observation.artifact_refs.clone(),
        verification_refs: runner_observation.verification_refs.clone(),
        human_merge_gate_required: true,
        local_path_hidden: true,
    })
}

pub(super) fn target_repo_runner_branch_name(
    execution_plan: &TargetRepoRunnerExecutionPlan,
    dedupe_execution: &TargetRepoRunnerDedupeLookupExecution,
) -> String {
    format!(
        "runx/{}/{}",
        safe_id(&execution_plan.checkout.target_repo),
        short_key_hash(&dedupe_execution.key)
    )
}

fn short_key_hash(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    let mut hex = String::with_capacity(12);
    for byte in digest.iter().take(6) {
        let _ = write!(&mut hex, "{byte:02x}");
    }
    hex
}

fn validate_git_mutation_readback(
    command: &TargetRepoRunnerGitMutationCommand,
    observation: &TargetRepoRunnerGitMutationObservation,
) -> Result<(), TargetRepoRunnerRuntimeError> {
    if observation.target_repo != command.target_repo {
        return Err(TargetRepoRunnerRuntimeError::CommandValidation {
            operation: "git_mutation",
            message: "git mutation readback target repo does not match command".to_owned(),
        });
    }
    if observation.branch != command.branch {
        return Err(TargetRepoRunnerRuntimeError::CommandValidation {
            operation: "git_mutation",
            message: "git mutation readback branch does not match command".to_owned(),
        });
    }
    validate_branch_for_operation(&observation.branch, "git_mutation")?;
    validate_head_sha(&observation.head_sha, "git_mutation")?;
    Ok(())
}

fn validate_readiness_boundary(
    execution_plan: &TargetRepoRunnerExecutionPlan,
    readiness: &TargetRepoRunnerReadinessObservation,
) -> Result<(), TargetRepoRunnerRuntimeError> {
    if readiness.target_repo != execution_plan.checkout.target_repo {
        return Err(TargetRepoRunnerRuntimeError::ReadinessMismatch(format!(
            "readiness target '{}' does not match execution target '{}'",
            readiness.target_repo, execution_plan.checkout.target_repo
        )));
    }
    if readiness.runner_id != execution_plan.readiness.runner_id {
        return Err(TargetRepoRunnerRuntimeError::ReadinessMismatch(format!(
            "readiness runner '{}' does not match execution runner '{}'",
            readiness.runner_id, execution_plan.readiness.runner_id
        )));
    }
    if readiness.scafld_ready != execution_plan.readiness.scafld_ready {
        return Err(TargetRepoRunnerRuntimeError::ReadinessMismatch(
            "readiness observation changed after execution planning".to_owned(),
        ));
    }
    Ok(())
}

fn validate_provider_dedupe_lookup_observation(
    command: &TargetRepoRunnerProviderDedupeLookupCommand,
    observation: &TargetRepoRunnerDedupeLookupObservation,
) -> Result<(), TargetRepoRunnerRuntimeError> {
    if observation.provider != command.provider {
        return Err(TargetRepoRunnerRuntimeError::CommandValidation {
            operation: "provider_dedupe_lookup",
            message: "provider lookup readback provider does not match command".to_owned(),
        });
    }
    if observation.target_repo != command.target_repo {
        return Err(TargetRepoRunnerRuntimeError::CommandValidation {
            operation: "provider_dedupe_lookup",
            message: "provider lookup readback target repo does not match command".to_owned(),
        });
    }
    if observation.key != command.dedupe_key {
        return Err(TargetRepoRunnerRuntimeError::CommandValidation {
            operation: "provider_dedupe_lookup",
            message: "provider lookup readback dedupe key does not match command".to_owned(),
        });
    }
    if observation.pull_requests.len() > usize::from(command.result_limit) {
        return Err(TargetRepoRunnerRuntimeError::CommandValidation {
            operation: "provider_dedupe_lookup",
            message: "provider lookup readback exceeded the command result limit".to_owned(),
        });
    }
    Ok(())
}

use pull_request::{
    target_repo_runner_pull_request_observation_request, validate_branch_for_operation,
    validate_head_sha, validate_pull_request_readback,
};

// rust-style-allow: long-function because revision receipt assembly must keep
// the act, seal, metadata, and signature hash in one auditable construction.
fn target_repo_runner_revision_receipt(
    execution: &TargetRepoRunnerFixtureExecution,
    runner_observation: Option<&TargetRepoRunnerGovernedRunnerObservation>,
    created_at: &str,
) -> Result<Receipt, TargetRepoRunnerRuntimeError> {
    let pull_request_ref = execution
        .pull_request_receipt
        .pull_request_ref
        .clone()
        .ok_or_else(|| {
            TargetRepoRunnerRuntimeError::Receipt("pull request ref is required".to_owned())
        })?;
    let act_id = "act_target_runner_pull_request";
    let criterion_id = "target_runner.pull_request_ready";
    let disposition_name = disposition_name(execution.disposition);
    let target_repo_ref = execution.pull_request_receipt.target_repo_ref.clone();
    let mut evidence_refs = vec![
        execution.pull_request_receipt.source_thread_ref.clone(),
        target_repo_ref.clone(),
        pull_request_ref.clone(),
    ];
    if let Some(source_issue_ref) = &execution.pull_request_receipt.source_issue_ref {
        evidence_refs.push(source_issue_ref.clone());
    }
    let artifact_refs =
        runner_observation.map_or_else(Vec::new, |observation| observation.artifact_refs.clone());
    let verification_refs = runner_observation.map_or_else(Vec::new, |observation| {
        observation.verification_refs.clone()
    });
    let summary = format!(
        "Target runner {disposition_name} pull request {} for {}.",
        pull_request_ref.uri, execution.execution_plan.checkout.target_repo
    );
    let reason_code = format!("target_runner_pr_{}", disposition_name);
    let act = revision_act(RevisionActInput {
        act_id,
        criterion_id,
        summary: &summary,
        created_at,
        target_repo_ref: &target_repo_ref,
        source_thread_ref: &execution.pull_request_receipt.source_thread_ref,
        source_issue_ref: execution.pull_request_receipt.source_issue_ref.as_ref(),
        pull_request_ref: &pull_request_ref,
        artifact_refs: &artifact_refs,
        verification_refs: &verification_refs,
    });
    let seal = receipt_seal(ReceiptSealInputs {
        reason_code: &reason_code,
        summary: &summary,
        created_at,
        act_id,
        criterion_id,
        evidence_refs: &evidence_refs,
        verification_refs: &verification_refs,
        artifact_refs: &artifact_refs,
    });
    let receipt_id = format!(
        "hrn_rcpt_target_runner_{}_{}",
        safe_id(&execution.execution_plan.checkout.target_repo),
        pull_request_id_fragment(&execution.pull_request)
    );
    let mut receipt = Receipt {
        schema: ReceiptSchema::V1,
        id: receipt_id.clone().into(),
        created_at: created_at.into(),
        canonicalization: RECEIPT_CANONICALIZATION.into(),
        issuer: local_target_runner_issuer(),
        signature: runx_contracts::ReceiptSignature {
            alg: SignatureAlgorithm::Ed25519,
            value: "sig:pending".into(),
        },
        digest: "sha256:pending".into(),
        idempotency: ReceiptIdempotency {
            intent_key: stable_hash(&format!(
                "target-runner:{}:{}",
                execution.execution_plan.checkout.target_repo, execution.dedupe_execution.key
            ))
            .into(),
            trigger_fingerprint: stable_hash(&execution.dedupe_execution.key).into(),
            content_hash: stable_hash(&receipt_id).into(),
        },
        subject: Subject {
            kind: receipt_subject_kind::SKILL.into(),
            reference: Reference::runx(ReferenceType::Harness, "target-runner"),
            input_context: None,
            commitments: Vec::new(),
        },
        authority: target_runner_authority(execution),
        signals: vec![execution.execution_plan.source_thread_ref.clone()],
        decisions: Vec::new(),
        acts: vec![act],
        seal,
        lineage: Some(Lineage::default()),
        metadata: Some(revision_receipt_metadata(execution)),
    };
    seal_revision_receipt(&mut receipt)?;
    Ok(receipt)
}

fn target_repo_runner_source_publication_request(
    execution: &TargetRepoRunnerFixtureExecution,
    revision_receipt: &Receipt,
    revision_projection: &TargetRepoRunnerRevisionReceiptProjection,
) -> TargetRepoRunnerSourcePublicationRequest {
    let publication = execution.source_publication_receipt.clone();
    let revision_receipt_ref = Reference::runx(ReferenceType::Receipt, &revision_receipt.id);
    let commands = source_publication_commands(&publication, &revision_receipt_ref);
    TargetRepoRunnerSourcePublicationRequest {
        publication,
        revision_receipt_ref,
        revision_projection: revision_projection.clone(),
        commands,
    }
}

fn source_publication_commands(
    publication: &TargetRepoRunnerSourcePublicationReceiptPlan,
    revision_receipt_ref: &Reference,
) -> Vec<TargetRepoRunnerSourcePublicationCommand> {
    let body = source_publication_body(publication, revision_receipt_ref);
    let mut commands = Vec::new();
    if let Some(source_issue_ref) = &publication.source_issue_ref {
        commands.push(
            TargetRepoRunnerSourcePublicationCommand::SourceIssueComment {
                target: source_issue_ref.clone(),
                body: body.clone(),
            },
        );
    }
    commands.push(
        TargetRepoRunnerSourcePublicationCommand::SourceThreadReply {
            target: publication.source_thread_ref.clone(),
            body,
        },
    );
    commands
}

fn source_publication_body(
    publication: &TargetRepoRunnerSourcePublicationReceiptPlan,
    revision_receipt_ref: &Reference,
) -> String {
    let target_repo = metadata_path_string(&publication.metadata, &["target_repo"])
        .or(publication.pull_request_ref.locator.as_deref())
        .unwrap_or("target repo");
    let dedupe_result =
        metadata_path_string(&publication.metadata, &["dedupe", "result"]).unwrap_or("unknown");
    let dedupe_key =
        metadata_path_string(&publication.metadata, &["dedupe", "key"]).unwrap_or("unknown");
    format!(
        "Target pull request ready: {}\nTarget repo: {target_repo}\nDedupe: {dedupe_result} ({dedupe_key})\nHuman review remains the merge gate.\nReceipt: {}",
        publication.pull_request_ref.uri, revision_receipt_ref.uri
    )
}

// rust-style-allow: long-function because source publication receipt assembly
// keeps the reply act, criteria, seal, metadata, and signature hash together.
fn target_repo_runner_source_publication_receipt_node(
    request: &TargetRepoRunnerSourcePublicationRequest,
    observation: &TargetRepoRunnerSourcePublicationObservation,
    created_at: &str,
) -> Result<Receipt, TargetRepoRunnerRuntimeError> {
    validate_source_publication_observation(request, observation)?;

    let act_id = "act_target_runner_source_publication";
    let criterion_id = "target_runner.source_publication_published";
    let target_refs = source_publication_target_refs(observation);
    let mut evidence_refs = vec![
        observation.pull_request_ref.clone(),
        observation.revision_receipt_ref.clone(),
    ];
    evidence_refs.extend(target_refs.clone());
    evidence_refs.extend(observation.published_refs.clone());
    let summary = format!(
        "Published target pull request {} to the source issue/thread.",
        observation.pull_request_ref.uri
    );
    // Role refs the projection reconstructs: PR (source role) + thread/issue
    // (target role) ride on the act's artifact_refs alongside published refs.
    let mut role_refs = vec![observation.pull_request_ref.clone()];
    role_refs.extend(target_refs.clone());
    role_refs.extend(observation.published_refs.clone());
    let success_criteria = vec![SuccessCriterion {
        criterion_id: criterion_id.into(),
        statement: "Target pull request is published to the source issue/thread".into(),
        required: true,
    }];
    let act = ReceiptAct {
        id: act_id.into(),
        form: ActForm::Reply,
        intent: Intent {
            purpose: format!(
                "Publish target pull request {} to source",
                observation.pull_request_ref.uri
            )
            .into(),
            legitimacy: "Target runner is authorized to reply on the source issue/thread".into(),
            success_criteria,
            constraints: Vec::new(),
            derived_from: vec![observation.source_thread_ref.clone()],
        },
        summary: summary.clone().into(),
        criterion_bindings: vec![CriterionBinding {
            criterion_id: criterion_id.into(),
            status: CriterionStatus::Verified,
            evidence_refs: evidence_refs.clone(),
            verification_refs: Vec::new(),
            summary: Some(summary.clone().into()),
        }],
        by: None,
        source_refs: vec![observation.source_thread_ref.clone()],
        target_refs: target_refs.clone(),
        artifact_refs: role_refs,
        context_ref: Some(Reference::runx(
            ReferenceType::Act,
            &format!("{act_id}_context"),
        )),
        closure: Closure {
            disposition: ClosureDisposition::Closed,
            reason_code: criterion_id.into(),
            summary: summary.clone().into(),
            closed_at: created_at.into(),
        },
        revision: None,
        verification: None,
    };
    let seal = receipt_seal(ReceiptSealInputs {
        reason_code: "target_runner_source_published",
        summary: &summary,
        created_at,
        act_id,
        criterion_id,
        evidence_refs: &evidence_refs,
        verification_refs: &[],
        artifact_refs: &observation.published_refs,
    });
    let target_repo =
        metadata_path_string(&request.publication.metadata, &["target_repo"]).unwrap_or("target");
    let receipt_id = format!(
        "hrn_rcpt_target_runner_source_publication_{}_{}",
        safe_id(target_repo),
        reference_id_fragment(&observation.pull_request_ref)
    );
    let mut receipt = Receipt {
        schema: ReceiptSchema::V1,
        id: receipt_id.clone().into(),
        created_at: created_at.into(),
        canonicalization: RECEIPT_CANONICALIZATION.into(),
        issuer: local_target_runner_issuer(),
        signature: runx_contracts::ReceiptSignature {
            alg: SignatureAlgorithm::Ed25519,
            value: "sig:pending".into(),
        },
        digest: "sha256:pending".into(),
        idempotency: ReceiptIdempotency {
            intent_key: stable_hash(&format!(
                "target-runner-source-publication:{}:{}",
                observation.source_thread_ref.uri, observation.pull_request_ref.uri
            ))
            .into(),
            trigger_fingerprint: stable_hash(&observation.pull_request_ref.uri).into(),
            content_hash: stable_hash(&receipt_id).into(),
        },
        subject: Subject {
            kind: receipt_subject_kind::SKILL.into(),
            reference: Reference::runx(ReferenceType::Harness, "target-runner-source-publication"),
            input_context: None,
            commitments: Vec::new(),
        },
        authority: source_publication_authority(request, created_at),
        signals: vec![observation.source_thread_ref.clone()],
        decisions: Vec::new(),
        acts: vec![act],
        seal,
        lineage: Some(Lineage {
            parent: Some(request.revision_receipt_ref.clone()),
            previous: Some(request.revision_receipt_ref.clone()),
            ..Lineage::default()
        }),
        metadata: Some(source_publication_receipt_metadata(request, observation)),
    };
    seal_revision_receipt(&mut receipt)?;
    Ok(receipt)
}

struct RevisionActInput<'a> {
    act_id: &'a str,
    criterion_id: &'a str,
    summary: &'a str,
    created_at: &'a str,
    target_repo_ref: &'a Reference,
    source_thread_ref: &'a Reference,
    source_issue_ref: Option<&'a Reference>,
    pull_request_ref: &'a Reference,
    artifact_refs: &'a [Reference],
    verification_refs: &'a [Reference],
}

// The act keeps its full intent, success criteria, criterion bindings, and the
// revision body inline (proof + training signal). The bulky agent-context I/O is
// referenced via `context_ref`. The role refs the projection needs
// (repo/PR/thread/issue) ride on the act's target/artifact refs.
// rust-style-allow: long-function - one cohesive ReceiptAct assembly (intent,
// criteria, bindings, refs, closure); the bulky change-set is already extracted
// and splitting the rest would scatter the receipt shape across helpers.
fn revision_act(input: RevisionActInput<'_>) -> ReceiptAct {
    let RevisionActInput {
        act_id,
        criterion_id,
        summary,
        created_at,
        target_repo_ref,
        source_thread_ref,
        source_issue_ref,
        pull_request_ref,
        artifact_refs,
        verification_refs,
    } = input;
    let target_refs = vec![target_repo_ref.clone(), pull_request_ref.clone()];
    let mut role_refs = vec![
        target_repo_ref.clone(),
        pull_request_ref.clone(),
        source_thread_ref.clone(),
    ];
    if let Some(source_issue_ref) = source_issue_ref {
        role_refs.push(source_issue_ref.clone());
    }
    role_refs.extend(artifact_refs.iter().cloned());
    let success_criteria = vec![SuccessCriterion {
        criterion_id: criterion_id.into(),
        statement: "Target pull request is ready for human review".into(),
        required: true,
    }];
    let revision = revision_change_set(act_id, summary, pull_request_ref, success_criteria.clone());
    ReceiptAct {
        id: act_id.into(),
        form: ActForm::Revision,
        intent: Intent {
            purpose: format!("Open target pull request {}", pull_request_ref.uri).into(),
            legitimacy: "Target runner is authorized to open the target pull request".into(),
            success_criteria,
            constraints: Vec::new(),
            derived_from: vec![source_thread_ref.clone()],
        },
        summary: summary.into(),
        criterion_bindings: vec![CriterionBinding {
            criterion_id: criterion_id.into(),
            status: CriterionStatus::Verified,
            evidence_refs: target_refs.clone(),
            verification_refs: verification_refs.to_vec(),
            summary: Some(summary.into()),
        }],
        by: None,
        source_refs: vec![source_thread_ref.clone()],
        target_refs,
        artifact_refs: role_refs,
        context_ref: Some(Reference::runx(
            ReferenceType::Act,
            &format!("{act_id}_context"),
        )),
        closure: Closure {
            disposition: ClosureDisposition::Closed,
            reason_code: criterion_id.into(),
            summary: summary.into(),
            closed_at: created_at.into(),
        },
        revision: Some(revision),
        verification: None,
    }
}

/// Builds the change-set (request + plan) carried by a revision act. Kept as a
/// helper so `revision_act` stays a single readable assembly of the act shape.
fn revision_change_set(
    act_id: &str,
    summary: &str,
    pull_request_ref: &Reference,
    success_criteria: Vec<SuccessCriterion>,
) -> RevisionDetails {
    RevisionDetails {
        change_request: ChangeRequest {
            request_id: format!("{act_id}_request").into(),
            summary: summary.into(),
            target_surfaces: vec![TargetSurface {
                surface_ref: pull_request_ref.clone(),
                mutating: true,
                rationale: Some("Open the target pull request".into()),
            }],
            success_criteria,
        },
        change_plan: ChangePlan {
            plan_id: format!("{act_id}_plan").into(),
            summary: summary.into(),
            steps: vec!["Prepare and publish the target pull request".into()],
            risks: Vec::new(),
        },
        target_surfaces: Vec::new(),
        invariants: Vec::new(),
        verification: None,
        handoff_refs: Vec::new(),
        revision_refs: Vec::new(),
    }
}

fn target_runner_authority(execution: &TargetRepoRunnerFixtureExecution) -> ReceiptAuthority {
    ReceiptAuthority {
        actor_ref: Reference::runx(ReferenceType::Principal, "target_runner"),
        authority_proof_refs: Vec::new(),
        grant_refs: Vec::new(),
        scope_refs: Vec::new(),
        terms: Vec::new(),
        attenuation: AuthorityAttenuation {
            parent_authority_ref: None,
            subset_proof: None,
        },
        mandate_ref: None,
        enforcement: ReceiptEnforcement {
            profile_hash: stable_hash(&format!(
                "target-runner:{}",
                execution.execution_plan.checkout.target_repo
            ))
            .into(),
            redaction_refs: Vec::new(),
            setup_refs: Vec::new(),
            teardown_refs: Vec::new(),
        },
    }
}

fn source_publication_authority(
    request: &TargetRepoRunnerSourcePublicationRequest,
    created_at: &str,
) -> ReceiptAuthority {
    let target_repo =
        metadata_path_string(&request.publication.metadata, &["target_repo"]).unwrap_or("target");
    ReceiptAuthority {
        actor_ref: Reference::runx(ReferenceType::Principal, "target_runner_source_publication"),
        authority_proof_refs: Vec::new(),
        grant_refs: Vec::new(),
        scope_refs: Vec::new(),
        terms: Vec::new(),
        attenuation: AuthorityAttenuation {
            parent_authority_ref: Some(request.revision_receipt_ref.clone()),
            subset_proof: Some(AuthoritySubsetProof {
                parent_authority_ref: request.revision_receipt_ref.clone(),
                comparison_algorithm: "runx.target-runner.publication-subset.v1".into(),
                result: AuthoritySubsetResult::Subset,
                compared_terms: Vec::new(),
                proof_ref: None,
                checked_at: created_at.into(),
            }),
        },
        mandate_ref: None,
        enforcement: ReceiptEnforcement {
            profile_hash: stable_hash(&format!("target-runner-source-publication:{target_repo}"))
                .into(),
            redaction_refs: Vec::new(),
            setup_refs: Vec::new(),
            teardown_refs: Vec::new(),
        },
    }
}

fn validate_source_publication_observation(
    request: &TargetRepoRunnerSourcePublicationRequest,
    observation: &TargetRepoRunnerSourcePublicationObservation,
) -> Result<(), TargetRepoRunnerRuntimeError> {
    if !same_reference(
        &observation.source_thread_ref,
        &request.publication.source_thread_ref,
    ) {
        return Err(TargetRepoRunnerRuntimeError::SourcePublicationMismatch(
            "source thread readback does not match publication plan".to_owned(),
        ));
    }
    if !same_reference(
        &observation.pull_request_ref,
        &request.publication.pull_request_ref,
    ) {
        return Err(TargetRepoRunnerRuntimeError::SourcePublicationMismatch(
            "target pull request readback does not match publication plan".to_owned(),
        ));
    }
    if !same_reference(
        &observation.revision_receipt_ref,
        &request.revision_receipt_ref,
    ) {
        return Err(TargetRepoRunnerRuntimeError::SourcePublicationMismatch(
            "revision receipt readback does not match publication request".to_owned(),
        ));
    }
    match (
        &request.publication.source_issue_ref,
        &observation.source_issue_ref,
    ) {
        (Some(expected), Some(actual)) if same_reference(actual, expected) => {}
        (Some(_), Some(_)) => {
            return Err(TargetRepoRunnerRuntimeError::SourcePublicationMismatch(
                "source issue readback does not match publication plan".to_owned(),
            ));
        }
        (Some(_), None) => {
            return Err(TargetRepoRunnerRuntimeError::SourcePublicationMismatch(
                "source issue publication readback is required".to_owned(),
            ));
        }
        (None, Some(_)) => {
            return Err(TargetRepoRunnerRuntimeError::SourcePublicationMismatch(
                "source issue readback was returned for a plan without a source issue".to_owned(),
            ));
        }
        (None, None) => {}
    }
    if observation.published_refs.len() < request.commands.len() {
        return Err(TargetRepoRunnerRuntimeError::SourcePublicationMismatch(
            "publication readback did not return a ref for every source command".to_owned(),
        ));
    }
    Ok(())
}

fn source_publication_target_refs(
    observation: &TargetRepoRunnerSourcePublicationObservation,
) -> Vec<Reference> {
    let mut target_refs = vec![observation.source_thread_ref.clone()];
    if let Some(source_issue_ref) = &observation.source_issue_ref {
        target_refs.push(source_issue_ref.clone());
    }
    target_refs
}

fn source_publication_receipt_metadata(
    request: &TargetRepoRunnerSourcePublicationRequest,
    observation: &TargetRepoRunnerSourcePublicationObservation,
) -> JsonObject {
    let mut metadata = request.publication.metadata.clone();
    let mut target_runner = JsonObject::new();
    target_runner.insert(
        "contract".to_owned(),
        JsonValue::String("runx.target_repo_runner.source_publication.v1".to_owned()),
    );
    target_runner.insert(
        "revision_receipt".to_owned(),
        JsonValue::String(request.revision_receipt_ref.uri.clone().into_string()),
    );
    target_runner.insert(
        "command_count".to_owned(),
        JsonValue::Number(JsonNumber::U64(request.commands.len() as u64)),
    );
    metadata.insert("target_runner".to_owned(), JsonValue::Object(target_runner));
    metadata.insert(
        "published_refs".to_owned(),
        JsonValue::Array(
            observation
                .published_refs
                .iter()
                .map(|reference| JsonValue::String(reference.uri.clone().into_string()))
                .collect(),
        ),
    );
    metadata
}

struct ReceiptSealInputs<'a> {
    reason_code: &'a str,
    summary: &'a str,
    created_at: &'a str,
    act_id: &'a str,
    criterion_id: &'a str,
    evidence_refs: &'a [Reference],
    verification_refs: &'a [Reference],
    artifact_refs: &'a [Reference],
}

fn receipt_seal(inputs: ReceiptSealInputs<'_>) -> Seal {
    let _ = inputs.act_id;
    let _ = inputs.artifact_refs;
    Seal {
        disposition: ClosureDisposition::Closed,
        reason_code: inputs.reason_code.into(),
        summary: inputs.summary.into(),
        closed_at: inputs.created_at.into(),
        last_observed_at: inputs.created_at.into(),
        criteria: vec![CriterionBinding {
            criterion_id: inputs.criterion_id.into(),
            status: CriterionStatus::Verified,
            verification_refs: inputs.verification_refs.to_vec(),
            evidence_refs: inputs.evidence_refs.to_vec(),
            summary: Some(inputs.summary.into()),
        }],
    }
}

fn seal_revision_receipt(receipt: &mut Receipt) -> Result<(), TargetRepoRunnerRuntimeError> {
    receipt.id = content_addressed_receipt_id(receipt)
        .map_err(|error| TargetRepoRunnerRuntimeError::Receipt(error.to_string()))?
        .into();
    let digest = canonical_receipt_body_digest(receipt)
        .map_err(|error| TargetRepoRunnerRuntimeError::Receipt(error.to_string()))?;
    receipt.digest = digest.clone().into();
    receipt.signature.value = format!("sig:{digest}").into();
    validate_receipt(receipt)
        .map_err(|verification| TargetRepoRunnerRuntimeError::Receipt(format!("{verification:?}")))
}

fn revision_receipt_metadata(execution: &TargetRepoRunnerFixtureExecution) -> JsonObject {
    let mut target_runner = JsonObject::new();
    target_runner.insert(
        "contract".to_owned(),
        JsonValue::String("runx.target_repo_runner.v1".to_owned()),
    );
    target_runner.insert(
        "disposition".to_owned(),
        JsonValue::String(disposition_name(execution.disposition).to_owned()),
    );
    target_runner.insert(
        "dedupe_key".to_owned(),
        JsonValue::String(execution.dedupe_execution.key.clone()),
    );
    target_runner.insert(
        "target_repo".to_owned(),
        JsonValue::String(execution.execution_plan.checkout.target_repo.clone()),
    );
    let mut metadata = execution.pull_request_receipt.metadata.clone();
    metadata.insert("target_runner".to_owned(), JsonValue::Object(target_runner));
    metadata
}

fn disposition_name(disposition: TargetRepoRunnerPullRequestDisposition) -> &'static str {
    match disposition {
        TargetRepoRunnerPullRequestDisposition::Create => "created",
        TargetRepoRunnerPullRequestDisposition::Reuse => "reused",
    }
}

fn stable_hash(value: &str) -> String {
    format!("sha256:{:x}", Sha256::digest(value.as_bytes()))
}

fn safe_id(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '_'
            }
        })
        .collect()
}

fn pull_request_id_fragment(pull_request: &TargetRepoRunnerExistingPullRequest) -> String {
    pull_request
        .number
        .map(|number| number.to_string())
        .unwrap_or_else(|| safe_id(&pull_request.url))
}

fn reference_id_fragment(reference: &Reference) -> String {
    reference
        .locator
        .as_deref()
        .map(safe_id)
        .unwrap_or_else(|| safe_id(&reference.uri))
}

fn metadata_path_string<'a>(object: &'a JsonObject, path: &[&str]) -> Option<&'a str> {
    let mut value = object.get(*path.first()?)?;
    for segment in &path[1..] {
        let JsonValue::Object(object) = value else {
            return None;
        };
        value = object.get(*segment)?;
    }
    value.as_str()
}
