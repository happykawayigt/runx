// rust-style-allow: large-file because receipt construction, explicit
// signature policy, and local proof sealing stay together until the runtime
// receipt builder is split out.
use runx_contracts::{
    ActForm, AuthorityAttenuation, AuthoritySubsetResult, ClosureDisposition, CriterionStatus,
    Decision, DecisionChoice, DecisionInputs, DecisionJustification, FanoutReceiptSyncPoint,
    JsonObject, JsonValue, Lineage, ProofKind, RECEIPT_CANONICALIZATION, Receipt, ReceiptAct,
    ReceiptAuthority, ReceiptCriterion, ReceiptEnforcement, ReceiptIdempotency, ReceiptIssuer,
    ReceiptIssuerType, ReceiptSchema, ReceiptSubjectKind, Reference, ReferenceType, Seal,
    SignatureAlgorithm, Subject,
};
use runx_receipts::{
    ReceiptJournal, ReceiptJournalDecision, ReceiptProofContext, ReceiptProofContextProvider,
    ReceiptSignature, ReceiptTreeConfig, SignatureVerificationFailure, SignatureVerifier,
    canonical_receipt_body_digest,
};

use crate::adapter::SkillOutput;
use crate::payment_supervisor::rebind_supervisor_proof_to_receipt;
use crate::{RuntimeError, StepRun};

use super::signing::{
    RuntimeReceiptSigner, RuntimeReceiptSigningError, is_local_pseudo_signature,
    validate_production_issuer,
};
pub fn step_receipt(
    graph_name: &str,
    step_id: &str,
    attempt: u32,
    output: &SkillOutput,
    created_at: &str,
) -> Result<Receipt, RuntimeError> {
    let disposition = disposition(output);
    step_receipt_with_disposition(StepReceiptWithDisposition {
        graph_name,
        step_id,
        attempt,
        output,
        created_at,
        reason_code: process_reason_code(&disposition),
        disposition,
        summary: format!("step {step_id} completed"),
    })
}

pub fn step_receipt_with_signature_policy(
    graph_name: &str,
    step_id: &str,
    attempt: u32,
    output: &SkillOutput,
    created_at: &str,
    signature_policy: RuntimeReceiptSignaturePolicy<'_>,
) -> Result<Receipt, RuntimeError> {
    let disposition = disposition(output);
    step_receipt_with_disposition_and_policy(
        StepReceiptWithDisposition {
            graph_name,
            step_id,
            attempt,
            output,
            created_at,
            reason_code: process_reason_code(&disposition),
            disposition,
            summary: format!("step {step_id} completed"),
        },
        signature_policy,
    )
}

pub(crate) struct StepReceiptWithDisposition<'a> {
    pub(crate) graph_name: &'a str,
    pub(crate) step_id: &'a str,
    pub(crate) attempt: u32,
    pub(crate) output: &'a SkillOutput,
    pub(crate) created_at: &'a str,
    pub(crate) disposition: ClosureDisposition,
    pub(crate) reason_code: String,
    pub(crate) summary: String,
}

pub(crate) fn step_receipt_with_disposition(
    params: StepReceiptWithDisposition<'_>,
) -> Result<Receipt, RuntimeError> {
    step_receipt_with_disposition_and_policy(
        params,
        RuntimeReceiptSignaturePolicy::local_development(),
    )
}

pub(crate) fn step_receipt_with_disposition_and_policy(
    params: StepReceiptWithDisposition<'_>,
    signature_policy: RuntimeReceiptSignaturePolicy<'_>,
) -> Result<Receipt, RuntimeError> {
    let StepReceiptWithDisposition {
        graph_name,
        step_id,
        attempt,
        output,
        created_at,
        disposition,
        reason_code,
        summary,
    } = params;
    let output_refs = output_refs(output);
    let act = observation_act(step_id, output, disposition.clone(), &output_refs);
    let seal_criterion = ReceiptCriterion {
        criterion_id: "process_exit".to_owned(),
        status: if output.succeeded() {
            CriterionStatus::Verified
        } else {
            CriterionStatus::Failed
        },
        evidence_refs: output_refs.source_refs.clone(),
        verification_refs: output_refs.verification_refs.clone(),
        summary: Some(output_summary(output)),
    };
    let seal = seal(disposition, reason_code, summary, created_at, vec![seal_criterion]);
    let mut receipt = build_receipt(BuildReceipt {
        id: step_receipt_id(graph_name, step_id, attempt),
        graph_name,
        node_id: step_id,
        kind: ReceiptSubjectKind::Skill,
        created_at,
        acts: vec![act],
        seal,
        children: Vec::new(),
        sync_points: Vec::new(),
        signal_refs: output_refs.signal_refs,
    });
    seal_receipt(&mut receipt, signature_policy)?;
    Ok(receipt)
}

