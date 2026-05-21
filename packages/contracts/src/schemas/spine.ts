import { Type, type Static } from "@sinclair/typebox";
import {
  JSON_SCHEMA_DRAFT_2020_12,
  RUNX_CONTRACT_IDS,
  RUNX_LOGICAL_SCHEMAS,
  type DeepReadonly,
  dateTimeStringSchema,
  stringEnum,
  unknownRecordSchema,
  validateContractSchema,
} from "../internal.js";
import { outputSchema } from "./output.js";

export const referenceTypes = [
  "github_issue",
  "github_pull_request",
  "github_repo",
  "slack_thread",
  "sentry_event",
  "signal",
  "act",
  "receipt",
  "graph_receipt",
  "harness_receipt",
  "artifact",
  "verification",
  "harness",
  "host",
  "deployment",
  "surface",
  "target",
  "opportunity",
  "thesis_assessment",
  "selection",
  "skill_binding",
  "target_transition_entry",
  "selection_cycle",
  "decision",
  "reflection_entry",
  "feed_entry",
  "principal",
  "authority_proof",
  "scope_admission",
  "grant",
  "mandate",
  "credential",
  "webhook_delivery",
  "redaction_policy",
  "external_url",
] as const;

export const signalTypes = [
  "issue_opened",
  "issue_comment",
  "pull_request_event",
  "review_event",
  "chat_message",
  "alert",
  "deployment_event",
  "payment_required",
  "schedule_tick",
  "operator_note",
  "system_event",
] as const;

export const signalTrustLevels = [
  "unverified",
  "observed",
  "verified_delivery",
  "verified_signature",
  "operator_attested",
] as const;

export const harnessStates = [
  "forming",
  "admitted",
  "running",
  "waiting",
  "delegated",
  "sealing",
  "sealed",
  "killed",
  "timed_out",
  "failed",
  "superseded",
] as const;

export const harnessSealDispositions = [
  "closed",
  "deferred",
  "superseded",
  "declined",
  "blocked",
  "failed",
  "killed",
  "timed_out",
] as const;

export const decisionChoices = [
  "open",
  "continue",
  "spawn_child",
  "escalate",
  "defer",
  "close",
  "decline",
  "monitor",
] as const;

export const actForms = [
  "revision",
  "reply",
  "review",
  "observation",
  "verification",
] as const;

export const targetLifecycleStates = [
  "candidate",
  "eligible",
  "active",
  "cooling_down",
  "blocked",
  "retired",
] as const;

export const thesisProofStrengths = [
  "weak",
  "moderate",
  "strong",
] as const;

export const authorityCostLevels = [
  "none",
  "low",
  "medium",
  "high",
] as const;

export const selectionCycleStates = [
  "open",
  "closed",
  "deferred",
  "no_action",
] as const;

export const criterionStatuses = [
  "verified",
  "failed",
  "pending",
  "not_applicable",
  "unknown",
] as const;

export const verificationStatuses = [
  "passed",
  "failed",
  "pending",
  "not_applicable",
  "missing",
] as const;

export const authorityResourceFamilies = [
  "github_repo",
  "workspace",
  "filesystem",
  "network",
  "deployment",
  "credential",
  "payment",
  "artifact",
  "harness",
  "publication",
] as const;

export const authorityVerbs = [
  "read",
  "write",
  "comment",
  "review",
  "approve",
  "merge",
  "create",
  "update",
  "delete",
  "execute",
  "verify",
  "quote",
  "reserve",
  "spend",
  "refund",
  "publish",
  "spawn_child",
] as const;

export const authorityCapabilities = [
  "filesystem_read",
  "filesystem_write",
  "network_egress",
  "secret_read",
  "process_spawn",
  "provider_mutation",
  "public_publication",
  "child_harness_spawn",
  "payment_single_use_spend",
] as const;

export const authorityConditionPredicates = [
  "signal_verified",
  "decision_selected",
  "host_posture_valid",
  "approval_present",
  "within_time_window",
  "within_budget",
  "sandbox_enforced",
  "payment_receipt_present",
  "payment_recovery_available",
] as const;

export const paymentCredentialForms = [
  "single_use_spend_capability",
] as const;

export const proofKinds = [
  "payment_rail",
] as const;

export const redactionCommitmentAlgorithms = [
  "sha256",
] as const;

export const referenceTypeSchema = stringEnum(referenceTypes);
export const signalTypeSchema = stringEnum(signalTypes);
export const signalTrustLevelSchema = stringEnum(signalTrustLevels);
export const harnessStateSchema = stringEnum(harnessStates);
export const harnessSealDispositionSchema = stringEnum(harnessSealDispositions);
export const decisionChoiceSchema = stringEnum(decisionChoices);
export const actFormSchema = stringEnum(actForms);
export const targetLifecycleStateSchema = stringEnum(targetLifecycleStates);
export const thesisProofStrengthSchema = stringEnum(thesisProofStrengths);
export const authorityCostLevelSchema = stringEnum(authorityCostLevels);
export const selectionCycleStateSchema = stringEnum(selectionCycleStates);
export const criterionStatusSchema = stringEnum(criterionStatuses);
export const verificationStatusSchema = stringEnum(verificationStatuses);
export const authorityResourceFamilySchema = stringEnum(authorityResourceFamilies);
export const authorityVerbSchema = stringEnum(authorityVerbs);
export const authorityCapabilitySchema = stringEnum(authorityCapabilities);
export const authorityConditionPredicateSchema = stringEnum(authorityConditionPredicates);
export const paymentCredentialFormSchema = stringEnum(paymentCredentialForms);
export const proofKindSchema = stringEnum(proofKinds);
export const redactionCommitmentAlgorithmSchema = stringEnum(redactionCommitmentAlgorithms);

export const referenceSchema = Type.Object(
  {
    schema: Type.Optional(Type.Literal(RUNX_LOGICAL_SCHEMAS.reference)),
    type: referenceTypeSchema,
    uri: Type.String({ minLength: 1 }),
    provider: Type.Optional(Type.String({ minLength: 1 })),
    locator: Type.Optional(Type.String({ minLength: 1 })),
    label: Type.Optional(Type.String({ minLength: 1 })),
    observed_at: Type.Optional(dateTimeStringSchema()),
    proof_kind: Type.Optional(proofKindSchema),
  },
  {
    $schema: JSON_SCHEMA_DRAFT_2020_12,
    $id: RUNX_CONTRACT_IDS.reference,
    "x-runx-schema": RUNX_LOGICAL_SCHEMAS.reference,
    additionalProperties: false,
  },
);

export const nullableReferenceSchema = Type.Union([referenceSchema, Type.Null()]);

export const actReferenceSchema = Type.Object(
  {
    harness_receipt_ref: referenceSchema,
    act_id: Type.String({ minLength: 1 }),
  },
  { additionalProperties: false },
);

