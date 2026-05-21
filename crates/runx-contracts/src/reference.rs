//! Reference contracts: typed references to receipts, acts, and external surfaces.
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReferenceType {
    GithubIssue,
    GithubPullRequest,
    GithubRepo,
    SlackThread,
    SentryEvent,
    Signal,
    Act,
    Receipt,
    GraphReceipt,
    HarnessReceipt,
    Artifact,
    Verification,
    Harness,
    Host,
    Deployment,
    Surface,
    Target,
    Opportunity,
    ThesisAssessment,
    Selection,
    SkillBinding,
    TargetTransitionEntry,
    SelectionCycle,
    Decision,
    ReflectionEntry,
    FeedEntry,
    Principal,
    AuthorityProof,
    ScopeAdmission,
    Grant,
    Mandate,
    Credential,
    WebhookDelivery,
    RedactionPolicy,
    ExternalUrl,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProofKind {
    PaymentRail,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reference {
    #[serde(rename = "type")]
    pub reference_type: ReferenceType,
    pub uri: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locator: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observed_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proof_kind: Option<ProofKind>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActRef {
    pub harness_receipt_ref: Reference,
    pub act_id: String,
}