pub fn graph_receipt(
    graph_name: &str,
    steps: &mut [StepRun],
    sync_points: Vec<FanoutReceiptSyncPoint>,
    created_at: &str,
) -> Result<Receipt, RuntimeError> {
    graph_receipt_with_disposition(
        graph_name,
        steps,
        sync_points,
        created_at,
        ClosureDisposition::Closed,
        "graph_closed".to_owned(),
        format!("graph {graph_name} completed"),
    )
}

pub fn graph_receipt_with_signature_policy(
    graph_name: &str,
    steps: &mut [StepRun],
    sync_points: Vec<FanoutReceiptSyncPoint>,
    created_at: &str,
    signature_policy: RuntimeReceiptSignaturePolicy<'_>,
) -> Result<Receipt, RuntimeError> {
    graph_receipt_with_disposition_and_policy(
        graph_name,
        steps,
        sync_points,
        created_at,
        GraphClosure {
            disposition: ClosureDisposition::Closed,
            reason_code: "graph_closed".to_owned(),
            summary: format!("graph {graph_name} completed"),
        },
        signature_policy,
    )
}

pub(crate) fn graph_receipt_with_disposition(
    graph_name: &str,
    steps: &mut [StepRun],
    sync_points: Vec<FanoutReceiptSyncPoint>,
    created_at: &str,
    disposition: ClosureDisposition,
    reason_code: String,
    summary: String,
) -> Result<Receipt, RuntimeError> {
    graph_receipt_with_disposition_and_policy(
        graph_name,
        steps,
        sync_points,
        created_at,
        GraphClosure {
            disposition,
            reason_code,
            summary,
        },
        RuntimeReceiptSignaturePolicy::local_development(),
    )
}

struct GraphClosure {
    disposition: ClosureDisposition,
    reason_code: String,
    summary: String,
}

fn graph_receipt_with_disposition_and_policy(
    graph_name: &str,
    steps: &mut [StepRun],
    sync_points: Vec<FanoutReceiptSyncPoint>,
    created_at: &str,
    closure: GraphClosure,
    signature_policy: RuntimeReceiptSignaturePolicy<'_>,
) -> Result<Receipt, RuntimeError> {
    let receipt_id = format!("hrn_rcpt_{graph_name}");
    attach_parent_to_child_receipts(steps, &receipt_id, signature_policy)?;
    let child_refs = steps
        .iter()
        .map(|step| child_receipt_reference(&step.receipt))
        .collect::<Vec<_>>();
    let seal = seal(
        closure.disposition,
        closure.reason_code,
        closure.summary,
        created_at,
        Vec::new(),
    );
    let mut receipt = build_receipt(BuildReceipt {
        id: receipt_id,
        graph_name,
        node_id: "graph",
        kind: ReceiptSubjectKind::Graph,
        created_at,
        acts: Vec::new(),
        seal,
        children: child_refs,
        sync_points,
        signal_refs: Vec::new(),
    });
    seal_receipt(&mut receipt, signature_policy)?;
    let children = steps
        .iter()
        .map(|step| step.receipt.clone())
        .collect::<Vec<_>>();
    validate_receipt_tree_with_policy(&receipt, &children, signature_policy)?;
    Ok(receipt)
}

fn validate_receipt_tree_with_policy(
    root: &Receipt,
    children: &[Receipt],
    signature_policy: RuntimeReceiptSignaturePolicy<'_>,
) -> Result<(), RuntimeError> {
    super::tree::validate_runtime_receipt_tree_with_policy(
        root,
        children.iter().cloned(),
        ReceiptTreeConfig::default(),
        signature_policy,
    )
    .map_err(receipt_error)
}

