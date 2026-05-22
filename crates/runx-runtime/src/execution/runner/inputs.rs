use runx_contracts::{JsonObject, JsonValue};
use runx_parser::GraphStep;

use runx_contracts::AuthorityVerb;

use super::authority::authority_denied;
use crate::RuntimeError;

pub(super) fn require_object_input<'a>(
    step: &GraphStep,
    inputs: &'a JsonObject,
    field: &str,
) -> Result<&'a JsonObject, RuntimeError> {
    match inputs.get(field) {
        Some(JsonValue::Object(object)) => Ok(object),
        Some(_) => Err(authority_denied(
            step,
            AuthorityVerb::Spend,
            format!("{field} must be an object before payment rail execution"),
        )),
        None => Err(authority_denied(
            step,
            AuthorityVerb::Spend,
            format!("{field} is required before payment rail execution"),
        )),
    }
}

pub(super) fn require_non_empty_string_field(
    step: &GraphStep,
    object: &JsonObject,
    field_path: &str,
) -> Result<String, RuntimeError> {
    let Some((_, field)) = field_path.rsplit_once('.') else {
        return Err(authority_denied(
            step,
            AuthorityVerb::Spend,
            format!("{field_path} is not a valid payment admission field"),
        ));
    };
    let Some(value) = object.get(field) else {
        return Err(authority_denied(
            step,
            AuthorityVerb::Spend,
            format!("{field_path} is required before payment rail execution"),
        ));
    };
    require_non_empty_string_value(step, value, field_path)
}

pub(super) fn require_non_empty_string_value(
    step: &GraphStep,
    value: &JsonValue,
    field_path: &str,
) -> Result<String, RuntimeError> {
    let JsonValue::String(value) = value else {
        return Err(authority_denied(
            step,
            AuthorityVerb::Spend,
            format!("{field_path} must be a string before payment rail execution"),
        ));
    };
    if value.trim().is_empty() {
        return Err(authority_denied(
            step,
            AuthorityVerb::Spend,
            format!("{field_path} must not be empty before payment rail execution"),
        ));
    }
    Ok(value.to_owned())
}

pub(super) fn require_reference_input(
    step: &GraphStep,
    inputs: &JsonObject,
    field: &str,
) -> Result<runx_contracts::Reference, RuntimeError> {
    match inputs.get(field) {
        Some(JsonValue::Object(_)) => required_typed_value(step, inputs.get(field), field),
        Some(_) => Err(authority_denied(
            step,
            AuthorityVerb::Spend,
            format!("{field} must be a Reference before payment rail execution"),
        )),
        None => Err(authority_denied(
            step,
            AuthorityVerb::Spend,
            format!("{field} is required before payment rail execution"),
        )),
    }
}

pub(super) fn optional_typed_input<T: serde::de::DeserializeOwned>(
    step: &GraphStep,
    object: &JsonObject,
    field_path: &str,
    field: &str,
) -> Result<Option<T>, RuntimeError> {
    let Some(value) = object.get(field) else {
        return Ok(None);
    };
    required_typed_value(step, Some(value), field_path).map(Some)
}

pub(super) fn required_typed_input<T: serde::de::DeserializeOwned>(
    step: &GraphStep,
    object: &JsonObject,
    field_path: &str,
    field: &str,
) -> Result<T, RuntimeError> {
    required_typed_value(step, object.get(field), field_path)
}

pub(super) fn optional_typed_vec_input<T: serde::de::DeserializeOwned>(
    step: &GraphStep,
    object: &JsonObject,
    field_path: &str,
    field: &str,
) -> Result<Option<Vec<T>>, RuntimeError> {
    optional_typed_input(step, object, field_path, field)
}

pub(super) fn required_typed_value<T: serde::de::DeserializeOwned>(
    step: &GraphStep,
    value: Option<&JsonValue>,
    field_path: &str,
) -> Result<T, RuntimeError> {
    let Some(value) = value else {
        return Err(authority_denied(
            step,
            AuthorityVerb::Spend,
            format!("{field_path} is required before payment rail execution"),
        ));
    };
    serde_json::from_value::<T>(
        serde_json::to_value(value)
            .map_err(|source| RuntimeError::json(format!("serializing {field_path}"), source))?,
    )
    .map_err(|source| {
        authority_denied(
            step,
            AuthorityVerb::Spend,
            format!("{field_path} is not valid typed payment authority: {source}"),
        )
    })
}

pub(super) fn required_input_string(
    step: &GraphStep,
    inputs: &JsonObject,
    field: &str,
) -> Result<String, RuntimeError> {
    let Some(value) = inputs.get(field) else {
        return Err(RuntimeError::InvalidRunStep {
            step_id: step.id.clone(),
            reason: format!("{field} input is required"),
        });
    };
    string_value(step, field, value)
}

pub(super) fn optional_input_string(
    step: &GraphStep,
    inputs: &JsonObject,
    field: &str,
) -> Result<Option<String>, RuntimeError> {
    let Some(value) = inputs.get(field) else {
        return Ok(None);
    };
    Ok(Some(string_value(step, field, value)?))
}

pub(super) fn string_value(
    step: &GraphStep,
    field: &str,
    value: &JsonValue,
) -> Result<String, RuntimeError> {
    Ok(string_value_ref(step, field, value)?.to_owned())
}

pub(super) fn string_value_ref<'a>(
    step: &GraphStep,
    field: &str,
    value: &'a JsonValue,
) -> Result<&'a str, RuntimeError> {
    let JsonValue::String(value) = value else {
        return Err(RuntimeError::InvalidRunStep {
            step_id: step.id.clone(),
            reason: format!("{field} must be a string"),
        });
    };
    Ok(value)
}