export const hashCommitmentSchema = Type.Object(
  {
    algorithm: redactionCommitmentAlgorithmSchema,
    value: Type.String({ minLength: 1 }),
    canonicalization: Type.String({ minLength: 1 }),
  },
  { additionalProperties: false },
);

export const redactionSchema = Type.Object(
  {
    schema: Type.Literal(RUNX_LOGICAL_SCHEMAS.redaction),
    redaction_id: Type.String({ minLength: 1 }),
    policy_ref: referenceSchema,
    redacted_fields: Type.Array(Type.String({ minLength: 1 })),
    hash_commitments: Type.Array(hashCommitmentSchema),
    canonicalization: Type.String({ minLength: 1 }),
    performed_by_ref: referenceSchema,
    performed_at: dateTimeStringSchema(),
  },
  {
    $schema: JSON_SCHEMA_DRAFT_2020_12,
    $id: RUNX_CONTRACT_IDS.redaction,
    "x-runx-schema": RUNX_LOGICAL_SCHEMAS.redaction,
    additionalProperties: false,
  },
);

export const fingerprintSchema = Type.Object(
  {
    algorithm: Type.Literal("sha256"),
    canonicalization: Type.String({ minLength: 1 }),
    value: Type.String({ minLength: 1 }),
    derived_from: Type.Array(referenceSchema, { minItems: 1 }),
  },
  { additionalProperties: false },
);

export const duplicateCandidateSchema = Type.Object(
  {
    candidate_ref: referenceSchema,
    confidence: Type.Number({ minimum: 0, maximum: 1 }),
    observed_at: dateTimeStringSchema(),
    evidence_refs: Type.Array(referenceSchema),
    reviewer_refs: Type.Optional(Type.Array(referenceSchema)),
  },
  { additionalProperties: false },
);

export const linksSchema = Type.Object(
  {
    duplicate_of: Type.Optional(nullableReferenceSchema),
    duplicate_candidates: Type.Optional(Type.Array(duplicateCandidateSchema)),
    supersedes: Type.Optional(Type.Array(referenceSchema)),
    superseded_by: Type.Optional(Type.Array(referenceSchema)),
    related: Type.Optional(Type.Array(referenceSchema)),
  },
  { additionalProperties: false },
);

export const signalAuthenticitySchema = Type.Object(
  {
    host_ref: referenceSchema,
    principal_ref: Type.Optional(referenceSchema),
    verified_by_ref: Type.Optional(referenceSchema),
    trust_level: signalTrustLevelSchema,
    verified_at: Type.Optional(dateTimeStringSchema()),
    signature_refs: Type.Optional(Type.Array(referenceSchema)),
    evidence_refs: Type.Optional(Type.Array(referenceSchema)),
  },
  { additionalProperties: false },
);

export const signalSchema = Type.Object(
  {
    schema: Type.Literal(RUNX_LOGICAL_SCHEMAS.signal),
    signal_id: Type.String({ minLength: 1 }),
    source_ref: referenceSchema,
    authenticity: Type.Optional(signalAuthenticitySchema),
    signal_type: signalTypeSchema,
    title: Type.String({ minLength: 1 }),
    body_preview: Type.Optional(Type.String({ minLength: 1, maxLength: 2000 })),
    observed_at: dateTimeStringSchema(),
    evidence_refs: Type.Optional(Type.Array(referenceSchema)),
    fingerprint: Type.Optional(fingerprintSchema),
    links: Type.Optional(linksSchema),
    extensions: Type.Optional(unknownRecordSchema()),
  },
  {
    $schema: JSON_SCHEMA_DRAFT_2020_12,
    $id: RUNX_CONTRACT_IDS.signal,
    "x-runx-schema": RUNX_LOGICAL_SCHEMAS.signal,
    additionalProperties: false,
  },
);

export const paymentAuthorityBoundsSchema = Type.Object(
  {
    currency: Type.String({ minLength: 1 }),
    max_per_call_minor: Type.Optional(Type.Integer({ minimum: 0 })),
    max_per_run_minor: Type.Optional(Type.Integer({ minimum: 0 })),
    max_per_period_minor: Type.Optional(Type.Integer({ minimum: 0 })),
    period: Type.Optional(Type.String({ minLength: 1 })),
    rails: Type.Array(Type.String({ minLength: 1 }), { minItems: 1 }),
    realm: Type.Optional(Type.String({ minLength: 1 })),
    counterparty: Type.Optional(Type.String({ minLength: 1 })),
    operation: Type.Optional(Type.String({ minLength: 1 })),
    quote_ttl_ms: Type.Optional(Type.Integer({ minimum: 0 })),
    approval_threshold_minor: Type.Optional(Type.Integer({ minimum: 0 })),
    credential_form: Type.Optional(paymentCredentialFormSchema),
    quote_required: Type.Optional(Type.Boolean()),
    reservation_required: Type.Optional(Type.Boolean()),
    idempotency_required: Type.Optional(Type.Boolean()),
    recovery_required: Type.Optional(Type.Boolean()),
    receipt_before_success: Type.Optional(Type.Boolean()),
    single_use_spend: Type.Optional(Type.Boolean()),
  },
  { additionalProperties: false },
);

export const authorityBoundsSchema = Type.Object(
  {
    repo_path_globs: Type.Optional(Type.Array(Type.String({ minLength: 1 }))),
    branch_patterns: Type.Optional(Type.Array(Type.String({ minLength: 1 }))),
    filesystem_roots: Type.Optional(Type.Array(Type.String({ minLength: 1 }))),
    network_destinations: Type.Optional(Type.Array(Type.String({ minLength: 1 }))),
    deployment_environments: Type.Optional(Type.Array(Type.String({ minLength: 1 }))),
    token_audiences: Type.Optional(Type.Array(Type.String({ minLength: 1 }))),
    max_spend_usd: Type.Optional(Type.Number({ minimum: 0 })),
    payment: Type.Optional(paymentAuthorityBoundsSchema),
    max_runtime_ms: Type.Optional(Type.Integer({ minimum: 0 })),
    max_fanout: Type.Optional(Type.Integer({ minimum: 0 })),
    max_child_depth: Type.Optional(Type.Integer({ minimum: 0 })),
  },
  { additionalProperties: false },
);

export const authorityConditionSchema = Type.Object(
  {
    condition_id: Type.String({ minLength: 1 }),
    predicate: authorityConditionPredicateSchema,
    refs: Type.Optional(Type.Array(referenceSchema)),
    parameters: Type.Optional(unknownRecordSchema()),
  },
  { additionalProperties: false },
);

export const authorityApprovalSchema = Type.Object(
  {
    approval_ref: referenceSchema,
    approved_by_ref: Type.Optional(referenceSchema),
    approved_at: Type.Optional(dateTimeStringSchema()),
    criterion_ids: Type.Optional(Type.Array(Type.String({ minLength: 1 }))),
  },
  { additionalProperties: false },
);