fn step_receipt_id(graph_name: &str, step_id: &str, attempt: u32) -> String {
    if attempt <= 1 {
        format!("hrn_rcpt_{graph_name}_{step_id}")
    } else {
        format!("hrn_rcpt_{graph_name}_{step_id}_attempt_{attempt}")
    }
}

fn process_reason_code(disposition: &ClosureDisposition) -> String {
    let suffix = match disposition {
        ClosureDisposition::Closed => "closed",
        ClosureDisposition::Deferred => "deferred",
        ClosureDisposition::Superseded => "superseded",
        ClosureDisposition::Declined => "declined",
        ClosureDisposition::Blocked => "blocked",
        ClosureDisposition::Failed => "failed",
        ClosureDisposition::Killed => "killed",
        ClosureDisposition::TimedOut => "timed_out",
    };
    format!("process_{suffix}")
}

struct BuildReceipt<'a> {
    id: String,
    graph_name: &'a str,
    node_id: &'a str,
    kind: ReceiptSubjectKind,
    created_at: &'a str,
    acts: Vec<ReceiptAct>,
    seal: Seal,
    children: Vec<Reference>,
    sync_points: Vec<FanoutReceiptSyncPoint>,
    signal_refs: Vec<Reference>,
}

fn build_receipt(parts: BuildReceipt<'_>) -> Receipt {
    let BuildReceipt {
        id,
        graph_name,
        node_id,
        kind,
        created_at,
        acts,
        seal,
        children,
        sync_points,
        signal_refs,
    } = parts;
    let lineage = Lineage {
        parent: None,
        previous: None,
        children,
        sync: sync_points,
        signal_refs,
        journal_ref: None,
        resume_ref: None,
    };
    Receipt {
        schema: ReceiptSchema::V1,
        id,
        created_at: created_at.to_owned(),
        canonicalization: RECEIPT_CANONICALIZATION.to_owned(),
        issuer: local_issuer(),
        signature: placeholder_signature(),
        digest: "sha256:runtime-skeleton".to_owned(),
        idempotency: idempotency(graph_name, node_id),
        subject: subject(graph_name, node_id, kind),
        authority: authority(),
        acts,
        seal,
        lineage: Some(lineage),
        metadata: None,
    }
}

/// The planner deliberation written beside the receipt and committed by
/// `lineage.journal_ref`.
fn receipt_journal(receipt: &Receipt) -> ReceiptJournal {
    let node_id = receipt
        .acts
        .first()
        .map(|act| act.id.clone())
        .unwrap_or_else(|| receipt.id.clone());
    let selected_act_id = receipt.acts.first().map(|act| act.id.clone());
    let signal_refs = receipt
        .lineage
        .as_ref()
        .map(|lineage| lineage.signal_refs.clone())
        .unwrap_or_default();
    ReceiptJournal {
        receipt_id: receipt.id.clone(),
        decisions: vec![ReceiptJournalDecision {
            decision: Decision {
                decision_id: format!("dec_{node_id}"),
                choice: DecisionChoice::Open,
                inputs: DecisionInputs {
                    signal_refs: signal_refs.clone(),
                    ..DecisionInputs::default()
                },
                proposed_intent: runx_contracts::Intent {
                    purpose: format!("Open runtime node {node_id}"),
                    legitimacy: "Local graph execution requested this node".to_owned(),
                    success_criteria: Vec::new(),
                    constraints: Vec::new(),
                    derived_from: Vec::new(),
                },
                selected_act_id,
                selected_harness_ref: None,
                justification: DecisionJustification {
                    summary: "runtime graph planner selected this node".to_owned(),
                    evidence_refs: signal_refs,
                },
                closure: None,
                artifact_refs: Vec::new(),
            },
        }],
    }
}

