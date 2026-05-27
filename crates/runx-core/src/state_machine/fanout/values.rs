use runx_contracts::{JsonObject, JsonValue};

pub(super) fn resolve_structured_field<'a>(
    outputs: Option<&'a JsonObject>,
    field_path: &str,
) -> Option<&'a JsonValue> {
    let mut current = outputs?;
    let mut parts = field_path.split('.').peekable();
    while let Some(part) = parts.next() {
        let value = current.get(part)?;
        if parts.peek().is_none() {
            return Some(value);
        }
        let JsonValue::Object(next) = value else {
            return None;
        };
        current = next;
    }
    None
}

pub(super) fn json_value_as_f64(value: &JsonValue) -> Option<f64> {
    match value {
        JsonValue::Number(number) => number.as_f64(),
        _ => None,
    }
}

pub(super) fn stable_value(value: &JsonValue) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "undefined".to_owned())
}