export const authorityTermSchema = Type.Object(
  {
    term_id: Type.String({ minLength: 1 }),
    principal_ref: referenceSchema,
    resource_ref: referenceSchema,
    resource_family: authorityResourceFamilySchema,
    verbs: Type.Array(authorityVerbSchema, { minItems: 1 }),
    bounds: authorityBoundsSchema,
    conditions: Type.Array(authorityConditionSchema),
    approvals: Type.Array(authorityApprovalSchema),
    capabilities: Type.Array(authorityCapabilitySchema),
    expires_at: Type.Optional(dateTimeStringSchema()),
    issued_by_ref: referenceSchema,
    credential_ref: Type.Optional(referenceSchema),
  },
  { additionalProperties: false },
);

export const authoritySubsetComparisonSchema = Type.Object(
  {
    child_term_id: Type.String({ minLength: 1 }),
    parent_term_id: Type.String({ minLength: 1 }),
    relation: stringEnum(["equal", "subset"] as const),
  },
  { additionalProperties: false },
);

export const authoritySubsetProofSchema = Type.Object(
  {
    parent_authority_ref: referenceSchema,
    comparison_algorithm: Type.String({ minLength: 1 }),
    result: Type.Literal("subset"),
    compared_terms: Type.Array(authoritySubsetComparisonSchema, { minItems: 1 }),
    proof_ref: Type.Optional(referenceSchema),
    checked_at: dateTimeStringSchema(),
  },
  {
    $schema: JSON_SCHEMA_DRAFT_2020_12,
    $id: RUNX_CONTRACT_IDS.authoritySubsetProof,
    "x-runx-schema": RUNX_LOGICAL_SCHEMAS.authoritySubsetProof,
    additionalProperties: false,
  },
);

export const harnessAuthorityAttenuationSchema = Type.Object(
  {
    parent_authority_ref: nullableReferenceSchema,
    subset_proof: Type.Union([authoritySubsetProofSchema, Type.Null()]),
  },
  { additionalProperties: false },
);

export const harnessAuthoritySchema = Type.Object(
  {
    schema: Type.Optional(Type.Literal(RUNX_LOGICAL_SCHEMAS.authority)),
    actor_ref: referenceSchema,
    authority_proof_refs: Type.Array(referenceSchema),
    grant_refs: Type.Array(referenceSchema),
    scope_refs: Type.Array(referenceSchema),
    policy_refs: Type.Array(referenceSchema),
    terms: Type.Array(authorityTermSchema),
    attenuation: harnessAuthorityAttenuationSchema,
    mandate_ref: Type.Optional(referenceSchema),
  },
  {
    $schema: JSON_SCHEMA_DRAFT_2020_12,
    $id: RUNX_CONTRACT_IDS.authority,
    "x-runx-schema": RUNX_LOGICAL_SCHEMAS.authority,
    additionalProperties: false,
  },
);

export const successCriterionSchema = Type.Object(
  {
    criterion_id: Type.String({ minLength: 1 }),
    statement: Type.String({ minLength: 1 }),
    required: Type.Boolean(),
  },
  { additionalProperties: false },
);

export const intentSchema = Type.Object(
  {
    purpose: Type.String({ minLength: 1 }),
    legitimacy: Type.String({ minLength: 1 }),
    output: Type.Optional(outputSchema),
    success_criteria: Type.Array(successCriterionSchema),
    constraints: Type.Array(Type.String({ minLength: 1 })),
    derived_from: Type.Array(referenceSchema),
  },
  { additionalProperties: false },
);

export const verificationCheckSchema = Type.Object(
  {
    check_id: Type.String({ minLength: 1 }),
    criterion_ids: Type.Array(Type.String({ minLength: 1 }), { minItems: 1 }),
    status: verificationStatusSchema,
    summary: Type.Optional(Type.String({ minLength: 1 })),
    checked_refs: Type.Optional(Type.Array(referenceSchema)),
    evidence_refs: Type.Array(referenceSchema),
    verified_at: Type.Optional(dateTimeStringSchema()),
  },
  { additionalProperties: false },
);

export const verificationSchema = Type.Object(
  {
    schema: Type.Optional(Type.Literal(RUNX_LOGICAL_SCHEMAS.verification)),
    verification_id: Type.Optional(Type.String({ minLength: 1 })),
    status: verificationStatusSchema,
    checks: Type.Array(verificationCheckSchema),
    verified_at: Type.Optional(dateTimeStringSchema()),
    evidence_refs: Type.Array(referenceSchema),
  },
  {
    $schema: JSON_SCHEMA_DRAFT_2020_12,
    $id: RUNX_CONTRACT_IDS.verification,
    "x-runx-schema": RUNX_LOGICAL_SCHEMAS.verification,
    additionalProperties: false,
  },
);

export const targetSurfaceSchema = Type.Object(
  {
    surface_ref: referenceSchema,
    mutating: Type.Boolean(),
    rationale: Type.Optional(Type.String({ minLength: 1 })),
  },
  { additionalProperties: false },
);

export const changeRequestSchema = Type.Object(
  {
    request_id: Type.String({ minLength: 1 }),
    summary: Type.String({ minLength: 1 }),
    target_surfaces: Type.Array(targetSurfaceSchema),
    success_criteria: Type.Array(successCriterionSchema),
  },
  { additionalProperties: false },
);

export const changePlanSchema = Type.Object(
  {
    plan_id: Type.String({ minLength: 1 }),
    summary: Type.String({ minLength: 1 }),
    steps: Type.Array(Type.String({ minLength: 1 })),
    risks: Type.Optional(Type.Array(Type.String({ minLength: 1 }))),
  },
  { additionalProperties: false },
);

export const revisionDetailsSchema = Type.Object(
  {
    change_request: changeRequestSchema,
    change_plan: changePlanSchema,
    target_surfaces: Type.Array(targetSurfaceSchema),
    invariants: Type.Array(Type.String({ minLength: 1 })),
    verification: Type.Optional(verificationSchema),
    handoff_refs: Type.Array(referenceSchema),
    revision_refs: Type.Optional(Type.Array(referenceSchema)),
  },
  { additionalProperties: false },
);

export const verificationDetailsSchema = Type.Object(
  {
    criterion_ids: Type.Array(Type.String({ minLength: 1 }), { minItems: 1 }),
    verification: verificationSchema,
    deployment_ref: Type.Optional(referenceSchema),
  },
  { additionalProperties: false },
);

export const criterionBindingSchema = Type.Object(
  {
    criterion_id: Type.String({ minLength: 1 }),
    status: criterionStatusSchema,
    evidence_refs: Type.Array(referenceSchema),
    verification_refs: Type.Array(referenceSchema),
    summary: Type.Optional(Type.String({ minLength: 1 })),
  },
  { additionalProperties: false },
);