fn observation_act(
    step_id: &str,
    output: &SkillOutput,
    _disposition: ClosureDisposition,
    refs: &OutputRefs,
) -> ReceiptAct {
    let mut artifact_refs = refs.artifact_refs.clone();
    artifact_refs.extend(refs.surface_refs.iter().cloned());
    ReceiptAct {
        id: format!("act_{step_id}"),
        form: ActForm::Observation,
        summary: format!("Executed graph step {step_id}"),
        criteria: vec![ReceiptCriterion {
            criterion_id: "process_exit".to_owned(),
            status: if output.succeeded() {
                CriterionStatus::Verified
            } else {
                CriterionStatus::Failed
            },
            evidence_refs: refs.source_refs.clone(),
            verification_refs: refs.verification_refs.clone(),
            summary: Some(output_summary(output)),
        }],
        by: None,
        artifact_refs,
        detail_ref: None,
    }
}

fn seal(
    disposition: ClosureDisposition,
    reason_code: String,
    summary: String,
    closed_at: &str,
    criteria: Vec<ReceiptCriterion>,
) -> Seal {
    Seal {
        disposition,
        reason_code,
        summary,
        closed_at: closed_at.to_owned(),
        criteria,
    }
}

fn subject(graph_name: &str, node_id: &str, kind: ReceiptSubjectKind) -> Subject {
    Subject {
        kind,
        // The subject reference retains the harness identity (`hrn_<graph>_<node>`)
        // so history/replay projections keep a stable subject id.
        reference: Reference::with_uri(
            ReferenceType::Harness,
            format!("hrn_{graph_name}_{node_id}"),
        ),
        commitments: Vec::new(),
    }
}

fn authority() -> ReceiptAuthority {
    ReceiptAuthority {
        actor_ref: Reference::runx(ReferenceType::Principal, "local_runtime"),
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
            profile_hash: "sha256:runtime-skeleton-enforcement".to_owned(),
            redaction_refs: Vec::new(),
            setup_refs: Vec::new(),
            teardown_refs: Vec::new(),
        },
    }
}

fn idempotency(graph_name: &str, node_id: &str) -> ReceiptIdempotency {
    ReceiptIdempotency {
        intent_key: format!("sha256:{graph_name}-{node_id}-intent"),
        trigger_fingerprint: format!("sha256:{graph_name}-{node_id}-trigger"),
        content_hash: format!("sha256:{graph_name}-{node_id}-content"),
    }
}

#[derive(Clone, Debug, Default)]
struct OutputRefs {
    signal_refs: Vec<Reference>,
    source_refs: Vec<Reference>,
    surface_refs: Vec<Reference>,
    artifact_refs: Vec<Reference>,
    verification_refs: Vec<Reference>,
}

fn output_refs(output: &SkillOutput) -> OutputRefs {
    let mut refs = OutputRefs::default();
    if let Some(request_id) = string_field(&output.metadata, "agent_request_id") {
        refs.source_refs.push(Reference {
            uri: format!("runx:agent_act:{request_id}"),
            reference_type: ReferenceType::Act,
            provider: None,
            locator: Some(request_id.to_owned()),
            label: Some("agent act request".to_owned()),
            observed_at: None,
            proof_kind: None,
        });
    }
    let Ok(JsonValue::Object(payload)) = serde_json::from_str::<JsonValue>(&output.stdout) else {
        return refs;
    };
    collect_payload_refs(&payload, &mut refs);
    refs
}

fn collect_payload_refs(payload: &JsonObject, refs: &mut OutputRefs) {
    collect_packet_refs(payload, refs);
    if let Some(signal) = object_field(payload, "signal") {
        collect_signal_refs(signal, refs);
    }
    if let Some(change_set) = object_field(payload, "change_set") {
        collect_change_set_refs(change_set, refs);
    }
    if let Some(artifact) = object_field(payload, "artifact") {
        collect_artifact_ref(artifact, refs);
    }
    if let Some(verification) = object_field(payload, "verification") {
        collect_verification_ref(verification, refs);
    }
    if let Some(rail_proof) = object_field(payload, "rail_proof") {
        collect_rail_proof_ref(rail_proof, refs);
    }
    if let Some(credential_envelope) = object_field(payload, "credential_envelope") {
        collect_credential_ref(credential_envelope, refs);
    }
}

