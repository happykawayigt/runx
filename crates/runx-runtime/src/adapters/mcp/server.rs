// rust-style-allow: large-file because the JSON-RPC dispatch loop, server
// state, tool-result builders, and host-result projections for `runx mcp
// serve` all sit on the same protocol surface.
use std::io::{Read, Write};

use runx_contracts::{JsonObject, JsonValue};

use super::framing::{content_length, find_header_end};
use super::jsonrpc::{PROTOCOL_VERSION, json_rpc_error, json_rpc_response};
use super::server_skill::{execute_mcp_server_skill, identifier_segment};
use super::types::{
    McpContent, McpHostRunResult, McpServerError, McpServerOptions, McpServerTool,
    McpServerToolBehavior, McpToolResult,
};

const MAX_SERVER_REQUEST_BYTES: usize = 4 * 1024 * 1024;

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
        McpHostRunResult::NeedsAgent {
            skill_name,
            run_id,
            request_count,
            runx,
        } => needs_agent_mcp_tool_result(skill_name, run_id, request_count, runx),
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

fn needs_agent_mcp_tool_result(
    skill_name: String,
    run_id: String,
    request_count: usize,
    runx: JsonObject,
) -> McpToolResult {
    mcp_host_tool_result(
        format!(
            "{skill_name} needs agent input at {run_id}. Continue by rerunning the same skill with --run-id {run_id} --answers answers.json after resolving {request_count} request(s)."
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

fn runx_content(runx: JsonObject) -> JsonObject {
    [("runx".to_owned(), JsonValue::Object(runx))].into()
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
pub(super) struct McpServerState {
    options: McpServerOptions,
    next_run_sequence: u64,
}

impl McpServerState {
    fn new(options: McpServerOptions) -> Self {
        Self {
            options,
            next_run_sequence: 0,
        }
    }

    pub(super) fn next_run_id(&mut self, skill_name: &str) -> String {
        self.next_run_sequence = self.next_run_sequence.saturating_add(1);
        format!(
            "rx_mcp_{}_{}",
            identifier_segment(skill_name),
            self.next_run_sequence
        )
    }
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
        McpServerToolBehavior::Skill(execution) => {
            match execute_mcp_server_skill(state, *execution, arguments) {
                Ok(result) => json_rpc_response(id, mcp_tool_result_json(&result)),
                Err(error) => json_rpc_error(id, -32000, &error.to_string()),
            }
        }
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