export const closureSchema = Type.Object(
  {
    disposition: harnessSealDispositionSchema,
    reason_code: Type.String({ minLength: 1 }),
    summary: Type.String({ minLength: 1 }),
    closed_at: dateTimeStringSchema(),
  },
  { additionalProperties: false },
);

export const actSchema = Type.Object(
  {
    schema: Type.Optional(Type.Literal(RUNX_LOGICAL_SCHEMAS.act)),
    act_id: Type.String({ minLength: 1 }),
    form: actFormSchema,
    intent: intentSchema,
    summary: Type.String({ minLength: 1 }),
    closure: closureSchema,
    criterion_bindings: Type.Array(criterionBindingSchema),
    source_refs: Type.Array(referenceSchema),
    target_refs: Type.Array(referenceSchema),
    surface_refs: Type.Array(referenceSchema),
    artifact_refs: Type.Array(referenceSchema),
    verification_refs: Type.Array(referenceSchema),
    harness_refs: Type.Array(referenceSchema),
    revision: Type.Optional(revisionDetailsSchema),
    verification: Type.Optional(verificationDetailsSchema),
    performed_at: dateTimeStringSchema(),
  },
  {
    $schema: JSON_SCHEMA_DRAFT_2020_12,
    $id: RUNX_CONTRACT_IDS.act,
    "x-runx-schema": RUNX_LOGICAL_SCHEMAS.act,
    additionalProperties: false,
  },
);

export const decisionInputsSchema = Type.Object(
  {
    signal_refs: Type.Array(referenceSchema),
    target_ref: nullableReferenceSchema,
    opportunity_refs: Type.Array(referenceSchema),
    selection_ref: nullableReferenceSchema,
  },
  { additionalProperties: false },
);

export const decisionJustificationSchema = Type.Object(
  {
    summary: Type.String({ minLength: 1 }),
    evidence_refs: Type.Array(referenceSchema),
  },
  { additionalProperties: false },
);

export const decisionSchema = Type.Object(
  {
    schema: Type.Optional(Type.Literal(RUNX_LOGICAL_SCHEMAS.decision)),
    decision_id: Type.String({ minLength: 1 }),
    choice: decisionChoiceSchema,
    inputs: decisionInputsSchema,
    proposed_intent: intentSchema,
    selected_act_id: Type.Union([Type.String({ minLength: 1 }), Type.Null()]),
    selected_harness_ref: nullableReferenceSchema,
    justification: decisionJustificationSchema,
    closure: Type.Union([closureSchema, Type.Null()]),
    artifact_refs: Type.Array(referenceSchema),
  },
  {
    $schema: JSON_SCHEMA_DRAFT_2020_12,
    $id: RUNX_CONTRACT_IDS.decision,
    "x-runx-schema": RUNX_LOGICAL_SCHEMAS.decision,
    additionalProperties: false,
  },
);

export const harnessSandboxSchema = Type.Object(
  {
    profile: Type.String({ minLength: 1 }),
    cwd_policy: stringEnum(["workspace", "readonly", "none", "custom"] as const),
    network: stringEnum(["none", "allowlist", "host", "custom"] as const),
    filesystem: stringEnum([
      "none",
      "read_only",
      "workspace",
      "workspace_read_artifact_write",
      "custom",
    ] as const),
  },
  { additionalProperties: false },
);

export const harnessEnforcementSchema = Type.Object(
  {
    harness_ref: Type.Optional(referenceSchema),
    version: Type.String({ minLength: 1 }),
    enforcement_profile_hash: Type.String({ minLength: 1 }),
    enforcer_ref: Type.Optional(referenceSchema),
    sandbox: harnessSandboxSchema,
    redaction_refs: Type.Array(referenceSchema),
    stdout_hash: Type.Optional(hashCommitmentSchema),
    stderr_hash: Type.Optional(hashCommitmentSchema),
    setup_receipt_refs: Type.Optional(Type.Array(referenceSchema)),
    teardown_receipt_refs: Type.Optional(Type.Array(referenceSchema)),
  },
  { additionalProperties: false },
);

export const harnessIdempotencySchema = Type.Object(
  {
    intent_key: Type.String({ minLength: 1 }),
    trigger_fingerprint: Type.String({ minLength: 1 }),
    content_hash: Type.String({ minLength: 1 }),
  },
  { additionalProperties: false },
);

export const harnessRevisionSchema = Type.Object(
  {
    sequence: Type.Integer({ minimum: 1 }),
    previous_ref: nullableReferenceSchema,
  },
  { additionalProperties: false },
);

export const harnessSealCriterionSchema = Type.Object(
  {
    criterion_id: Type.String({ minLength: 1 }),
    status: criterionStatusSchema,
    act_id: Type.Optional(Type.String({ minLength: 1 })),
    verification_refs: Type.Array(referenceSchema),
    evidence_refs: Type.Array(referenceSchema),
    summary: Type.Optional(Type.String({ minLength: 1 })),
  },
  { additionalProperties: false },
);

export const receiptVerificationSummarySchema = Type.Object(
  {
    signature_valid: Type.Boolean(),
    hash_commitments_valid: Type.Boolean(),
    authority_attenuation_valid: Type.Boolean(),
    criteria_bound: Type.Boolean(),
    redaction_valid: Type.Boolean(),
    external_attestations_present: Type.Boolean(),
  },
  { additionalProperties: false },
);

export const harnessSealSchema = Type.Object(
  {
    disposition: harnessSealDispositionSchema,
    reason_code: Type.String({ minLength: 1 }),
    summary: Type.String({ minLength: 1 }),
    closed_at: dateTimeStringSchema(),
    last_observed_at: dateTimeStringSchema(),
    canonicalization: Type.String({ minLength: 1 }),
    digest: Type.String({ minLength: 1 }),
    criteria: Type.Array(harnessSealCriterionSchema),
    verification_summary: Type.Optional(receiptVerificationSummarySchema),
    redaction_refs: Type.Array(referenceSchema),
    artifact_refs: Type.Array(referenceSchema),
    hash_commitments: Type.Array(hashCommitmentSchema),
  },
  { additionalProperties: false },
);

export const harnessSchema = Type.Object(
  {
    schema: Type.Optional(Type.Literal(RUNX_LOGICAL_SCHEMAS.harness)),
    harness_id: Type.String({ minLength: 1 }),
    parent_harness_ref: nullableReferenceSchema,
    state: harnessStateSchema,
    host_ref: referenceSchema,
    harness_ref: referenceSchema,
    authority: harnessAuthoritySchema,
    enforcement: harnessEnforcementSchema,
    idempotency: harnessIdempotencySchema,
    revision: harnessRevisionSchema,
    signal_refs: Type.Array(referenceSchema),
    decisions: Type.Array(decisionSchema),
    acts: Type.Array(actSchema),
    child_harness_receipt_refs: Type.Array(referenceSchema),
    artifact_refs: Type.Array(referenceSchema),
    seal: Type.Union([harnessSealSchema, Type.Null()]),
  },
  {
    $schema: JSON_SCHEMA_DRAFT_2020_12,
    $id: RUNX_CONTRACT_IDS.harness,
    "x-runx-schema": RUNX_LOGICAL_SCHEMAS.harness,
    additionalProperties: false,
  },
);

