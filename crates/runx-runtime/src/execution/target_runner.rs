//! Runtime support for target-repo runner execution.

use std::fmt;

use serde::Serialize;
use sha2::{Digest, Sha256};

use runx_contracts::{
    Act, ActForm, Authority, AuthorityAttenuation, ChangePlan, ChangeRequest, Closure,
    ClosureDisposition, CriterionBinding, CriterionStatus, Decision, DecisionChoice,
    DecisionInputs, DecisionJustification, Harness, HarnessEnforcement, HarnessIdempotency,
    HarnessReceipt, HarnessReceiptSchema, HarnessRevision, HarnessSandbox, HarnessSeal,
    HarnessState, Intent, JsonObject, JsonValue, ReceiptIssuer, ReceiptIssuerType,
    ReceiptVerificationSummary, Reference, ReferenceType, RevisionDetails, SealCriterion,
    SignatureAlgorithm, SuccessCriterion, TargetRepoRunnerDedupeLookupExecution,
    TargetRepoRunnerDedupeLookupObservation, TargetRepoRunnerDedupeLookupPlan,
    TargetRepoRunnerDedupeResult, TargetRepoRunnerExecutionPlan,
    TargetRepoRunnerExistingPullRequest, TargetRepoRunnerPlan, TargetRepoRunnerPlanError,
    TargetRepoRunnerPullRequestDisposition, TargetRepoRunnerPullRequestReceiptPlan,
    TargetRepoRunnerReadinessObservation, TargetRepoRunnerSourcePublicationReceiptPlan,
    TargetSurface, Verification, VerificationCheck, VerificationStatus,
    apply_target_repo_runner_dedupe_lookup_execution, execute_target_repo_runner_dedupe_lookup,
    plan_target_repo_runner_execution, plan_target_repo_runner_pull_request_receipt,
    plan_target_repo_runner_source_publication_receipt,
};
use runx_receipts::{canonical_receipt_body_digest, validate_harness_receipt};

