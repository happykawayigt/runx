use std::collections::BTreeMap;

use runx_runtime::registry::{DefaultRuntimeHttpTransport, RuntimeHttpError};
use serde::Deserialize;

pub(crate) const DEFAULT_BASE_URL: &str = "https://api.runx.ai";
const BASE_URL_ENV: &str = "RUNX_PUBLIC_API_BASE_URL";

pub(crate) fn resolve_base_url(explicit: Option<&str>, env: &BTreeMap<String, String>) -> String {
    explicit
        .and_then(normalize_non_empty_base_url)
        .or_else(|| {
            env.get(BASE_URL_ENV)
                .and_then(|value| normalize_non_empty_base_url(value))
        })
        .unwrap_or_else(|| DEFAULT_BASE_URL.to_owned())
}

pub(crate) fn private_network_allowed(
    explicit: bool,
    env: &BTreeMap<String, String>,
    env_key: &str,
) -> bool {
    explicit || env.get(env_key).is_some_and(|value| truthy_env(value))
}

pub(crate) fn transport(
    allow_private_network: bool,
) -> Result<DefaultRuntimeHttpTransport, RuntimeHttpError> {
    if allow_private_network {
        return DefaultRuntimeHttpTransport::with_private_network_access();
    }
    DefaultRuntimeHttpTransport::new()
}

pub(crate) fn parse_error(body: &str) -> Option<ErrorPayload> {
    serde_json::from_str::<ErrorEnvelope>(body)
        .ok()
        .map(|envelope| envelope.error)
        .or_else(|| {
            serde_json::from_str::<PlainErrorEnvelope>(body)
                .ok()
                .map(|envelope| match envelope.error {
                    PlainError::Message(detail) => ErrorPayload {
                        code: plain_error_code(&detail).to_owned(),
                        detail,
                        hint: None,
                        retry_after_seconds: None,
                    },
                    PlainError::Payload(payload) => payload,
                })
        })
}

fn normalize_non_empty_base_url(value: &str) -> Option<String> {
    let normalized = value.trim().trim_end_matches('/');
    (!normalized.is_empty()).then(|| normalized.to_owned())
}

fn truthy_env(value: &str) -> bool {
    matches!(value.trim(), "1" | "true" | "TRUE" | "yes" | "YES")
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub(crate) struct ErrorPayload {
    pub code: String,
    pub detail: String,
    #[serde(default)]
    pub hint: Option<String>,
    #[serde(default)]
    pub retry_after_seconds: Option<u32>,
}

#[derive(Deserialize)]
struct ErrorEnvelope {
    error: ErrorPayload,
}

#[derive(Deserialize)]
struct PlainErrorEnvelope {
    error: PlainError,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum PlainError {
    Message(String),
    Payload(ErrorPayload),
}

fn plain_error_code(detail: &str) -> &'static str {
    if detail.contains("Missing required scope") {
        "missing_scope"
    } else {
        "api_error"
    }
}
