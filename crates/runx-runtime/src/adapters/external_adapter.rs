// rust-style-allow: large-file because the process supervisor, contract
// validation, timeout handling, and frame normalization must stay adjacent to
// keep the external adapter boundary auditable.
use std::collections::BTreeMap;
use std::io::{Read, Write};
#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::path::{Component, Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use runx_contracts::{
    CredentialDeliveryPurpose, EXTERNAL_ADAPTER_PROTOCOL_VERSION, ExternalAdapterCancellationFrame,
    ExternalAdapterCredentialPurpose, ExternalAdapterCredentialReference,
    ExternalAdapterCredentialRequest, ExternalAdapterHostResolutionFrame,
    ExternalAdapterInvocation, ExternalAdapterManifest, ExternalAdapterResponse,
    ExternalAdapterStatus, ExternalAdapterTelemetryValue, ExternalAdapterTransportKind, JsonNumber,
    JsonObject, JsonValue, Reference, ReferenceType,
};
use thiserror::Error;

use crate::RuntimeError;
use crate::adapter::{InvocationStatus, SkillAdapter, SkillInvocation, SkillOutput};
use crate::credentials::CredentialDelivery;
use crate::receipts::paths::RUNX_RECEIPT_DIR_ENV;
use crate::time::now_iso8601;

const MANIFEST_INLINE_FIELD: &str = "external_adapter_manifest";
const MANIFEST_PATH_FIELD: &str = "external_adapter_manifest_path";
const MANIFEST_NESTED_FIELD: &str = "external_adapter";
const MANIFEST_NESTED_MANIFEST_FIELD: &str = "manifest";
const MANIFEST_NESTED_PATH_FIELD: &str = "manifest_path";
const INVOCATION_SCHEMA: &str = "runx.external_adapter.invocation.v1";
const MANIFEST_SCHEMA: &str = "runx.external_adapter.manifest.v1";
const RESPONSE_SCHEMA: &str = "runx.external_adapter.response.v1";
const CREDENTIAL_REQUEST_SCHEMA: &str = "runx.external_adapter.credential_request.v1";
const HOST_RESOLUTION_SCHEMA: &str = "runx.external_adapter.host_resolution.v1";
const CANCELLATION_SCHEMA: &str = "runx.external_adapter.cancellation.v1";
const CREDENTIAL_DELIVERY_OBSERVATIONS_METADATA: &str = "credential_delivery_observations";
const HOST_RESOLUTION_FRAME_ID_METADATA: &str = "external_adapter_host_resolution_frame_id";
const HOST_RESOLUTION_REQUEST_METADATA: &str = "external_adapter_host_resolution_request";
const POLL_INTERVAL: Duration = Duration::from_millis(10);
const FORCE_KILL_GRACE: Duration = Duration::from_millis(100);
const RESPONSE_LIMIT_BYTES: usize = 1024 * 1024;

#[derive(Clone, Debug, PartialEq)]
pub struct ExternalAdapterProcessOutcome {
    pub response: ExternalAdapterResponse,
    pub process_exit_code: Option<i32>,
    pub duration_ms: u64,
}

#[derive(Clone, Debug)]
pub struct ExternalAdapterSkillAdapter<
    R = InlineExternalAdapterManifestResolver,
    S = ExternalAdapterProcessSupervisor,
> {
    manifest_resolver: R,
    supervisor: S,
}

impl<R, S> ExternalAdapterSkillAdapter<R, S> {
    #[must_use]
    pub const fn new(manifest_resolver: R, supervisor: S) -> Self {
        Self {
            manifest_resolver,
            supervisor,
        }
    }
}

impl Default
    for ExternalAdapterSkillAdapter<
        InlineExternalAdapterManifestResolver,
        ExternalAdapterProcessSupervisor,
    >
{
    fn default() -> Self {
        Self::new(
            InlineExternalAdapterManifestResolver,
            ExternalAdapterProcessSupervisor,
        )
    }
}

impl<R, S> SkillAdapter for ExternalAdapterSkillAdapter<R, S>
where
    R: ExternalAdapterManifestResolver,
    S: ExternalAdapterSupervisor,
{
    fn adapter_type(&self) -> &'static str {
        "external-adapter"
    }

    fn invoke(&self, request: SkillInvocation) -> Result<SkillOutput, RuntimeError> {
        if request.source.source_type != "external-adapter" {
            return Err(RuntimeError::UnsupportedAdapter {
                adapter_type: request.source.source_type,
            });
        }
        let skill_name = request.skill_name.clone();
        invoke_external_adapter_skill(request, &self.manifest_resolver, &self.supervisor).map_err(
            |error| RuntimeError::SkillFailed {
                skill_name,
                message: error.to_string(),
            },
        )
    }
}

pub trait ExternalAdapterManifestResolver {
    fn resolve_manifest(
        &self,
        request: &SkillInvocation,
    ) -> Result<ExternalAdapterManifest, ExternalAdapterSkillAdapterError>;
}

pub trait ExternalAdapterSupervisor {
    fn invoke_external_adapter(
        &self,
        manifest: &ExternalAdapterManifest,
        invocation: &ExternalAdapterInvocation,
        credential_delivery: &CredentialDelivery,
    ) -> Result<ExternalAdapterProcessOutcome, ExternalAdapterSupervisorError>;
}

