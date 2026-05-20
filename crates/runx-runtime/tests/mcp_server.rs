#![cfg(feature = "mcp")]

use std::io::Cursor;
use std::path::PathBuf;

use runx_contracts::{HarnessReceiptSchema, HarnessState, JsonObject, JsonValue};
use runx_runtime::adapters::mcp::{
    McpContent, McpHostRunResult, McpServerExecutionOptions, McpServerOptions, McpServerTool,
    McpServerToolBehavior, McpToolResult, mcp_tool_result_from_host_result, serve_mcp_json_rpc,
};
use runx_runtime::receipt_store::LocalReceiptStore;

#[test]
fn mcp_server_initializes_lists_and_calls_tools() -> Result<(), Box<dyn std::error::Error>> {
    let responses = run_server(vec![
        request(1, "initialize", JsonObject::new()),
        request(2, "tools/list", JsonObject::new()),
        request(
            3,
            "tools/call",
            [
                ("name".to_owned(), JsonValue::String("echo".to_owned())),
                ("arguments".to_owned(), JsonValue::Object(JsonObject::new())),
            ]
            .into(),
        ),
    ])?;

    assert_eq!(
        path(&responses[0], &["result", "protocolVersion"]),
        Some(&JsonValue::String("2025-06-18".to_owned()))
    );
    assert_eq!(
        path(&responses[1], &["result", "tools", "0", "name"]),
        Some(&JsonValue::String("echo".to_owned()))
    );
    assert_eq!(
        path(&responses[1], &["result", "tools", "1", "name"]),
        Some(&JsonValue::String("runx_resume".to_owned()))
    );
    assert_eq!(
        path(&responses[2], &["result", "content", "0", "text"]),
        Some(&JsonValue::String("hello from server".to_owned()))
    );
    Ok(())
}

#[test]
fn mcp_server_skill_tool_execution_returns_completed_runx_structured_content()
-> Result<(), Box<dyn std::error::Error>> {
    let responses = run_server_with_options(
        vec![request(
            1,
            "tools/call",
            [
                ("name".to_owned(), JsonValue::String("mcp-echo".to_owned())),
                (
                    "arguments".to_owned(),
                    JsonValue::Object(
                        [(
                            "message".to_owned(),
                            JsonValue::String("hello from mcp server".to_owned()),
                        )]
                        .into(),
                    ),
                ),
            ]
            .into(),
        )],
        skill_server_options()?,
    )?;

    assert_no_json_rpc_error(&responses[0]);
    assert_eq!(
        path(
            &responses[0],
            &["result", "structuredContent", "runx", "status"]
        ),
        Some(&JsonValue::String("completed".to_owned()))
    );
    assert_eq!(path(&responses[0], &["result", "isError"]), None);
    Ok(())
}

#[test]
fn mcp_server_single_skill_call_writes_sealed_harness_receipt()
-> Result<(), Box<dyn std::error::Error>> {
    let receipt_root = tempfile::tempdir()?;
    let responses = run_server_with_options(
        vec![request(
            1,
            "tools/call",
            [
                ("name".to_owned(), JsonValue::String("mcp-echo".to_owned())),
                (
                    "arguments".to_owned(),
                    JsonValue::Object(
                        [(
                            "message".to_owned(),
                            JsonValue::String("receipt proof".to_owned()),
                        )]
                        .into(),
                    ),
                ),
            ]
            .into(),
        )],
        skill_server_options_with_receipt_dir(receipt_root.path().to_path_buf())?,
    )?;

    assert_no_json_rpc_error(&responses[0]);
    assert_eq!(
        path(
            &responses[0],
            &["result", "structuredContent", "runx", "status"]
        ),
        Some(&JsonValue::String("completed".to_owned()))
    );
    let JsonValue::String(receipt_id) = path(
        &responses[0],
        &["result", "structuredContent", "runx", "receiptId"],
    )
    .ok_or("missing runx receipt id")?
    else {
        return Err("runx receipt id must be a string".into());
    };

    let receipt = LocalReceiptStore::new(receipt_root.path()).read_exact(receipt_id)?;
    assert_eq!(receipt.schema, HarnessReceiptSchema::V1);
    assert_eq!(receipt.harness.state, HarnessState::Sealed);
    assert_eq!(receipt.harness.seal.as_ref(), Some(&receipt.seal));
    assert_eq!(receipt.id, *receipt_id);
    Ok(())
}