export const artifactSchema = Type.Object(
  {
    schema: Type.Literal(RUNX_LOGICAL_SCHEMAS.artifact),
    artifact_id: Type.String({ minLength: 1 }),
    artifact_ref: referenceSchema,
    produced_by: Type.Object(
      {
        harness_receipt_ref: Type.Optional(referenceSchema),
        harness_ref: Type.Optional(referenceSchema),
        act_ref: Type.Optional(actReferenceSchema),
        decision_ref: Type.Optional(referenceSchema),
        signal_ref: Type.Optional(referenceSchema),
      },
      { additionalProperties: false },
    ),
    media_type: Type.String({ minLength: 1 }),
    created_at: dateTimeStringSchema(),
    size_bytes: Type.Integer({ minimum: 0 }),
    hash: hashCommitmentSchema,
    redaction_refs: Type.Array(referenceSchema),
    source_refs: Type.Array(referenceSchema),
    data_ref: Type.Optional(referenceSchema),
    summary: Type.Optional(Type.String({ minLength: 1 })),
    extensions: Type.Optional(unknownRecordSchema()),
  },
  {
    $schema: JSON_SCHEMA_DRAFT_2020_12,
    $id: RUNX_CONTRACT_IDS.artifact,
    "x-runx-schema": RUNX_LOGICAL_SCHEMAS.artifact,
    additionalProperties: false,
  },
);

export const harnessReceiptIssuerSchema = Type.Object(
  {
    type: stringEnum(["local", "hosted", "ci", "verifier"] as const),
    kid: Type.String({ minLength: 1 }),
    public_key_sha256: Type.String({ minLength: 1 }),
  },
  { additionalProperties: false },
);

export const harnessReceiptSignatureSchema = Type.Object(
  {
    alg: Type.Literal("Ed25519"),
    value: Type.String({ minLength: 1 }),
  },
  { additionalProperties: false },
);

export const fanoutReceiptSyncPointSchema = Type.Object(
  {
    group_id: Type.String({ minLength: 1 }),
    strategy: stringEnum(["all", "any", "quorum"] as const),
    decision: stringEnum(["proceed", "halt", "pause", "escalate"] as const),
    rule_fired: Type.String({ minLength: 1 }),
    reason: Type.String({ minLength: 1 }),
    branch_count: Type.Integer({ minimum: 0 }),
    success_count: Type.Integer({ minimum: 0 }),
    failure_count: Type.Integer({ minimum: 0 }),
    required_successes: Type.Integer({ minimum: 0 }),
    branch_receipts: Type.Array(Type.String({ minLength: 1 })),
    gate: Type.Optional(unknownRecordSchema()),
  },
  { additionalProperties: false },
);

export const harnessReceiptEnvelopeSchema = Type.Object(
  {
    schema: Type.Literal(RUNX_LOGICAL_SCHEMAS.harnessReceipt),
    id: Type.String({ minLength: 1 }),
    created_at: dateTimeStringSchema(),
    issuer: harnessReceiptIssuerSchema,
    signature: harnessReceiptSignatureSchema,
    harness: harnessSchema,
    seal: harnessSealSchema,
    sync_points: Type.Optional(Type.Array(fanoutReceiptSyncPointSchema)),
    metadata: Type.Optional(unknownRecordSchema()),
  },
  {
    $schema: JSON_SCHEMA_DRAFT_2020_12,
    $id: RUNX_CONTRACT_IDS.harnessReceipt,
    "x-runx-schema": RUNX_LOGICAL_SCHEMAS.harnessReceipt,
    additionalProperties: false,
  },
);

export const harnessReceiptSchema = harnessReceiptEnvelopeSchema;

export const targetCooldownSchema = Type.Object(
  {
    state: stringEnum(["none", "cooling_down"] as const),
    until: Type.Optional(dateTimeStringSchema()),
    reason_code: Type.Optional(Type.String({ minLength: 1 })),
  },
  { additionalProperties: false },
);

export const targetSchema = Type.Object(
  {
    schema: Type.Literal(RUNX_LOGICAL_SCHEMAS.target),
    target_id: Type.String({ minLength: 1 }),
    target_ref: referenceSchema,
    title: Type.String({ minLength: 1 }),
    summary: Type.Optional(Type.String({ minLength: 1 })),
    lifecycle_state: targetLifecycleStateSchema,
    authority_refs: Type.Array(referenceSchema),
    fingerprint: fingerprintSchema,
    links: Type.Optional(linksSchema),
    cooldown: targetCooldownSchema,
    verification_recipe_refs: Type.Array(referenceSchema),
    owner_refs: Type.Optional(Type.Array(referenceSchema)),
    created_at: dateTimeStringSchema(),
    updated_at: dateTimeStringSchema(),
  },
  {
    $schema: JSON_SCHEMA_DRAFT_2020_12,
    $id: RUNX_CONTRACT_IDS.target,
    "x-runx-schema": RUNX_LOGICAL_SCHEMAS.target,
    additionalProperties: false,
  },
);

export const opportunitySchema = Type.Object(
  {
    schema: Type.Literal(RUNX_LOGICAL_SCHEMAS.opportunity),
    opportunity_id: Type.String({ minLength: 1 }),
    target_ref: referenceSchema,
    summary: Type.String({ minLength: 1 }),
    proposed_form: actFormSchema,
    value_score: Type.Integer({ minimum: 0, maximum: 100 }),
    risk_score: Type.Integer({ minimum: 0, maximum: 100 }),
    freshness_expires_at: dateTimeStringSchema(),
    fingerprint: fingerprintSchema,
    links: Type.Optional(linksSchema),
    source_refs: Type.Array(referenceSchema),
    evidence_refs: Type.Array(referenceSchema),
    discovered_at: dateTimeStringSchema(),
  },
  {
    $schema: JSON_SCHEMA_DRAFT_2020_12,
    $id: RUNX_CONTRACT_IDS.opportunity,
    "x-runx-schema": RUNX_LOGICAL_SCHEMAS.opportunity,
    additionalProperties: false,
  },
);

