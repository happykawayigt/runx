use std::time::{Duration, Instant};

use runx_contracts::{JsonObject, JsonValue, sha256_hex};

use crate::RuntimeError;
use crate::adapter::{InvocationStatus, SkillAdapter, SkillInvocation, SkillOutput};
use crate::credentials::CredentialDelivery;
use crate::sandbox::{prepare_mcp_process_sandbox, sandbox_metadata};

use super::sandbox_metadata::mcp_process_sandbox_metadata;
use super::templates::map_mcp_arguments;
use super::transport::ProcessMcpTransport;
use super::types::{McpToolCallRequest, McpTransport};

const DEFAULT_TIMEOUT_MS: u64 = 60_000;
const MIN_TIMEOUT_MS: u64 = 50;

#[derive(Clone, Debug)]
pub struct McpAdapter<T = ProcessMcpTransport> {
    transport: T,
}

impl<T> McpAdapter<T> {
    #[must_use]
    pub const fn new(transport: T) -> Self {
        Self { transport }
    }
}

impl Default for McpAdapter<ProcessMcpTransport> {
    fn default() -> Self {
        Self::new(ProcessMcpTransport)
    }
}

impl<T> SkillAdapter for McpAdapter<T>
where
    T: McpTransport,
{
    fn adapter_type(&self) -> &'static str {
        "mcp"
    }

    fn invoke(&self, request: SkillInvocation) -> Result<SkillOutput, RuntimeError> {
        let started = Instant::now();
        let prepared = match prepare_mcp_tool_call(request, started)? {
            Ok(prepared) => prepared,
            Err(output) => return Ok(output),
        };
        match self.transport.call_tool(prepared.request) {
            Ok(result) => Ok(SkillOutput {
                status: InvocationStatus::Success,
                stdout: prepared
                    .credential_delivery
                    .redact_text(super::templates::stringify_mcp_tool_result(&result)?),
                stderr: String::new(),
                exit_code: Some(0),
                duration_ms: duration_ms(started),
                metadata: prepared.success_metadata,
            }),
            Err(error) => Ok(failure(
                prepared
                    .credential_delivery
                    .redact_text(error.sanitized_message()),
                started,
                prepared.failure_metadata,
            )),
        }
    }
}

#[derive(Debug)]
struct PreparedMcpToolCall {
    request: McpToolCallRequest,
    credential_delivery: CredentialDelivery,
    success_metadata: JsonObject,
    failure_metadata: JsonObject,
}

fn prepare_mcp_tool_call(
    invocation: SkillInvocation,
    started: Instant,
) -> Result<Result<PreparedMcpToolCall, SkillOutput>, RuntimeError> {
    let SkillInvocation {
        source,
        inputs,
        resolved_inputs,
        skill_directory,
        env,
        credential_delivery,
        ..
    } = invocation;
    if source.source_type != runx_parser::SourceKind::Mcp {
        return Err(RuntimeError::UnsupportedAdapter {
            adapter_type: source.source_type.as_str().to_owned(),
        });
    }
    let Some(server) = source.server.clone() else {
        return Ok(Err(missing_mcp_metadata(started)));
    };
    let Some(tool) = source.tool.clone().filter(|tool| !tool.is_empty()) else {
        return Ok(Err(missing_mcp_metadata(started)));
    };
    let arguments = map_mcp_arguments(source.arguments.as_ref(), &inputs, &resolved_inputs)?;
    let sandbox = match prepare_mcp_process_sandbox(&source, &server, &skill_directory, &env) {
        Ok(plan) => plan,
        Err(RuntimeError::SandboxViolation { message }) => {
            return Ok(Err(failure(
                format!("MCP sandbox denied: {message}"),
                started,
                metadata_for(&source, Some(sandbox_metadata(source.sandbox.as_ref())))?,
            )));
        }
        Err(error) => return Err(error),
    };
    let success_metadata = metadata_for(
        &source,
        Some(mcp_process_sandbox_metadata(
            source.sandbox.as_ref(),
            &sandbox,
            &env,
        )?),
    )?;
    let failure_metadata = metadata_for(&source, None)?;
    Ok(Ok(PreparedMcpToolCall {
        request: McpToolCallRequest {
            server,
            tool,
            arguments,
            timeout: timeout_from_source(source.timeout_seconds),
            sandbox,
            secret_env: credential_delivery.secret_env().clone(),
        },
        credential_delivery,
        success_metadata,
        failure_metadata,
    }))
}

fn missing_mcp_metadata(started: Instant) -> SkillOutput {
    failure(
        "MCP source requires server and tool metadata.",
        started,
        JsonObject::new(),
    )
}

fn metadata_for(
    source: &runx_parser::SkillSource,
    sandbox: Option<JsonObject>,
) -> Result<JsonObject, RuntimeError> {
    let mut mcp = JsonObject::new();
    mcp.insert(
        "tool".to_owned(),
        JsonValue::String(source.tool.clone().unwrap_or_default()),
    );
    let server = source.server.as_ref();
    mcp.insert(
        "server_command_hash".to_owned(),
        JsonValue::String(sha256_hex(
            server
                .map(|server| server.command.as_bytes())
                .unwrap_or(b""),
        )),
    );
    let args = serde_json::to_string(&server.map(|server| &server.args))
        .map_err(|source| RuntimeError::json("serializing MCP server args", source))?;
    mcp.insert(
        "server_args_hash".to_owned(),
        JsonValue::String(sha256_hex(args.as_bytes())),
    );

    let mut metadata = JsonObject::new();
    metadata.insert("mcp".to_owned(), JsonValue::Object(mcp));
    if let Some(sandbox) = sandbox.filter(|sandbox| !sandbox.is_empty()) {
        metadata.insert("sandbox".to_owned(), JsonValue::Object(sandbox));
    }
    Ok(metadata)
}

pub(super) fn failure(
    message: impl Into<String>,
    started: Instant,
    metadata: JsonObject,
) -> SkillOutput {
    let message = message.into();
    SkillOutput {
        status: InvocationStatus::Failure,
        stdout: String::new(),
        stderr: message,
        exit_code: None,
        duration_ms: duration_ms(started),
        metadata,
    }
}

pub(super) fn duration_ms(started: Instant) -> u64 {
    let millis = started.elapsed().as_millis();
    u64::try_from(millis).unwrap_or(u64::MAX)
}

fn timeout_from_source(timeout_seconds: Option<u64>) -> Duration {
    let timeout_ms = timeout_seconds
        .map(|seconds| seconds.saturating_mul(1000))
        .unwrap_or(DEFAULT_TIMEOUT_MS)
        .max(MIN_TIMEOUT_MS);
    Duration::from_millis(timeout_ms)
}
