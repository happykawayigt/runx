//! Shared Rust contract types for runx JSON and protocol boundaries.

pub mod act;
pub mod act_assignment;
pub mod artifact;
pub mod aster;
pub mod authority;
pub mod cli;
pub mod credential_delivery;
pub mod decision;
pub mod doctor;
pub mod execution;
pub mod external_adapter;
pub mod fingerprint;
pub mod harness;
pub mod host_protocol;
pub mod json;
pub mod links;
pub mod maturity;
pub mod operational_policy;
pub mod post_merge_observer;
pub mod receipts;
pub mod redaction;
pub mod reference;
pub mod registry;
pub mod signal;
pub mod target_runner;
pub mod thread_outbox_provider;
pub mod tools;
pub mod verification;

pub use act::{
    Act, ActForm, ChangePlan, ChangeRequest, CriterionBinding, CriterionStatus, GovernedActRef,
    Intent, RevisionDetails, SuccessCriterion, TargetSurface, VerificationDetails,
};
pub use act_assignment::{
    ActAssignment, ActAssignmentActor, ActAssignmentHost, ActAssignmentHostKind,
    ActAssignmentIdempotency, BuildActAssignment, IntentKeyInput, derive_content_hash,
    derive_intent_key, derive_trigger_key,
};
pub use artifact::{ARTIFACT_SCHEMA, Artifact, ArtifactProducedBy, ArtifactSchema};
pub use aster::{
    AuthorityCostLevel, FeedEntry, FeedEntrySchema, Opportunity, OpportunitySchema,
    ReflectionEntry, ReflectionEntrySchema, Selection, SelectionCycle, SelectionCycleSchema,
    SelectionCycleState, SelectionSchema, SkillBinding, SkillBindingSchema, Target, TargetCooldown,
    TargetCooldownState, TargetLifecycleState, TargetSchema, TargetTransitionEntry,
    TargetTransitionEntrySchema, ThesisAssessment, ThesisAssessmentSchema, ThesisProofStrength,
};
pub use authority::{
    Authority, AuthorityApproval, AuthorityAttenuation, AuthorityBounds, AuthorityCapability,
    AuthorityCondition, AuthorityConditionPredicate, AuthorityResourceFamily,
    AuthoritySubsetComparison, AuthoritySubsetProof, AuthoritySubsetRelation,
    AuthoritySubsetResult, AuthorityTerm, AuthorityVerb, PaymentAuthorityBounds,
    PaymentCredentialForm,
};
pub use credential_delivery::{
    CredentialDeliveryBrokerResponse, CredentialDeliveryEnvBinding, CredentialDeliveryHandle,
    CredentialDeliveryMode, CredentialDeliveryObservation, CredentialDeliveryObservationStatus,
    CredentialDeliveryProfile, CredentialDeliveryPurpose, CredentialDeliveryRequest,
    CredentialDeliveryStatus, CredentialMaterialRole,
};
pub use decision::{
    Closure, ClosureDisposition, Decision, DecisionChoice, DecisionInputs, DecisionJustification,
};
pub use doctor::{
    DoctorDiagnostic, DoctorDiagnosticSeverity, DoctorLocation, DoctorRepair,
    DoctorRepairConfidence, DoctorRepairKind, DoctorRepairRisk, DoctorReport, DoctorReportSchema,
    DoctorStatus, DoctorSummary,
};
pub use execution::{
    ExecutionSemantics, GovernedDisposition, InputContextCapture, OutcomeState, ReceiptOutcome,
    ReceiptSurfaceRef,
};
pub use external_adapter::{
    EXTERNAL_ADAPTER_PROTOCOL_VERSION, ExternalAdapterArtifactObservation,
    ExternalAdapterCancellationFrame, ExternalAdapterCredentialNeed,
    ExternalAdapterCredentialPurpose, ExternalAdapterCredentialReference,
    ExternalAdapterCredentialRequest, ExternalAdapterErrorObservation,
    ExternalAdapterHostResolutionFrame, ExternalAdapterInvocation, ExternalAdapterManifest,
    ExternalAdapterResponse, ExternalAdapterSandboxIntent, ExternalAdapterStatus,
    ExternalAdapterTelemetryObservation, ExternalAdapterTelemetryValue, ExternalAdapterTimeouts,
    ExternalAdapterTransport, ExternalAdapterTransportKind,
};
pub use fingerprint::{Fingerprint, FingerprintAlgorithm, hex_lower, sha256_hex, sha256_prefixed};
pub use harness::{
    FanoutReceiptDecision, FanoutReceiptStrategy, FanoutReceiptSyncPoint, Lineage,
    RECEIPT_CANONICALIZATION, RECEIPT_SCHEMA, Receipt, ReceiptAct, ReceiptAuthority,
    ReceiptCommitment, ReceiptCommitmentScope, ReceiptCriterion, ReceiptEnforcement,
    ReceiptIdempotency, ReceiptIssuer, ReceiptIssuerType, ReceiptSchema, ReceiptSignature,
    ReceiptSubjectKind, RunnerProvenance, Seal, SignatureAlgorithm, Subject,
};
pub use host_protocol::{
    AgentActInvocation, AgentActSourceType, ApprovalDecision, ApprovalGate, ExecutionEvent,
    HostNeedsAgentState, HostRunApproval, HostRunApprovalDecision, HostRunKind, HostRunLineage,
    HostRunLineageKind, HostRunResult, HostRunState, HostRunVerification,
    HostRunVerificationStatus, HostTerminalState, Question, ResolutionRequest, ResolutionResponse,
    ResolutionResponseActor,
};
pub use json::{JsonNumber, JsonObject, JsonValue};
pub use links::{DuplicateCandidate, Links};
pub use operational_policy::{
    OperationalPolicy, OperationalPolicyAction, OperationalPolicyAdmission,
    OperationalPolicyAdmissionRequest, OperationalPolicyAdmissionStatus,
    OperationalPolicyAutomationPermissions, OperationalPolicyDedupePolicy,
    OperationalPolicyDedupeStrategy, OperationalPolicyDuplicateBehavior, OperationalPolicyError,
    OperationalPolicyMissingBehavior, OperationalPolicyOutcomeCloseMode,
    OperationalPolicyOutcomePolicy, OperationalPolicyOwnerRoute, OperationalPolicyPublishMode,
    OperationalPolicyReadback, OperationalPolicyRunnerKind, OperationalPolicyRunnerReadback,
    OperationalPolicyRunnerRule, OperationalPolicyRunnerState, OperationalPolicySchema,
    OperationalPolicySentryPolicy, OperationalPolicySourceProvider,
    OperationalPolicySourceReadback, OperationalPolicySourceRule,
    OperationalPolicySourceThreadPolicy, OperationalPolicyTargetReadback,
    OperationalPolicyTargetRule, OperationalPolicyValidationFinding,
    admit_operational_policy_request, lint_operational_policy_contract,
    project_operational_policy_readback, validate_operational_policy_contract,
    validate_operational_policy_semantics,
};
pub use post_merge_observer::{
    PostMergeObserverClosureState, PostMergeObserverCriterionPlan,
    PostMergeObserverIdempotencyPlan, PostMergeObserverPlan, PostMergeObserverPlanError,
    PostMergeObserverPlanRequest, PostMergeObserverProviderPlan, PostMergeObserverPublicationPlan,
    PostMergeObserverPublicationProjection, PostMergeObserverRuntimeDecision,
    PostMergeObserverRuntimeDedupePlan, PostMergeObserverSignalSource,
    PostMergeObserverSourceIssuePlan, PostMergeProvider, PostMergePullRequestObservation,
    PostMergePullRequestState, PostMergeSourceIssueDisposition, PostMergeVerificationObservation,
    PostMergeVerificationStatus, plan_post_merge_observer_closure,
    plan_post_merge_observer_runtime_dedupe, project_post_merge_observer_publication_from_receipt,
};
pub use redaction::{HashAlgorithm, HashCommitment, REDACTION_SCHEMA, Redaction, RedactionSchema};
pub use reference::{ActRef, ProofKind, Reference, ReferenceType};
pub use signal::{
    SIGNAL_SCHEMA, Signal, SignalAuthenticity, SignalSchema, SignalTrustLevel, SignalType,
};
pub use target_runner::{
    TargetRepoRunnerCheckoutPlan, TargetRepoRunnerDedupeComponent,
    TargetRepoRunnerDedupeLookupExecution, TargetRepoRunnerDedupeLookupObservation,
    TargetRepoRunnerDedupeLookupPlan, TargetRepoRunnerDedupeLookupQuery,
    TargetRepoRunnerDedupePlan, TargetRepoRunnerDedupeResult, TargetRepoRunnerExecutionPlan,
    TargetRepoRunnerExistingPullRequest, TargetRepoRunnerOwnerPlan, TargetRepoRunnerPlan,
    TargetRepoRunnerPlanError, TargetRepoRunnerPlanRequest, TargetRepoRunnerProvider,
    TargetRepoRunnerProviderPullRequest, TargetRepoRunnerPullRequestDisposition,
    TargetRepoRunnerPullRequestReceiptPlan, TargetRepoRunnerReadinessObservation,
    TargetRepoRunnerReadinessPlan, TargetRepoRunnerRunnerPlan, TargetRepoRunnerSourceContext,
    TargetRepoRunnerSourcePlan, TargetRepoRunnerSourcePublicationReceiptPlan,
    TargetRepoRunnerSourceThreadPlan, TargetRepoRunnerTargetPlan,
    apply_target_repo_runner_dedupe_lookup_execution, execute_target_repo_runner_dedupe_lookup,
    plan_target_repo_runner, plan_target_repo_runner_dedupe_lookup,
    plan_target_repo_runner_execution, plan_target_repo_runner_pull_request_receipt,
    plan_target_repo_runner_source_publication_receipt,
};
pub use thread_outbox_provider::{
    THREAD_OUTBOX_PROVIDER_PROTOCOL_VERSION, ThreadOutboxProviderCredentialNeed,
    ThreadOutboxProviderCredentialProfile, ThreadOutboxProviderError, ThreadOutboxProviderFetch,
    ThreadOutboxProviderFetchProviderTarget, ThreadOutboxProviderFetchTarget,
    ThreadOutboxProviderFetchThreadTarget, ThreadOutboxProviderIdempotency,
    ThreadOutboxProviderIdempotencyObservation, ThreadOutboxProviderIdempotencyStatus,
    ThreadOutboxProviderLocator, ThreadOutboxProviderManifest, ThreadOutboxProviderObservation,
    ThreadOutboxProviderObservationStatus, ThreadOutboxProviderOperation,
    ThreadOutboxProviderPayloadFormat, ThreadOutboxProviderPush,
    ThreadOutboxProviderReadbackSummary, ThreadOutboxProviderReceiptCapabilities,
    ThreadOutboxProviderReceiptContext, ThreadOutboxProviderRedactionCapabilities,
    ThreadOutboxProviderRenderedPayload, ThreadOutboxProviderThreadLocator,
    ThreadOutboxProviderTransport, ThreadOutboxProviderTransportKind,
};
pub use verification::{
    ReceiptVerificationSummary, Verification, VerificationCheck, VerificationStatus,
};