export const thesisAssessmentSchema = Type.Object(
  {
    schema: Type.Literal(RUNX_LOGICAL_SCHEMAS.thesisAssessment),
    assessment_id: Type.String({ minLength: 1 }),
    target_ref: referenceSchema,
    opportunity_ref: referenceSchema,
    thesis_ref: referenceSchema,
    score: Type.Integer({ minimum: 0, maximum: 100 }),
    rubric_refs: Type.Array(referenceSchema),
    proof_strength: thesisProofStrengthSchema,
    authority_cost: authorityCostLevelSchema,
    rationale: Type.String({ minLength: 1 }),
    evidence_refs: Type.Array(referenceSchema),
    assessed_at: dateTimeStringSchema(),
  },
  {
    $schema: JSON_SCHEMA_DRAFT_2020_12,
    $id: RUNX_CONTRACT_IDS.thesisAssessment,
    "x-runx-schema": RUNX_LOGICAL_SCHEMAS.thesisAssessment,
    additionalProperties: false,
  },
);

export const selectionSchema = Type.Object(
  {
    schema: Type.Literal(RUNX_LOGICAL_SCHEMAS.selection),
    selection_id: Type.String({ minLength: 1 }),
    cycle_ref: referenceSchema,
    opportunity_ref: referenceSchema,
    candidate_refs: Type.Array(referenceSchema, { minItems: 1 }),
    rank: Type.Integer({ minimum: 1 }),
    score: Type.Integer({ minimum: 0, maximum: 100 }),
    selected: Type.Boolean(),
    reason: Type.String({ minLength: 1 }),
    cooldown_until: Type.Optional(dateTimeStringSchema()),
    decision_ref: nullableReferenceSchema,
    evidence_refs: Type.Array(referenceSchema),
    selected_at: dateTimeStringSchema(),
  },
  {
    $schema: JSON_SCHEMA_DRAFT_2020_12,
    $id: RUNX_CONTRACT_IDS.selection,
    "x-runx-schema": RUNX_LOGICAL_SCHEMAS.selection,
    additionalProperties: false,
  },
);

export const skillBindingSchema = Type.Object(
  {
    schema: Type.Literal(RUNX_LOGICAL_SCHEMAS.skillBinding),
    binding_id: Type.String({ minLength: 1 }),
    skill_ref: referenceSchema,
    scope_family: authorityResourceFamilySchema,
    allowed_act_forms: Type.Array(actFormSchema, { minItems: 1 }),
    authority_refs: Type.Array(referenceSchema),
    policy_refs: Type.Array(referenceSchema),
    harness_template_ref: nullableReferenceSchema,
    active: Type.Boolean(),
    created_at: dateTimeStringSchema(),
    updated_at: dateTimeStringSchema(),
  },
  {
    $schema: JSON_SCHEMA_DRAFT_2020_12,
    $id: RUNX_CONTRACT_IDS.skillBinding,
    "x-runx-schema": RUNX_LOGICAL_SCHEMAS.skillBinding,
    additionalProperties: false,
  },
);

export const targetTransitionEntrySchema = Type.Object(
  {
    schema: Type.Literal(RUNX_LOGICAL_SCHEMAS.targetTransitionEntry),
    entry_id: Type.String({ minLength: 1 }),
    target_ref: referenceSchema,
    from_state: Type.Union([targetLifecycleStateSchema, Type.Null()]),
    to_state: targetLifecycleStateSchema,
    reason_code: Type.String({ minLength: 1 }),
    summary: Type.String({ minLength: 1 }),
    source_refs: Type.Array(referenceSchema),
    decision_ref: nullableReferenceSchema,
    harness_receipt_ref: nullableReferenceSchema,
    recorded_at: dateTimeStringSchema(),
  },
  {
    $schema: JSON_SCHEMA_DRAFT_2020_12,
    $id: RUNX_CONTRACT_IDS.targetTransitionEntry,
    "x-runx-schema": RUNX_LOGICAL_SCHEMAS.targetTransitionEntry,
    additionalProperties: false,
  },
);

export const selectionCycleSchema = Type.Object(
  {
    schema: Type.Literal(RUNX_LOGICAL_SCHEMAS.selectionCycle),
    cycle_id: Type.String({ minLength: 1 }),
    state: selectionCycleStateSchema,
    started_at: dateTimeStringSchema(),
    closed_at: Type.Union([dateTimeStringSchema(), Type.Null()]),
    input_refs: Type.Array(referenceSchema),
    target_refs: Type.Array(referenceSchema),
    opportunity_refs: Type.Array(referenceSchema),
    ranked_selection_refs: Type.Array(referenceSchema),
    chosen_selection_ref: nullableReferenceSchema,
    decision_ref: nullableReferenceSchema,
    harness_receipt_ref: nullableReferenceSchema,
    no_action_closure: Type.Union([closureSchema, Type.Null()]),
    fingerprint: fingerprintSchema,
  },
  {
    $schema: JSON_SCHEMA_DRAFT_2020_12,
    $id: RUNX_CONTRACT_IDS.selectionCycle,
    "x-runx-schema": RUNX_LOGICAL_SCHEMAS.selectionCycle,
    additionalProperties: false,
  },
);

export const reflectionEntrySchema = Type.Object(
  {
    schema: Type.Literal(RUNX_LOGICAL_SCHEMAS.reflectionEntry),
    reflection_id: Type.String({ minLength: 1 }),
    target_ref: nullableReferenceSchema,
    opportunity_ref: nullableReferenceSchema,
    selection_ref: nullableReferenceSchema,
    decision_ref: nullableReferenceSchema,
    harness_receipt_refs: Type.Array(referenceSchema),
    act_refs: Type.Array(actReferenceSchema),
    summary: Type.String({ minLength: 1 }),
    lessons: Type.Array(Type.String({ minLength: 1 })),
    follow_up_refs: Type.Array(referenceSchema),
    evidence_refs: Type.Array(referenceSchema),
    recorded_at: dateTimeStringSchema(),
  },
  {
    $schema: JSON_SCHEMA_DRAFT_2020_12,
    $id: RUNX_CONTRACT_IDS.reflectionEntry,
    "x-runx-schema": RUNX_LOGICAL_SCHEMAS.reflectionEntry,
    additionalProperties: false,
  },
);

export const feedEntrySchema = Type.Object(
  {
    schema: Type.Literal(RUNX_LOGICAL_SCHEMAS.feedEntry),
    feed_entry_id: Type.String({ minLength: 1 }),
    public_at: dateTimeStringSchema(),
    title: Type.String({ minLength: 1 }),
    summary: Type.String({ minLength: 1 }),
    target_ref: nullableReferenceSchema,
    opportunity_ref: nullableReferenceSchema,
    selection_ref: nullableReferenceSchema,
    decision_refs: Type.Array(referenceSchema, { minItems: 1 }),
    harness_receipt_refs: Type.Array(referenceSchema, { minItems: 1 }),
    act_refs: Type.Array(actReferenceSchema, { minItems: 1 }),
    verification_refs: Type.Array(referenceSchema, { minItems: 1 }),
    evidence_refs: Type.Array(referenceSchema, { minItems: 1 }),
    artifact_refs: Type.Array(referenceSchema),
    redaction_policy_ref: referenceSchema,
    redaction_refs: Type.Array(referenceSchema),
  },
  {
    $schema: JSON_SCHEMA_DRAFT_2020_12,
    $id: RUNX_CONTRACT_IDS.feedEntry,
    "x-runx-schema": RUNX_LOGICAL_SCHEMAS.feedEntry,
    additionalProperties: false,
  },
);

