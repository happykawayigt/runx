use std::collections::BTreeMap;

use runx_contracts::{JsonObject, JsonValue};
use serde::{Deserialize, Serialize};

use crate::skill::{
    SkillArtifactContract, SkillIdempotencyPolicy, SkillInput, SkillRetryPolicy, SkillSource,
    validate_skill_artifact_contract, validate_skill_source,
};
use crate::{ParseError, ValidationError, assert_yaml_parity_subset};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RawToolManifestIr {
    pub document: JsonObject,
    pub raw: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidatedTool {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub source: SkillSource,
    pub inputs: BTreeMap<String, SkillInput>,
    pub scopes: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub risk: Option<JsonValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime: Option<JsonValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry: Option<SkillRetryPolicy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idempotency: Option<SkillIdempotencyPolicy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mutating: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifacts: Option<SkillArtifactContract>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runx: Option<JsonObject>,
    pub raw: RawToolManifestIr,
}

pub fn parse_tool_manifest_yaml(yaml: &str) -> Result<RawToolManifestIr, ParseError> {
    assert_yaml_parity_subset("tool_manifest", yaml)?;
    let parsed: JsonValue =
        serde_norway::from_str(yaml).map_err(|error| ParseError::InvalidYaml {
            field: "tool_manifest".to_owned(),
            message: error.to_string(),
        })?;
    manifest_from_value(parsed, yaml, "Tool manifest YAML must parse to an object.")
}

pub fn parse_tool_manifest_json(json: &str) -> Result<RawToolManifestIr, ParseError> {
    let parsed: JsonValue =
        serde_json::from_str(json).map_err(|error| ParseError::InvalidJson {
            field: "tool_manifest".to_owned(),
            message: format!("Tool manifest JSON is invalid: {error}"),
        })?;
    manifest_from_value(parsed, json, "Tool manifest JSON must parse to an object.")
}

pub fn validate_tool_manifest(raw: RawToolManifestIr) -> Result<ValidatedTool, ValidationError> {
    let runx = optional_object(raw.document.get("runx"), "runx")?;
    let risk = raw.document.get("risk").cloned();
    let source = validate_tool_source(
        validate_skill_source(
            &required_object(raw.document.get("source"), "source")?.clone(),
            runx.as_ref(),
        )?,
        "source.type",
    )?;
    Ok(ValidatedTool {
        name: required_string(raw.document.get("name"), "name")?,
        description: optional_string(raw.document.get("description"), "description")?,
        source,
        inputs: validate_inputs(
            optional_object(raw.document.get("inputs"), "inputs")?.unwrap_or_default(),
        )?,
        scopes: optional_string_array(raw.document.get("scopes"), "scopes")?.unwrap_or_default(),
        risk: risk.clone(),
        runtime: raw.document.get("runtime").cloned(),
        retry: validate_retry(
            first_value(
                raw.document.get("retry"),
                field_value(runx.as_ref(), "retry"),
            ),
            "retry",
        )?,
        idempotency: validate_idempotency(
            first_value(
                raw.document.get("idempotency"),
                field_value(runx.as_ref(), "idempotency"),
            ),
            "idempotency",
        )?,
        mutating: validate_mutating(
            first_value(
                first_value(
                    raw.document.get("mutating"),
                    nested_value(risk.as_ref(), "mutating"),
                ),
                field_value(runx.as_ref(), "mutating"),
            ),
            "mutating",
        )?,
        artifacts: validate_skill_artifact_contract(
            field_value(runx.as_ref(), "artifacts"),
            "runx.artifacts",
        )?,
        runx,
        raw,
    })
}

fn validate_tool_source(source: SkillSource, field: &str) -> Result<SkillSource, ValidationError> {
    if matches!(
        source.source_type.as_str(),
        "cli-tool" | "mcp" | "a2a" | "catalog" | "http"
    ) {
        return Ok(source);
    }
    Err(validation_error(format!(
        "{field} must be one of cli-tool, mcp, a2a, catalog, or http for tool manifests."
    )))
}

fn manifest_from_value(
    value: JsonValue,
    raw: &str,
    object_error: &str,
) -> Result<RawToolManifestIr, ParseError> {
    let JsonValue::Object(document) = value else {
        return Err(ParseError::InvalidDocument {
            field: "tool_manifest".to_owned(),
            message: object_error.to_owned(),
        });
    };
    Ok(RawToolManifestIr {
        document,
        raw: raw.to_owned(),
    })
}

fn validate_inputs(inputs: JsonObject) -> Result<BTreeMap<String, SkillInput>, ValidationError> {
    inputs
        .into_iter()
        .map(|(name, value)| {
            let field = format!("inputs.{name}");
            let input = required_object(Some(&value), &field)?;
            Ok((
                name.clone(),
                SkillInput {
                    input_type: optional_string(input.get("type"), &format!("{field}.type"))?
                        .unwrap_or_else(|| "string".to_owned()),
                    required: optional_bool(input.get("required"), &format!("{field}.required"))?
                        .unwrap_or(false),
                    description: optional_string(
                        input.get("description"),
                        &format!("{field}.description"),
                    )?,
                    default: input.get("default").cloned(),
                },
            ))
        })
        .collect()
}

fn validate_retry(
    value: Option<&JsonValue>,
    field: &str,
) -> Result<Option<SkillRetryPolicy>, ValidationError> {
    let Some(retry) = optional_object(value, field)? else {
        return Ok(None);
    };
    let max_attempts =
        optional_u64(retry.get("max_attempts"), &format!("{field}.max_attempts"))?.unwrap_or(1);
    if max_attempts == 0 {
        return Err(validation_error(format!(
            "{field}.max_attempts must be a positive integer."
        )));
    }
    Ok(Some(SkillRetryPolicy { max_attempts }))
}

fn validate_idempotency(
    value: Option<&JsonValue>,
    field: &str,
) -> Result<Option<SkillIdempotencyPolicy>, ValidationError> {
    match value {
        None | Some(JsonValue::Null) => Ok(None),
        Some(JsonValue::String(value)) if value.trim().is_empty() => {
            Err(validation_error(format!("{field} must not be empty.")))
        }
        Some(JsonValue::String(value)) => Ok(Some(SkillIdempotencyPolicy {
            key: Some(value.clone()),
        })),
        Some(value) => {
            let record = required_object(Some(value), field)?;
            Ok(Some(SkillIdempotencyPolicy {
                key: optional_non_empty_string(record.get("key"), &format!("{field}.key"))?,
            }))
        }
    }
}

fn validate_mutating(
    value: Option<&JsonValue>,
    field: &str,
) -> Result<Option<bool>, ValidationError> {
    optional_bool(value, field)
}

fn validation_error(message: impl Into<String>) -> ValidationError {
    ValidationError::InvalidField {
        field: "tool_manifest".to_owned(),
        message: message.into(),
    }
}

fn required_string(value: Option<&JsonValue>, field: &str) -> Result<String, ValidationError> {
    match optional_string(value, field)? {
        Some(value) if !value.is_empty() => Ok(value),
        _ => Err(ValidationError::MissingField {
            field: field.to_owned(),
        }),
    }
}

fn optional_string(
    value: Option<&JsonValue>,
    field: &str,
) -> Result<Option<String>, ValidationError> {
    match value {
        None | Some(JsonValue::Null) => Ok(None),
        Some(JsonValue::String(value)) => Ok(Some(value.clone())),
        Some(_) => Err(validation_error(format!("{field} must be a string."))),
    }
}

fn optional_non_empty_string(
    value: Option<&JsonValue>,
    field: &str,
) -> Result<Option<String>, ValidationError> {
    let Some(value) = optional_string(value, field)? else {
        return Ok(None);
    };
    if value.trim().is_empty() {
        return Err(validation_error(format!("{field} must not be empty.")));
    }
    Ok(Some(value))
}

fn required_object<'a>(
    value: Option<&'a JsonValue>,
    field: &str,
) -> Result<&'a JsonObject, ValidationError> {
    match value {
        Some(JsonValue::Object(value)) => Ok(value),
        None | Some(JsonValue::Null) => Err(validation_error(format!("{field} is required."))),
        Some(_) => Err(validation_error(format!("{field} must be an object."))),
    }
}