#[derive(Clone, Debug, PartialEq)]
pub struct TargetRepoRunnerFixtureExecutionInput {
    pub plan: TargetRepoRunnerPlan,
    pub readiness: TargetRepoRunnerReadinessObservation,
    pub dedupe: TargetRepoRunnerDedupeLookupObservation,
    pub created_pull_request: Option<TargetRepoRunnerExistingPullRequest>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct TargetRepoRunnerFixtureExecution {
    pub execution_plan: TargetRepoRunnerExecutionPlan,
    pub dedupe_execution: TargetRepoRunnerDedupeLookupExecution,
    pub deduped_plan: TargetRepoRunnerPlan,
    pub disposition: TargetRepoRunnerPullRequestDisposition,
    pub pull_request: TargetRepoRunnerExistingPullRequest,
    pub pull_request_receipt: TargetRepoRunnerPullRequestReceiptPlan,
    pub source_publication_receipt: TargetRepoRunnerSourcePublicationReceiptPlan,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct TargetRepoRunnerLiveExecution {
    pub readiness: TargetRepoRunnerReadinessObservation,
    pub dedupe_observation: TargetRepoRunnerDedupeLookupObservation,
    pub runner_observation: Option<TargetRepoRunnerGovernedRunnerObservation>,
    pub execution: TargetRepoRunnerFixtureExecution,
    pub revision_receipt: HarnessReceipt,
    pub revision_projection: TargetRepoRunnerRevisionReceiptProjection,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct TargetRepoRunnerGovernedRunnerInvocation {
    pub execution_plan: TargetRepoRunnerExecutionPlan,
    pub deduped_plan: TargetRepoRunnerPlan,
    pub disposition: TargetRepoRunnerPullRequestDisposition,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TargetRepoRunnerGovernedRunnerObservation {
    pub runner_id: String,
    pub target_repo: String,
    pub summary: String,
    pub revision_refs: Vec<Reference>,
    pub artifact_refs: Vec<Reference>,
    pub verification_refs: Vec<Reference>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct TargetRepoRunnerPullRequestObservationRequest {
    pub disposition: TargetRepoRunnerPullRequestDisposition,
    pub target_repo: String,
    pub dedupe_key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub existing_pull_request: Option<TargetRepoRunnerExistingPullRequest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runner_observation: Option<TargetRepoRunnerGovernedRunnerObservation>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct TargetRepoRunnerRevisionReceiptProjection {
    pub receipt_ref: Reference,
    pub act_id: String,
    pub disposition: TargetRepoRunnerPullRequestDisposition,
    pub target_repo_ref: Reference,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_issue_ref: Option<Reference>,
    pub source_thread_ref: Reference,
    pub pull_request_ref: Reference,
    pub summary: String,
    pub metadata: JsonObject,
}

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
        plan: &TargetRepoRunnerPlan,
    ) -> Result<TargetRepoRunnerReadinessObservation, TargetRepoRunnerAdapterError>;

    fn provider_dedupe_lookup(
        &mut self,
        lookup: &TargetRepoRunnerDedupeLookupPlan,
    ) -> Result<TargetRepoRunnerDedupeLookupObservation, TargetRepoRunnerAdapterError>;

    fn invoke_governed_runner(
        &mut self,
        invocation: &TargetRepoRunnerGovernedRunnerInvocation,
    ) -> Result<TargetRepoRunnerGovernedRunnerObservation, TargetRepoRunnerAdapterError>;

    fn observe_pull_request(
        &mut self,
        request: &TargetRepoRunnerPullRequestObservationRequest,
    ) -> Result<TargetRepoRunnerExistingPullRequest, TargetRepoRunnerAdapterError>;
}

#[derive(Clone, Debug, PartialEq)]
pub enum TargetRepoRunnerRuntimeError {
    Plan(TargetRepoRunnerPlanError),
    Adapter(TargetRepoRunnerAdapterError),
    Receipt(String),
    ReceiptProjection(String),
    ReadinessMismatch(String),
    CheckoutNotScafldReady { target_repo: String },
    CreatedPullRequestRequired { target_repo: String },
}

impl fmt::Display for TargetRepoRunnerRuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Plan(error) => write!(formatter, "{error}"),
            Self::Adapter(error) => write!(formatter, "{error}"),
            Self::Receipt(message) => {
                write!(formatter, "target repo runner receipt failed: {message}")
            }
            Self::ReceiptProjection(message) => {
                write!(
                    formatter,
                    "target repo runner receipt projection failed: {message}"
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
            Self::Receipt(_)
            | Self::ReceiptProjection(_)
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

pub fn execute_target_repo_runner_with_adapter<A: TargetRepoRunnerAdapter>(
    plan: &TargetRepoRunnerPlan,
    adapter: &mut A,
    created_at: &str,
) -> Result<TargetRepoRunnerLiveExecution, TargetRepoRunnerRuntimeError> {
    let readiness = adapter.checkout_readiness(plan)?;
    let execution_plan = plan_target_repo_runner_execution(plan, &readiness)?;
    let dedupe_observation = adapter.provider_dedupe_lookup(&execution_plan.provider_lookup)?;
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
    let pull_request =
        adapter.observe_pull_request(&TargetRepoRunnerPullRequestObservationRequest {
            disposition,
            target_repo: execution_plan.checkout.target_repo.clone(),
            dedupe_key: execution_plan.provider_lookup.key.clone(),
            existing_pull_request: dedupe_execution.existing_pull_request.clone(),
            runner_observation: runner_observation.clone(),
        })?;

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

    Ok(TargetRepoRunnerLiveExecution {
        readiness,
        dedupe_observation,
        runner_observation,
        execution,
        revision_receipt,
        revision_projection,
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

fn target_repo_runner_revision_receipt(
    execution: &TargetRepoRunnerFixtureExecution,
    runner_observation: Option<&TargetRepoRunnerGovernedRunnerObservation>,
    created_at: &str,
) -> Result<HarnessReceipt, TargetRepoRunnerRuntimeError> {
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
    let act = revision_act(RevisionActInput {
        act_id,
        criterion_id,
        created_at,
        disposition: execution.disposition,
        summary: &summary,
        target_repo_ref: &target_repo_ref,
        source_thread_ref: &execution.pull_request_receipt.source_thread_ref,
        source_issue_ref: execution.pull_request_receipt.source_issue_ref.as_ref(),
        pull_request_ref: &pull_request_ref,
        artifact_refs: &artifact_refs,
        verification_refs: &verification_refs,
        runner_observation,
    });
    let seal = receipt_seal(ReceiptSealInputs {
        disposition: execution.disposition,
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
    let harness = receipt_harness(
        execution,
        &receipt_id,
        act,
        seal.clone(),
        created_at,
        &evidence_refs,
        &artifact_refs,
    );
    let mut receipt = HarnessReceipt {
        schema: HarnessReceiptSchema::V1,
        id: receipt_id,
        created_at: created_at.to_owned(),
        issuer: local_issuer(),
        signature: runx_contracts::ReceiptSignature {
            alg: SignatureAlgorithm::Ed25519,
            value: "sig:pending".to_owned(),
        },
        harness,
        seal,
        sync_points: Vec::new(),
        metadata: Some(revision_receipt_metadata(execution)),
    };
    seal_revision_receipt(&mut receipt)?;
    Ok(receipt)
}

struct RevisionActInput<'a> {
    act_id: &'a str,
    criterion_id: &'a str,
    created_at: &'a str,
    disposition: TargetRepoRunnerPullRequestDisposition,
    summary: &'a str,
    target_repo_ref: &'a Reference,
    source_thread_ref: &'a Reference,
    source_issue_ref: Option<&'a Reference>,
    pull_request_ref: &'a Reference,
    artifact_refs: &'a [Reference],
    verification_refs: &'a [Reference],
    runner_observation: Option<&'a TargetRepoRunnerGovernedRunnerObservation>,
}

fn revision_act(input: RevisionActInput<'_>) -> Act {
    let RevisionActInput {
        act_id,
        criterion_id,
        created_at,
        disposition,
        summary,
        target_repo_ref,
        source_thread_ref,
        source_issue_ref,
        pull_request_ref,
        artifact_refs,
        verification_refs,
        runner_observation,
    } = input;
    let mut source_refs = vec![source_thread_ref.clone()];
    if let Some(source_issue_ref) = source_issue_ref {
        source_refs.push(source_issue_ref.clone());
    }
    let target_refs = vec![target_repo_ref.clone(), pull_request_ref.clone()];
    let success_criterion = SuccessCriterion {
        criterion_id: criterion_id.to_owned(),
        statement: "Target pull request is ready and linked to the source thread.".to_owned(),
        required: true,
    };
    Act {
        schema: None,
        act_id: act_id.to_owned(),
        form: ActForm::Revision,
        intent: Intent {
            purpose: "Run the governed target runner and surface the target pull request."
                .to_owned(),
            legitimacy: "Operational policy admitted this target repo runner execution.".to_owned(),
            success_criteria: vec![success_criterion.clone()],
            constraints: vec![
                "Dedupe must run before creating a target pull request.".to_owned(),
                "Public output must not include local checkout paths.".to_owned(),
            ],
            derived_from: source_refs.clone(),
        },
        summary: summary.to_owned(),
        closure: Closure {
            disposition: ClosureDisposition::Closed,
            reason_code: format!("target_runner_pr_{}", disposition_name(disposition)),
            summary: summary.to_owned(),
            closed_at: created_at.to_owned(),
        },
        criterion_bindings: vec![CriterionBinding {
            criterion_id: criterion_id.to_owned(),
            status: CriterionStatus::Verified,
            evidence_refs: target_refs.clone(),
            verification_refs: verification_refs.to_vec(),
            summary: Some(summary.to_owned()),
        }],
        source_refs,
        target_refs: target_refs.clone(),
        surface_refs: target_refs.clone(),
        artifact_refs: artifact_refs.to_vec(),
        verification_refs: verification_refs.to_vec(),
        harness_refs: Vec::new(),
        revision: Some(revision_details(
            disposition,
            &success_criterion,
            target_repo_ref,
            pull_request_ref,
            runner_observation,
            verification_refs,
            created_at,
        )),
        verification: None,
        performed_at: created_at.to_owned(),
    }
}

fn revision_details(
    disposition: TargetRepoRunnerPullRequestDisposition,
    success_criterion: &SuccessCriterion,
    target_repo_ref: &Reference,
    pull_request_ref: &Reference,
    runner_observation: Option<&TargetRepoRunnerGovernedRunnerObservation>,
    verification_refs: &[Reference],
    created_at: &str,
) -> RevisionDetails {
    let target_surfaces = vec![
        TargetSurface {
            surface_ref: target_repo_ref.clone(),
            mutating: true,
            rationale: Some("Target runner is authorized for this repository.".to_owned()),
        },
        TargetSurface {
            surface_ref: pull_request_ref.clone(),
            mutating: true,
            rationale: Some(format!(
                "Pull request path was {} for the dedupe key.",
                disposition_name(disposition)
            )),
        },
    ];
    RevisionDetails {
        change_request: ChangeRequest {
            request_id: format!("change_target_runner_pr_{}", disposition_name(disposition)),
            summary: "Prepare the target pull request for human review.".to_owned(),
            target_surfaces: target_surfaces.clone(),
            success_criteria: vec![success_criterion.clone()],
        },
        change_plan: ChangePlan {
            plan_id: "plan_target_runner_pr".to_owned(),
            summary: "Use provider dedupe, run the governed target runner when needed, and record the target pull request.".to_owned(),
            steps: vec![
                "Check out and verify the target repo readiness.".to_owned(),
                "Look up provider pull requests for the dedupe key.".to_owned(),
                "Create or reuse the target pull request observation.".to_owned(),
            ],
            risks: Vec::new(),
        },
        target_surfaces,
        invariants: vec![
            "No mutation occurs before scafld readiness is observed.".to_owned(),
            "Dedupe is authoritative for create versus reuse.".to_owned(),
        ],
        verification: Some(Verification {
            schema: None,
            verification_id: Some("ver_target_runner_pr_ready".to_owned()),
            status: VerificationStatus::Passed,
            checks: vec![VerificationCheck {
                check_id: "check_target_runner_pr_ready".to_owned(),
                criterion_ids: vec![success_criterion.criterion_id.clone()],
                status: VerificationStatus::Passed,
                summary: Some(runner_observation.map_or_else(
                    || "Existing pull request was reused.".to_owned(),
                    |observation| observation.summary.clone(),
                )),
                checked_refs: vec![target_repo_ref.clone(), pull_request_ref.clone()],
                evidence_refs: verification_refs.to_vec(),
                verified_at: Some(created_at.to_owned()),
            }],
            verified_at: Some(created_at.to_owned()),
            evidence_refs: verification_refs.to_vec(),
        }),
        handoff_refs: Vec::new(),
        revision_refs: runner_observation.map_or_else(
            || vec![pull_request_ref.clone()],
            |observation| {
                let mut refs = observation.revision_refs.clone();
                if !refs.iter().any(|reference| reference.uri == pull_request_ref.uri) {
                    refs.push(pull_request_ref.clone());
                }
                refs
            },
        ),
    }
}

fn receipt_harness(
    execution: &TargetRepoRunnerFixtureExecution,
    receipt_id: &str,
    act: Act,
    seal: HarnessSeal,
    created_at: &str,
    evidence_refs: &[Reference],
    artifact_refs: &[Reference],
) -> Harness {
    let decision_id = "dec_target_runner_pr";
    Harness {
        schema: None,
        harness_id: format!(
            "hrn_target_runner_{}",
            safe_id(&execution.execution_plan.checkout.target_repo)
        ),
        parent_harness_ref: None,
        state: HarnessState::Sealed,
        host_ref: reference(ReferenceType::Host, "target_runner_adapter"),
        harness_ref: reference(ReferenceType::Harness, "target-runner"),
        authority: Authority {
            schema: None,
            actor_ref: reference(ReferenceType::Principal, "target_runner"),
            authority_proof_refs: Vec::new(),
            grant_refs: Vec::new(),
            scope_refs: Vec::new(),
            policy_refs: Vec::new(),
            terms: Vec::new(),
            attenuation: AuthorityAttenuation {
                parent_authority_ref: None,
                subset_proof: None,
            },
            mandate_ref: None,
        },
        enforcement: HarnessEnforcement {
            harness_ref: None,
            version: "target-runner-adapter.v1".to_owned(),
            enforcement_profile_hash: stable_hash(&format!(
                "target-runner:{}",
                execution.execution_plan.checkout.target_repo
            )),
            enforcer_ref: None,
            sandbox: HarnessSandbox {
                profile: "target-runner-adapter".to_owned(),
                cwd_policy: "target-checkout-hidden".to_owned(),
                network: "adapter-declared".to_owned(),
                filesystem: "target-repo-scoped".to_owned(),
            },
            redaction_refs: Vec::new(),
            stdout_hash: None,
            stderr_hash: None,
            setup_receipt_refs: Vec::new(),
            teardown_receipt_refs: Vec::new(),
        },
        idempotency: HarnessIdempotency {
            intent_key: stable_hash(&format!(
                "target-runner:{}:{}",
                execution.execution_plan.checkout.target_repo, execution.dedupe_execution.key
            )),
            trigger_fingerprint: stable_hash(&execution.dedupe_execution.key),
            content_hash: stable_hash(receipt_id),
        },
        revision: HarnessRevision {
            sequence: 1,
            previous_ref: None,
        },
        signal_refs: vec![execution.execution_plan.source_thread_ref.clone()],
        decisions: vec![Decision {
            decision_id: decision_id.to_owned(),
            choice: DecisionChoice::Close,
            inputs: DecisionInputs {
                signal_refs: vec![execution.execution_plan.source_thread_ref.clone()],
                target_ref: Some(execution.execution_plan.target_repo_ref.clone()),
                opportunity_refs: Vec::new(),
                selection_ref: None,
            },
            proposed_intent: act.intent.clone(),
            selected_act_id: Some(act.act_id.clone()),
            selected_harness_ref: None,
            justification: DecisionJustification {
                summary: "Selected the policy-admitted target runner path.".to_owned(),
                evidence_refs: evidence_refs.to_vec(),
            },
            closure: Some(Closure {
                disposition: ClosureDisposition::Closed,
                reason_code: "target_runner_decision_closed".to_owned(),
                summary: "Target pull request path was recorded.".to_owned(),
                closed_at: created_at.to_owned(),
            }),
            artifact_refs: artifact_refs.to_vec(),
        }],
        acts: vec![act],
        child_harness_receipt_refs: Vec::new(),
        artifact_refs: artifact_refs.to_vec(),
        seal: Some(seal),
    }
}

struct ReceiptSealInputs<'a> {
    disposition: TargetRepoRunnerPullRequestDisposition,
    summary: &'a str,
    created_at: &'a str,
    act_id: &'a str,
    criterion_id: &'a str,
    evidence_refs: &'a [Reference],
    verification_refs: &'a [Reference],
    artifact_refs: &'a [Reference],
}

fn receipt_seal(inputs: ReceiptSealInputs<'_>) -> HarnessSeal {
    HarnessSeal {
        disposition: ClosureDisposition::Closed,
        reason_code: format!("target_runner_pr_{}", disposition_name(inputs.disposition)),
        summary: inputs.summary.to_owned(),
        closed_at: inputs.created_at.to_owned(),
        last_observed_at: inputs.created_at.to_owned(),
        canonicalization: "runx.harness-receipt.c14n.v1".to_owned(),
        digest: "sha256:pending".to_owned(),
        criteria: vec![SealCriterion {
            criterion_id: inputs.criterion_id.to_owned(),
            status: CriterionStatus::Verified,
            act_id: Some(inputs.act_id.to_owned()),
            verification_refs: inputs.verification_refs.to_vec(),
            evidence_refs: inputs.evidence_refs.to_vec(),
            summary: Some(inputs.summary.to_owned()),
        }],
        verification_summary: Some(ReceiptVerificationSummary {
            signature_valid: true,
            hash_commitments_valid: true,
            authority_attenuation_valid: true,
            criteria_bound: true,
            redaction_valid: true,
            external_attestations_present: !inputs.verification_refs.is_empty(),
        }),
        redaction_refs: Vec::new(),
        artifact_refs: inputs.artifact_refs.to_vec(),
        hash_commitments: Vec::new(),
    }
}

fn seal_revision_receipt(receipt: &mut HarnessReceipt) -> Result<(), TargetRepoRunnerRuntimeError> {
    let digest = canonical_receipt_body_digest(receipt)
        .map_err(|error| TargetRepoRunnerRuntimeError::Receipt(error.to_string()))?;
    receipt.seal.digest = digest.clone();
    if let Some(harness_seal) = receipt.harness.seal.as_mut() {
        harness_seal.digest = digest.clone();
    }
    receipt.signature.value = format!("sig:{digest}");
    validate_harness_receipt(receipt)
        .map_err(|verification| TargetRepoRunnerRuntimeError::Receipt(format!("{verification:?}")))
}

pub fn project_target_repo_runner_revision_receipt(
    receipt: &HarnessReceipt,
) -> Result<TargetRepoRunnerRevisionReceiptProjection, TargetRepoRunnerRuntimeError> {
    if receipt.harness.state != HarnessState::Sealed {
        return Err(TargetRepoRunnerRuntimeError::ReceiptProjection(
            "receipt harness is not sealed".to_owned(),
        ));
    }
    let act = receipt
        .harness
        .acts
        .iter()
        .find(|act| act.form == ActForm::Revision)
        .ok_or_else(|| {
            TargetRepoRunnerRuntimeError::ReceiptProjection("revision act is required".to_owned())
        })?;
    let metadata = receipt.metadata.clone().ok_or_else(|| {
        TargetRepoRunnerRuntimeError::ReceiptProjection(
            "target runner metadata is required".to_owned(),
        )
    })?;
    let pull_request_ref = act
        .target_refs
        .iter()
        .find(|reference| reference.reference_type == ReferenceType::GithubPullRequest)
        .cloned()
        .ok_or_else(|| {
            TargetRepoRunnerRuntimeError::ReceiptProjection(
                "pull request ref is required".to_owned(),
            )
        })?;
    let target_repo_ref = act
        .target_refs
        .iter()
        .find(|reference| reference.reference_type == ReferenceType::GithubRepo)
        .cloned()
        .ok_or_else(|| {
            TargetRepoRunnerRuntimeError::ReceiptProjection(
                "target repo ref is required".to_owned(),
            )
        })?;
    let source_thread_ref = act.source_refs.first().cloned().ok_or_else(|| {
        TargetRepoRunnerRuntimeError::ReceiptProjection("source thread ref is required".to_owned())
    })?;
    let source_issue_ref = act
        .source_refs
        .iter()
        .find(|reference| reference.reference_type == ReferenceType::GithubIssue)
        .cloned();
    Ok(TargetRepoRunnerRevisionReceiptProjection {
        receipt_ref: reference(ReferenceType::HarnessReceipt, &receipt.id),
        act_id: act.act_id.clone(),
        disposition: projection_disposition(&metadata)?,
        target_repo_ref,
        source_issue_ref,
        source_thread_ref,
        pull_request_ref,
        summary: receipt.seal.summary.clone(),
        metadata,
    })
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

fn projection_disposition(
    metadata: &JsonObject,
) -> Result<TargetRepoRunnerPullRequestDisposition, TargetRepoRunnerRuntimeError> {
    let Some(JsonValue::Object(target_runner)) = metadata.get("target_runner") else {
        return Err(TargetRepoRunnerRuntimeError::ReceiptProjection(
            "target runner metadata object is required".to_owned(),
        ));
    };
    match target_runner.get("disposition").and_then(json_string) {
        Some("created") => Ok(TargetRepoRunnerPullRequestDisposition::Create),
        Some("reused") => Ok(TargetRepoRunnerPullRequestDisposition::Reuse),
        _ => Err(TargetRepoRunnerRuntimeError::ReceiptProjection(
            "target runner disposition is invalid".to_owned(),
        )),
    }
}

fn disposition_name(disposition: TargetRepoRunnerPullRequestDisposition) -> &'static str {
    match disposition {
        TargetRepoRunnerPullRequestDisposition::Create => "created",
        TargetRepoRunnerPullRequestDisposition::Reuse => "reused",
    }
}

fn local_issuer() -> ReceiptIssuer {
    ReceiptIssuer {
        issuer_type: ReceiptIssuerType::Local,
        kid: "target-runner-runtime".to_owned(),
        public_key_sha256: "sha256:target-runner-runtime-public".to_owned(),
    }
}

fn reference(reference_type: ReferenceType, id: &str) -> Reference {
    Reference {
        uri: format!("runx:{}:{id}", reference_type_name(&reference_type)),
        reference_type,
        provider: None,
        locator: None,
        label: None,
        observed_at: None,
        proof_kind: None,
    }
}

fn reference_type_name(reference_type: &ReferenceType) -> &'static str {
    match reference_type {
        ReferenceType::GithubIssue => "github_issue",
        ReferenceType::GithubPullRequest => "github_pull_request",
        ReferenceType::GithubRepo => "github_repo",
        ReferenceType::SlackThread => "slack_thread",
        ReferenceType::SentryEvent => "sentry_event",
        ReferenceType::Signal => "signal",
        ReferenceType::Act => "act",
        ReferenceType::Receipt => "receipt",
        ReferenceType::GraphReceipt => "graph_receipt",
        ReferenceType::HarnessReceipt => "harness_receipt",
        ReferenceType::Artifact => "artifact",
        ReferenceType::Verification => "verification",
        ReferenceType::Harness => "harness",
        ReferenceType::Host => "host",
        ReferenceType::Deployment => "deployment",
        ReferenceType::Surface => "surface",
        ReferenceType::Target => "target",
        ReferenceType::Opportunity => "opportunity",
        ReferenceType::ThesisAssessment => "thesis_assessment",
        ReferenceType::Selection => "selection",
        ReferenceType::SkillBinding => "skill_binding",
        ReferenceType::TargetTransitionEntry => "target_transition_entry",
        ReferenceType::SelectionCycle => "selection_cycle",
        ReferenceType::Decision => "decision",
        ReferenceType::ReflectionEntry => "reflection_entry",
        ReferenceType::FeedEntry => "feed_entry",
        ReferenceType::Principal => "principal",
        ReferenceType::AuthorityProof => "authority_proof",
        ReferenceType::ScopeAdmission => "scope_admission",
        ReferenceType::Grant => "grant",
        ReferenceType::Mandate => "mandate",
        ReferenceType::Credential => "credential",
        ReferenceType::WebhookDelivery => "webhook_delivery",
        ReferenceType::RedactionPolicy => "redaction_policy",
        ReferenceType::ExternalUrl => "external_url",
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

fn json_string(value: &JsonValue) -> Option<&str> {
    match value {
        JsonValue::String(value) => Some(value.as_str()),
        _ => None,
    }
}
