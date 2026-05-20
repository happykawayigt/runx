//! Boundary JSON model: deterministic value, object, and number types for cross-language contracts.
use std::collections::BTreeMap;
use std::fmt;

use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub type JsonObject = BTreeMap<String, JsonValue>;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(JsonNumber),
    String(String),
    Array(Vec<JsonValue>),
    Object(JsonObject),
}

/// Strict JSON number representation for public serde boundaries.
///
/// Public serialization rejects non-finite floats. Act assignment idempotency
/// hashing deliberately uses a separate JSON.stringify-compatible writer that
/// hashes non-finite floats as `null`.
#[derive(Clone, Debug, PartialEq)]
pub enum JsonNumber {
    I64(i64),
    U64(u64),
    F64(f64),
}

impl JsonNumber {
    #[must_use]
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::I64(value) => Some(*value as f64),
            Self::U64(value) => Some(*value as f64),
            Self::F64(value) if value.is_finite() => Some(*value),
            Self::F64(_) => None,
        }
    }
}

impl Serialize for JsonNumber {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match *self {
            Self::I64(value) => serializer.serialize_i64(value),
            Self::U64(value) => serializer.serialize_u64(value),
            Self::F64(value) if value.is_finite() && value.fract() == 0.0 => {
                serialize_whole_f64(value, serializer)
            }
            Self::F64(value) if value.is_finite() => serializer.serialize_f64(value),
            Self::F64(_) => Err(serde::ser::Error::custom(
                "non-finite numbers are not valid JSON",
            )),
        }
    }
}

fn serialize_whole_f64<S>(value: f64, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    if value >= i64::MIN as f64 && value <= i64::MAX as f64 {
        serializer.serialize_i64(value as i64)
    } else if value >= 0.0 && value <= u64::MAX as f64 {
        serializer.serialize_u64(value as u64)
    } else {
        serializer.serialize_f64(value)
    }
}

impl<'de> Deserialize<'de> for JsonNumber {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(JsonNumberVisitor)
    }
}

struct JsonNumberVisitor;

impl Visitor<'_> for JsonNumberVisitor {
    type Value = JsonNumber;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a finite JSON number")
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(JsonNumber::I64(value))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(i64::try_from(value).map_or(JsonNumber::U64(value), JsonNumber::I64))
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if value.is_finite() {
            Ok(JsonNumber::F64(value))
        } else {
            Err(E::custom("non-finite numbers are not valid JSON"))
        }
    }
}

impl fmt::Display for JsonNumber {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::I64(value) => write!(formatter, "{value}"),
            Self::U64(value) => write!(formatter, "{value}"),
            Self::F64(value) if value.is_finite() && value == 0.0 => formatter.write_str("0"),
            Self::F64(value) if value.is_finite() && value.fract() == 0.0 => {
                write!(formatter, "{value:.0}")
            }
            Self::F64(value) => write!(formatter, "{value}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{JsonNumber, JsonValue};

    #[test]
    fn json_value_round_trips_objects_with_sorted_keys() -> Result<(), serde_json::Error> {
        let value = JsonValue::Object(
            [
                ("z".to_owned(), JsonValue::String("last".to_owned())),
                ("a".to_owned(), JsonValue::Number(JsonNumber::I64(1))),
            ]
            .into_iter()
            .collect(),
        );

        let json = serde_json::to_string(&value)?;
        let decoded: JsonValue = serde_json::from_str(&json)?;

        assert_eq!(json, r#"{"a":1,"z":"last"}"#);
        assert_eq!(decoded, value);
        Ok(())
    }

    #[test]
    fn json_value_preserves_fractional_numbers() -> Result<(), serde_json::Error> {
        let value = JsonValue::Number(JsonNumber::F64(0.91));

        let json = serde_json::to_string(&value)?;
        let decoded: JsonValue = serde_json::from_str(&json)?;

        assert_eq!(json, "0.91");
        assert_eq!(decoded, value);
        Ok(())
    }

    #[test]
    fn json_number_serializes_whole_floats_as_json_integers() -> Result<(), serde_json::Error> {
        let value = JsonValue::Number(JsonNumber::F64(1.0));

        let json = serde_json::to_string(&value)?;

        assert_eq!(json, "1");
        Ok(())
    }

    #[test]
    fn json_number_rejects_non_finite_float_serialization() {
        let value = JsonValue::Number(JsonNumber::F64(f64::NAN));

        let result = serde_json::to_string(&value);

        assert!(result.is_err());
    }
}