fn collect_packet_refs(payload: &JsonObject, refs: &mut OutputRefs) {
    for packet_key in [
        "payment_quote_packet",
        "payment_reservation_packet",
        "payment_approval",
        "payment_rail_packet",
        "payment_recovery_packet",
    ] {
        let Some(packet) = object_field(payload, packet_key) else {
            continue;
        };
        if let Some(data) = object_field(packet, "data") {
            collect_payload_refs(data, refs);
        }
    }
}

fn collect_signal_refs(signal: &JsonObject, refs: &mut OutputRefs) {
    if let Some(signal_id) = string_field(signal, "signal_id") {
        refs.signal_refs
            .push(Reference::runx(ReferenceType::Signal, signal_id));
    }
    if let Some(events) = array_field(signal, "source_events") {
        refs.source_refs
            .extend(events.iter().filter_map(source_event_ref));
    }
    if let Some(artifact) = object_field(signal, "artifact") {
        collect_artifact_ref(artifact, refs);
    }
}

fn collect_change_set_refs(change_set: &JsonObject, refs: &mut OutputRefs) {
    if let Some(surfaces) = array_field(change_set, "target_surfaces") {
        refs.surface_refs
            .extend(surfaces.iter().filter_map(target_surface_ref));
    }
}

fn collect_artifact_ref(artifact: &JsonObject, refs: &mut OutputRefs) {
    if let Some(artifact_id) = string_field(artifact, "artifact_id") {
        refs.artifact_refs
            .push(Reference::runx(ReferenceType::Artifact, artifact_id));
    }
}

fn collect_verification_ref(verification: &JsonObject, refs: &mut OutputRefs) {
    if let Some(verification_id) = string_field(verification, "verification_id") {
        refs.verification_refs.push(Reference::runx(
            ReferenceType::Verification,
            verification_id,
        ));
    }
}

fn collect_rail_proof_ref(rail_proof: &JsonObject, refs: &mut OutputRefs) {
    if let Some(proof_ref) = string_field(rail_proof, "proof_ref") {
        refs.verification_refs.push(Reference {
            uri: proof_ref.to_owned(),
            reference_type: ReferenceType::Verification,
            provider: None,
            locator: string_field(rail_proof, "idempotency_key").map(str::to_owned),
            label: Some("payment rail proof".to_owned()),
            observed_at: None,
            proof_kind: Some(ProofKind::PaymentRail),
        });
    }
}

fn collect_credential_ref(credential_envelope: &JsonObject, refs: &mut OutputRefs) {
    if let Some(credential_ref) = string_field(credential_envelope, "credential_ref") {
        refs.source_refs.push(Reference {
            uri: credential_ref.to_owned(),
            reference_type: ReferenceType::Credential,
            provider: None,
            locator: None,
            label: Some("scoped payment credential".to_owned()),
            observed_at: None,
            proof_kind: None,
        });
    }
}

fn source_event_ref(value: &JsonValue) -> Option<Reference> {
    let JsonValue::Object(event) = value else {
        return None;
    };
    let locator =
        string_field(event, "source_locator").or_else(|| string_field(event, "thread_locator"))?;
    let provider = string_field(event, "provider");
    Some(Reference {
        uri: locator.to_owned(),
        reference_type: source_reference_type(provider),
        provider: provider.map(str::to_owned),
        locator: Some(locator.to_owned()),
        label: string_field(event, "title").map(str::to_owned),
        observed_at: None,
        proof_kind: None,
    })
}

fn source_reference_type(provider: Option<&str>) -> ReferenceType {
    match provider {
        Some("github") => ReferenceType::GithubIssue,
        Some("slack") => ReferenceType::SlackThread,
        _ => ReferenceType::ExternalUrl,
    }
}

fn target_surface_ref(value: &JsonValue) -> Option<Reference> {
    let JsonValue::Object(surface) = value else {
        return None;
    };
    string_field(surface, "surface")
        .map(|surface_id| Reference::runx(ReferenceType::Surface, surface_id))
}

fn object_field<'a>(object: &'a JsonObject, field: &str) -> Option<&'a JsonObject> {
    match object.get(field) {
        Some(JsonValue::Object(value)) => Some(value),
        _ => None,
    }
}