export type ReferenceTypeContract = DeepReadonly<Static<typeof referenceTypeSchema>>;
export type SignalTypeContract = DeepReadonly<Static<typeof signalTypeSchema>>;
export type SignalTrustLevelContract = DeepReadonly<Static<typeof signalTrustLevelSchema>>;
export type HarnessStateContract = DeepReadonly<Static<typeof harnessStateSchema>>;
export type HarnessSealDispositionContract = DeepReadonly<Static<typeof harnessSealDispositionSchema>>;
export type DecisionChoiceContract = DeepReadonly<Static<typeof decisionChoiceSchema>>;
export type ActFormContract = DeepReadonly<Static<typeof actFormSchema>>;
export type TargetLifecycleStateContract = DeepReadonly<Static<typeof targetLifecycleStateSchema>>;
export type ThesisProofStrengthContract = DeepReadonly<Static<typeof thesisProofStrengthSchema>>;
export type AuthorityCostLevelContract = DeepReadonly<Static<typeof authorityCostLevelSchema>>;
export type SelectionCycleStateContract = DeepReadonly<Static<typeof selectionCycleStateSchema>>;
export type CriterionStatusContract = DeepReadonly<Static<typeof criterionStatusSchema>>;
export type VerificationStatusContract = DeepReadonly<Static<typeof verificationStatusSchema>>;
export type AuthorityResourceFamilyContract = DeepReadonly<Static<typeof authorityResourceFamilySchema>>;
export type AuthorityVerbContract = DeepReadonly<Static<typeof authorityVerbSchema>>;
export type AuthorityCapabilityContract = DeepReadonly<Static<typeof authorityCapabilitySchema>>;
export type AuthorityConditionPredicateContract = DeepReadonly<Static<typeof authorityConditionPredicateSchema>>;
export type PaymentCredentialFormContract = DeepReadonly<Static<typeof paymentCredentialFormSchema>>;
export type ProofKindContract = DeepReadonly<Static<typeof proofKindSchema>>;
export type ReferenceContract = DeepReadonly<Static<typeof referenceSchema>>;
export type ActReferenceContract = DeepReadonly<Static<typeof actReferenceSchema>>;
export type HashCommitmentContract = DeepReadonly<Static<typeof hashCommitmentSchema>>;
export type RedactionContract = DeepReadonly<Static<typeof redactionSchema>>;
export type FingerprintContract = DeepReadonly<Static<typeof fingerprintSchema>>;
export type LinksContract = DeepReadonly<Static<typeof linksSchema>>;
export type SignalAuthenticityContract = DeepReadonly<Static<typeof signalAuthenticitySchema>>;
export type SignalContract = DeepReadonly<Static<typeof signalSchema>>;
export type PaymentAuthorityBoundsContract = DeepReadonly<Static<typeof paymentAuthorityBoundsSchema>>;
export type AuthorityBoundsContract = DeepReadonly<Static<typeof authorityBoundsSchema>>;
export type AuthorityConditionContract = DeepReadonly<Static<typeof authorityConditionSchema>>;
export type AuthorityApprovalContract = DeepReadonly<Static<typeof authorityApprovalSchema>>;
export type AuthorityTermContract = DeepReadonly<Static<typeof authorityTermSchema>>;
export type AuthoritySubsetProofContract = DeepReadonly<Static<typeof authoritySubsetProofSchema>>;
export type HarnessAuthorityContract = DeepReadonly<Static<typeof harnessAuthoritySchema>>;
export type SuccessCriterionContract = DeepReadonly<Static<typeof successCriterionSchema>>;
export type IntentContract = DeepReadonly<Static<typeof intentSchema>>;
export type VerificationCheckContract = DeepReadonly<Static<typeof verificationCheckSchema>>;
export type VerificationContract = DeepReadonly<Static<typeof verificationSchema>>;
export type TargetSurfaceContract = DeepReadonly<Static<typeof targetSurfaceSchema>>;
export type ChangeRequestContract = DeepReadonly<Static<typeof changeRequestSchema>>;
export type ChangePlanContract = DeepReadonly<Static<typeof changePlanSchema>>;
export type RevisionDetailsContract = DeepReadonly<Static<typeof revisionDetailsSchema>>;
export type VerificationDetailsContract = DeepReadonly<Static<typeof verificationDetailsSchema>>;
export type CriterionBindingContract = DeepReadonly<Static<typeof criterionBindingSchema>>;
export type ActContract = DeepReadonly<Static<typeof actSchema>>;
export type DecisionInputsContract = DeepReadonly<Static<typeof decisionInputsSchema>>;
export type DecisionJustificationContract = DeepReadonly<Static<typeof decisionJustificationSchema>>;
export type ClosureRecordContract = DeepReadonly<Static<typeof closureSchema>>;
export type DecisionContract = DeepReadonly<Static<typeof decisionSchema>>;
export type HarnessSandboxContract = DeepReadonly<Static<typeof harnessSandboxSchema>>;
export type HarnessEnforcementContract = DeepReadonly<Static<typeof harnessEnforcementSchema>>;
export type HarnessIdempotencyContract = DeepReadonly<Static<typeof harnessIdempotencySchema>>;
export type HarnessRevisionContract = DeepReadonly<Static<typeof harnessRevisionSchema>>;
export type HarnessSealCriterionContract = DeepReadonly<Static<typeof harnessSealCriterionSchema>>;
export type ReceiptVerificationSummaryContract = DeepReadonly<Static<typeof receiptVerificationSummarySchema>>;
export type HarnessSealContract = DeepReadonly<Static<typeof harnessSealSchema>>;
export type HarnessContract = DeepReadonly<Static<typeof harnessSchema>>;
export type ArtifactContract = DeepReadonly<Static<typeof artifactSchema>>;
export type HarnessReceiptIssuerContract = DeepReadonly<Static<typeof harnessReceiptIssuerSchema>>;
export type HarnessReceiptSignatureContract = DeepReadonly<Static<typeof harnessReceiptSignatureSchema>>;
export type FanoutReceiptSyncPointContract = DeepReadonly<Static<typeof fanoutReceiptSyncPointSchema>>;
export type HarnessReceiptContract = DeepReadonly<Static<typeof harnessReceiptEnvelopeSchema>>;
export type HarnessReceiptEnvelopeContract = HarnessReceiptContract;
export type TargetCooldownContract = DeepReadonly<Static<typeof targetCooldownSchema>>;
export type TargetContract = DeepReadonly<Static<typeof targetSchema>>;
export type OpportunityContract = DeepReadonly<Static<typeof opportunitySchema>>;
export type ThesisAssessmentContract = DeepReadonly<Static<typeof thesisAssessmentSchema>>;
export type SelectionContract = DeepReadonly<Static<typeof selectionSchema>>;
export type SkillBindingContract = DeepReadonly<Static<typeof skillBindingSchema>>;
export type TargetTransitionEntryContract = DeepReadonly<Static<typeof targetTransitionEntrySchema>>;
export type SelectionCycleContract = DeepReadonly<Static<typeof selectionCycleSchema>>;
export type ReflectionEntryContract = DeepReadonly<Static<typeof reflectionEntrySchema>>;
export type FeedEntryContract = DeepReadonly<Static<typeof feedEntrySchema>>;

