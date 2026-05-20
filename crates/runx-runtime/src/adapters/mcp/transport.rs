// rust-style-allow: large-file because the client-side transport keeps stdio
// framing, response buffering, and bounded read/write helpers adjacent to the
// transport implementations they coordinate.
use std::io::{Read, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::{Duration, Instant};

use runx_contracts::{JsonNumber, JsonObject, JsonValue};

use crate::sandbox::SandboxPlan;

use super::framing::{content_length, find_header_end};
use super::jsonrpc::{
    initialize_request, initialized_notification, parse_mcp_tools_list, text_content,
    tool_call_request, tools_list_request,
};
use super::templates::{env_value, js_string};
use super::types::{
    McpListToolsRequest, McpToolCallRequest, McpToolDescriptor, McpTransport, McpTransportError,
};

const MAX_CLIENT_RESPONSE_BYTES: usize = 1024 * 1024;
const POLL_INTERVAL: Duration = Duration::from_millis(10);

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

fn terminate_child(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}