impl ExternalAdapterSupervisor for ExternalAdapterProcessSupervisor {
    fn invoke_external_adapter(
        &self,
        manifest: &ExternalAdapterManifest,
        invocation: &ExternalAdapterInvocation,
        credential_delivery: &CredentialDelivery,
    ) -> Result<ExternalAdapterProcessOutcome, ExternalAdapterSupervisorError> {
        ExternalAdapterProcessSupervisor::invoke_with_delivery(
            self,
            manifest,
            invocation,
            credential_delivery,
        )
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct InlineExternalAdapterManifestResolver;

impl ExternalAdapterManifestResolver for InlineExternalAdapterManifestResolver {
    fn resolve_manifest(
        &self,
        request: &SkillInvocation,
    ) -> Result<ExternalAdapterManifest, ExternalAdapterSkillAdapterError> {
        if let Some(value) = inline_manifest_value(&request.source.raw) {
            let JsonValue::Object(_) = value else {
                return Err(ExternalAdapterSkillAdapterError::InvalidInlineManifestShape);
            };
            return manifest_from_value(value);
        }
        if let Some(relative_path) = manifest_path_value(&request.source.raw)? {
            return manifest_from_path(&request.skill_directory, &relative_path);
        }
        Err(ExternalAdapterSkillAdapterError::MissingManifest)
    }
}

#[derive(Debug, Error)]
pub enum ExternalAdapterSkillAdapterError {
    #[error(
        "external adapter source is missing a manifest at source.external_adapter.manifest, source.external_adapter.manifest_path, source.external_adapter_manifest, or source.external_adapter_manifest_path"
    )]
    MissingManifest,
    #[error("external adapter inline manifest must be an object")]
    InvalidInlineManifestShape,
    #[error(
        "external adapter manifest_path must be a relative path below the skill directory: '{path}'"
    )]
    InvalidManifestPath { path: String },
    #[error(
        "external adapter manifest_path '{path}' escapes the skill directory '{skill_directory}'"
    )]
    ManifestPathEscapesSkillDirectory {
        path: String,
        skill_directory: String,
    },
    #[error("external adapter manifest file '{path}' could not be read: {source}")]
    ManifestRead {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("external adapter source metadata '{field}' must be a string when present")]
    InvalidSourceMetadata { field: &'static str },
    #[error(
        "external adapter response exit_code {actual} does not fit in a runtime process exit code"
    )]
    ExitCodeOutOfRange { actual: i64 },
    #[error("external adapter JSON failed while {context}: {source}")]
    Json {
        context: String,
        #[source]
        source: serde_json::Error,
    },
    #[error(transparent)]
    Supervisor(#[from] ExternalAdapterSupervisorError),
}

#[derive(Debug, Error)]
pub enum ExternalAdapterSupervisorError {
    #[error("external adapter manifest uses unsupported protocol version '{actual}'")]
    UnsupportedManifestProtocol { actual: String },
    #[error("external adapter invocation uses unsupported protocol version '{actual}'")]
    UnsupportedInvocationProtocol { actual: String },
    #[error("external adapter response uses unsupported protocol version '{actual}'")]
    UnsupportedResponseProtocol { actual: String },
    #[error("external adapter manifest schema '{actual}' is unsupported")]
    UnsupportedManifestSchema { actual: String },
    #[error("external adapter invocation schema '{actual}' is unsupported")]
    UnsupportedInvocationSchema { actual: String },
    #[error("external adapter response schema '{actual}' is unsupported")]
    UnsupportedResponseSchema { actual: String },
    #[error("external adapter manifest uses unsupported transport '{kind:?}'")]
    UnsupportedTransport { kind: ExternalAdapterTransportKind },
    #[error("external adapter process transport is missing command")]
    MissingProcessCommand,
    #[error("external adapter process command is empty")]
    EmptyProcessCommand,
    #[error(
        "external adapter invocation adapter id '{invocation_adapter_id}' does not match manifest adapter id '{manifest_adapter_id}'"
    )]
    AdapterIdMismatch {
        manifest_adapter_id: String,
        invocation_adapter_id: String,
    },
    #[error("external adapter '{adapter_id}' does not support source type '{source_type}'")]
    UnsupportedSourceType {
        adapter_id: String,
        source_type: String,
    },
    #[error("external adapter startup timeout must be greater than zero")]
    InvalidStartupTimeout,
    #[error("external adapter invocation timeout must be greater than zero")]
    InvalidInvocationTimeout,
    #[error("external adapter invocation env value '{key}' must be a string")]
    InvalidEnvValue { key: String },
    #[error("external adapter process timed out after {timeout_ms}ms")]
    TimedOut {
        timeout_ms: u64,
        cancellation: Box<ExternalAdapterCancellationFrame>,
    },
    #[error("external adapter process exited before returning an accepted response: {exit_status}")]
    ProcessFailed { exit_status: String },
    #[error("external adapter process returned no stdout response")]
    EmptyResponse,
    #[error("external adapter process response exceeded {limit_bytes} bytes")]
    ResponseTooLarge { limit_bytes: usize },
    #[error("external adapter process made an unexpected credential request '{request_id}'")]
    UnexpectedCredentialRequest { request_id: String },
    #[error("external adapter process returned unsupported frame schema '{schema}'")]
    UnsupportedFrameSchema { schema: String },
    #[error("external adapter response {field} was '{actual}', expected '{expected}'")]
    ResponseMismatch {
        field: &'static str,
        expected: String,
        actual: String,
    },
    #[error("external adapter process I/O failed while {context}: {source}")]
    Io {
        context: String,
        #[source]
        source: std::io::Error,
    },
    #[error("external adapter JSON failed while {context}: {source}")]
    Json {
        context: String,
        #[source]
        source: serde_json::Error,
    },
}

#[derive(Clone, Debug, Default)]
pub struct ExternalAdapterProcessSupervisor;

