#![cfg(any(feature = "mcp", feature = "mcp-rmcp"))]

use std::io::Cursor;
#[cfg(any(feature = "mcp", feature = "mcp-rmcp"))]
use std::path::PathBuf;

#[cfg(feature = "mcp")]
use runx_contracts::{HarnessReceiptSchema, HarnessState};
use runx_contracts::{JsonObject, JsonValue};
#[cfg(feature = "mcp")]
use runx_runtime::adapters::mcp::McpServerExecutionOptions;
use runx_runtime::adapters::mcp::{
    McpContent, McpHostRunResult, McpServerOptions, McpServerTool, McpServerToolBehavior,
    McpToolResult, mcp_tool_result_from_host_result, serve_mcp_json_rpc,
};
#[cfg(feature = "mcp")]
use runx_runtime::receipt_store::LocalReceiptStore;

#[test]
#[cfg(feature = "mcp")]
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
    assert_eq!(path(&responses[1], &["result", "tools", "1", "name"]), None);
    assert_eq!(
        path(&responses[2], &["result", "content", "0", "text"]),
        Some(&JsonValue::String("hello from server".to_owned()))
    );
    Ok(())
}

#[test]
#[cfg(feature = "mcp")]
fn mcp_server_matches_recorded_stdio_wire_contract() -> Result<(), Box<dyn std::error::Error>> {
    for fixture_name in ["basic-lifecycle", "error-paths"] {
        let input = frame_jsonl_fixture(fixture_name, "requests")?;
        let expected = frame_jsonl_fixture(fixture_name, "responses")?;
        let output = run_raw_output_with_options(input, server_options())?;

        assert_eq!(
            String::from_utf8_lossy(&output),
            String::from_utf8_lossy(&expected),
            "{fixture_name} raw MCP stdio response bytes changed"
        );
    }
    Ok(())
}

#[test]
#[cfg(feature = "mcp-rmcp")]
fn mcp_rmcp_server_runs_basic_lifecycle() -> Result<(), Box<dyn std::error::Error>> {
    let responses = run_server(vec![
        rmcp_initialize_request(1),
        initialized_notification(),
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
        path(&responses[2], &["result", "content", "0", "text"]),
        Some(&JsonValue::String("hello from server".to_owned()))
    );
    Ok(())
}

#[test]
#[cfg(feature = "mcp-rmcp")]
fn mcp_rmcp_server_replays_recorded_basic_lifecycle_fixture()
-> Result<(), Box<dyn std::error::Error>> {
    let input = frame_jsonl_fixture("basic-lifecycle", "requests")?;
    let responses = run_raw_with_options(input, server_options())?;

    assert_eq!(
        path(&responses[0], &["result", "protocolVersion"]),
        Some(&JsonValue::String("2025-06-18".to_owned()))
    );
    assert_eq!(
        path(&responses[1], &["result", "tools", "0", "name"]),
        Some(&JsonValue::String("echo".to_owned()))
    );
    assert_eq!(
        path(&responses[2], &["result", "content", "0", "text"]),
        Some(&JsonValue::String("hello from server".to_owned()))
    );
    Ok(())
}

#[test]
#[cfg(feature = "mcp")]
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
        Some(&JsonValue::String("completed".to_owned())),
        "unexpected MCP server skill response: {:#?}",
        responses[0]
    );
    assert_eq!(path(&responses[0], &["result", "isError"]), None);
    Ok(())
}

