//! Governance receipt contracts: the flat `runx.receipt.v1` shape, seals,
//! fanout sync, lineage, and signatures.
//!
//! One flat artifact, each top-level key answering one question: integrity
//! (envelope), dedup (idempotency), what ran (subject), what allowed it
//! (authority), what was done (acts), the outcome (seal), and graph/resume
//! lineage. Planner deliberation and full per-act detail are referenced
//! (`lineage.journal_ref`, `acts[].detail_ref`), not inlined. Verification is a
//! read-time projection, never part of the signed body.
use serde::{Deserialize, Serialize};

use crate::{
    ActForm, AuthorityAttenuation, AuthorityTerm, ClosureDisposition, CriterionStatus,
    HashAlgorithm, JsonObject, Reference,
};

/// Logical schema name for the governance receipt.
pub const RECEIPT_SCHEMA: &str = "runx.receipt.v1";

/// The canonicalization byte contract this receipt's digest commits under.
pub const RECEIPT_CANONICALIZATION: &str = "runx.receipt.c14n.v1";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReceiptSchema {
    #[serde(rename = "runx.receipt.v1")]
    V1,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FanoutReceiptStrategy {
    All,
    Any,
    Quorum,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FanoutReceiptDecision {
    Proceed,
    Halt,
    Pause,
    Escalate,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FanoutReceiptSyncPoint {
    pub group_id: String,
    pub strategy: FanoutReceiptStrategy,
    pub decision: FanoutReceiptDecision,
    pub rule_fired: String,
    pub reason: String,
    pub branch_count: usize,
    pub success_count: usize,
    pub failure_count: usize,
    pub required_successes: usize,
    #[serde(default)]
    pub branch_receipts: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gate: Option<JsonObject>,
}

/// Scoped byte commitment; unifies the old hash_commitments + enforcement.std*_hash.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReceiptCommitmentScope {
    Input,
    Output,
    Stdout,
    Stderr,
    Error,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReceiptCommitment {
    pub scope: ReceiptCommitmentScope,
    pub algorithm: HashAlgorithm,
    pub value: String,
    pub canonicalization: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReceiptSubjectKind {
    Skill,
    Graph,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Subject {
    pub kind: ReceiptSubjectKind,
    #[serde(rename = "ref")]
    pub reference: Reference,
    #[serde(default)]
    pub commitments: Vec<ReceiptCommitment>,
}

/// Enforcement profile is hashed; the granted authority stays readable.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReceiptEnforcement {
    pub profile_hash: String,
    #[serde(default)]
    pub redaction_refs: Vec<Reference>,
    #[serde(default)]
    pub setup_refs: Vec<Reference>,
    #[serde(default)]
    pub teardown_refs: Vec<Reference>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReceiptAuthority {
    pub actor_ref: Reference,
    #[serde(default)]
    pub grant_refs: Vec<Reference>,
    #[serde(default)]
    pub scope_refs: Vec<Reference>,
    #[serde(default)]
    pub authority_proof_refs: Vec<Reference>,
    pub attenuation: AuthorityAttenuation,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mandate_ref: Option<Reference>,
    #[serde(default)]
    pub terms: Vec<AuthorityTerm>,
    pub enforcement: ReceiptEnforcement,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReceiptIdempotency {
    pub intent_key: String,
    pub trigger_fingerprint: String,
    pub content_hash: String,
}

/// Runner provenance for agent acts (drives the trainable-export projection).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunnerProvenance {
    pub provider: Option<String>,
    pub model: Option<String>,
    pub prompt_version: Option<String>,
}

/// Result binding only: criterion_id -> status. The skill declares the criteria.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReceiptCriterion {
    pub criterion_id: String,
    pub status: CriterionStatus,
    #[serde(default)]
    pub evidence_refs: Vec<Reference>,
    #[serde(default)]
    pub verification_refs: Vec<Reference>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReceiptAct {
    pub id: String,
    pub form: ActForm,
    pub summary: String,
    #[serde(default)]
    pub criteria: Vec<ReceiptCriterion>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub by: Option<RunnerProvenance>,
    #[serde(default)]
    pub artifact_refs: Vec<Reference>,
    // Full intent/target/source/surface refs and form-specific bodies live here.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail_ref: Option<Reference>,
}

/// Exactly one seal. `deferred` expresses a suspended (waiting/delegated) run.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Seal {
    pub disposition: ClosureDisposition,
    pub reason_code: String,
    pub summary: String,
    pub closed_at: String,
    #[serde(default)]
    pub criteria: Vec<ReceiptCriterion>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Lineage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<Reference>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous: Option<Reference>,
    #[serde(default)]
    pub children: Vec<Reference>,
    #[serde(default)]
    pub sync: Vec<FanoutReceiptSyncPoint>,
    #[serde(default)]
    pub signal_refs: Vec<Reference>,
    // Commits the planner deliberation (former decisions[]) by reference.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub journal_ref: Option<Reference>,
    // Open resolution request when seal.disposition == "deferred".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resume_ref: Option<Reference>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReceiptIssuerType {
    Local,
    Hosted,
    Ci,
    Verifier,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReceiptIssuer {
    #[serde(rename = "type")]
    pub issuer_type: ReceiptIssuerType,
    pub kid: String,
    pub public_key_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum SignatureAlgorithm {
    Ed25519,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReceiptSignature {
    pub alg: SignatureAlgorithm,
    pub value: String,
}

/// The single signed governance receipt: `runx.receipt.v1`.
///
/// `metadata` is a runtime-local read aid (skill name, source type, actor
/// labels for history projection) and is NOT part of the canonical signed body
/// (the canonicalizer strips it); it never appears in the TS contract.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Receipt {
    pub schema: ReceiptSchema,
    pub id: String,
    pub created_at: String,
    pub canonicalization: String,
    pub issuer: ReceiptIssuer,
    pub signature: ReceiptSignature,
    pub digest: String,
    pub idempotency: ReceiptIdempotency,
    pub subject: Subject,
    pub authority: ReceiptAuthority,
    #[serde(default)]
    pub acts: Vec<ReceiptAct>,
    pub seal: Seal,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lineage: Option<Lineage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<JsonObject>,
}
