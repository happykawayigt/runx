// rust-style-allow: large-file because the MCP slice keeps the adapter,
// fixture transport, bounded stdio framing, argument mapping, and sanitization
// beside each other until server routing introduces natural module boundaries.
use std::collections::BTreeMap;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::{Duration, Instant};

use runx_contracts::{
    ApprovalGate, ExecutionEvent, JsonNumber, JsonObject, JsonValue, Question, ResolutionRequest,
    ResolutionResponse,
};
use runx_core::state_machine::GraphStatus;
use runx_parser::{SkillInput, SkillMcpServer, SkillSandbox, ValidatedSkill};
use sha2::{Digest, Sha256};

use crate::RuntimeError;
use crate::adapter::{InvocationStatus, SkillAdapter, SkillInvocation, SkillOutput};
use crate::caller::Caller;
use crate::receipt_store::LocalReceiptStore;
use crate::receipts::step_receipt;
use crate::sandbox::{SandboxPlan, prepare_mcp_process_sandbox, sandbox_metadata};
use crate::{GraphCheckpoint, GraphRun, Runtime, RuntimeOptions, StepRun};

const DEFAULT_TIMEOUT_MS: u64 = 60_000;
const MIN_TIMEOUT_MS: u64 = 50;
const MAX_CLIENT_RESPONSE_BYTES: usize = 1024 * 1024;
const MAX_SERVER_REQUEST_BYTES: usize = 4 * 1024 * 1024;
const DEFAULT_SANDBOX_ENV_ALLOWLIST: [&str; 9] = [
    "PATH",
    "HOME",
    "TMPDIR",
    "TMP",
    "TEMP",
    "SystemRoot",
    "WINDIR",
    "COMSPEC",
    "PATHEXT",
];
const POLL_INTERVAL: Duration = Duration::from_millis(10);
const PROTOCOL_VERSION: &str = "2025-06-18";
const TEMPLATE_OPEN: &str = "\x7b\x7b";
const TEMPLATE_CLOSE: &str = "\x7d\x7d";

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
                stdout: stringify_mcp_tool_result(&result)?,
                stderr: String::new(),
                exit_code: Some(0),
                duration_ms: duration_ms(started),
                metadata: prepared.success_metadata,
            }),
            Err(error) => Ok(failure(
                error.sanitized_message(),
                started,
                prepared.failure_metadata,
            )),
        }
    }
}

#[derive(Debug)]
struct PreparedMcpToolCall {
    request: McpToolCallRequest,
    success_metadata: JsonObject,
    failure_metadata: JsonObject,
}

#[derive(Clone, Debug, PartialEq)]
pub struct McpToolCallRequest {
    pub server: SkillMcpServer,
    pub tool: String,
    pub arguments: JsonObject,
    pub timeout: Duration,
    pub sandbox: SandboxPlan,
}

#[derive(Clone, Debug, PartialEq)]
pub struct McpListToolsRequest {
    pub server: SkillMcpServer,
    pub timeout: Duration,
    pub sandbox: SandboxPlan,
}

#[derive(Clone, Debug, PartialEq)]
pub struct McpToolDescriptor {
    pub name: String,
    pub description: Option<String>,
    pub input_schema: Option<JsonObject>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct McpServerOptions {
    pub package_name: String,
    pub package_version: String,
    pub tools: Vec<McpServerTool>,
}

impl McpServerOptions {
    pub fn from_skill_paths(
        skill_paths: &[PathBuf],
        package_name: impl Into<String>,
        package_version: impl Into<String>,
    ) -> Result<Self, RuntimeError> {
        Self::from_skill_paths_with_execution(
            skill_paths,
            package_name,
            package_version,
            McpServerExecutionOptions::default(),
        )
    }