impl ExternalAdapterProcessSupervisor {
    pub fn invoke(
        &self,
        manifest: &ExternalAdapterManifest,
        invocation: &ExternalAdapterInvocation,
    ) -> Result<ExternalAdapterProcessOutcome, ExternalAdapterSupervisorError> {
        self.invoke_with_delivery(manifest, invocation, &CredentialDelivery::none())
    }

    pub fn invoke_with_delivery(
        &self,
        manifest: &ExternalAdapterManifest,
        invocation: &ExternalAdapterInvocation,
        credential_delivery: &CredentialDelivery,
    ) -> Result<ExternalAdapterProcessOutcome, ExternalAdapterSupervisorError> {
        validate_invocation_contract(manifest, invocation)?;
        let started = Instant::now();
        let command = process_command(manifest)?;
        let mut child = spawn_process(command, manifest, invocation, credential_delivery)?;
        let stdout = capture_pipe(child.stdout.take(), "opening external adapter stdout pipe")?;
        let stderr = capture_pipe(child.stderr.take(), "opening external adapter stderr pipe")?;
        if let Err(error) = write_invocation(&mut child, invocation) {
            let _cleanup = kill_timed_out_process(&mut child, KillSignal::Force);
            let _wait = child.wait();
            let _stdout = join_capture(stdout, "collecting failed external adapter stdout");
            let _stderr = join_capture(stderr, "collecting failed external adapter stderr");
            return Err(error);
        }
        let timeout = Duration::from_millis(manifest.timeouts.invocation_ms);
        let wait_result = wait_for_exit(&mut child, timeout)?;

        let status = match wait_result {
            WaitResult::Exited(status) => status,
            WaitResult::TimedOut => {
                let _stdout = join_capture(stdout, "collecting timed out external adapter stdout");
                let _stderr = join_capture(stderr, "collecting timed out external adapter stderr");
                return Err(ExternalAdapterSupervisorError::TimedOut {
                    timeout_ms: manifest.timeouts.invocation_ms,
                    cancellation: Box::new(timeout_cancellation_frame(
                        manifest,
                        invocation,
                        manifest.timeouts.invocation_ms,
                    )),
                });
            }
        };
        let stdout = join_capture(stdout, "collecting external adapter stdout")?;
        let _stderr = join_capture(stderr, "collecting external adapter stderr")?;
        if !status.success() {
            return Err(ExternalAdapterSupervisorError::ProcessFailed {
                exit_status: status.to_string(),
            });
        }
        if stdout.truncated {
            return Err(ExternalAdapterSupervisorError::ResponseTooLarge {
                limit_bytes: RESPONSE_LIMIT_BYTES,
            });
        }
        let response = parse_response(&stdout.bytes, credential_delivery)?;
        validate_response_contract(invocation, &response)?;
        Ok(ExternalAdapterProcessOutcome {
            response,
            process_exit_code: status.code(),
            duration_ms: duration_ms(started),
        })
    }
}

fn invoke_external_adapter_skill<R, S>(
    request: SkillInvocation,
    manifest_resolver: &R,
    supervisor: &S,
) -> Result<SkillOutput, ExternalAdapterSkillAdapterError>
where
    R: ExternalAdapterManifestResolver,
    S: ExternalAdapterSupervisor,
{
    let manifest = manifest_resolver.resolve_manifest(&request)?;
    let invocation = skill_invocation_contract(&request, &manifest)?;
    let outcome =
        supervisor.invoke_external_adapter(&manifest, &invocation, &request.credential_delivery)?;
    skill_output_from_outcome(outcome, &request.credential_delivery)
}

fn inline_manifest_value(source: &JsonObject) -> Option<&JsonValue> {
    source.get(MANIFEST_INLINE_FIELD).or_else(|| {
        let JsonValue::Object(external_adapter) = source.get(MANIFEST_NESTED_FIELD)? else {
            return None;
        };
        external_adapter.get(MANIFEST_NESTED_MANIFEST_FIELD)
    })
}

fn manifest_path_value(
    source: &JsonObject,
) -> Result<Option<String>, ExternalAdapterSkillAdapterError> {
    if let Some(value) = source.get(MANIFEST_PATH_FIELD) {
        let JsonValue::String(path) = value else {
            return Err(ExternalAdapterSkillAdapterError::InvalidSourceMetadata {
                field: MANIFEST_PATH_FIELD,
            });
        };
        return Ok(Some(path.clone()));
    }
    let Some(JsonValue::Object(external_adapter)) = source.get(MANIFEST_NESTED_FIELD) else {
        return Ok(None);
    };
    match external_adapter.get(MANIFEST_NESTED_PATH_FIELD) {
        Some(JsonValue::String(path)) => Ok(Some(path.clone())),
        Some(_) => Err(ExternalAdapterSkillAdapterError::InvalidSourceMetadata {
            field: MANIFEST_NESTED_PATH_FIELD,
        }),
        None => Ok(None),
    }
}

fn manifest_from_value(
    value: &JsonValue,
) -> Result<ExternalAdapterManifest, ExternalAdapterSkillAdapterError> {
    let value = serde_json::to_value(value).map_err(|source| {
        json_adapter_error("serializing external adapter inline manifest", source)
    })?;
    serde_json::from_value(value)
        .map_err(|source| json_adapter_error("validating external adapter inline manifest", source))
}