fn array_field<'a>(object: &'a JsonObject, field: &str) -> Option<&'a Vec<JsonValue>> {
    match object.get(field) {
        Some(JsonValue::Array(value)) => Some(value),
        _ => None,
    }
}

fn string_field<'a>(object: &'a JsonObject, field: &str) -> Option<&'a str> {
    match object.get(field) {
        Some(JsonValue::String(value)) => Some(value),
        _ => None,
    }
}

fn disposition(output: &SkillOutput) -> ClosureDisposition {
    if output.succeeded() {
        ClosureDisposition::Closed
    } else {
        ClosureDisposition::Failed
    }
}

fn output_summary(output: &SkillOutput) -> String {
    if output.succeeded() {
        "cli-tool exited successfully".to_owned()
    } else if !output.stderr.is_empty() {
        output.stderr.clone()
    } else {
        format!("cli-tool failed with exit code {:?}", output.exit_code)
    }
}

fn child_receipt_reference(receipt: &Receipt) -> Reference {
    Reference {
        locator: Some(receipt.digest.clone()),
        ..Reference::runx(ReferenceType::Receipt, &receipt.id)
    }
}

fn attach_parent_to_child_receipts(
    steps: &mut [StepRun],
    parent_receipt_id: &str,
    signature_policy: RuntimeReceiptSignaturePolicy<'_>,
) -> Result<(), RuntimeError> {
    let parent_ref = Reference::runx(ReferenceType::Receipt, parent_receipt_id);
    for step in steps {
        step.receipt
            .lineage
            .get_or_insert_with(Lineage::default)
            .parent = Some(parent_ref.clone());
        seal_receipt(&mut step.receipt, signature_policy)?;
        // Re-bind any supervisor proof to the re-sealed child digest so payment
        // ledger projection validates against the final receipt body.
        rebind_supervisor_proof_to_receipt(&mut step.output.metadata, &step.receipt).map_err(
            |error| RuntimeError::ReceiptInvalid {
                message: error.to_string(),
            },
        )?;
    }
    Ok(())
}

fn local_issuer() -> ReceiptIssuer {
    ReceiptIssuer {
        issuer_type: ReceiptIssuerType::Local,
        kid: "runtime-skeleton".to_owned(),
        public_key_sha256: "sha256:runtime-skeleton-public".to_owned(),
    }
}

fn placeholder_signature() -> ReceiptSignature {
    ReceiptSignature {
        alg: SignatureAlgorithm::Ed25519,
        value: "sig:pending".to_owned(),
    }
}

fn seal_receipt(
    receipt: &mut Receipt,
    signature_policy: RuntimeReceiptSignaturePolicy<'_>,
) -> Result<(), RuntimeError> {
    signature_policy.prepare_receipt(receipt)?;
    // Write the planner deliberation as a separate journal artifact and commit
    // it by hash on lineage.journal_ref before the body digest is computed.
    let journal = receipt_journal(receipt);
    let journal_digest = journal.digest();
    receipt
        .lineage
        .get_or_insert_with(Lineage::default)
        .journal_ref = Some(Reference {
        locator: Some(journal_digest),
        ..Reference::runx(ReferenceType::Decision, &format!("{}_journal", receipt.id))
    });
    let digest =
        canonical_receipt_body_digest(receipt).map_err(|error| RuntimeError::ReceiptInvalid {
            message: error.to_string(),
        })?;
    receipt.digest = digest.clone();
    signature_policy.sign_receipt(receipt, &digest)?;

    let proof_contexts = RuntimeReceiptProofContextProvider::new(signature_policy);
    let context = proof_contexts.proof_context(receipt);
    validate_receipt_proof_allowing_journal(receipt, &context).map_err(receipt_error)
}

/// Plain proof verification reports the journal-dependent decision integrity as
/// `unverified`; at seal time that is the expected (not-yet-confirmed) state, so
/// the runtime confirms it with the in-hand journal and ignores the lone
/// journal-dependent finding.
fn validate_receipt_proof_allowing_journal(
    receipt: &Receipt,
    context: &ReceiptProofContext<'_>,
) -> Result<(), runx_receipts::ReceiptVerification> {
    let verification = runx_receipts::verify_receipt_proof(receipt, context);
    let blocking: Vec<_> = verification
        .findings
        .iter()
        .filter(|finding| {
            !matches!(
                finding.code,
                runx_receipts::ReceiptFindingCode::DecisionIntegrityUnverified
            )
        })
        .cloned()
        .collect();
    if blocking.is_empty() {
        Ok(())
    } else {
        Err(runx_receipts::ReceiptVerification::from_findings(blocking))
    }
}