    pub fn from_skill_paths_with_execution(
        skill_paths: &[PathBuf],
        package_name: impl Into<String>,
        package_version: impl Into<String>,
        execution: McpServerExecutionOptions,
    ) -> Result<Self, RuntimeError> {
        if let Some(runner) = &execution.runner {
            return Err(RuntimeError::UnsupportedRunnerSelection {
                runner: runner.clone(),
            });
        }
        let mut tools = skill_paths
            .iter()
            .map(|path| load_mcp_server_tool(path, &execution))
            .collect::<Result<Vec<_>, _>>()?;
        tools.push(resume_tool_definition());
        Ok(Self {
            package_name: package_name.into(),
            package_version: package_version.into(),
            tools,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct McpServerExecutionOptions {
    pub runner: Option<String>,
    pub receipt_dir: Option<PathBuf>,
    pub env: BTreeMap<String, String>,
}

impl Default for McpServerExecutionOptions {
    fn default() -> Self {
        Self {
            runner: None,
            receipt_dir: None,
            env: std::env::vars().collect(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct McpServerTool {
    pub name: String,
    pub description: String,
    pub input_schema: JsonObject,
    pub result: McpServerToolBehavior,
}

#[derive(Clone, Debug, PartialEq)]
pub enum McpServerToolBehavior {
    Fixed(McpToolResult),
    NotImplemented(String),
    ResumeNotImplemented,
    Skill(Box<McpServerSkillExecution>),
    Resume,
}

#[derive(Clone, Debug, PartialEq)]
pub struct McpServerSkillExecution {
    pub skill_path: PathBuf,
    pub skill: ValidatedSkill,
    pub receipt_dir: Option<PathBuf>,
    pub env: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct McpToolResult {
    pub content: Vec<McpContent>,
    pub structured_content: Option<JsonObject>,
    pub is_error: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct McpContent {
    pub text: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum McpHostRunResult {
    Completed {
        skill_name: String,
        output: String,
        receipt_id: String,
        runx: JsonObject,
    },
    Paused {
        skill_name: String,
        run_id: String,
        request_count: usize,
        runx: JsonObject,
    },
    Denied {
        skill_name: String,
        receipt_id: Option<String>,
        runx: JsonObject,
    },
    Escalated {
        skill_name: String,
        receipt_id: String,
        error: String,
        runx: JsonObject,
    },
    Failed {
        skill_name: String,
        receipt_id: Option<String>,
        error: String,
        runx: JsonObject,
    },
}

#[derive(Debug)]
pub struct McpServerError {
    message: String,
}

impl McpServerError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for McpServerError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for McpServerError {}

pub trait McpTransport {
    fn call_tool(&self, request: McpToolCallRequest) -> Result<JsonValue, McpTransportError>;
}

impl<T> McpTransport for &T
where
    T: McpTransport + ?Sized,
{
    fn call_tool(&self, request: McpToolCallRequest) -> Result<JsonValue, McpTransportError> {
        (**self).call_tool(request)
    }
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
        ..
    } = invocation;
    if source.source_type != "mcp" {
        return Err(RuntimeError::UnsupportedAdapter {
            adapter_type: source.source_type,
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
        },
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct McpTransportError {
    kind: McpTransportErrorKind,
    message: String,
}

impl McpTransportError {
    #[must_use]
    pub fn failed(message: impl Into<String>) -> Self {
        Self {
            kind: McpTransportErrorKind::Failed,
            message: message.into(),
        }
    }

    #[must_use]
    pub fn tool_error(code: i64, message: impl Into<String>) -> Self {
        Self {
            kind: McpTransportErrorKind::ToolError(code),
            message: message.into(),
        }
    }

    #[must_use]
    pub fn timeout(timeout: Duration) -> Self {
        let timeout_ms = u64::try_from(timeout.as_millis()).unwrap_or(u64::MAX);
        Self {
            kind: McpTransportErrorKind::Timeout,
            message: format!("MCP call timed out after {timeout_ms}ms."),
        }
    }

    #[must_use]
    pub fn sanitized_message(&self) -> String {
        match self.kind {
            McpTransportErrorKind::ToolError(code) => {
                format!("MCP tool returned error {code}.")
            }
            McpTransportErrorKind::Timeout => self.message.clone(),
            McpTransportErrorKind::Failed => "MCP adapter failed.".to_owned(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum McpTransportErrorKind {
    ToolError(i64),
    Timeout,
    Failed,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct FixtureMcpTransport;

impl FixtureMcpTransport {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl McpTransport for FixtureMcpTransport {
    fn call_tool(&self, request: McpToolCallRequest) -> Result<JsonValue, McpTransportError> {
        match request.tool.as_str() {
            "echo" => Ok(text_content(js_string(request.arguments.get("message")))),
            "env" => Ok(text_content(env_value(
                &request.sandbox.env,
                request.arguments.get("name"),
            ))),
            "fail" => Err(McpTransportError::tool_error(
                -32000,
                format!(
                    "fixture failure: {}",
                    js_string(request.arguments.get("message"))
                ),
            )),
            "sleep" => Err(McpTransportError::timeout(request.timeout)),
            "malformed-json" => Err(McpTransportError::failed("MCP server sent invalid JSON.")),
            _ => Err(McpTransportError::tool_error(-32601, "tool not found")),
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ProcessMcpTransport;

impl ProcessMcpTransport {
    pub fn list_tools(
        &self,
        request: McpListToolsRequest,
    ) -> Result<Vec<McpToolDescriptor>, McpTransportError> {
        let mut client = initialize_mcp_client(&request.sandbox, request.timeout)?;
        let result = client.request(2, &tools_list_request(2))?;
        Ok(parse_mcp_tools_list(result))
    }
}

impl McpTransport for ProcessMcpTransport {
    fn call_tool(&self, request: McpToolCallRequest) -> Result<JsonValue, McpTransportError> {
        let mut client = initialize_mcp_client(&request.sandbox, request.timeout)?;
        client.request(2, &tool_call_request(2, &request.tool, &request.arguments))
    }
}

pub fn serve_mcp_json_rpc(
    input: impl Read,
    output: impl Write,
    options: McpServerOptions,
) -> Result<(), McpServerError> {
    assert_unique_server_tool_names(&options.tools)?;
    serve_mcp_json_rpc_checked(input, output, options)
}

pub fn mcp_tool_result_from_host_result(result: McpHostRunResult) -> McpToolResult {
    match result {
        McpHostRunResult::Completed {
            skill_name,
            output,
            receipt_id,
            runx,
        } => completed_mcp_tool_result(skill_name, output, receipt_id, runx),
        McpHostRunResult::Paused {
            skill_name,
            run_id,
            request_count,
            runx,
        } => paused_mcp_tool_result(skill_name, run_id, request_count, runx),
        McpHostRunResult::Denied {
            skill_name,
            receipt_id,
            runx,
        } => denied_mcp_tool_result(skill_name, receipt_id, runx),
        McpHostRunResult::Escalated {
            skill_name,
            receipt_id,
            error,
            runx,
        } => escalated_mcp_tool_result(skill_name, receipt_id, error, runx),
        McpHostRunResult::Failed {
            skill_name,
            receipt_id,
            error,
            runx,
        } => failed_mcp_tool_result(skill_name, receipt_id, error, runx),
    }
}

fn completed_mcp_tool_result(
    skill_name: String,
    output: String,
    receipt_id: String,
    runx: JsonObject,
) -> McpToolResult {
    let text = if output.trim().is_empty() {
        format!("{skill_name} completed. Inspect receipt {receipt_id}.")
    } else {
        output
    };
    mcp_host_tool_result(text, runx, false)
}

fn paused_mcp_tool_result(
    skill_name: String,
    run_id: String,
    request_count: usize,
    runx: JsonObject,
) -> McpToolResult {
    mcp_host_tool_result(
        format!(
            "{skill_name} paused at {run_id}. Resume with runx_resume after resolving {request_count} request(s)."
        ),
        runx,
        false,
    )
}

fn denied_mcp_tool_result(
    skill_name: String,
    receipt_id: Option<String>,
    runx: JsonObject,
) -> McpToolResult {
    let text = match receipt_id {
        Some(receipt_id) => format!("{skill_name} was denied by policy (receipt {receipt_id})."),
        None => format!("{skill_name} was denied by policy."),
    };
    mcp_host_tool_result(text, runx, true)
}

fn escalated_mcp_tool_result(
    skill_name: String,
    receipt_id: String,
    error: String,
    runx: JsonObject,
) -> McpToolResult {
    mcp_host_tool_result(
        format!("{skill_name} escalated. Inspect receipt {receipt_id}. {error}")
            .trim()
            .to_owned(),
        runx,
        true,
    )
}

fn failed_mcp_tool_result(
    skill_name: String,
    receipt_id: Option<String>,
    error: String,
    runx: JsonObject,
) -> McpToolResult {
    mcp_host_tool_result(
        format!(
            "{skill_name} failed. Inspect receipt {}. {error}",
            receipt_id.unwrap_or_else(|| "n/a".to_owned())
        )
        .trim()
        .to_owned(),
        runx,
        true,
    )
}

fn mcp_host_tool_result(text: String, runx: JsonObject, is_error: bool) -> McpToolResult {
    McpToolResult {
        content: vec![McpContent { text }],
        structured_content: Some(runx_content(runx)),
        is_error,
    }
}

fn serve_mcp_json_rpc_checked(
    mut input: impl Read,
    mut output: impl Write,
    options: McpServerOptions,
) -> Result<(), McpServerError> {
    let mut state = McpServerState::new(options);
    let mut buffer = Vec::new();
    let mut chunk = [0_u8; 8192];
    loop {
        let read = input
            .read(&mut chunk)
            .map_err(|error| McpServerError::new(format!("MCP request read failed: {error}")))?;
        if read == 0 {
            return Ok(());
        }
        buffer.extend_from_slice(&chunk[..read]);
        if buffer.len() > MAX_SERVER_REQUEST_BYTES {
            return Err(McpServerError::new(format!(
                "MCP request exceeded {MAX_SERVER_REQUEST_BYTES}-byte size limit."
            )));
        }
        write_available_server_responses(&mut buffer, &mut output, &mut state)?;
    }
}

#[derive(Debug)]
struct McpServerState {
    options: McpServerOptions,
    pending: BTreeMap<String, PendingMcpRun>,
    next_run_sequence: u64,
}

impl McpServerState {
    fn new(options: McpServerOptions) -> Self {
        Self {
            options,
            pending: BTreeMap::new(),
            next_run_sequence: 0,
        }
    }

    fn next_run_id(&mut self, skill_name: &str) -> String {
        self.next_run_sequence = self.next_run_sequence.saturating_add(1);
        format!(
            "rx_mcp_{}_{}",
            identifier_segment(skill_name),
            self.next_run_sequence
        )
    }
}

#[derive(Clone, Debug)]
struct PendingMcpRun {
    execution: McpServerSkillExecution,
    inputs: JsonObject,
    request: ResolutionRequest,
    checkpoint: Option<GraphCheckpoint>,
}

fn write_available_server_responses(
    buffer: &mut Vec<u8>,
    output: &mut impl Write,
    state: &mut McpServerState,
) -> Result<(), McpServerError> {
    while let Some(header_end) = find_header_end(buffer) {
        let header = std::str::from_utf8(&buffer[..header_end])
            .map_err(|_| McpServerError::new("MCP request header must be UTF-8."))?;
        let Some(content_length) = content_length(header) else {
            return Err(McpServerError::new(
                "MCP request requires a Content-Length header.",
            ));
        };
        if content_length > MAX_SERVER_REQUEST_BYTES {
            return Err(McpServerError::new(format!(
                "MCP request declared Content-Length {content_length}, exceeding {MAX_SERVER_REQUEST_BYTES}-byte limit."
            )));
        }
        let body_start = header_end + 4;
        let body_end = body_start.saturating_add(content_length);
        if buffer.len() < body_end {
            return Ok(());
        }
        let body = buffer[body_start..body_end].to_vec();
        buffer.drain(..body_end);
        let response = match serde_json::from_slice::<JsonValue>(&body) {
            Ok(request) => handle_mcp_server_request(state, request),
            Err(_) => Some(json_rpc_error(JsonValue::Null, -32700, "parse error")),
        };
        if let Some(response) = response {
            write_framed_json(output, &response)?;
        }
    }
    Ok(())
}

fn handle_mcp_server_request(state: &mut McpServerState, request: JsonValue) -> Option<JsonValue> {
    let JsonValue::Object(record) = request else {
        return Some(json_rpc_error(JsonValue::Null, -32600, "invalid request"));
    };
    let id = record.get("id").cloned().unwrap_or(JsonValue::Null);
    let method = match record.get("method") {
        Some(JsonValue::String(method)) => method.as_str(),
        _ => return Some(json_rpc_error(id, -32600, "invalid request")),
    };
    match method {
        "initialize" => Some(json_rpc_response(
            id,
            initialize_server_result(&state.options),
        )),
        "ping" => Some(json_rpc_response(id, JsonValue::Object(JsonObject::new()))),
        "tools/list" => Some(json_rpc_response(
            id,
            tools_list_result(&state.options.tools),
        )),
        "tools/call" => Some(handle_mcp_server_tool_call(state, id, record.get("params"))),
        _ if matches!(record.get("id"), None | Some(JsonValue::Null)) => None,
        _ => Some(json_rpc_error(id, -32601, "method not found")),
    }
}

fn handle_mcp_server_tool_call(
    state: &mut McpServerState,
    id: JsonValue,
    params: Option<&JsonValue>,
) -> JsonValue {
    let Some(JsonValue::Object(params)) = params else {
        return json_rpc_error(id, -32602, "invalid tool call");
    };
    let Some(JsonValue::String(name)) = params.get("name") else {
        return json_rpc_error(id, -32602, "invalid tool call");
    };
    if let Some(arguments) = params.get("arguments")
        && !matches!(arguments, JsonValue::Object(_))
    {
        return json_rpc_error(id, -32602, "tool arguments must be an object");
    }
    let Some(tool) = state.options.tools.iter().find(|tool| &tool.name == name) else {
        return json_rpc_error(id, -32601, &format!("tool not found: {name}"));
    };
    let arguments = match params.get("arguments") {
        Some(JsonValue::Object(arguments)) => arguments.clone(),
        _ => JsonObject::new(),
    };
    match tool.result.clone() {
        McpServerToolBehavior::Fixed(result) => {
            json_rpc_response(id, mcp_tool_result_json(&result))
        }
        McpServerToolBehavior::NotImplemented(message) => json_rpc_error(id, -32000, &message),
        McpServerToolBehavior::ResumeNotImplemented => json_rpc_error(
            id,
            -32000,
            "Rust MCP runx_resume execution is not wired yet.",
        ),
        McpServerToolBehavior::Skill(execution) => {
            match execute_mcp_server_skill(state, *execution, arguments) {
                Ok(result) => json_rpc_response(id, mcp_tool_result_json(&result)),
                Err(error) => json_rpc_error(id, -32000, &error.to_string()),
            }
        }
        McpServerToolBehavior::Resume => match resume_mcp_server_run(state, arguments) {
            Ok(result) => json_rpc_response(id, mcp_tool_result_json(&result)),
            Err(error) => json_rpc_error(id, -32000, &error.to_string()),
        },
    }
}

fn initialize_server_result(options: &McpServerOptions) -> JsonValue {
    JsonValue::Object(
        [
            (
                "protocolVersion".to_owned(),
                JsonValue::String(PROTOCOL_VERSION.to_owned()),
            ),
            (
                "capabilities".to_owned(),
                JsonValue::Object(
                    [("tools".to_owned(), JsonValue::Object(JsonObject::new()))].into(),
                ),
            ),
            (
                "serverInfo".to_owned(),
                JsonValue::Object(
                    [
                        (
                            "name".to_owned(),
                            JsonValue::String(options.package_name.clone()),
                        ),
                        (
                            "version".to_owned(),
                            JsonValue::String(options.package_version.clone()),
                        ),
                    ]
                    .into(),
                ),
            ),
        ]
        .into(),
    )
}

fn tools_list_result(tools: &[McpServerTool]) -> JsonValue {
    JsonValue::Object(
        [(
            "tools".to_owned(),
            JsonValue::Array(tools.iter().map(server_tool_json).collect()),
        )]
        .into(),
    )
}

fn server_tool_json(tool: &McpServerTool) -> JsonValue {
    JsonValue::Object(
        [
            ("name".to_owned(), JsonValue::String(tool.name.clone())),
            (
                "description".to_owned(),
                JsonValue::String(tool.description.clone()),
            ),
            (
                "inputSchema".to_owned(),
                JsonValue::Object(tool.input_schema.clone()),
            ),
        ]
        .into(),
    )
}

fn mcp_tool_result_json(result: &McpToolResult) -> JsonValue {
    let mut record = JsonObject::new();
    record.insert(
        "content".to_owned(),
        JsonValue::Array(
            result
                .content
                .iter()
                .map(|entry| {
                    JsonValue::Object(
                        [
                            ("type".to_owned(), JsonValue::String("text".to_owned())),
                            ("text".to_owned(), JsonValue::String(entry.text.clone())),
                        ]
                        .into(),
                    )
                })
                .collect(),
        ),
    );
    if let Some(structured_content) = &result.structured_content {
        record.insert(
            "structuredContent".to_owned(),
            JsonValue::Object(structured_content.clone()),
        );
    }
    if result.is_error {
        record.insert("isError".to_owned(), JsonValue::Bool(true));
    }
    JsonValue::Object(record)
}

fn json_rpc_response(id: JsonValue, result: JsonValue) -> JsonValue {
    JsonValue::Object(
        [
            ("jsonrpc".to_owned(), JsonValue::String("2.0".to_owned())),
            ("id".to_owned(), id),
            ("result".to_owned(), result),
        ]
        .into(),
    )
}

fn json_rpc_error(id: JsonValue, code: i64, message: &str) -> JsonValue {
    JsonValue::Object(
        [
            ("jsonrpc".to_owned(), JsonValue::String("2.0".to_owned())),
            ("id".to_owned(), id),
            (
                "error".to_owned(),
                JsonValue::Object(
                    [
                        ("code".to_owned(), JsonValue::Number(JsonNumber::I64(code))),
                        ("message".to_owned(), JsonValue::String(message.to_owned())),
                    ]
                    .into(),
                ),
            ),
        ]
        .into(),
    )
}

fn write_framed_json(output: &mut impl Write, message: &JsonValue) -> Result<(), McpServerError> {
    let body = serde_json::to_vec(message).map_err(|error| {
        McpServerError::new(format!("MCP response serialization failed: {error}"))
    })?;
    write!(output, "Content-Length: {}\r\n\r\n", body.len())
        .and_then(|()| output.write_all(&body))
        .map_err(|error| McpServerError::new(format!("MCP response write failed: {error}")))
}

fn assert_unique_server_tool_names(tools: &[McpServerTool]) -> Result<(), McpServerError> {
    let mut seen = std::collections::BTreeSet::new();
    for tool in tools {
        if !seen.insert(tool.name.as_str()) {
            return Err(McpServerError::new(format!(
                "runx mcp serve received duplicate tool name '{}'. Serve unique skill names only.",
                tool.name
            )));
        }
    }
    Ok(())
}

fn load_mcp_server_tool(
    skill_path: &Path,
    execution: &McpServerExecutionOptions,
) -> Result<McpServerTool, RuntimeError> {
    let skill = load_skill_for_mcp(skill_path)?;
    Ok(McpServerTool {
        name: skill.name.clone(),
        description: skill
            .description
            .clone()
            .unwrap_or_else(|| format!("runx skill {}", skill.name)),
        input_schema: skill_inputs_to_json_schema(&skill.inputs),
        result: McpServerToolBehavior::Skill(Box::new(McpServerSkillExecution {
            skill_path: skill_path.to_path_buf(),
            skill,
            receipt_dir: execution.receipt_dir.clone(),
            env: execution.env.clone(),
        })),
    })
}

fn load_skill_for_mcp(skill_path: &Path) -> Result<ValidatedSkill, RuntimeError> {
    let manifest_path = if skill_path.is_dir() {
        skill_path.join("SKILL.md")
    } else {
        skill_path.to_path_buf()
    };
    if !manifest_path.exists() {
        return Err(RuntimeError::SkillFileMissing {
            path: manifest_path,
        });
    }
    let source = fs::read_to_string(&manifest_path)
        .map_err(|source| RuntimeError::io("reading skill markdown", source))?;
    let raw = runx_parser::parse_skill_markdown(&source)?;
    runx_parser::validate_skill(raw).map_err(RuntimeError::from)
}

fn skill_inputs_to_json_schema(inputs: &BTreeMap<String, SkillInput>) -> JsonObject {
    let properties = inputs
        .iter()
        .map(|(name, input)| (name.clone(), JsonValue::Object(skill_input_schema(input))))
        .collect::<JsonObject>();
    let required = inputs
        .iter()
        .filter(|(_name, input)| input.required)
        .map(|(name, _input)| JsonValue::String(name.clone()))
        .collect::<Vec<_>>();
    [
        ("type".to_owned(), JsonValue::String("object".to_owned())),
        ("properties".to_owned(), JsonValue::Object(properties)),
        ("required".to_owned(), JsonValue::Array(required)),
        ("additionalProperties".to_owned(), JsonValue::Bool(false)),
    ]
    .into()
}

fn skill_input_schema(input: &SkillInput) -> JsonObject {
    let mut schema = JsonObject::new();
    if let Some(input_type) = normalize_input_type(&input.input_type) {
        schema.insert("type".to_owned(), JsonValue::String(input_type.to_owned()));
    }
    if let Some(description) = &input.description {
        schema.insert(
            "description".to_owned(),
            JsonValue::String(description.clone()),
        );
    }
    if let Some(default) = &input.default {
        schema.insert("default".to_owned(), default.clone());
    }
    schema
}

fn normalize_input_type(input_type: &str) -> Option<&str> {
    match input_type {
        "string" | "number" | "integer" | "boolean" | "object" | "array" => Some(input_type),
        _ => None,
    }
}

fn resume_tool_definition() -> McpServerTool {
    McpServerTool {
        name: "runx_resume".to_owned(),
        description:
            "Resume a paused runx run by run id with zero or more structured resolution payloads."
                .to_owned(),
        input_schema: resume_input_schema(),
        result: McpServerToolBehavior::Resume,
    }
}

fn resume_input_schema() -> JsonObject {
    let properties = [
        (
            "run_id".to_owned(),
            JsonValue::Object(resume_run_id_schema()),
        ),
        (
            "responses".to_owned(),
            JsonValue::Object(resume_responses_schema()),
        ),
    ]
    .into();
    [
        ("type".to_owned(), JsonValue::String("object".to_owned())),
        ("properties".to_owned(), JsonValue::Object(properties)),
        (
            "required".to_owned(),
            JsonValue::Array(vec![JsonValue::String("run_id".to_owned())]),
        ),
        ("additionalProperties".to_owned(), JsonValue::Bool(false)),
    ]
    .into()
}

fn resume_run_id_schema() -> JsonObject {
    [
        ("type".to_owned(), JsonValue::String("string".to_owned())),
        (
            "description".to_owned(),
            JsonValue::String("Paused run id to continue.".to_owned()),
        ),
    ]
    .into()
}

fn resume_responses_schema() -> JsonObject {
    [
        ("type".to_owned(), JsonValue::String("array".to_owned())),
        (
            "description".to_owned(),
            JsonValue::String(
                "Structured response payloads keyed by pending request id.".to_owned(),
            ),
        ),
        (
            "items".to_owned(),
            JsonValue::Object(resume_response_item_schema()),
        ),
    ]
    .into()
}

fn resume_response_item_schema() -> JsonObject {
    [
        ("type".to_owned(), JsonValue::String("object".to_owned())),
        (
            "properties".to_owned(),
            JsonValue::Object(resume_response_properties()),
        ),
        (
            "required".to_owned(),
            JsonValue::Array(vec![
                JsonValue::String("request_id".to_owned()),
                JsonValue::String("payload".to_owned()),
            ]),
        ),
        ("additionalProperties".to_owned(), JsonValue::Bool(false)),
    ]
    .into()
}

fn resume_response_properties() -> JsonObject {
    [
        (
            "request_id".to_owned(),
            JsonValue::Object([("type".to_owned(), JsonValue::String("string".to_owned()))].into()),
        ),
        (
            "actor".to_owned(),
            JsonValue::Object(
                [
                    ("type".to_owned(), JsonValue::String("string".to_owned())),
                    (
                        "enum".to_owned(),
                        JsonValue::Array(vec![
                            JsonValue::String("human".to_owned()),
                            JsonValue::String("agent".to_owned()),
                        ]),
                    ),
                ]
                .into(),
            ),
        ),
        ("payload".to_owned(), JsonValue::Object(JsonObject::new())),
    ]
    .into()
}

fn execute_mcp_server_skill(
    state: &mut McpServerState,
    execution: McpServerSkillExecution,
    inputs: JsonObject,
) -> Result<McpToolResult, RuntimeError> {
    let inputs = apply_input_defaults(&execution.skill, inputs);
    if let Some(request) = input_resolution_request(&execution.skill, &inputs) {
        let skill_name = execution.skill.name.clone();
        let run_id = state.next_run_id(&execution.skill.name);
        state.pending.insert(
            run_id.clone(),
            PendingMcpRun {
                execution: execution.clone(),
                inputs,
                request: request.clone(),
                checkpoint: None,
            },
        );
        return Ok(mcp_tool_result_from_host_result(McpHostRunResult::Paused {
            skill_name: skill_name.clone(),
            run_id: run_id.clone(),
            request_count: 1,
            runx: paused_runx(&skill_name, &run_id, &[request])?,
        }));
    }

    let run_id = state.next_run_id(&execution.skill.name);
    if execution.skill.source.source_type == "graph" {
        return execute_mcp_server_graph(state, &run_id, execution, inputs);
    }
    complete_mcp_server_skill(&run_id, execution, inputs)
}

fn resume_mcp_server_run(
    state: &mut McpServerState,
    arguments: JsonObject,
) -> Result<McpToolResult, RuntimeError> {
    let run_id = required_string_argument(&arguments, "run_id", "runx_resume.run_id")?;
    let Some(pending) = state.pending.remove(&run_id) else {
        return Err(RuntimeError::ReceiptInvalid {
            message: format!(
                "Run '{run_id}' cannot be resumed because no pending skill path was recorded."
            ),
        });
    };
    let mut inputs = pending.inputs.clone();
    apply_resume_responses(&mut inputs, &pending.request, arguments.get("responses"))?;
    if pending.checkpoint.is_some() {
        return resume_mcp_server_graph(state, &run_id, pending, arguments.get("responses"));
    }
    if let Some(request) = input_resolution_request(&pending.execution.skill, &inputs) {
        let skill_name = pending.execution.skill.name.clone();
        state.pending.insert(
            run_id.clone(),
            PendingMcpRun {
                execution: pending.execution,
                inputs,
                request: request.clone(),
                checkpoint: None,
            },
        );
        return Ok(mcp_tool_result_from_host_result(McpHostRunResult::Paused {
            skill_name: skill_name.clone(),
            run_id: run_id.clone(),
            request_count: 1,
            runx: paused_runx(&skill_name, &run_id, &[request])?,
        }));
    }
    complete_mcp_server_skill(&run_id, pending.execution, inputs)
}

fn execute_mcp_server_graph(
    state: &mut McpServerState,
    run_id: &str,
    execution: McpServerSkillExecution,
    inputs: JsonObject,
) -> Result<McpToolResult, RuntimeError> {
    let graph =
        execution
            .skill
            .source
            .graph
            .clone()
            .ok_or_else(|| RuntimeError::UnsupportedAdapter {
                adapter_type: "graph".to_owned(),
            })?;
    let graph_dir = skill_directory_for_execution(&execution.skill_path);
    let runtime = Runtime::new(
        McpServerGraphAdapter,
        RuntimeOptions {
            created_at: "2026-05-20T00:00:00Z".to_owned(),
            env: execution.env.clone(),
        },
    );
    let mut caller = McpServerCaller::default();
    let checkpoint =
        runtime.run_graph_until_steps_with_caller(&graph_dir, &graph, 1, &mut caller)?;
    if let Some(request) = caller.requests.first().cloned() {
        state.pending.insert(
            run_id.to_owned(),
            PendingMcpRun {
                execution: execution.clone(),
                inputs,
                request: request.clone(),
                checkpoint: Some(checkpoint),
            },
        );
        return Ok(mcp_tool_result_from_host_result(McpHostRunResult::Paused {
            skill_name: execution.skill.name.clone(),
            run_id: run_id.to_owned(),
            request_count: 1,
            runx: paused_runx(&execution.skill.name, run_id, &[request])?,
        }));
    }
    let run = runtime.resume_graph_with_caller(&graph_dir, graph, checkpoint, &mut caller)?;
    graph_run_mcp_result(&execution.skill.name, run_id, run)
}

fn resume_mcp_server_graph(
    _state: &mut McpServerState,
    run_id: &str,
    mut pending: PendingMcpRun,
    responses: Option<&JsonValue>,
) -> Result<McpToolResult, RuntimeError> {
    let Some(mut checkpoint) = pending.checkpoint.take() else {
        return Err(RuntimeError::ReceiptInvalid {
            message: format!("Run '{run_id}' is not a graph checkpoint."),
        });
    };
    apply_graph_approval_response(&mut checkpoint, &pending.request, responses)?;
    let graph = pending
        .execution
        .skill
        .source
        .graph
        .clone()
        .ok_or_else(|| RuntimeError::UnsupportedAdapter {
            adapter_type: "graph".to_owned(),
        })?;
    let graph_dir = skill_directory_for_execution(&pending.execution.skill_path);
    let runtime = Runtime::new(
        McpServerGraphAdapter,
        RuntimeOptions {
            created_at: "2026-05-20T00:00:00Z".to_owned(),
            env: pending.execution.env.clone(),
        },
    );
    let mut caller = McpServerCaller::default();
    let run = runtime.resume_graph_with_caller(&graph_dir, graph, checkpoint, &mut caller)?;
    graph_run_mcp_result(&pending.execution.skill.name, run_id, run)
}

fn graph_run_mcp_result(
    skill_name: &str,
    run_id: &str,
    run: GraphRun,
) -> Result<McpToolResult, RuntimeError> {
    let status = if run.state.status == GraphStatus::Succeeded {
        "completed"
    } else {
        "failed"
    };
    let result = if status == "completed" {
        McpHostRunResult::Completed {
            skill_name: skill_name.to_owned(),
            output: String::new(),
            receipt_id: run.receipt.id.clone(),
            runx: terminal_runx("completed", skill_name, run_id, &run.receipt.id),
        }
    } else {
        McpHostRunResult::Failed {
            skill_name: skill_name.to_owned(),
            receipt_id: Some(run.receipt.id.clone()),
            error: format!("graph ended with status {:?}", run.state.status),
            runx: terminal_runx("failed", skill_name, run_id, &run.receipt.id),
        }
    };
    Ok(mcp_tool_result_from_host_result(result))
}

fn complete_mcp_server_skill(
    run_id: &str,
    execution: McpServerSkillExecution,
    inputs: JsonObject,
) -> Result<McpToolResult, RuntimeError> {
    let output = invoke_mcp_server_skill(&execution, inputs)?;
    let receipt = step_receipt(
        run_id,
        &execution.skill.name,
        1,
        &output,
        "2026-05-20T00:00:00Z",
    )?;
    if let Some(receipt_dir) = &execution.receipt_dir {
        LocalReceiptStore::new(receipt_dir)
            .write_receipt(&receipt)
            .map_err(|source| RuntimeError::ReceiptInvalid {
                message: source.to_string(),
            })?;
    }
    let result = if output.succeeded() {
        McpHostRunResult::Completed {
            skill_name: execution.skill.name.clone(),
            output: output.stdout.clone(),
            receipt_id: receipt.id.clone(),
            runx: completed_runx(&execution.skill.name, run_id, &receipt.id, &output),
        }
    } else {
        McpHostRunResult::Failed {
            skill_name: execution.skill.name.clone(),
            receipt_id: Some(receipt.id.clone()),
            error: if output.stderr.is_empty() {
                "skill execution failed".to_owned()
            } else {
                output.stderr.clone()
            },
            runx: terminal_runx("failed", &execution.skill.name, run_id, &receipt.id),
        }
    };
    Ok(mcp_tool_result_from_host_result(result))
}

fn invoke_mcp_server_skill(
    execution: &McpServerSkillExecution,
    inputs: JsonObject,
) -> Result<SkillOutput, RuntimeError> {
    let invocation = SkillInvocation {
        skill_name: execution.skill.name.clone(),
        source: execution.skill.source.clone(),
        inputs,
        resolved_inputs: JsonObject::new(),
        skill_directory: skill_directory_for_execution(&execution.skill_path),
        env: execution.env.clone(),
    };
    match execution.skill.source.source_type.as_str() {
        "mcp" => McpAdapter::default().invoke(invocation),
        "cli-tool" => invoke_cli_tool_server_skill(invocation),
        "graph" => Err(RuntimeError::UnsupportedAdapter {
            adapter_type: "graph".to_owned(),
        }),
        other => Err(RuntimeError::UnsupportedAdapter {
            adapter_type: other.to_owned(),
        }),
    }
}

#[cfg(feature = "cli-tool")]
fn invoke_cli_tool_server_skill(invocation: SkillInvocation) -> Result<SkillOutput, RuntimeError> {
    crate::adapters::cli_tool::CliToolAdapter.invoke(invocation)
}

#[cfg(not(feature = "cli-tool"))]
fn invoke_cli_tool_server_skill(invocation: SkillInvocation) -> Result<SkillOutput, RuntimeError> {
    Err(RuntimeError::UnsupportedAdapter {
        adapter_type: invocation.source.source_type,
    })
}

#[derive(Clone, Copy, Debug, Default)]
struct McpServerGraphAdapter;

impl SkillAdapter for McpServerGraphAdapter {
    fn adapter_type(&self) -> &'static str {
        "mcp-server-graph"
    }

    fn invoke(&self, request: SkillInvocation) -> Result<SkillOutput, RuntimeError> {
        match request.source.source_type.as_str() {
            "mcp" => McpAdapter::default().invoke(request),
            "cli-tool" => invoke_cli_tool_server_skill(request),
            other => Err(RuntimeError::UnsupportedAdapter {
                adapter_type: other.to_owned(),
            }),
        }
    }
}

#[derive(Default)]
struct McpServerCaller {
    requests: Vec<ResolutionRequest>,
}

impl Caller for McpServerCaller {
    fn report(&mut self, _event: ExecutionEvent) -> Result<(), RuntimeError> {
        Ok(())
    }

    fn resolve(
        &mut self,
        request: ResolutionRequest,
    ) -> Result<Option<ResolutionResponse>, RuntimeError> {
        self.requests.push(request);
        Ok(None)
    }
}

fn apply_input_defaults(skill: &ValidatedSkill, mut inputs: JsonObject) -> JsonObject {
    for (name, input) in &skill.inputs {
        if !inputs.contains_key(name)
            && let Some(default) = &input.default
        {
            inputs.insert(name.clone(), default.clone());
        }
    }
    inputs
}

fn input_resolution_request(
    skill: &ValidatedSkill,
    inputs: &JsonObject,
) -> Option<ResolutionRequest> {
    let questions = skill
        .inputs
        .iter()
        .filter(|(name, input)| input.required && missing_input(inputs.get(*name)))
        .map(|(name, input)| Question {
            id: name.clone(),
            prompt: input
                .description
                .clone()
                .unwrap_or_else(|| format!("Provide {name}.")),
            description: input.description.clone(),
            required: true,
            question_type: input.input_type.clone(),
        })
        .collect::<Vec<_>>();
    (!questions.is_empty()).then(|| ResolutionRequest::Input {
        id: format!(
            "input.{}.{}",
            identifier_segment(&skill.name),
            questions
                .iter()
                .map(|question| identifier_segment(&question.id))
                .collect::<Vec<_>>()
                .join(".")
        ),
        questions,
    })
}

fn missing_input(value: Option<&JsonValue>) -> bool {
    match value {
        None | Some(JsonValue::Null) => true,
        Some(JsonValue::String(value)) => value.is_empty(),
        Some(_) => false,
    }
}

fn apply_resume_responses(
    inputs: &mut JsonObject,
    request: &ResolutionRequest,
    responses: Option<&JsonValue>,
) -> Result<(), RuntimeError> {
    let Some(responses) = responses else {
        return Ok(());
    };
    let JsonValue::Array(responses) = responses else {
        return Err(RuntimeError::ReceiptInvalid {
            message: "runx_resume.responses must be an array when provided.".to_owned(),
        });
    };
    for (index, response) in responses.iter().enumerate() {
        let JsonValue::Object(response) = response else {
            return Err(RuntimeError::ReceiptInvalid {
                message: format!("runx_resume.responses[{index}] must be an object."),
            });
        };
        let request_id = required_string_argument(
            response,
            "request_id",
            &format!("runx_resume.responses[{index}].request_id"),
        )?;
        if response
            .get("actor")
            .is_some_and(|actor| !matches!(actor, JsonValue::String(value) if value == "human" || value == "agent"))
        {
            return Err(RuntimeError::ReceiptInvalid {
                message: format!(
                    "runx_resume.responses[{index}].actor must be human or agent when provided."
                ),
            });
        }
        let Some(payload) = response.get("payload") else {
            return Err(RuntimeError::ReceiptInvalid {
                message: format!("runx_resume.responses[{index}].payload is required."),
            });
        };
        if request_id == resolution_request_id(request) {
            merge_resolution_payload(inputs, request, payload.clone());
        }
    }
    Ok(())
}

fn apply_graph_approval_response(
    checkpoint: &mut GraphCheckpoint,
    request: &ResolutionRequest,
    responses: Option<&JsonValue>,
) -> Result<(), RuntimeError> {
    let ResolutionRequest::Approval { gate, .. } = request else {
        return Err(RuntimeError::ReceiptInvalid {
            message: "graph resume expected an approval request.".to_owned(),
        });
    };
    let (approved, actor) = approval_resume_response(request, responses)?;
    resolve_graph_approval_step(checkpoint, gate, approved, actor)
}

fn resolve_graph_approval_step(
    checkpoint: &mut GraphCheckpoint,
    gate: &ApprovalGate,
    approved: bool,
    actor: String,
) -> Result<(), RuntimeError> {
    let graph_name = checkpoint.graph_name.clone();
    let (step_id, outputs, receipt_id) = {
        let step = pending_approval_step_mut(checkpoint, gate)?;
        apply_resumed_approval_to_step(&graph_name, step, approved, actor)?
    };
    sync_checkpoint_state_step(checkpoint, &step_id, outputs, receipt_id);
    Ok(())
}

fn apply_resumed_approval_to_step(
    graph_name: &str,
    step: &mut StepRun,
    approved: bool,
    actor: String,
) -> Result<(String, JsonObject, String), RuntimeError> {
    let approval = pending_approval_packet_mut(step)?;
    approval.insert("approved".to_owned(), JsonValue::Bool(approved));
    approval.insert(
        "status".to_owned(),
        JsonValue::String(if approved { "approved" } else { "denied" }.to_owned()),
    );
    approval.insert("actor".to_owned(), JsonValue::String(actor));
    complete_resumed_approval_step(graph_name, step)?;
    Ok((
        step.step_id.clone(),
        step.outputs.clone(),
        step.receipt.id.clone(),
    ))
}

fn complete_resumed_approval_step(
    graph_name: &str,
    step: &mut StepRun,
) -> Result<(), RuntimeError> {
    let stdout = serde_json::to_string(&step.outputs)
        .map_err(|source| RuntimeError::json("serializing resumed approval output", source))?;
    step.output = SkillOutput {
        status: InvocationStatus::Success,
        stdout,
        stderr: String::new(),
        exit_code: Some(0),
        duration_ms: 0,
        metadata: JsonObject::new(),
    };
    step.receipt = step_receipt(
        graph_name,
        &step.step_id,
        step.attempt,
        &step.output,
        "2026-05-20T00:00:00Z",
    )?;
    Ok(())
}

fn pending_approval_step_mut<'a>(
    checkpoint: &'a mut GraphCheckpoint,
    gate: &ApprovalGate,
) -> Result<&'a mut StepRun, RuntimeError> {
    checkpoint
        .steps
        .iter_mut()
        .find(|step| step_contains_approval_gate(step, gate))
        .ok_or_else(|| RuntimeError::ReceiptInvalid {
            message: format!(
                "pending graph checkpoint does not contain approval gate {}",
                gate.id
            ),
        })
}

fn pending_approval_packet_mut(step: &mut StepRun) -> Result<&mut JsonObject, RuntimeError> {
    step.outputs
        .values_mut()
        .find_map(approval_packet_mut)
        .ok_or_else(|| RuntimeError::ReceiptInvalid {
            message: "pending graph checkpoint does not contain approval output.".to_owned(),
        })
}

fn sync_checkpoint_state_step(
    checkpoint: &mut GraphCheckpoint,
    step_id: &str,
    outputs: JsonObject,
    receipt_id: String,
) {
    if let Some(state_step) = checkpoint
        .state
        .steps
        .iter_mut()
        .find(|candidate| candidate.step_id == step_id)
    {
        state_step.outputs = Some(outputs);
        state_step.receipt_id = Some(receipt_id);
    }
}

fn approval_resume_response(
    request: &ResolutionRequest,
    responses: Option<&JsonValue>,
) -> Result<(bool, String), RuntimeError> {
    let Some(responses) = responses else {
        return Err(RuntimeError::ReceiptInvalid {
            message: "runx_resume.responses is required for approval resume.".to_owned(),
        });
    };
    let JsonValue::Array(responses) = responses else {
        return Err(RuntimeError::ReceiptInvalid {
            message: "runx_resume.responses must be an array when provided.".to_owned(),
        });
    };
    for (index, response) in responses.iter().enumerate() {
        let JsonValue::Object(response) = response else {
            return Err(RuntimeError::ReceiptInvalid {
                message: format!("runx_resume.responses[{index}] must be an object."),
            });
        };
        let request_id = required_string_argument(
            response,
            "request_id",
            &format!("runx_resume.responses[{index}].request_id"),
        )?;
        if request_id != resolution_request_id(request) {
            continue;
        }
        let actor = match response.get("actor") {
            Some(JsonValue::String(actor)) if actor == "human" || actor == "agent" => actor.clone(),
            Some(_) => {
                return Err(RuntimeError::ReceiptInvalid {
                    message: format!(
                        "runx_resume.responses[{index}].actor must be human or agent when provided."
                    ),
                });
            }
            None => "human".to_owned(),
        };
        let Some(payload) = response.get("payload") else {
            return Err(RuntimeError::ReceiptInvalid {
                message: format!("runx_resume.responses[{index}].payload is required."),
            });
        };
        let JsonValue::Bool(approved) = payload else {
            return Err(RuntimeError::ReceiptInvalid {
                message: format!(
                    "runx_resume.responses[{index}].payload must be boolean for approval requests."
                ),
            });
        };
        return Ok((*approved, actor));
    }
    Err(RuntimeError::ReceiptInvalid {
        message: format!(
            "runx_resume.responses did not include pending request {}.",
            resolution_request_id(request)
        ),
    })
}

fn step_contains_approval_gate(step: &StepRun, gate: &ApprovalGate) -> bool {
    step.outputs.values().any(|value| {
        approval_packet(value).is_some_and(|packet| {
            packet.get("gate_id") == Some(&JsonValue::String(gate.id.clone()))
        })
    })
}

fn approval_packet(value: &JsonValue) -> Option<&JsonObject> {
    let JsonValue::Object(packet) = value else {
        return None;
    };
    let Some(JsonValue::Object(data)) = packet.get("data") else {
        return None;
    };
    Some(data)
}

fn approval_packet_mut(value: &mut JsonValue) -> Option<&mut JsonObject> {
    let JsonValue::Object(packet) = value else {
        return None;
    };
    let Some(JsonValue::Object(data)) = packet.get_mut("data") else {
        return None;
    };
    Some(data)
}

fn merge_resolution_payload(
    inputs: &mut JsonObject,
    request: &ResolutionRequest,
    payload: JsonValue,
) {
    match (request, payload) {
        (ResolutionRequest::Input { questions, .. }, JsonValue::Object(payload)) => {
            for question in questions {
                if let Some(value) = payload.get(&question.id) {
                    inputs.insert(question.id.clone(), value.clone());
                }
            }
        }
        (ResolutionRequest::Input { questions, .. }, payload) if questions.len() == 1 => {
            inputs.insert(questions[0].id.clone(), payload);
        }
        _ => {}
    }
}

fn required_string_argument(
    record: &JsonObject,
    field: &str,
    label: &str,
) -> Result<String, RuntimeError> {
    match record.get(field) {
        Some(JsonValue::String(value)) if !value.trim().is_empty() => Ok(value.clone()),
        _ => Err(RuntimeError::ReceiptInvalid {
            message: format!("{label} is required."),
        }),
    }
}

fn resolution_request_id(request: &ResolutionRequest) -> &str {
    match request {
        ResolutionRequest::Input { id, .. }
        | ResolutionRequest::Approval { id, .. }
        | ResolutionRequest::AgentAct { id, .. } => id,
    }
}

fn completed_runx(
    skill_name: &str,
    run_id: &str,
    receipt_id: &str,
    output: &SkillOutput,
) -> JsonObject {
    let mut runx = terminal_runx("completed", skill_name, run_id, receipt_id);
    runx.insert(
        "output".to_owned(),
        JsonValue::String(output.stdout.clone()),
    );
    runx
}

fn terminal_runx(status: &str, skill_name: &str, run_id: &str, receipt_id: &str) -> JsonObject {
    [
        ("status".to_owned(), JsonValue::String(status.to_owned())),
        (
            "skillName".to_owned(),
            JsonValue::String(skill_name.to_owned()),
        ),
        ("runId".to_owned(), JsonValue::String(run_id.to_owned())),
        (
            "receiptId".to_owned(),
            JsonValue::String(receipt_id.to_owned()),
        ),
        ("events".to_owned(), JsonValue::Array(Vec::new())),
    ]
    .into()
}

fn paused_runx(
    skill_name: &str,
    run_id: &str,
    requests: &[ResolutionRequest],
) -> Result<JsonObject, RuntimeError> {
    Ok([
        ("status".to_owned(), JsonValue::String("paused".to_owned())),
        (
            "skillName".to_owned(),
            JsonValue::String(skill_name.to_owned()),
        ),
        ("runId".to_owned(), JsonValue::String(run_id.to_owned())),
        (
            "requests".to_owned(),
            JsonValue::Array(
                requests
                    .iter()
                    .map(serde_json_value)
                    .collect::<Result<Vec<_>, _>>()?,
            ),
        ),
        ("events".to_owned(), JsonValue::Array(Vec::new())),
    ]
    .into())
}

fn serde_json_value<T: serde::Serialize>(value: &T) -> Result<JsonValue, RuntimeError> {
    let serialized = serde_json::to_string(value)
        .map_err(|source| RuntimeError::json("serializing MCP host result", source))?;
    serde_json::from_str(&serialized)
        .map_err(|source| RuntimeError::json("deserializing MCP host result", source))
}

fn skill_directory_for_execution(skill_path: &Path) -> PathBuf {
    if skill_path.is_dir() {
        skill_path.to_path_buf()
    } else {
        skill_path
            .parent()
            .map_or_else(|| PathBuf::from("."), Path::to_path_buf)
    }
}

fn identifier_segment(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_owned()
}

fn runx_content(runx: JsonObject) -> JsonObject {
    [("runx".to_owned(), JsonValue::Object(runx))].into()
}

pub fn map_mcp_arguments(
    argument_template: Option<&JsonObject>,
    inputs: &JsonObject,
    resolved_inputs: &JsonObject,
) -> Result<JsonObject, RuntimeError> {
    let Some(template) = argument_template else {
        let mut merged = inputs.clone();
        merged.extend(resolved_inputs.clone());
        return Ok(merged);
    };
    template
        .iter()
        .map(|(key, value)| {
            let mapped = match value {
                JsonValue::String(template) => {
                    map_template_string(template, inputs, resolved_inputs)?
                }
                other => other.clone(),
            };
            Ok((key.clone(), mapped))
        })
        .collect()
}

pub fn stringify_mcp_tool_result(result: &JsonValue) -> Result<String, RuntimeError> {
    if let JsonValue::Object(record) = result
        && let Some(JsonValue::Array(content)) = record.get("content")
    {
        return content
            .iter()
            .map(stringify_content_entry)
            .collect::<Result<Vec<_>, _>>()
            .map(|entries| entries.join("\n"));
    }

    match result {
        JsonValue::String(value) => Ok(value.clone()),
        value => serde_json::to_string(value)
            .map_err(|source| RuntimeError::json("serializing MCP tool result", source)),
    }
}

fn map_template_string(
    template: &str,
    inputs: &JsonObject,
    resolved_inputs: &JsonObject,
) -> Result<JsonValue, RuntimeError> {
    if let Some(key) = exact_template_key(template) {
        return Ok(resolved_inputs
            .get(key)
            .or_else(|| inputs.get(key))
            .cloned()
            .unwrap_or(JsonValue::Null));
    }

    let mut rendered = String::new();
    let mut rest = template;
    while let Some(start) = rest.find(TEMPLATE_OPEN) {
        let (prefix, after_start) = rest.split_at(start);
        rendered.push_str(prefix);
        let after_start = &after_start[2..];
        let Some(end) = after_start.find(TEMPLATE_CLOSE) else {
            rendered.push_str(TEMPLATE_OPEN);
            rendered.push_str(after_start);
            return Ok(JsonValue::String(rendered));
        };
        let raw_key = &after_start[..end];
        let key = raw_key.trim();
        if valid_template_key(key) {
            rendered.push_str(&stringify_mcp_input(
                resolved_inputs.get(key).or_else(|| inputs.get(key)),
            )?);
        } else {
            rendered.push_str(TEMPLATE_OPEN);
            rendered.push_str(raw_key);
            rendered.push_str(TEMPLATE_CLOSE);
        }
        rest = &after_start[end + 2..];
    }
    rendered.push_str(rest);
    Ok(JsonValue::String(rendered))
}

fn exact_template_key(template: &str) -> Option<&str> {
    let trimmed = template.trim();
    let inner = trimmed
        .strip_prefix(TEMPLATE_OPEN)?
        .strip_suffix(TEMPLATE_CLOSE)?
        .trim();
    if valid_template_key(inner) {
        Some(inner)
    } else {
        None
    }
}

fn valid_template_key(key: &str) -> bool {
    !key.is_empty()
        && key
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.' | '-'))
}

fn stringify_mcp_input(value: Option<&JsonValue>) -> Result<String, RuntimeError> {
    match value {
        None | Some(JsonValue::Null) => Ok(String::new()),
        Some(JsonValue::String(value)) => Ok(value.clone()),
        Some(value) => serde_json::to_string(value)
            .map_err(|source| RuntimeError::json("serializing MCP template input", source)),
    }
}

fn stringify_content_entry(entry: &JsonValue) -> Result<String, RuntimeError> {
    if let JsonValue::Object(record) = entry
        && record.get("type") == Some(&JsonValue::String("text".to_owned()))
        && let Some(JsonValue::String(text)) = record.get("text")
    {
        return Ok(text.clone());
    }
    serde_json::to_string(entry)
        .map_err(|source| RuntimeError::json("serializing MCP content entry", source))
}

fn spawn_mcp_server(plan: &SandboxPlan) -> Result<Child, McpTransportError> {
    Command::new(&plan.command)
        .args(&plan.args)
        .current_dir(&plan.cwd)
        .env_clear()
        .envs(&plan.env)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| McpTransportError::failed("MCP server failed to spawn."))
}

struct InitializedMcpClient {
    child: Child,
    stdin: ChildStdin,
    rx: Receiver<Result<JsonValue, McpTransportError>>,
    deadline: Instant,
    timeout: Duration,
}

impl InitializedMcpClient {
    fn request(&mut self, id: i64, message: &JsonValue) -> Result<JsonValue, McpTransportError> {
        write_message(&mut self.stdin, message)?;
        wait_for_response(&mut self.child, &self.rx, id, self.deadline, self.timeout)
    }

    fn notify(&mut self, message: &JsonValue) -> Result<(), McpTransportError> {
        write_message(&mut self.stdin, message)
    }
}

impl Drop for InitializedMcpClient {
    fn drop(&mut self) {
        terminate_child(&mut self.child);
    }
}

fn initialize_mcp_client(
    sandbox: &SandboxPlan,
    timeout: Duration,
) -> Result<InitializedMcpClient, McpTransportError> {
    let mut child = spawn_mcp_server(sandbox)?;
    let Some(stdin) = child.stdin.take() else {
        terminate_child(&mut child);
        return Err(McpTransportError::failed("MCP server stdin unavailable."));
    };
    let Some(stdout) = child.stdout.take() else {
        terminate_child(&mut child);
        return Err(McpTransportError::failed("MCP server stdout unavailable."));
    };
    drain_stderr(child.stderr.take());
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || read_stdout_frames(stdout, tx));
    let mut client = InitializedMcpClient {
        child,
        stdin,
        rx,
        deadline: Instant::now() + timeout,
        timeout,
    };
    client.request(1, &initialize_request(1))?;
    client.notify(&initialized_notification())?;
    Ok(client)
}

fn write_message(stdin: &mut impl Write, message: &JsonValue) -> Result<(), McpTransportError> {
    let body = serde_json::to_vec(message)
        .map_err(|_| McpTransportError::failed("MCP request serialization failed."))?;
    let header = format!("Content-Length: {}\r\n\r\n", body.len());
    stdin
        .write_all(header.as_bytes())
        .and_then(|()| stdin.write_all(&body))
        .map_err(|_| McpTransportError::failed("MCP server stdin write failed."))
}

fn wait_for_response(
    child: &mut Child,
    rx: &Receiver<Result<JsonValue, McpTransportError>>,
    id: i64,
    deadline: Instant,
    timeout: Duration,
) -> Result<JsonValue, McpTransportError> {
    loop {
        let now = Instant::now();
        if now >= deadline {
            terminate_child(child);
            return Err(McpTransportError::timeout(timeout));
        }
        let remaining = deadline.saturating_duration_since(now);
        match rx.recv_timeout(POLL_INTERVAL.min(remaining)) {
            Ok(Ok(message)) => {
                if response_id(&message) != Some(id) {
                    continue;
                }
                return response_result(message);
            }
            Ok(Err(error)) => return Err(error),
            Err(RecvTimeoutError::Timeout) => {
                if process_exited(child)? {
                    return Err(McpTransportError::failed(
                        "MCP server exited before responding.",
                    ));
                }
            }
            Err(RecvTimeoutError::Disconnected) => {
                return Err(McpTransportError::failed(
                    "MCP server exited before responding.",
                ));
            }
        }
    }
}

fn process_exited(child: &mut Child) -> Result<bool, McpTransportError> {
    child
        .try_wait()
        .map(|status| status.is_some())
        .map_err(|_| McpTransportError::failed("MCP server status check failed."))
}

fn read_stdout_frames(mut stdout: impl Read, tx: Sender<Result<JsonValue, McpTransportError>>) {
    let mut buffer = Vec::new();
    let mut chunk = [0_u8; 8192];
    loop {
        match stdout.read(&mut chunk) {
            Ok(0) => return,
            Ok(read) => {
                buffer.extend_from_slice(&chunk[..read]);
                match parse_available_messages(&mut buffer) {
                    Ok(messages) => {
                        for message in messages {
                            if tx.send(Ok(message)).is_err() {
                                return;
                            }
                        }
                    }
                    Err(error) => {
                        let _ = tx.send(Err(error));
                        return;
                    }
                }
                if buffered_client_response_exceeds_limit(&buffer) {
                    let _ = tx.send(Err(McpTransportError::failed(
                        "MCP server response exceeded size limit.",
                    )));
                    return;
                }
            }
            Err(_) => {
                let _ = tx.send(Err(McpTransportError::failed(
                    "MCP server stdout read failed.",
                )));
                return;
            }
        }
    }
}

fn drain_stderr(stderr: Option<impl Read + Send + 'static>) {
    if let Some(mut stderr) = stderr {
        thread::spawn(move || {
            let mut sink = [0_u8; 8192];
            let mut read_total = 0_usize;
            while read_total < MAX_CLIENT_RESPONSE_BYTES {
                match stderr.read(&mut sink) {
                    Ok(0) | Err(_) => return,
                    Ok(read) => read_total = read_total.saturating_add(read),
                }
            }
        });
    }
}

fn parse_available_messages(buffer: &mut Vec<u8>) -> Result<Vec<JsonValue>, McpTransportError> {
    let mut messages = Vec::new();
    while let Some(header_end) = find_header_end(buffer) {
        if header_end > MAX_CLIENT_RESPONSE_BYTES {
            return Err(McpTransportError::failed(
                "MCP server response exceeded size limit.",
            ));
        }
        let header = std::str::from_utf8(&buffer[..header_end])
            .map_err(|_| McpTransportError::failed("MCP server sent an invalid header."))?;
        let Some(content_length) = content_length(header) else {
            return Err(McpTransportError::failed(
                "MCP server sent a response without Content-Length.",
            ));
        };
        if content_length > MAX_CLIENT_RESPONSE_BYTES {
            return Err(McpTransportError::failed(
                "MCP server response exceeded size limit.",
            ));
        }
        let body_start = header_end + 4;
        let body_end = body_start.saturating_add(content_length);
        if buffer.len() < body_end {
            break;
        }
        let body = buffer[body_start..body_end].to_vec();
        buffer.drain(..body_end);
        let message = serde_json::from_slice::<JsonValue>(&body)
            .map_err(|_| McpTransportError::failed("MCP server sent invalid JSON."))?;
        messages.push(message);
    }
    Ok(messages)
}

fn buffered_client_response_exceeds_limit(buffer: &[u8]) -> bool {
    if buffer.len() <= MAX_CLIENT_RESPONSE_BYTES {
        return false;
    }
    let Some(header_end) = find_header_end(buffer) else {
        return true;
    };
    if header_end > MAX_CLIENT_RESPONSE_BYTES {
        return true;
    }
    let Ok(header) = std::str::from_utf8(&buffer[..header_end]) else {
        return false;
    };
    let Some(content_length) = content_length(header) else {
        return false;
    };
    if content_length > MAX_CLIENT_RESPONSE_BYTES {
        return false;
    }
    let Some(body_end) = header_end
        .checked_add(4)
        .and_then(|body_start| body_start.checked_add(content_length))
    else {
        return true;
    };
    buffer.len() > body_end
}

fn find_header_end(buffer: &[u8]) -> Option<usize> {
    buffer.windows(4).position(|window| window == b"\r\n\r\n")
}

fn content_length(header: &str) -> Option<usize> {
    header.lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        if !name.trim().eq_ignore_ascii_case("Content-Length") {
            return None;
        }
        value.trim().parse::<usize>().ok()
    })
}

fn response_id(message: &JsonValue) -> Option<i64> {
    let JsonValue::Object(record) = message else {
        return None;
    };
    match record.get("id") {
        Some(JsonValue::Number(JsonNumber::I64(value))) => Some(*value),
        Some(JsonValue::Number(JsonNumber::U64(value))) => i64::try_from(*value).ok(),
        _ => None,
    }
}

fn response_result(message: JsonValue) -> Result<JsonValue, McpTransportError> {
    let JsonValue::Object(mut record) = message else {
        return Err(McpTransportError::failed(
            "MCP server response was invalid.",
        ));
    };
    if let Some(JsonValue::Object(error)) = record.remove("error") {
        let code = error_code(&error);
        return Err(McpTransportError::tool_error(
            code,
            "MCP server returned error.",
        ));
    }
    Ok(record.remove("result").unwrap_or(JsonValue::Null))
}

fn error_code(error: &JsonObject) -> i64 {
    match error.get("code") {
        Some(JsonValue::Number(JsonNumber::I64(value))) => *value,
        Some(JsonValue::Number(JsonNumber::U64(value))) => i64::try_from(*value).unwrap_or(0),
        _ => 0,
    }
}

fn initialize_request(id: i64) -> JsonValue {
    json_rpc_request(
        id,
        "initialize",
        [
            (
                "protocolVersion".to_owned(),
                JsonValue::String(PROTOCOL_VERSION.to_owned()),
            ),
            (
                "capabilities".to_owned(),
                JsonValue::Object(JsonObject::new()),
            ),
            (
                "clientInfo".to_owned(),
                JsonValue::Object(
                    [
                        ("name".to_owned(), JsonValue::String("runx".to_owned())),
                        ("version".to_owned(), JsonValue::String("0.0.0".to_owned())),
                    ]
                    .into(),
                ),
            ),
        ]
        .into(),
    )
}

fn initialized_notification() -> JsonValue {
    JsonValue::Object(
        [
            ("jsonrpc".to_owned(), JsonValue::String("2.0".to_owned())),
            (
                "method".to_owned(),
                JsonValue::String("notifications/initialized".to_owned()),
            ),
            ("params".to_owned(), JsonValue::Object(JsonObject::new())),
        ]
        .into(),
    )
}

fn tool_call_request(id: i64, tool: &str, arguments: &JsonObject) -> JsonValue {
    json_rpc_request(
        id,
        "tools/call",
        [
            ("name".to_owned(), JsonValue::String(tool.to_owned())),
            ("arguments".to_owned(), JsonValue::Object(arguments.clone())),
        ]
        .into(),
    )
}

fn tools_list_request(id: i64) -> JsonValue {
    json_rpc_request(id, "tools/list", JsonObject::new())
}

fn parse_mcp_tools_list(result: JsonValue) -> Vec<McpToolDescriptor> {
    let JsonValue::Object(record) = result else {
        return Vec::new();
    };
    let Some(JsonValue::Array(tools)) = record.get("tools") else {
        return Vec::new();
    };

    tools
        .iter()
        .filter_map(|entry| {
            let JsonValue::Object(tool) = entry else {
                return None;
            };
            let Some(JsonValue::String(name)) = tool.get("name") else {
                return None;
            };
            if name.trim().is_empty() {
                return None;
            }
            Some(McpToolDescriptor {
                name: name.clone(),
                description: match tool.get("description") {
                    Some(JsonValue::String(description)) => Some(description.clone()),
                    _ => None,
                },
                input_schema: input_schema(tool),
            })
        })
        .collect()
}

fn input_schema(tool: &JsonObject) -> Option<JsonObject> {
    match tool.get("inputSchema").or_else(|| tool.get("input_schema")) {
        Some(JsonValue::Object(schema)) => Some(schema.clone()),
        _ => None,
    }
}

fn json_rpc_request(id: i64, method: &str, params: JsonObject) -> JsonValue {
    JsonValue::Object(
        [
            ("jsonrpc".to_owned(), JsonValue::String("2.0".to_owned())),
            ("id".to_owned(), JsonValue::Number(JsonNumber::I64(id))),
            ("method".to_owned(), JsonValue::String(method.to_owned())),
            ("params".to_owned(), JsonValue::Object(params)),
        ]
        .into(),
    )
}

fn terminate_child(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

fn text_content(text: String) -> JsonValue {
    JsonValue::Object(
        [(
            "content".to_owned(),
            JsonValue::Array(vec![JsonValue::Object(
                [
                    ("type".to_owned(), JsonValue::String("text".to_owned())),
                    ("text".to_owned(), JsonValue::String(text)),
                ]
                .into(),
            )]),
        )]
        .into(),
    )
}

fn env_value(env: &BTreeMap<String, String>, name: Option<&JsonValue>) -> String {
    env.get(&js_string(name)).cloned().unwrap_or_default()
}

fn js_string(value: Option<&JsonValue>) -> String {
    match value {
        None | Some(JsonValue::Null) => String::new(),
        Some(JsonValue::String(value)) => value.clone(),
        Some(JsonValue::Bool(value)) => value.to_string(),
        Some(JsonValue::Number(value)) => json_number_string(value),
        Some(JsonValue::Array(values)) => values
            .iter()
            .map(|value| js_string(Some(value)))
            .collect::<Vec<_>>()
            .join(","),
        Some(JsonValue::Object(_)) => "[object Object]".to_owned(),
    }
}

fn json_number_string(value: &JsonNumber) -> String {
    match value {
        JsonNumber::I64(value) => value.to_string(),
        JsonNumber::U64(value) => value.to_string(),
        JsonNumber::F64(value) if value.fract() == 0.0 => format!("{value:.0}"),
        JsonNumber::F64(value) => value.to_string(),
    }
}

fn timeout_from_source(timeout_seconds: Option<u64>) -> Duration {
    let timeout_ms = timeout_seconds
        .map(|seconds| seconds.saturating_mul(1000))
        .unwrap_or(DEFAULT_TIMEOUT_MS)
        .max(MIN_TIMEOUT_MS);
    Duration::from_millis(timeout_ms)
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

fn mcp_process_sandbox_metadata(
    sandbox: Option<&SkillSandbox>,
    plan: &SandboxPlan,
    env: &BTreeMap<String, String>,
) -> Result<JsonObject, RuntimeError> {
    let config = mcp_sandbox_metadata_config(sandbox);
    let mut metadata = mcp_sandbox_location_metadata(&config, plan, env)?;
    metadata.insert(
        "env".to_owned(),
        JsonValue::Object(mcp_sandbox_env_metadata(
            config.env_allowlist,
            config.inherited_ambient,
        )),
    );
    metadata.insert(
        "network".to_owned(),
        JsonValue::Object(mcp_sandbox_network_metadata(config.profile, config.network)),
    );
    metadata.insert(
        "writable_paths".to_owned(),
        mcp_sandbox_writable_paths_metadata(config.writable_paths),
    );
    metadata.insert(
        "require_enforcement".to_owned(),
        JsonValue::Bool(config.require_enforcement),
    );
    metadata.insert(
        "filesystem".to_owned(),
        JsonValue::Object(mcp_sandbox_filesystem_metadata(config.profile)),
    );
    metadata.insert(
        "approval".to_owned(),
        JsonValue::Object(mcp_sandbox_approval_metadata(&config)),
    );
    metadata.insert(
        "runtime".to_owned(),
        JsonValue::Object(mcp_sandbox_runtime_metadata(config.profile)),
    );
    Ok(metadata)
}

struct McpSandboxMetadataConfig<'a> {
    profile: &'a str,
    cwd_policy: &'a str,
    env_allowlist: Option<&'a Vec<String>>,
    approved_escalation: bool,
    require_enforcement: bool,
    network: bool,
    writable_paths: &'a [String],
    inherited_ambient: bool,
}

fn mcp_sandbox_metadata_config(sandbox: Option<&SkillSandbox>) -> McpSandboxMetadataConfig<'_> {
    let profile = sandbox
        .map(|sandbox| sandbox.profile.as_str())
        .unwrap_or("readonly");
    let approved_escalation = sandbox
        .and_then(|sandbox| sandbox.approved_escalation)
        .unwrap_or(false);
    let env_allowlist = sandbox.and_then(|sandbox| sandbox.env_allowlist.as_ref());
    McpSandboxMetadataConfig {
        profile,
        cwd_policy: sandbox
            .and_then(|sandbox| sandbox.cwd_policy.as_deref())
            .unwrap_or("skill-directory"),
        env_allowlist,
        approved_escalation,
        require_enforcement: sandbox
            .and_then(|sandbox| sandbox.require_enforcement)
            .unwrap_or(false),
        network: sandbox.and_then(|sandbox| sandbox.network).unwrap_or(false),
        writable_paths: sandbox
            .map(|sandbox| sandbox.writable_paths.as_slice())
            .unwrap_or(&[]),
        inherited_ambient: env_allowlist.is_none()
            && profile == "unrestricted-local-dev"
            && approved_escalation,
    }
}

fn mcp_sandbox_location_metadata(
    config: &McpSandboxMetadataConfig<'_>,
    plan: &SandboxPlan,
    env: &BTreeMap<String, String>,
) -> Result<JsonObject, RuntimeError> {
    let mut metadata = JsonObject::new();
    metadata.insert(
        "profile".to_owned(),
        JsonValue::String(config.profile.to_owned()),
    );
    metadata.insert("cwd".to_owned(), JsonValue::String(path_string(&plan.cwd)));
    metadata.insert(
        "workspace_root".to_owned(),
        JsonValue::String(path_string(&workspace_root(env)?)),
    );
    metadata.insert(
        "cwd_policy".to_owned(),
        JsonValue::String(config.cwd_policy.to_owned()),
    );
    Ok(metadata)
}

fn mcp_sandbox_writable_paths_metadata(writable_paths: &[String]) -> JsonValue {
    JsonValue::Array(
        writable_paths
            .iter()
            .cloned()
            .map(JsonValue::String)
            .collect(),
    )
}

fn mcp_sandbox_approval_metadata(config: &McpSandboxMetadataConfig<'_>) -> JsonObject {
    [
        (
            "required".to_owned(),
            JsonValue::Bool(config.profile == "unrestricted-local-dev"),
        ),
        (
            "approved".to_owned(),
            JsonValue::Bool(config.approved_escalation),
        ),
    ]
    .into()
}

fn mcp_sandbox_env_metadata(
    env_allowlist: Option<&Vec<String>>,
    inherited_ambient: bool,
) -> JsonObject {
    if inherited_ambient {
        return [(
            "mode".to_owned(),
            JsonValue::String("ambient-inherited".to_owned()),
        )]
        .into();
    }

    let allowlist = env_allowlist
        .cloned()
        .unwrap_or_else(|| {
            DEFAULT_SANDBOX_ENV_ALLOWLIST
                .into_iter()
                .map(str::to_owned)
                .collect()
        })
        .into_iter()
        .map(JsonValue::String)
        .collect();
    [
        (
            "mode".to_owned(),
            JsonValue::String(if env_allowlist.is_some() {
                "allowlist".to_owned()
            } else {
                "default-allowlist".to_owned()
            }),
        ),
        ("allowlist".to_owned(), JsonValue::Array(allowlist)),
    ]
    .into()
}

fn mcp_sandbox_network_metadata(profile: &str, network: bool) -> JsonObject {
    [
        ("declared".to_owned(), JsonValue::Bool(network)),
        (
            "enforcement".to_owned(),
            JsonValue::String(if profile == "unrestricted-local-dev" {
                "host-ambient".to_owned()
            } else {
                "not-enforced-local".to_owned()
            }),
        ),
    ]
    .into()
}

fn mcp_sandbox_filesystem_metadata(profile: &str) -> JsonObject {
    [
        (
            "enforcement".to_owned(),
            JsonValue::String(if profile == "unrestricted-local-dev" {
                "host-ambient".to_owned()
            } else {
                "not-enforced-local".to_owned()
            }),
        ),
        (
            "readonly_paths".to_owned(),
            JsonValue::Bool(profile != "unrestricted-local-dev"),
        ),
        ("writable_paths_enforced".to_owned(), JsonValue::Bool(false)),
        ("private_tmp".to_owned(), JsonValue::Bool(false)),
    ]
    .into()
}

fn mcp_sandbox_runtime_metadata(profile: &str) -> JsonObject {
    if profile == "unrestricted-local-dev" {
        return [(
            "enforcer".to_owned(),
            JsonValue::String("direct".to_owned()),
        )]
        .into();
    }
    [
        (
            "enforcer".to_owned(),
            JsonValue::String("declared-policy-only".to_owned()),
        ),
        (
            "reason".to_owned(),
            JsonValue::String(format!(
                "local sandbox profile '{profile}' requires Linux bubblewrap support for filesystem and network enforcement"
            )),
        ),
    ]
    .into()
}

fn workspace_root(env: &BTreeMap<String, String>) -> Result<PathBuf, RuntimeError> {
    if let Some(path) = env.get("RUNX_CWD").or_else(|| env.get("INIT_CWD")) {
        return absolute_path(path);
    }
    std::env::current_dir().map_err(|source| RuntimeError::io("resolving workspace cwd", source))
}

fn absolute_path(path: &str) -> Result<PathBuf, RuntimeError> {
    let path = PathBuf::from(path);
    if path.is_absolute() {
        Ok(path)
    } else {
        Ok(std::env::current_dir()
            .map_err(|source| RuntimeError::io("resolving relative workspace cwd", source))?
            .join(path))
    }
}

fn path_string(path: &Path) -> String {
    path.components()
        .collect::<PathBuf>()
        .to_string_lossy()
        .replace('\\', "/")
}

fn failure(message: impl Into<String>, started: Instant, metadata: JsonObject) -> SkillOutput {
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

fn duration_ms(started: Instant) -> u64 {
    let millis = started.elapsed().as_millis();
    u64::try_from(millis).unwrap_or(u64::MAX)
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    format!("{digest:x}")
}