fn manifest_from_path(
    skill_directory: &Path,
    relative_path: &str,
) -> Result<ExternalAdapterManifest, ExternalAdapterSkillAdapterError> {
    validate_manifest_relative_path(relative_path)?;
    let skill_directory_display = skill_directory.to_string_lossy().into_owned();
    let skill_directory = skill_directory.canonicalize().map_err(|source| {
        ExternalAdapterSkillAdapterError::ManifestRead {
            path: skill_directory_display.clone(),
            source,
        }
    })?;
    let manifest_path = skill_directory.join(relative_path);
    let canonical_manifest_path = manifest_path.canonicalize().map_err(|source| {
        ExternalAdapterSkillAdapterError::ManifestRead {
            path: manifest_path.to_string_lossy().into_owned(),
            source,
        }
    })?;
    if !canonical_manifest_path.starts_with(&skill_directory) {
        return Err(
            ExternalAdapterSkillAdapterError::ManifestPathEscapesSkillDirectory {
                path: relative_path.to_owned(),
                skill_directory: skill_directory_display,
            },
        );
    }
    let bytes = std::fs::read(canonical_manifest_path.as_path()).map_err(|source| {
        ExternalAdapterSkillAdapterError::ManifestRead {
            path: canonical_manifest_path.to_string_lossy().into_owned(),
            source,
        }
    })?;
    serde_json::from_slice(&bytes)
        .map_err(|source| json_adapter_error("validating external adapter manifest file", source))
}

fn validate_manifest_relative_path(
    relative_path: &str,
) -> Result<(), ExternalAdapterSkillAdapterError> {
    let path = Path::new(relative_path);
    let valid = !relative_path.trim().is_empty()
        && path.is_relative()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)));
    if valid {
        Ok(())
    } else {
        Err(ExternalAdapterSkillAdapterError::InvalidManifestPath {
            path: relative_path.to_owned(),
        })
    }
}

fn skill_invocation_contract(
    request: &SkillInvocation,
    manifest: &ExternalAdapterManifest,
) -> Result<ExternalAdapterInvocation, ExternalAdapterSkillAdapterError> {
    let invocation_id = optional_source_string(&request.source.raw, "invocation_id")?
        .unwrap_or_else(|| {
            format!(
                "external_adapter.{}.invoke",
                identifier_segment(&request.skill_name)
            )
        });
    let run_id = optional_source_string(&request.source.raw, "run_id")?
        .unwrap_or_else(|| format!("run_{}", identifier_segment(&request.skill_name)));
    let step_id = optional_source_string(&request.source.raw, "step_id")?
        .unwrap_or_else(|| identifier_segment(&request.skill_name));
    let skill_ref = optional_source_string(&request.source.raw, "skill_ref")?
        .unwrap_or_else(|| request.skill_name.clone());
    Ok(ExternalAdapterInvocation {
        schema: INVOCATION_SCHEMA.to_owned(),
        protocol_version: EXTERNAL_ADAPTER_PROTOCOL_VERSION.to_owned(),
        invocation_id,
        adapter_id: manifest.adapter_id.clone(),
        run_id: run_id.clone(),
        step_id,
        source_type: request.source.source_type.clone(),
        skill_ref,
        harness_ref: reference(ReferenceType::Harness, &format!("runx:harness:{run_id}")),
        host_ref: reference(ReferenceType::Host, "runx:host:runtime"),
        inputs: request.inputs.clone(),
        resolved_inputs: (!request.resolved_inputs.is_empty())
            .then(|| request.resolved_inputs.clone()),
        cwd: Some(invocation_cwd(request)),
        receipt_dir: request.env.get(RUNX_RECEIPT_DIR_ENV).cloned(),
        env: invocation_env(&request.env),
        credential_refs: external_adapter_credential_refs(&request.credential_delivery),
        metadata: None,
    })
}

fn external_adapter_credential_refs(
    credential_delivery: &CredentialDelivery,
) -> Option<Vec<ExternalAdapterCredentialReference>> {
    let observation = credential_delivery.public_observation()?;
    (!observation.credential_refs.is_empty()).then(|| {
        observation
            .credential_refs
            .iter()
            .cloned()
            .map(|credential_ref| ExternalAdapterCredentialReference {
                credential_ref,
                provider: observation.provider.clone(),
                purpose: external_adapter_credential_purpose(&observation.purpose),
            })
            .collect()
    })
}

const fn external_adapter_credential_purpose(
    purpose: &CredentialDeliveryPurpose,
) -> ExternalAdapterCredentialPurpose {
    match purpose {
        CredentialDeliveryPurpose::ProviderApi => ExternalAdapterCredentialPurpose::ProviderApi,
        CredentialDeliveryPurpose::Registry => ExternalAdapterCredentialPurpose::Registry,
        CredentialDeliveryPurpose::ArtifactStore => ExternalAdapterCredentialPurpose::ArtifactStore,
        CredentialDeliveryPurpose::WebhookVerification => {
            ExternalAdapterCredentialPurpose::WebhookVerification
        }
    }
}

fn optional_source_string(
    source: &JsonObject,
    field: &'static str,
) -> Result<Option<String>, ExternalAdapterSkillAdapterError> {
    match source.get(field) {
        Some(JsonValue::String(value)) => Ok(Some(value.clone())),
        Some(_) => Err(ExternalAdapterSkillAdapterError::InvalidSourceMetadata { field }),
        None => Ok(None),
    }
}

fn invocation_cwd(request: &SkillInvocation) -> String {
    let Some(cwd) = request.source.cwd.as_ref() else {
        return request.skill_directory.to_string_lossy().into_owned();
    };
    let path = Path::new(cwd);
    if path.is_absolute() {
        return cwd.clone();
    }
    request
        .skill_directory
        .join(PathBuf::from(cwd))
        .to_string_lossy()
        .into_owned()
}

fn invocation_env(env: &BTreeMap<String, String>) -> Option<JsonObject> {
    (!env.is_empty()).then(|| {
        env.iter()
            .map(|(key, value)| (key.clone(), JsonValue::String(value.clone())))
            .collect()
    })
}