#[test]
fn mcp_server_missing_required_skill_input_pauses_with_request()
-> Result<(), Box<dyn std::error::Error>> {
    let responses = run_server_with_options(
        vec![request(
            1,
            "tools/call",
            [
                ("name".to_owned(), JsonValue::String("mcp-echo".to_owned())),
                ("arguments".to_owned(), JsonValue::Object(JsonObject::new())),
            ]
            .into(),
        )],
        skill_server_options()?,
    )?;

    assert_no_json_rpc_error(&responses[0]);
    assert_eq!(
        path(
            &responses[0],
            &["result", "structuredContent", "runx", "status"]
        ),
        Some(&JsonValue::String("paused".to_owned()))
    );
    assert_eq!(
        path(
            &responses[0],
            &[
                "result",
                "structuredContent",
                "runx",
                "requests",
                "0",
                "kind"
            ],
        ),
        Some(&JsonValue::String("input".to_owned()))
    );
    assert_eq!(
        path(
            &responses[0],
            &[
                "result",
                "structuredContent",
                "runx",
                "requests",
                "0",
                "questions",
                "0",
                "id"
            ],
        ),
        Some(&JsonValue::String("message".to_owned()))
    );
    assert_eq!(path(&responses[0], &["result", "isError"]), None);
    Ok(())
}

#[test]
fn mcp_server_resume_with_response_completes_paused_skill_run()
-> Result<(), Box<dyn std::error::Error>> {
    let responses = run_server_with_options(
        vec![
            request(
                1,
                "tools/call",
                [
                    ("name".to_owned(), JsonValue::String("mcp-echo".to_owned())),
                    ("arguments".to_owned(), JsonValue::Object(JsonObject::new())),
                ]
                .into(),
            ),
            request(
                2,
                "tools/call",
                [
                    (
                        "name".to_owned(),
                        JsonValue::String("runx_resume".to_owned()),
                    ),
                    (
                        "arguments".to_owned(),
                        JsonValue::Object(
                            [
                                (
                                    "run_id".to_owned(),
                                    JsonValue::String("rx_mcp_mcp_echo_1".to_owned()),
                                ),
                                (
                                    "responses".to_owned(),
                                    JsonValue::Array(vec![JsonValue::Object(
                                        [
                                            (
                                                "request_id".to_owned(),
                                                JsonValue::String(
                                                    "input.mcp_echo.message".to_owned(),
                                                ),
                                            ),
                                            (
                                                "actor".to_owned(),
                                                JsonValue::String("human".to_owned()),
                                            ),
                                            (
                                                "payload".to_owned(),
                                                JsonValue::Object(
                                                    [(
                                                        "message".to_owned(),
                                                        JsonValue::String(
                                                            "hello after resume".to_owned(),
                                                        ),
                                                    )]
                                                    .into(),
                                                ),
                                            ),
                                        ]
                                        .into(),
                                    )]),
                                ),
                            ]
                            .into(),
                        ),
                    ),
                ]
                .into(),
            ),
        ],
        skill_server_options()?,
    )?;

    assert_no_json_rpc_error(&responses[0]);
    assert_eq!(
        path(
            &responses[0],
            &["result", "structuredContent", "runx", "status"]
        ),
        Some(&JsonValue::String("paused".to_owned()))
    );
    assert_no_json_rpc_error(&responses[1]);
    assert_eq!(
        path(
            &responses[1],
            &["result", "structuredContent", "runx", "status"]
        ),
        Some(&JsonValue::String("completed".to_owned()))
    );
    assert_eq!(path(&responses[1], &["result", "isError"]), None);
    Ok(())
}

