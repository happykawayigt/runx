use runx_contracts::{JsonNumber, JsonObject, JsonValue};

use super::types::McpToolDescriptor;

pub(super) const PROTOCOL_VERSION: &str = "2025-06-18";

pub(super) fn json_rpc_response(id: JsonValue, result: JsonValue) -> JsonValue {
    JsonValue::Object(
        [
            ("jsonrpc".to_owned(), JsonValue::String("2.0".to_owned())),
            ("id".to_owned(), id),
            ("result".to_owned(), result),
        ]
        .into(),
    )
}

pub(super) fn json_rpc_error(id: JsonValue, code: i64, message: &str) -> JsonValue {
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

pub(super) fn json_rpc_request(id: i64, method: &str, params: JsonObject) -> JsonValue {
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

pub(super) fn initialize_request(id: i64) -> JsonValue {
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

pub(super) fn initialized_notification() -> JsonValue {
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

pub(super) fn tool_call_request(id: i64, tool: &str, arguments: &JsonObject) -> JsonValue {
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

pub(super) fn tools_list_request(id: i64) -> JsonValue {
    json_rpc_request(id, "tools/list", JsonObject::new())
}

pub(super) fn parse_mcp_tools_list(result: JsonValue) -> Vec<McpToolDescriptor> {
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

pub(super) fn text_content(text: String) -> JsonValue {
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