fn skill_output_from_outcome(
    outcome: ExternalAdapterProcessOutcome,
    credential_delivery: &CredentialDelivery,
) -> Result<SkillOutput, ExternalAdapterSkillAdapterError> {
    let response = outcome.response;
    let status = runtime_status(&response.status);
    let stdout = response_stdout(&response)?;
    let stderr = response.stderr.clone().unwrap_or_default();
    let exit_code = response_exit_code(&response)?;
    let mut metadata = response.metadata.clone().unwrap_or_default();
    metadata.insert(
        "adapter_id".to_owned(),
        JsonValue::String(response.adapter_id.clone()),
    );
    metadata.insert(
        "external_adapter_status".to_owned(),
        JsonValue::String(external_adapter_status_label(&response.status).to_owned()),
    );
    if let Some(process_exit_code) = outcome.process_exit_code {
        metadata.insert(
            "process_exit_code".to_owned(),
            JsonValue::Number(JsonNumber::I64(i64::from(process_exit_code))),
        );
    }
    add_credential_delivery_metadata(&mut metadata, credential_delivery)?;

    Ok(SkillOutput {
        status,
        stdout,
        stderr,
        exit_code,
        duration_ms: outcome.duration_ms,
        metadata,
    })
}

fn add_credential_delivery_metadata(
    metadata: &mut JsonObject,
    credential_delivery: &CredentialDelivery,
) -> Result<(), ExternalAdapterSkillAdapterError> {
    let Some(observation) = credential_delivery.public_observation() else {
        return Ok(());
    };
    let observation: JsonValue = serde_json::to_value(observation)
        .and_then(serde_json::from_value)
        .map_err(|source| {
            json_adapter_error("serializing credential delivery observation", source)
        })?;
    metadata.insert(
        CREDENTIAL_DELIVERY_OBSERVATIONS_METADATA.to_owned(),
        JsonValue::Array(vec![observation]),
    );
    Ok(())
}

fn runtime_status(status: &ExternalAdapterStatus) -> InvocationStatus {
    match status {
        ExternalAdapterStatus::Completed => InvocationStatus::Success,
        ExternalAdapterStatus::Failed
        | ExternalAdapterStatus::HostResolutionRequested
        | ExternalAdapterStatus::Cancelled => InvocationStatus::Failure,
    }
}

fn response_stdout(
    response: &ExternalAdapterResponse,
) -> Result<String, ExternalAdapterSkillAdapterError> {
    if let Some(stdout) = response.stdout.clone() {
        return Ok(stdout);
    }
    let Some(output) = response.output.as_ref() else {
        return Ok(String::new());
    };
    serde_json::to_string(&JsonValue::Object(output.clone()))
        .map_err(|source| json_adapter_error("serializing external adapter output", source))
}

fn response_exit_code(
    response: &ExternalAdapterResponse,
) -> Result<Option<i32>, ExternalAdapterSkillAdapterError> {
    let Some(exit_code) = response.exit_code.flatten() else {
        return Ok(None);
    };
    i32::try_from(exit_code)
        .map(Some)
        .map_err(|_| ExternalAdapterSkillAdapterError::ExitCodeOutOfRange { actual: exit_code })
}

fn external_adapter_status_label(status: &ExternalAdapterStatus) -> &'static str {
    match status {
        ExternalAdapterStatus::Completed => "completed",
        ExternalAdapterStatus::Failed => "failed",
        ExternalAdapterStatus::HostResolutionRequested => "host_resolution_requested",
        ExternalAdapterStatus::Cancelled => "cancelled",
    }
}

fn reference(reference_type: ReferenceType, uri: &str) -> Reference {
    Reference {
        reference_type,
        uri: uri.to_owned(),
        provider: None,
        locator: None,
        label: None,
        observed_at: None,
        proof_kind: None,
    }
}

fn identifier_segment(value: &str) -> String {
    normalize_request_id(value)
        .trim_matches(['.', '_', '-'])
        .replace('.', "-")
}

fn normalize_request_id(value: &str) -> String {
    let mut normalized = String::new();
    let mut replaced = false;
    for character in value.chars() {
        if character.is_ascii_alphanumeric() || matches!(character, '_' | '.' | '-') {
            normalized.push(character);
            replaced = false;
        } else if !replaced {
            normalized.push('_');
            replaced = true;
        }
    }
    if normalized.trim_matches(['.', '_', '-']).is_empty() {
        return "skill".to_owned();
    }
    normalized
}