#[test]
fn mcp_server_graph_approval_round_trip_pauses_then_resumes()
-> Result<(), Box<dyn std::error::Error>> {
    let responses = run_server_with_options(
        vec![
            request(
                1,
                "tools/call",
                [
                    (
                        "name".to_owned(),
                        JsonValue::String("mcp-approval-graph".to_owned()),
                    ),
                    ("arguments".to_owned(), JsonValue::Object(JsonObject::new())),
                ]
                .into(),
            ),
            request(
                2,
                "tools/call",
                [
                    (
                        "name".to_owned(),
                        JsonValue::String("runx_resume".to_owned()),
                    ),
                    (
                        "arguments".to_owned(),
                        JsonValue::Object(
                            [
                                (
                                    "run_id".to_owned(),
                                    JsonValue::String("rx_mcp_mcp_approval_graph_1".to_owned()),
                                ),
                                (
                                    "responses".to_owned(),
                                    JsonValue::Array(vec![JsonValue::Object(
                                        [
                                            (
                                                "request_id".to_owned(),
                                                JsonValue::String("approve_approval".to_owned()),
                                            ),
                                            (
                                                "actor".to_owned(),
                                                JsonValue::String("human".to_owned()),
                                            ),
                                            ("payload".to_owned(), JsonValue::Bool(true)),
                                        ]
                                        .into(),
                                    )]),
                                ),
                            ]
                            .into(),
                        ),
                    ),
                ]
                .into(),
            ),
        ],
        approval_graph_server_options()?,
    )?;

    assert_no_json_rpc_error(&responses[0]);
    assert_eq!(
        path(
            &responses[0],
            &["result", "structuredContent", "runx", "status"]
        ),
        Some(&JsonValue::String("paused".to_owned()))
    );
    assert_eq!(
        path(
            &responses[0],
            &[
                "result",
                "structuredContent",
                "runx",
                "requests",
                "0",
                "kind"
            ],
        ),
        Some(&JsonValue::String("approval".to_owned()))
    );
    assert_eq!(
        path(
            &responses[0],
            &[
                "result",
                "structuredContent",
                "runx",
                "requests",
                "0",
                "gate",
                "id"
            ],
        ),
        Some(&JsonValue::String("mcp-approval".to_owned()))
    );
    assert_no_json_rpc_error(&responses[1]);
    assert_eq!(
        path(
            &responses[1],
            &["result", "structuredContent", "runx", "status"]
        ),
        Some(&JsonValue::String("completed".to_owned()))
    );
    assert_eq!(path(&responses[1], &["result", "isError"]), None);
    Ok(())
}

#[test]
fn mcp_server_reports_duplicate_tool_names() -> Result<(), Box<dyn std::error::Error>> {
    let options = McpServerOptions {
        package_name: "runx-cli".to_owned(),
        package_version: "0.0.0".to_owned(),
        tools: vec![fixed_tool("dup"), fixed_tool("dup")],
    };

    let error = match serve_mcp_json_rpc(Cursor::new(Vec::new()), Vec::new(), options) {
        Ok(()) => return Err("duplicate tool names fail before serving".into()),
        Err(error) => error,
    };

    assert_eq!(
        error.to_string(),
        "runx mcp serve received duplicate tool name 'dup'. Serve unique skill names only."
    );
    Ok(())
}

#[test]
fn mcp_server_json_rpc_errors_match_lifecycle_contract() -> Result<(), Box<dyn std::error::Error>> {
    let responses = run_server(vec![
        request(1, "unknown/method", JsonObject::new()),
        request(2, "tools/call", JsonObject::new()),
        request(
            3,
            "tools/call",
            [
                ("name".to_owned(), JsonValue::String("missing".to_owned())),
                ("arguments".to_owned(), JsonValue::Object(JsonObject::new())),
            ]
            .into(),
        ),
        request(
            4,
            "tools/call",
            [
                ("name".to_owned(), JsonValue::String("echo".to_owned())),
                (
                    "arguments".to_owned(),
                    JsonValue::String("not an object".to_owned()),
                ),
            ]
            .into(),
        ),
    ])?;

    assert_error(&responses[0], -32601, "method not found");
    assert_error(&responses[1], -32602, "invalid tool call");
    assert_error(&responses[2], -32601, "tool not found: missing");
    assert_error(&responses[3], -32602, "tool arguments must be an object");
    Ok(())
}