#[test]
#[cfg(feature = "mcp")]
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
        Some(&JsonValue::String("completed".to_owned())),
        "unexpected MCP server skill response: {:#?}",
        responses[0]
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
#[cfg(feature = "mcp")]
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
        Some(&JsonValue::String("needs_agent".to_owned()))
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
#[cfg(feature = "mcp")]
fn mcp_server_graph_approval_pauses_with_request() -> Result<(), Box<dyn std::error::Error>> {
    let responses = run_server_with_options(
        vec![request(
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
        )],
        approval_graph_server_options()?,
    )?;

    assert_no_json_rpc_error(&responses[0]);
    assert_eq!(
        path(
            &responses[0],
            &["result", "structuredContent", "runx", "status"]
        ),
        Some(&JsonValue::String("needs_agent".to_owned()))
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
    Ok(())
}

#[test]
#[cfg(feature = "mcp")]
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
#[cfg(feature = "mcp")]
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
#[cfg(feature = "mcp")]
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
#[cfg(feature = "mcp")]
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

    let needs_agent = mcp_tool_result_from_host_result(McpHostRunResult::NeedsAgent {
        skill_name: "echo".to_owned(),
        run_id: "run-1".to_owned(),
        request_count: 2,
        runx: runx_status("needs_agent"),
    });
    assert_eq!(
        needs_agent.content[0].text,
        "echo needs agent input at run-1. Continue by rerunning the same skill with --run-id run-1 --answers answers.json after resolving 2 request(s)."
    );
    assert!(!needs_agent.is_error);

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

#[cfg(feature = "mcp")]
fn run_raw(input: Vec<u8>) -> Result<Vec<JsonValue>, Box<dyn std::error::Error>> {
    run_raw_with_options(input, server_options())
}

fn run_raw_with_options(
    input: Vec<u8>,
    options: McpServerOptions,
) -> Result<Vec<JsonValue>, Box<dyn std::error::Error>> {
    parse_frames(&run_raw_output_with_options(input, options)?)
}

fn run_raw_output_with_options(
    input: Vec<u8>,
    options: McpServerOptions,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut output = Vec::new();
    serve_mcp_json_rpc(Cursor::new(input), &mut output, options)?;
    Ok(output)
}

#[cfg(feature = "mcp")]
fn skill_server_options() -> Result<McpServerOptions, Box<dyn std::error::Error>> {
    Ok(McpServerOptions::from_skill_paths_with_execution(
        &[repo_root()?.join("fixtures/skills/mcp-echo")],
        "runx-cli",
        "0.0.0",
        mcp_server_execution_options(None)?,
    )?)
}

#[cfg(feature = "mcp")]
fn skill_server_options_with_receipt_dir(
    receipt_dir: PathBuf,
) -> Result<McpServerOptions, Box<dyn std::error::Error>> {
    Ok(McpServerOptions::from_skill_paths_with_execution(
        &[repo_root()?.join("fixtures/skills/mcp-echo")],
        "runx-cli",
        "0.0.0",
        mcp_server_execution_options(Some(receipt_dir))?,
    )?)
}

#[cfg(feature = "mcp")]
fn approval_graph_server_options() -> Result<McpServerOptions, Box<dyn std::error::Error>> {
    Ok(McpServerOptions::from_skill_paths_with_execution(
        &[repo_root()?.join("fixtures/skills/mcp-approval-graph")],
        "runx-cli",
        "0.0.0",
        mcp_server_execution_options(None)?,
    )?)
}

#[cfg(feature = "mcp")]
fn mcp_server_execution_options(
    receipt_dir: Option<PathBuf>,
) -> Result<McpServerExecutionOptions, Box<dyn std::error::Error>> {
    let mut env = std::env::vars().collect::<std::collections::BTreeMap<_, _>>();
    env.insert(
        "RUNX_CWD".to_owned(),
        repo_root()?.to_string_lossy().into_owned(),
    );
    Ok(McpServerExecutionOptions {
        runner: None,
        receipt_dir,
        env,
    })
}

#[cfg(any(feature = "mcp", feature = "mcp-rmcp"))]
fn repo_root() -> Result<PathBuf, Box<dyn std::error::Error>> {
    Ok(PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()?)
}

fn server_options() -> McpServerOptions {
    McpServerOptions {
        package_name: "runx-cli".to_owned(),
        package_version: "0.0.0".to_owned(),
        tools: vec![fixed_tool("echo")],
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

#[cfg(feature = "mcp-rmcp")]
fn rmcp_initialize_request(id: i64) -> JsonValue {
    request(
        id,
        "initialize",
        [
            (
                "protocolVersion".to_owned(),
                JsonValue::String("2025-06-18".to_owned()),
            ),
            (
                "capabilities".to_owned(),
                JsonValue::Object(JsonObject::new()),
            ),
            (
                "clientInfo".to_owned(),
                JsonValue::Object(
                    [
                        ("name".to_owned(), JsonValue::String("runx-test".to_owned())),
                        ("version".to_owned(), JsonValue::String("0.0.0".to_owned())),
                    ]
                    .into(),
                ),
            ),
        ]
        .into(),
    )
}

#[cfg(feature = "mcp-rmcp")]
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

fn frame(message: &JsonValue) -> Result<Vec<u8>, serde_json::Error> {
    let body = serde_json::to_vec(message)?;
    let mut framed = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
    framed.extend(body);
    Ok(framed)
}

#[cfg(any(feature = "mcp", feature = "mcp-rmcp"))]
fn frame_jsonl_fixture(
    fixture_name: &str,
    kind: &str,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let path = repo_root()?.join(format!(
        "fixtures/runtime/adapters/mcp/wire-contract/{fixture_name}.{kind}.jsonl"
    ));
    let mut framed = Vec::new();
    for line in std::fs::read_to_string(path)?
        .lines()
        .filter(|line| !line.is_empty())
    {
        let _: JsonValue = serde_json::from_str(line)?;
        framed.extend(format!("Content-Length: {}\r\n\r\n", line.len()).as_bytes());
        framed.extend(line.as_bytes());
    }
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

#[cfg(feature = "mcp")]
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

#[cfg(feature = "mcp")]
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