fn validate_invocation_contract(
    manifest: &ExternalAdapterManifest,
    invocation: &ExternalAdapterInvocation,
) -> Result<(), ExternalAdapterSupervisorError> {
    if manifest.schema != MANIFEST_SCHEMA {
        return Err(ExternalAdapterSupervisorError::UnsupportedManifestSchema {
            actual: manifest.schema.clone(),
        });
    }
    if invocation.schema != INVOCATION_SCHEMA {
        return Err(
            ExternalAdapterSupervisorError::UnsupportedInvocationSchema {
                actual: invocation.schema.clone(),
            },
        );
    }
    if manifest.protocol_version != EXTERNAL_ADAPTER_PROTOCOL_VERSION {
        return Err(
            ExternalAdapterSupervisorError::UnsupportedManifestProtocol {
                actual: manifest.protocol_version.clone(),
            },
        );
    }
    if invocation.protocol_version != EXTERNAL_ADAPTER_PROTOCOL_VERSION {
        return Err(
            ExternalAdapterSupervisorError::UnsupportedInvocationProtocol {
                actual: invocation.protocol_version.clone(),
            },
        );
    }
    if manifest.adapter_id != invocation.adapter_id {
        return Err(ExternalAdapterSupervisorError::AdapterIdMismatch {
            manifest_adapter_id: manifest.adapter_id.clone(),
            invocation_adapter_id: invocation.adapter_id.clone(),
        });
    }
    if !manifest
        .supported_source_types
        .iter()
        .any(|source_type| source_type == &invocation.source_type)
    {
        return Err(ExternalAdapterSupervisorError::UnsupportedSourceType {
            adapter_id: manifest.adapter_id.clone(),
            source_type: invocation.source_type.clone(),
        });
    }
    if manifest.timeouts.startup_ms == 0 {
        return Err(ExternalAdapterSupervisorError::InvalidStartupTimeout);
    }
    if manifest.timeouts.invocation_ms == 0 {
        return Err(ExternalAdapterSupervisorError::InvalidInvocationTimeout);
    }
    if manifest.transport.kind != ExternalAdapterTransportKind::Process {
        return Err(ExternalAdapterSupervisorError::UnsupportedTransport {
            kind: manifest.transport.kind.clone(),
        });
    }
    Ok(())
}

fn process_command(
    manifest: &ExternalAdapterManifest,
) -> Result<&str, ExternalAdapterSupervisorError> {
    let command = manifest
        .transport
        .command
        .as_deref()
        .ok_or(ExternalAdapterSupervisorError::MissingProcessCommand)?;
    if command.trim().is_empty() {
        return Err(ExternalAdapterSupervisorError::EmptyProcessCommand);
    }
    Ok(command)
}

fn spawn_process(
    process_command: &str,
    manifest: &ExternalAdapterManifest,
    invocation: &ExternalAdapterInvocation,
    credential_delivery: &CredentialDelivery,
) -> Result<Child, ExternalAdapterSupervisorError> {
    let mut command = Command::new(process_command);
    if let Some(args) = manifest.transport.args.as_ref() {
        command.args(args);
    }
    if let Some(cwd) = invocation.cwd.as_ref() {
        command.current_dir(cwd);
    }
    command
        .env_clear()
        .envs(process_env(invocation, credential_delivery)?)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    configure_process_group(&mut command);
    command
        .spawn()
        .map_err(|source| io_error("spawning external adapter process", source))
}

fn process_env(
    invocation: &ExternalAdapterInvocation,
    credential_delivery: &CredentialDelivery,
) -> Result<BTreeMap<String, String>, ExternalAdapterSupervisorError> {
    let mut env = BTreeMap::new();
    if let Some(scoped_env) = invocation.env.as_ref() {
        for (key, value) in scoped_env {
            let JsonValue::String(value) = value else {
                return Err(ExternalAdapterSupervisorError::InvalidEnvValue { key: key.clone() });
            };
            env.insert(key.clone(), value.clone());
        }
    }
    if let Some(receipt_dir) = invocation.receipt_dir.as_ref() {
        env.insert("RUNX_RECEIPT_DIR".to_owned(), receipt_dir.clone());
    }
    for (key, value) in credential_delivery.secret_env().iter() {
        env.insert(key.to_owned(), value.to_owned());
    }
    Ok(env)
}

fn write_invocation(
    child: &mut Child,
    invocation: &ExternalAdapterInvocation,
) -> Result<(), ExternalAdapterSupervisorError> {
    let Some(mut stdin) = child.stdin.take() else {
        return Ok(());
    };
    serde_json::to_writer(&mut stdin, invocation)
        .map_err(|source| json_error("serializing external adapter invocation", source))?;
    stdin
        .write_all(b"\n")
        .map_err(|source| io_error("writing external adapter invocation", source))?;
    Ok(())
}

fn parse_response(
    bytes: &[u8],
    credential_delivery: &CredentialDelivery,
) -> Result<ExternalAdapterResponse, ExternalAdapterSupervisorError> {
    let bytes = trim_ascii_whitespace(bytes);
    if bytes.is_empty() {
        return Err(ExternalAdapterSupervisorError::EmptyResponse);
    }
    let frame: ExternalAdapterFrameSchema = serde_json::from_slice(bytes)
        .map_err(|source| json_error("parsing external adapter response frame", source))?;
    match frame.schema.as_str() {
        RESPONSE_SCHEMA => {
            let mut response: ExternalAdapterResponse =
                serde_json::from_slice(bytes).map_err(|source| {
                    json_error("validating external adapter response frame", source)
                })?;
            redact_response(&mut response, credential_delivery);
            Ok(response)
        }
        CREDENTIAL_REQUEST_SCHEMA => {
            let request: ExternalAdapterCredentialRequest =
                serde_json::from_slice(bytes).map_err(|source| {
                    json_error(
                        "validating unexpected external adapter credential request",
                        source,
                    )
                })?;
            Err(
                ExternalAdapterSupervisorError::UnexpectedCredentialRequest {
                    request_id: credential_delivery.redact_text(request.request_id),
                },
            )
        }
        HOST_RESOLUTION_SCHEMA => {
            let frame: ExternalAdapterHostResolutionFrame =
                serde_json::from_slice(bytes).map_err(|source| {
                    json_error("validating external adapter host-resolution frame", source)
                })?;
            host_resolution_response(frame, credential_delivery)
        }
        other => Err(ExternalAdapterSupervisorError::UnsupportedFrameSchema {
            schema: credential_delivery.redact_text(other),
        }),
    }
}

#[derive(Debug, serde::Deserialize)]
struct ExternalAdapterFrameSchema {
    schema: String,
}