#[test]
fn mcp_server_rejects_oversized_requests() -> Result<(), Box<dyn std::error::Error>> {
    let mut input = b"Content-Length: 4194305\r\n\r\n".to_vec();
    input.extend(std::iter::repeat_n(b' ', 4_194_305));
    let error = match serve_mcp_json_rpc(Cursor::new(input), Vec::new(), server_options()) {
        Ok(()) => return Err("oversized request fails".into()),
        Err(error) => error,
    };

    assert_eq!(
        error.to_string(),
        "MCP request declared Content-Length 4194305, exceeding 4194304-byte limit."
    );
    Ok(())
}

#[test]
fn mcp_server_parse_error_is_json_rpc_error() -> Result<(), Box<dyn std::error::Error>> {
    let responses = run_raw(b"Content-Length: 1\r\n\r\n{".to_vec())?;

    assert_error(&responses[0], -32700, "parse error");
    Ok(())
}

#[test]
fn mcp_server_host_result_conversion_covers_terminal_statuses() {
    let completed = mcp_tool_result_from_host_result(McpHostRunResult::Completed {
        skill_name: "echo".to_owned(),
        output: String::new(),
        receipt_id: "receipt-1".to_owned(),
        runx: runx_status("completed"),
    });
    assert_eq!(
        completed.content[0].text,
        "echo completed. Inspect receipt receipt-1."
    );
    assert!(!completed.is_error);

    let paused = mcp_tool_result_from_host_result(McpHostRunResult::Paused {
        skill_name: "echo".to_owned(),
        run_id: "run-1".to_owned(),
        request_count: 2,
        runx: runx_status("paused"),
    });
    assert_eq!(
        paused.content[0].text,
        "echo paused at run-1. Resume with runx_resume after resolving 2 request(s)."
    );
    assert!(!paused.is_error);

    for result in [
        McpHostRunResult::Denied {
            skill_name: "echo".to_owned(),
            receipt_id: Some("receipt-2".to_owned()),
            runx: runx_status("denied"),
        },
        McpHostRunResult::Escalated {
            skill_name: "echo".to_owned(),
            receipt_id: "receipt-3".to_owned(),
            error: "needs approval".to_owned(),
            runx: runx_status("escalated"),
        },
        McpHostRunResult::Failed {
            skill_name: "echo".to_owned(),
            receipt_id: None,
            error: "boom".to_owned(),
            runx: runx_status("failed"),
        },
    ] {
        assert!(mcp_tool_result_from_host_result(result).is_error);
    }
}

fn run_server(requests: Vec<JsonValue>) -> Result<Vec<JsonValue>, Box<dyn std::error::Error>> {
    run_server_with_options(requests, server_options())
}

fn run_server_with_options(
    requests: Vec<JsonValue>,
    options: McpServerOptions,
) -> Result<Vec<JsonValue>, Box<dyn std::error::Error>> {
    let input = requests
        .iter()
        .map(frame)
        .collect::<Result<Vec<_>, _>>()?
        .concat();
    run_raw_with_options(input, options)
}

fn run_raw(input: Vec<u8>) -> Result<Vec<JsonValue>, Box<dyn std::error::Error>> {
    run_raw_with_options(input, server_options())
}

fn run_raw_with_options(
    input: Vec<u8>,
    options: McpServerOptions,
) -> Result<Vec<JsonValue>, Box<dyn std::error::Error>> {
    let mut output = Vec::new();
    serve_mcp_json_rpc(Cursor::new(input), &mut output, options)?;
    parse_frames(&output)
}

fn skill_server_options() -> Result<McpServerOptions, Box<dyn std::error::Error>> {
    Ok(McpServerOptions::from_skill_paths(
        &[repo_root()?.join("fixtures/skills/mcp-echo")],
        "runx-cli",
        "0.0.0",
    )?)
}

fn skill_server_options_with_receipt_dir(
    receipt_dir: PathBuf,
) -> Result<McpServerOptions, Box<dyn std::error::Error>> {
    Ok(McpServerOptions::from_skill_paths_with_execution(
        &[repo_root()?.join("fixtures/skills/mcp-echo")],
        "runx-cli",
        "0.0.0",
        McpServerExecutionOptions {
            runner: None,
            receipt_dir: Some(receipt_dir),
            env: std::env::vars().collect(),
        },
    )?)
}

