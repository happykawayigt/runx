use std::collections::BTreeMap;

use runx_contracts::{JsonNumber, JsonObject, JsonValue};

use crate::RuntimeError;

const TEMPLATE_OPEN: &str = "\x7b\x7b";
const TEMPLATE_CLOSE: &str = "\x7d\x7d";

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

pub(super) fn env_value(env: &BTreeMap<String, String>, name: Option<&JsonValue>) -> String {
    env.get(&js_string(name)).cloned().unwrap_or_default()
}

pub(super) fn js_string(value: Option<&JsonValue>) -> String {
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