fn host_resolution_response(
    frame: ExternalAdapterHostResolutionFrame,
    credential_delivery: &CredentialDelivery,
) -> Result<ExternalAdapterResponse, ExternalAdapterSupervisorError> {
    let request: JsonValue = serde_json::to_value(&frame.request)
        .and_then(serde_json::from_value)
        .map_err(|source| {
            json_error(
                "serializing external adapter host-resolution request",
                source,
            )
        })?;
    let mut metadata = JsonObject::new();
    metadata.insert(
        HOST_RESOLUTION_FRAME_ID_METADATA.to_owned(),
        JsonValue::String(frame.frame_id.clone()),
    );
    metadata.insert(HOST_RESOLUTION_REQUEST_METADATA.to_owned(), request);
    let mut response = ExternalAdapterResponse {
        schema: RESPONSE_SCHEMA.to_owned(),
        protocol_version: frame.protocol_version,
        invocation_id: frame.invocation_id,
        adapter_id: frame.adapter_id,
        status: ExternalAdapterStatus::HostResolutionRequested,
        stdout: None,
        stderr: Some("external adapter requested host resolution".to_owned()),
        exit_code: Some(None),
        output: None,
        artifacts: None,
        errors: None,
        telemetry: None,
        metadata: Some(metadata),
        observed_at: frame.requested_at,
    };
    redact_response(&mut response, credential_delivery);
    Ok(response)
}

fn redact_response(
    response: &mut ExternalAdapterResponse,
    credential_delivery: &CredentialDelivery,
) {
    response.schema = credential_delivery.redact_text(std::mem::take(&mut response.schema));
    response.protocol_version =
        credential_delivery.redact_text(std::mem::take(&mut response.protocol_version));
    response.invocation_id =
        credential_delivery.redact_text(std::mem::take(&mut response.invocation_id));
    response.adapter_id = credential_delivery.redact_text(std::mem::take(&mut response.adapter_id));
    response.observed_at =
        credential_delivery.redact_text(std::mem::take(&mut response.observed_at));
    if let Some(stdout) = response.stdout.take() {
        response.stdout = Some(credential_delivery.redact_text(stdout));
    }
    if let Some(stderr) = response.stderr.take() {
        response.stderr = Some(credential_delivery.redact_text(stderr));
    }
    if let Some(output) = response.output.as_mut() {
        redact_json_object(output, credential_delivery);
    }
    if let Some(metadata) = response.metadata.as_mut() {
        redact_json_object(metadata, credential_delivery);
    }
    if let Some(artifacts) = response.artifacts.as_mut() {
        for artifact in artifacts {
            if let Some(summary) = artifact.summary.take() {
                artifact.summary = Some(credential_delivery.redact_text(summary));
            }
        }
    }
    if let Some(errors) = response.errors.as_mut() {
        for error in errors {
            error.code = credential_delivery.redact_text(std::mem::take(&mut error.code));
            error.message = credential_delivery.redact_text(std::mem::take(&mut error.message));
        }
    }
    if let Some(telemetry) = response.telemetry.as_mut() {
        for observation in telemetry {
            observation.name =
                credential_delivery.redact_text(std::mem::take(&mut observation.name));
            if let Some(unit) = observation.unit.take() {
                observation.unit = Some(credential_delivery.redact_text(unit));
            }
            if let ExternalAdapterTelemetryValue::String(value) = &mut observation.value {
                *value = credential_delivery.redact_text(std::mem::take(value));
            }
        }
    }
}

fn redact_json_object(object: &mut JsonObject, credential_delivery: &CredentialDelivery) {
    for value in object.values_mut() {
        redact_json_value(value, credential_delivery);
    }
}

fn redact_json_value(value: &mut JsonValue, credential_delivery: &CredentialDelivery) {
    match value {
        JsonValue::String(text) => {
            *text = credential_delivery.redact_text(std::mem::take(text));
        }
        JsonValue::Array(values) => {
            for value in values {
                redact_json_value(value, credential_delivery);
            }
        }
        JsonValue::Object(object) => redact_json_object(object, credential_delivery),
        JsonValue::Null | JsonValue::Bool(_) | JsonValue::Number(_) => {}
    }
}

fn validate_response_contract(
    invocation: &ExternalAdapterInvocation,
    response: &ExternalAdapterResponse,
) -> Result<(), ExternalAdapterSupervisorError> {
    if response.schema != RESPONSE_SCHEMA {
        return Err(ExternalAdapterSupervisorError::UnsupportedResponseSchema {
            actual: response.schema.clone(),
        });
    }
    if response.protocol_version != EXTERNAL_ADAPTER_PROTOCOL_VERSION {
        return Err(
            ExternalAdapterSupervisorError::UnsupportedResponseProtocol {
                actual: response.protocol_version.clone(),
            },
        );
    }
    if response.adapter_id != invocation.adapter_id {
        return Err(ExternalAdapterSupervisorError::ResponseMismatch {
            field: "adapter_id",
            expected: invocation.adapter_id.clone(),
            actual: response.adapter_id.clone(),
        });
    }
    if response.invocation_id != invocation.invocation_id {
        return Err(ExternalAdapterSupervisorError::ResponseMismatch {
            field: "invocation_id",
            expected: invocation.invocation_id.clone(),
            actual: response.invocation_id.clone(),
        });
    }
    Ok(())
}