pub(crate) fn proof_context<'a>(
    signature_verifier: Option<&'a dyn SignatureVerifier>,
    receipt: &Receipt,
) -> ReceiptProofContext<'a> {
    ReceiptProofContext {
        signature_verifier,
        authority_verified: authority_attenuation_verified(&receipt.authority.attenuation),
        external_attestations_verified: true,
        verified_redaction_refs: std::collections::BTreeSet::new(),
        verified_hash_commitments: std::collections::BTreeSet::new(),
    }
}

#[derive(Clone, Copy)]
pub struct RuntimeReceiptSignaturePolicy<'a> {
    mode: RuntimeReceiptSignatureMode,
    production_signer: Option<&'a dyn RuntimeReceiptSigner>,
    production_verifier: Option<&'a dyn SignatureVerifier>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RuntimeReceiptSignatureMode {
    LocalDevelopment,
    Production,
}

impl std::fmt::Debug for RuntimeReceiptSignaturePolicy<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RuntimeReceiptSignaturePolicy")
            .field("mode", &self.mode)
            .field(
                "production_signer_supplied",
                &self.production_signer.is_some(),
            )
            .field(
                "production_verifier_supplied",
                &self.production_verifier.is_some(),
            )
            .finish()
    }
}

impl<'a> RuntimeReceiptSignaturePolicy<'a> {
    #[must_use]
    pub fn local_development() -> Self {
        Self {
            mode: RuntimeReceiptSignatureMode::LocalDevelopment,
            production_signer: None,
            production_verifier: None,
        }
    }

    #[must_use]
    pub fn production(verifier: &'a dyn SignatureVerifier) -> Self {
        Self {
            mode: RuntimeReceiptSignatureMode::Production,
            production_signer: None,
            production_verifier: Some(verifier),
        }
    }

    #[must_use]
    pub fn production_signing(
        signer: &'a dyn RuntimeReceiptSigner,
        verifier: &'a dyn SignatureVerifier,
    ) -> Self {
        Self {
            mode: RuntimeReceiptSignatureMode::Production,
            production_signer: Some(signer),
            production_verifier: Some(verifier),
        }
    }

    #[must_use]
    pub fn production_signing_without_verifier(signer: &'a dyn RuntimeReceiptSigner) -> Self {
        Self {
            mode: RuntimeReceiptSignatureMode::Production,
            production_signer: Some(signer),
            production_verifier: None,
        }
    }

    #[must_use]
    pub fn production_without_verifier() -> Self {
        Self {
            mode: RuntimeReceiptSignatureMode::Production,
            production_signer: None,
            production_verifier: None,
        }
    }

    #[must_use]
    pub fn allows_local_pseudo_signatures(&self) -> bool {
        self.mode == RuntimeReceiptSignatureMode::LocalDevelopment
    }

    #[must_use]
    pub fn can_report_production_verified(&self) -> bool {
        self.mode == RuntimeReceiptSignatureMode::Production && self.production_verifier.is_some()
    }

    fn prepare_receipt(self, receipt: &mut Receipt) -> Result<(), RuntimeError> {
        if self.allows_local_pseudo_signatures() {
            receipt.issuer = local_issuer();
            return Ok(());
        }
        let Some(signer) = self.production_signer else {
            return Err(signing_error(RuntimeReceiptSigningError::MissingSigner));
        };
        if self.production_verifier.is_none() {
            return Err(signing_error(RuntimeReceiptSigningError::MissingVerifier));
        }
        let issuer = signer.issuer();
        validate_production_issuer(&issuer).map_err(signing_error)?;
        receipt.issuer = issuer;
        Ok(())
    }