fn optional_object(
    value: Option<&JsonValue>,
    field: &str,
) -> Result<Option<JsonObject>, ValidationError> {
    match value {
        None | Some(JsonValue::Null) => Ok(None),
        Some(JsonValue::Object(value)) => Ok(Some(value.clone())),
        Some(_) => Err(validation_error(format!("{field} must be an object."))),
    }
}

fn optional_string_array(
    value: Option<&JsonValue>,
    field: &str,
) -> Result<Option<Vec<String>>, ValidationError> {
    match value {
        None | Some(JsonValue::Null) => Ok(None),
        Some(JsonValue::Array(values)) => values
            .iter()
            .map(|value| match value {
                JsonValue::String(value) => Ok(value.clone()),
                _ => Err(validation_error(format!(
                    "{field} must be an array of strings."
                ))),
            })
            .collect::<Result<Vec<_>, _>>()
            .map(Some),
        Some(_) => Err(validation_error(format!(
            "{field} must be an array of strings."
        ))),
    }
}

fn optional_bool(value: Option<&JsonValue>, field: &str) -> Result<Option<bool>, ValidationError> {
    match value {
        None | Some(JsonValue::Null) => Ok(None),
        Some(JsonValue::Bool(value)) => Ok(Some(*value)),
        Some(_) => Err(validation_error(format!("{field} must be a boolean."))),
    }
}

fn optional_u64(value: Option<&JsonValue>, field: &str) -> Result<Option<u64>, ValidationError> {
    match value {
        None | Some(JsonValue::Null) => Ok(None),
        Some(JsonValue::Number(number)) => {
            let Some(value) = number.as_f64() else {
                return Err(validation_error(format!(
                    "{field} must be a finite number."
                )));
            };
            if value.fract() == 0.0 && value >= 0.0 && value <= u64::MAX as f64 {
                Ok(Some(value as u64))
            } else {
                Err(validation_error(format!(
                    "{field} must be a positive integer."
                )))
            }
        }
        Some(_) => Err(validation_error(format!(
            "{field} must be a finite number."
        ))),
    }
}

fn first_value<'a>(
    left: Option<&'a JsonValue>,
    right: Option<&'a JsonValue>,
) -> Option<&'a JsonValue> {
    match left {
        None | Some(JsonValue::Null) => right,
        Some(value) => Some(value),
    }
}

fn field_value<'a>(object: Option<&'a JsonObject>, field: &str) -> Option<&'a JsonValue> {
    object.and_then(|object| object.get(field))
}

fn nested_value<'a>(value: Option<&'a JsonValue>, field: &str) -> Option<&'a JsonValue> {
    match value {
        Some(JsonValue::Object(object)) => object.get(field),
        _ => None,
    }
}