fn approval_graph_server_options() -> Result<McpServerOptions, Box<dyn std::error::Error>> {
    Ok(McpServerOptions::from_skill_paths(
        &[repo_root()?.join("fixtures/skills/mcp-approval-graph")],
        "runx-cli",
        "0.0.0",
    )?)
}

fn repo_root() -> Result<PathBuf, Box<dyn std::error::Error>> {
    Ok(PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()?)
}

fn server_options() -> McpServerOptions {
    let mut tools = vec![fixed_tool("echo")];
    tools.push(McpServerTool {
        name: "runx_resume".to_owned(),
        description:
            "Resume a paused runx run by run id with zero or more structured resolution payloads."
                .to_owned(),
        input_schema: JsonObject::new(),
        result: McpServerToolBehavior::ResumeNotImplemented,
    });
    McpServerOptions {
        package_name: "runx-cli".to_owned(),
        package_version: "0.0.0".to_owned(),
        tools,
    }
}

fn fixed_tool(name: &str) -> McpServerTool {
    McpServerTool {
        name: name.to_owned(),
        description: "fixture tool".to_owned(),
        input_schema: [
            ("type".to_owned(), JsonValue::String("object".to_owned())),
            (
                "properties".to_owned(),
                JsonValue::Object(JsonObject::new()),
            ),
            ("required".to_owned(), JsonValue::Array(Vec::new())),
            ("additionalProperties".to_owned(), JsonValue::Bool(false)),
        ]
        .into(),
        result: McpServerToolBehavior::Fixed(McpToolResult {
            content: vec![McpContent {
                text: "hello from server".to_owned(),
            }],
            structured_content: Some(runx_content("completed")),
            is_error: false,
        }),
    }
}

fn request(id: i64, method: &str, params: JsonObject) -> JsonValue {
    JsonValue::Object(
        [
            ("jsonrpc".to_owned(), JsonValue::String("2.0".to_owned())),
            (
                "id".to_owned(),
                JsonValue::Number(runx_contracts::JsonNumber::I64(id)),
            ),
            ("method".to_owned(), JsonValue::String(method.to_owned())),
            ("params".to_owned(), JsonValue::Object(params)),
        ]
        .into(),
    )
}

fn frame(message: &JsonValue) -> Result<Vec<u8>, serde_json::Error> {
    let body = serde_json::to_vec(message)?;
    let mut framed = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
    framed.extend(body);
    Ok(framed)
}

fn parse_frames(mut bytes: &[u8]) -> Result<Vec<JsonValue>, Box<dyn std::error::Error>> {
    let mut messages = Vec::new();
    while !bytes.is_empty() {
        let header_end = bytes
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .ok_or("missing frame header")?;
        let header = std::str::from_utf8(&bytes[..header_end])?;
        let length = header
            .lines()
            .find_map(|line| line.strip_prefix("Content-Length: "))
            .ok_or("missing content length")?
            .parse::<usize>()?;
        let body_start = header_end + 4;
        let body_end = body_start + length;
        messages.push(serde_json::from_slice(&bytes[body_start..body_end])?);
        bytes = &bytes[body_end..];
    }
    Ok(messages)
}

fn path<'a>(value: &'a JsonValue, path: &[&str]) -> Option<&'a JsonValue> {
    let mut current = value;
    for segment in path {
        current = match current {
            JsonValue::Object(record) => record.get(*segment)?,
            JsonValue::Array(values) => values.get(segment.parse::<usize>().ok()?)?,
            _ => return None,
        };
    }
    Some(current)
}

fn assert_error(message: &JsonValue, code: i64, text: &str) {
    assert_eq!(
        path(message, &["error", "code"]),
        Some(&JsonValue::Number(runx_contracts::JsonNumber::I64(code)))
    );
    assert_eq!(
        path(message, &["error", "message"]),
        Some(&JsonValue::String(text.to_owned()))
    );
}

fn assert_no_json_rpc_error(message: &JsonValue) {
    assert_eq!(
        path(message, &["error"]),
        None,
        "unexpected JSON-RPC error: {message:?}"
    );
}

fn runx_status(status: &str) -> JsonObject {
    [("status".to_owned(), JsonValue::String(status.to_owned()))].into()
}

fn runx_content(status: &str) -> JsonObject {
    [("runx".to_owned(), JsonValue::Object(runx_status(status)))].into()
}
