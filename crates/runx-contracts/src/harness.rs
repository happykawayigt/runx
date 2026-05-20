//! Harness spine contracts: receipts, seals, sandbox declarations, fanout sync, signatures.
use serde::{Deserialize, Serialize};

use crate::{
    Act, Authority, ClosureDisposition, Decision, HashCommitment, JsonObject,
    ReceiptVerificationSummary, Reference,
};

pub const HARNESS_RECEIPT_SCHEMA: &str = "runx.harness_receipt.v1";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum HarnessReceiptSchema {
    #[serde(rename = "runx.harness_receipt.v1")]
    V1,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HarnessState {
    Forming,
    Admitted,
    Running,
    Waiting,
    Delegated,
    Sealing,
    Sealed,
    Killed,
    TimedOut,
    Failed,
    Superseded,
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HarnessSandbox {
    pub profile: String,
    pub cwd_policy: String,
    pub network: String,
    pub filesystem: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HarnessEnforcement {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub harness_ref: Option<Reference>,
    pub version: String,
    pub enforcement_profile_hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enforcer_ref: Option<Reference>,
    pub sandbox: HarnessSandbox,
    #[serde(default)]
    pub redaction_refs: Vec<Reference>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stdout_hash: Option<HashCommitment>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stderr_hash: Option<HashCommitment>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub setup_receipt_refs: Vec<Reference>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub teardown_receipt_refs: Vec<Reference>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HarnessIdempotency {
    pub intent_key: String,
    pub trigger_fingerprint: String,
    pub content_hash: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HarnessRevision {
    pub sequence: u64,
    pub previous_ref: Option<Reference>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SealCriterion {
    pub criterion_id: String,
    pub status: crate::CriterionStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub act_id: Option<String>,
    #[serde(default)]
    pub verification_refs: Vec<Reference>,
    #[serde(default)]
    pub evidence_refs: Vec<Reference>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HarnessSeal {
    pub disposition: ClosureDisposition,
    pub reason_code: String,
    pub summary: String,
    pub closed_at: String,
    pub last_observed_at: String,
    pub canonicalization: String,
    pub digest: String,
    pub criteria: Vec<SealCriterion>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verification_summary: Option<ReceiptVerificationSummary>,
    #[serde(default)]
    pub redaction_refs: Vec<Reference>,
    #[serde(default)]
    pub artifact_refs: Vec<Reference>,
    #[serde(default)]
    pub hash_commitments: Vec<HashCommitment>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Harness {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    pub harness_id: String,
    pub parent_harness_ref: Option<Reference>,
    pub state: HarnessState,
    pub host_ref: Reference,
    pub harness_ref: Reference,
    pub authority: Authority,
    pub enforcement: HarnessEnforcement,
    pub idempotency: HarnessIdempotency,
    pub revision: HarnessRevision,
    #[serde(default)]
    pub signal_refs: Vec<Reference>,
    #[serde(default)]
    pub decisions: Vec<Decision>,
    #[serde(default)]
    pub acts: Vec<Act>,
    #[serde(default)]
    pub child_harness_receipt_refs: Vec<Reference>,
    #[serde(default)]
    pub artifact_refs: Vec<Reference>,
    pub seal: Option<HarnessSeal>,
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HarnessReceipt {
    pub schema: HarnessReceiptSchema,
    pub id: String,
    pub created_at: String,
    pub issuer: ReceiptIssuer,
    pub signature: ReceiptSignature,
    pub harness: Harness,
    pub seal: HarnessSeal,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sync_points: Vec<FanoutReceiptSyncPoint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<JsonObject>,
}
