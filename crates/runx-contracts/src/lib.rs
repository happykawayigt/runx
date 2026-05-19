//! Shared Rust contract types for runx JSON and protocol boundaries.

pub mod act;
pub mod act_assignment;
pub mod artifact;
pub mod aster;
pub mod authority;
pub mod cli;
pub mod decision;
pub mod doctor;
pub mod execution;
pub mod fingerprint;
pub mod harness;
pub mod host_protocol;
pub mod json;
pub mod links;
pub mod operational_policy;
pub mod receipts;
pub mod redaction;
pub mod reference;
pub mod registry;
pub mod signal;
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
pub use fingerprint::{Fingerprint, FingerprintAlgorithm};
pub use harness::{
    FanoutReceiptDecision, FanoutReceiptStrategy, FanoutReceiptSyncPoint, HARNESS_RECEIPT_SCHEMA,
    Harness, HarnessEnforcement, HarnessIdempotency, HarnessReceipt, HarnessReceiptSchema,
    HarnessRevision, HarnessSandbox, HarnessSeal, HarnessState, ReceiptIssuer, ReceiptIssuerType,
    ReceiptSignature, SealCriterion, SignatureAlgorithm,
};
pub use host_protocol::{
    AgentActInvocation, AgentActSourceType, ApprovalDecision, ApprovalGate, ExecutionEvent,
    HostPausedState, HostRunApproval, HostRunApprovalDecision, HostRunKind, HostRunLineage,
    HostRunLineageKind, HostRunResult, HostRunState, HostRunVerification,
    HostRunVerificationStatus, HostTerminalState, Question, ResolutionRequest, ResolutionResponse,
    ResolutionResponseActor,
};
pub use json::{JsonNumber, JsonObject, JsonValue};
pub use links::{DuplicateCandidate, Links};
pub use operational_policy::{
    OperationalPolicy, OperationalPolicyAction, OperationalPolicyAutomationPermissions,
    OperationalPolicyDedupePolicy, OperationalPolicyDedupeStrategy,
    OperationalPolicyDuplicateBehavior, OperationalPolicyError, OperationalPolicyMissingBehavior,
    OperationalPolicyOutcomeCloseMode, OperationalPolicyOutcomePolicy, OperationalPolicyOwnerRoute,
    OperationalPolicyPublishMode, OperationalPolicyReadback, OperationalPolicyRunnerKind,
    OperationalPolicyRunnerReadback, OperationalPolicyRunnerRule, OperationalPolicyRunnerState,
    OperationalPolicySchema, OperationalPolicySentryPolicy, OperationalPolicySourceProvider,
    OperationalPolicySourceReadback, OperationalPolicySourceRule,
    OperationalPolicySourceThreadPolicy, OperationalPolicyTargetReadback,
    OperationalPolicyTargetRule, OperationalPolicyValidationFinding,
    lint_operational_policy_contract, project_operational_policy_readback,
    validate_operational_policy_contract, validate_operational_policy_semantics,
};
pub use redaction::{HashAlgorithm, HashCommitment, REDACTION_SCHEMA, Redaction, RedactionSchema};
pub use reference::{ActRef, Reference, ReferenceType};
pub use signal::{
    SIGNAL_SCHEMA, Signal, SignalAuthenticity, SignalSchema, SignalTrustLevel, SignalType,
};
pub use verification::{
    ReceiptVerificationSummary, Verification, VerificationCheck, VerificationStatus,
};