    fn sign_receipt(
        self,
        receipt: &mut Receipt,
        body_digest: &str,
    ) -> Result<(), RuntimeError> {
        if self.allows_local_pseudo_signatures() {
            receipt.signature.value = format!("sig:{body_digest}");
            return Ok(());
        }
        let Some(signer) = self.production_signer else {
            return Err(signing_error(RuntimeReceiptSigningError::MissingSigner));
        };
        let Some(verifier) = self.production_verifier else {
            return Err(signing_error(RuntimeReceiptSigningError::MissingVerifier));
        };
        let signature = signer
            .sign_receipt_body(body_digest)
            .map_err(signing_error)?;
        if signature.alg != SignatureAlgorithm::Ed25519 {
            return Err(signing_error(
                RuntimeReceiptSigningError::UnsupportedAlgorithm,
            ));
        }
        if is_local_pseudo_signature(&signature.value) {
            return Err(signing_error(RuntimeReceiptSigningError::PseudoSignature));
        }
        receipt.signature = signature;
        verifier
            .verify(&receipt.issuer, &receipt.signature, body_digest)
            .map_err(RuntimeReceiptSigningError::SignatureVerification)
            .map_err(signing_error)
    }

    fn verifier(self) -> Option<RuntimeReceiptSignatureVerifier<'a>> {
        if self.mode == RuntimeReceiptSignatureMode::Production
            && self.production_verifier.is_none()
        {
            return None;
        }
        Some(RuntimeReceiptSignatureVerifier { policy: self })
    }
}

pub(crate) struct RuntimeReceiptProofContextProvider<'a> {
    signature_verifier: Option<RuntimeReceiptSignatureVerifier<'a>>,
}

impl RuntimeReceiptProofContextProvider<'static> {
    pub(crate) fn local_development() -> Self {
        Self::new(RuntimeReceiptSignaturePolicy::local_development())
    }
}

impl<'a> RuntimeReceiptProofContextProvider<'a> {
    pub(crate) fn new(signature_policy: RuntimeReceiptSignaturePolicy<'a>) -> Self {
        Self {
            signature_verifier: signature_policy.verifier(),
        }
    }
}

impl ReceiptProofContextProvider for RuntimeReceiptProofContextProvider<'_> {
    fn proof_context<'a>(&'a self, receipt: &Receipt) -> ReceiptProofContext<'a> {
        proof_context(
            self.signature_verifier
                .as_ref()
                .map(|verifier| verifier as &dyn SignatureVerifier),
            receipt,
        )
    }
}

struct RuntimeReceiptSignatureVerifier<'a> {
    policy: RuntimeReceiptSignaturePolicy<'a>,
}

impl SignatureVerifier for RuntimeReceiptSignatureVerifier<'_> {
    fn verify(
        &self,
        issuer: &ReceiptIssuer,
        signature: &ReceiptSignature,
        body_digest: &str,
    ) -> Result<(), SignatureVerificationFailure> {
        if is_local_pseudo_signature(&signature.value) {
            return if self.policy.allows_local_pseudo_signatures()
                && signature.value == format!("sig:{body_digest}")
            {
                Ok(())
            } else if self.policy.allows_local_pseudo_signatures() {
                Err(SignatureVerificationFailure::SignatureMismatch)
            } else {
                Err(SignatureVerificationFailure::MalformedSignature)
            };
        }
        let Some(verifier) = self.policy.production_verifier else {
            return Err(SignatureVerificationFailure::MissingKey);
        };
        verifier.verify(issuer, signature, body_digest)
    }
}

fn signing_error(error: RuntimeReceiptSigningError) -> RuntimeError {
    RuntimeError::ReceiptInvalid {
        message: error.to_string(),
    }
}

fn authority_attenuation_verified(attenuation: &AuthorityAttenuation) -> bool {
    match (&attenuation.parent_authority_ref, &attenuation.subset_proof) {
        (Some(parent), Some(proof)) => {
            proof.parent_authority_ref == *parent
                && matches!(proof.result, AuthoritySubsetResult::Subset)
        }
        (Some(_), None) | (None, Some(_)) => false,
        (None, None) => false,
    }
}

fn receipt_error(verification: runx_receipts::ReceiptVerification) -> RuntimeError {
    RuntimeError::ReceiptInvalid {
        message: format!("{:?}", verification.findings),
    }
}