fn timeout_cancellation_frame(
    manifest: &ExternalAdapterManifest,
    invocation: &ExternalAdapterInvocation,
    timeout_ms: u64,
) -> ExternalAdapterCancellationFrame {
    ExternalAdapterCancellationFrame {
        schema: CANCELLATION_SCHEMA.to_owned(),
        protocol_version: EXTERNAL_ADAPTER_PROTOCOL_VERSION.to_owned(),
        frame_id: format!("{}_timeout_cancel", invocation.invocation_id),
        invocation_id: invocation.invocation_id.clone(),
        adapter_id: manifest.adapter_id.clone(),
        reason: format!("invocation timeout after {timeout_ms}ms"),
        requested_at: now_iso8601(),
    }
}

fn capture_pipe<R>(
    pipe: Option<R>,
    context: &'static str,
) -> Result<JoinHandle<std::io::Result<CapturedOutput>>, ExternalAdapterSupervisorError>
where
    R: Read + Send + 'static,
{
    pipe.map(capture_stream)
        .ok_or_else(|| io_error(context, std::io::Error::other("pipe was not captured")))
}

fn capture_stream<R>(mut reader: R) -> JoinHandle<std::io::Result<CapturedOutput>>
where
    R: Read + Send + 'static,
{
    thread::spawn(move || {
        let mut captured = Vec::new();
        let mut truncated = false;
        let mut buffer = [0_u8; 8192];
        loop {
            let count = reader.read(&mut buffer)?;
            if count == 0 {
                return Ok(CapturedOutput {
                    bytes: captured,
                    truncated,
                });
            }
            let remaining = RESPONSE_LIMIT_BYTES.saturating_sub(captured.len());
            if remaining > 0 {
                captured.extend_from_slice(&buffer[..count.min(remaining)]);
            }
            if count > remaining {
                truncated = true;
            }
        }
    })
}

fn join_capture(
    handle: JoinHandle<std::io::Result<CapturedOutput>>,
    context: &'static str,
) -> Result<CapturedOutput, ExternalAdapterSupervisorError> {
    match handle.join() {
        Ok(Ok(output)) => Ok(output),
        Ok(Err(source)) => Err(io_error(context, source)),
        Err(_) => Err(io_error(
            context,
            std::io::Error::other("output reader thread failed"),
        )),
    }
}

fn wait_for_exit(
    child: &mut Child,
    timeout: Duration,
) -> Result<WaitResult, ExternalAdapterSupervisorError> {
    let started = Instant::now();
    loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|source| io_error("polling external adapter process", source))?
        {
            return Ok(WaitResult::Exited(status));
        }
        if started.elapsed() >= timeout {
            kill_timed_out_process(child, KillSignal::Terminate)?;
            thread::sleep(FORCE_KILL_GRACE);
            kill_timed_out_process(child, KillSignal::Force)?;
            child.wait().map_err(|source| {
                io_error("waiting for timed out external adapter process", source)
            })?;
            return Ok(WaitResult::TimedOut);
        }
        thread::sleep(POLL_INTERVAL);
    }
}

#[cfg(unix)]
fn configure_process_group(command: &mut Command) {
    command.process_group(0);
}

#[cfg(not(unix))]
fn configure_process_group(_command: &mut Command) {}

enum KillSignal {
    Terminate,
    Force,
}

impl KillSignal {
    #[cfg(unix)]
    fn kill_arg(&self) -> &'static str {
        match self {
            Self::Terminate => "-TERM",
            Self::Force => "-KILL",
        }
    }
}

#[cfg(unix)]
fn kill_timed_out_process(
    child: &mut Child,
    signal: KillSignal,
) -> Result<(), ExternalAdapterSupervisorError> {
    let process_group = format!("-{}", child.id());
    let status = Command::new("/bin/kill")
        .arg(signal.kill_arg())
        .arg(&process_group)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    if status.is_ok_and(|status| status.success()) {
        return Ok(());
    }
    if child
        .try_wait()
        .map_err(|source| io_error("polling timed out external adapter process", source))?
        .is_some()
    {
        return Ok(());
    }
    kill_direct_child_if_running(child)
}

#[cfg(not(unix))]
fn kill_timed_out_process(
    child: &mut Child,
    _signal: KillSignal,
) -> Result<(), ExternalAdapterSupervisorError> {
    kill_direct_child_if_running(child)
}

fn kill_direct_child_if_running(child: &mut Child) -> Result<(), ExternalAdapterSupervisorError> {
    if child
        .try_wait()
        .map_err(|source| io_error("polling timed out external adapter process", source))?
        .is_some()
    {
        return Ok(());
    }
    child
        .kill()
        .map_err(|source| io_error("killing timed out external adapter process", source))
}

fn trim_ascii_whitespace(bytes: &[u8]) -> &[u8] {
    let start = bytes
        .iter()
        .position(|byte| !byte.is_ascii_whitespace())
        .unwrap_or(bytes.len());
    let end = bytes
        .iter()
        .rposition(|byte| !byte.is_ascii_whitespace())
        .map_or(start, |index| index + 1);
    &bytes[start..end]
}

fn duration_ms(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
}

fn io_error(context: impl Into<String>, source: std::io::Error) -> ExternalAdapterSupervisorError {
    ExternalAdapterSupervisorError::Io {
        context: context.into(),
        source,
    }
}

fn json_error(
    context: impl Into<String>,
    source: serde_json::Error,
) -> ExternalAdapterSupervisorError {
    ExternalAdapterSupervisorError::Json {
        context: context.into(),
        source,
    }
}

fn json_adapter_error(
    context: impl Into<String>,
    source: serde_json::Error,
) -> ExternalAdapterSkillAdapterError {
    ExternalAdapterSkillAdapterError::Json {
        context: context.into(),
        source,
    }
}

struct CapturedOutput {
    bytes: Vec<u8>,
    truncated: bool,
}

enum WaitResult {
    Exited(ExitStatus),
    TimedOut,
}