export function validateReferenceContract(value: unknown, label = "reference"): ReferenceContract {
  return validateContractSchema(referenceSchema, value, label);
}

export function validateSignalContract(value: unknown, label = "signal"): SignalContract {
  return validateContractSchema(signalSchema, value, label);
}

export function validateAuthorityContract(value: unknown, label = "authority"): HarnessAuthorityContract {
  return validateContractSchema(harnessAuthoritySchema, value, label);
}

export function validateAuthoritySubsetProofContract(
  value: unknown,
  label = "authority_subset_proof",
): AuthoritySubsetProofContract {
  return validateContractSchema(authoritySubsetProofSchema, value, label);
}

export function validateDecisionContract(value: unknown, label = "decision"): DecisionContract {
  return validateContractSchema(decisionSchema, value, label);
}

export function validateActContract(value: unknown, label = "act"): ActContract {
  const act = validateContractSchema(actSchema, value, label);
  assertActFormDetails(act, label);
  return act;
}

export function validateVerificationContract(value: unknown, label = "verification"): VerificationContract {
  return validateContractSchema(verificationSchema, value, label);
}

export function validateHarnessContract(value: unknown, label = "harness"): HarnessContract {
  const harness = validateContractSchema(harnessSchema, value, label);
  assertHarnessSealState(harness, label);
  for (const [index, act] of harness.acts.entries()) {
    assertActFormDetails(act, `${label}.acts[${index}]`);
  }
  return harness;
}

export function validateHarnessReceiptContract(
  value: unknown,
  label = "harness_receipt",
): HarnessReceiptContract {
  const receipt = validateContractSchema(harnessReceiptEnvelopeSchema, value, label);
  assertHarnessSealState(receipt.harness, `${label}.harness`);
  if (!sameJsonValue(receipt.harness.seal, receipt.seal)) {
    throw new Error(`${label}.seal must match ${label}.harness.seal.`);
  }
  return receipt;
}

export function validateSpineArtifactContract(value: unknown, label = "artifact"): ArtifactContract {
  return validateContractSchema(artifactSchema, value, label);
}

export function validateRedactionContract(value: unknown, label = "redaction"): RedactionContract {
  return validateContractSchema(redactionSchema, value, label);
}

export function validateTargetContract(value: unknown, label = "target"): TargetContract {
  return validateContractSchema(targetSchema, value, label);
}

export function validateOpportunityContract(value: unknown, label = "opportunity"): OpportunityContract {
  return validateContractSchema(opportunitySchema, value, label);
}

export function validateThesisAssessmentContract(
  value: unknown,
  label = "thesis_assessment",
): ThesisAssessmentContract {
  return validateContractSchema(thesisAssessmentSchema, value, label);
}

export function validateSelectionContract(value: unknown, label = "selection"): SelectionContract {
  return validateContractSchema(selectionSchema, value, label);
}

export function validateSkillBindingContract(value: unknown, label = "skill_binding"): SkillBindingContract {
  return validateContractSchema(skillBindingSchema, value, label);
}

export function validateTargetTransitionEntryContract(
  value: unknown,
  label = "target_transition_entry",
): TargetTransitionEntryContract {
  return validateContractSchema(targetTransitionEntrySchema, value, label);
}

export function validateSelectionCycleContract(
  value: unknown,
  label = "selection_cycle",
): SelectionCycleContract {
  return validateContractSchema(selectionCycleSchema, value, label);
}

export function validateReflectionEntryContract(
  value: unknown,
  label = "reflection_entry",
): ReflectionEntryContract {
  return validateContractSchema(reflectionEntrySchema, value, label);
}

export function validateFeedEntryContract(value: unknown, label = "feed_entry"): FeedEntryContract {
  return validateContractSchema(feedEntrySchema, value, label);
}

function assertActFormDetails(act: ActContract, label: string): void {
  if (act.form === "revision") {
    if (!act.revision) {
      throw new Error(`${label}.revision is required when form is revision.`);
    }
    if (act.verification) {
      throw new Error(`${label}.verification must be omitted when form is revision.`);
    }
    return;
  }
  if (act.form === "verification") {
    if (!act.verification) {
      throw new Error(`${label}.verification is required when form is verification.`);
    }
    if (act.revision) {
      throw new Error(`${label}.revision must be omitted when form is verification.`);
    }
    return;
  }
  if (act.revision || act.verification) {
    throw new Error(`${label} must not carry revision or verification details when form is ${act.form}.`);
  }
}

function assertHarnessSealState(harness: HarnessContract, label: string): void {
  const terminalStates = new Set<HarnessStateContract>([
    "sealed",
    "killed",
    "timed_out",
    "failed",
    "superseded",
  ]);
  const terminal = terminalStates.has(harness.state);
  if (terminal && harness.seal === null) {
    throw new Error(`${label}.seal is required when state is ${harness.state}.`);
  }
  if (!terminal && harness.seal !== null) {
    throw new Error(`${label}.seal must be null when state is ${harness.state}.`);
  }
}

function sameJsonValue(left: unknown, right: unknown): boolean {
  if (left === right) {
    return true;
  }
  if (Array.isArray(left) || Array.isArray(right)) {
    return Array.isArray(left)
      && Array.isArray(right)
      && left.length === right.length
      && left.every((entry, index) => sameJsonValue(entry, right[index]));
  }
  if (!left || !right || typeof left !== "object" || typeof right !== "object") {
    return false;
  }
  const leftRecord = left as Readonly<Record<string, unknown>>;
  const rightRecord = right as Readonly<Record<string, unknown>>;
  const leftKeys = Object.keys(leftRecord).sort();
  const rightKeys = Object.keys(rightRecord).sort();
  return leftKeys.length === rightKeys.length
    && leftKeys.every((key, index) => key === rightKeys[index] && sameJsonValue(leftRecord[key], rightRecord[key]));
}
