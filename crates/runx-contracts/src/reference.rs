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

impl ReferenceType {
    /// Stable snake_case wire name for this reference type. Matches the serde
    /// representation and the `runx:<name>:<id>` URI segment.
    pub fn as_str(&self) -> &'static str {
        match self {
            ReferenceType::GithubIssue => "github_issue",
            ReferenceType::GithubPullRequest => "github_pull_request",
            ReferenceType::GithubRepo => "github_repo",
            ReferenceType::SlackThread => "slack_thread",
            ReferenceType::SentryEvent => "sentry_event",
            ReferenceType::Signal => "signal",
            ReferenceType::Act => "act",
            ReferenceType::Receipt => "receipt",
            ReferenceType::GraphReceipt => "graph_receipt",
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

impl Reference {
    /// A reference to an explicit URI, with no provider/locator/label/proof.
    pub fn with_uri(reference_type: ReferenceType, uri: impl Into<String>) -> Self {
        Self {
            reference_type,
            uri: uri.into(),
            provider: None,
            locator: None,
            label: None,
            observed_at: None,
            proof_kind: None,
        }
    }

    /// A reference whose URI is the canonical `runx:<type>:<id>` scheme.
    pub fn runx(reference_type: ReferenceType, id: &str) -> Self {
        let uri = format!("runx:{}:{id}", reference_type.as_str());
        Self::with_uri(reference_type, uri)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActRef {
    pub receipt_ref: Reference,
    pub act_id: String,
}
