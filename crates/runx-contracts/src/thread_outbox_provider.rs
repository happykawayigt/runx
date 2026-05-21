//! Thread outbox provider contract types.
use serde::{Deserialize, Serialize};

use crate::{
    CredentialDeliveryMode, CredentialDeliveryObservation, CredentialDeliveryPurpose, Reference,
};

pub const THREAD_OUTBOX_PROVIDER_PROTOCOL_VERSION: &str = "runx.thread_outbox_provider.v1";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThreadOutboxProviderOperation {
    Push,
    Fetch,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThreadOutboxProviderTransportKind {
    Process,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThreadOutboxProviderPayloadFormat {
    Markdown,
    PlainText,
    Json,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThreadOutboxProviderObservationStatus {
    Accepted,
    Skipped,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThreadOutboxProviderIdempotencyStatus {
    Created,
    Replayed,
    Skipped,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThreadOutboxProviderTransport {
    pub kind: ThreadOutboxProviderTransportKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThreadOutboxProviderCredentialNeed {
    pub provider: String,
    pub purpose: CredentialDeliveryPurpose,
    pub profile_id: String,
    pub delivery_mode: CredentialDeliveryMode,
    pub required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope_refs: Option<Vec<Reference>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThreadOutboxProviderReceiptCapabilities {
    pub idempotent_push: bool,
    pub readback: bool,
    pub stable_provider_event_hash: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThreadOutboxProviderRedactionCapabilities {
    pub redacts_credentials: bool,
    pub redacts_provider_payloads: bool,
    pub supports_redaction_refs: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThreadOutboxProviderManifest {
    pub schema: String,
    pub protocol_version: String,
    pub adapter_id: String,
    pub provider: String,
    pub name: String,
    pub version: String,
    pub supported_operations: Vec<ThreadOutboxProviderOperation>,
    pub transport: ThreadOutboxProviderTransport,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credential_needs: Option<Vec<ThreadOutboxProviderCredentialNeed>>,
    pub receipt_capabilities: ThreadOutboxProviderReceiptCapabilities,
    pub redaction_capabilities: ThreadOutboxProviderRedactionCapabilities,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThreadOutboxProviderThreadLocator {
    pub provider: String,
    pub thread_ref: Reference,
    pub locator: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThreadOutboxProviderLocator {
    pub provider: String,
    pub locator: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_ref: Option<Reference>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThreadOutboxProviderIdempotency {
    pub key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_hash: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThreadOutboxProviderIdempotencyObservation {
    pub key: String,
    pub status: ThreadOutboxProviderIdempotencyStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_observation_ref: Option<Reference>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThreadOutboxProviderRenderedPayload {
    pub format: ThreadOutboxProviderPayloadFormat,
    pub body: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redaction_refs: Option<Vec<Reference>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThreadOutboxProviderCredentialProfile {
    pub provider: String,
    pub purpose: CredentialDeliveryPurpose,
    pub profile_id: String,
    pub delivery_mode: CredentialDeliveryMode,
    pub credential_refs: Vec<Reference>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThreadOutboxProviderReceiptContext {
    pub harness_ref: Reference,
    pub host_ref: Reference,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authority_proof_refs: Option<Vec<Reference>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope_refs: Option<Vec<Reference>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThreadOutboxProviderPush {
    pub schema: String,
    pub protocol_version: String,
    pub push_id: String,
    pub adapter_id: String,
    pub provider: String,
    pub outbox_entry_id: String,
    pub thread_locator: ThreadOutboxProviderThreadLocator,
    pub idempotency: ThreadOutboxProviderIdempotency,
    pub payload: ThreadOutboxProviderRenderedPayload,
    pub provider_profile: ThreadOutboxProviderCredentialProfile,
    pub credential_delivery_refs: Vec<Reference>,
    pub receipt_context: ThreadOutboxProviderReceiptContext,
    pub requested_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ThreadOutboxProviderFetchTarget {
    Thread(ThreadOutboxProviderFetchThreadTarget),
    Provider(ThreadOutboxProviderFetchProviderTarget),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThreadOutboxProviderFetchThreadTarget {
    pub thread_locator: ThreadOutboxProviderThreadLocator,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThreadOutboxProviderFetchProviderTarget {
    pub provider_locator: ThreadOutboxProviderLocator,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThreadOutboxProviderFetch {
    pub schema: String,
    pub protocol_version: String,
    pub fetch_id: String,
    pub adapter_id: String,
    pub provider: String,
    pub target: ThreadOutboxProviderFetchTarget,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub readback_cursor: Option<String>,
    pub idempotency: ThreadOutboxProviderIdempotency,
    pub provider_profile: ThreadOutboxProviderCredentialProfile,
    pub credential_delivery_refs: Vec<Reference>,
    pub receipt_context: ThreadOutboxProviderReceiptContext,
    pub requested_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThreadOutboxProviderReadbackSummary {
    pub item_count: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_provider_event_id_hash: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThreadOutboxProviderError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThreadOutboxProviderObservation {
    pub schema: String,
    pub protocol_version: String,
    pub observation_id: String,
    pub adapter_id: String,
    pub provider: String,
    pub operation: ThreadOutboxProviderOperation,
    pub request_id: String,
    pub status: ThreadOutboxProviderObservationStatus,
    pub idempotency: ThreadOutboxProviderIdempotencyObservation,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_locator: Option<ThreadOutboxProviderLocator>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_event_id_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub readback_summary: Option<ThreadOutboxProviderReadbackSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delivery_observations: Option<Vec<CredentialDeliveryObservation>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redaction_refs: Option<Vec<Reference>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<Vec<ThreadOutboxProviderError>>,
    pub observed_at: String,
}
