//! Doctor diagnostic report contracts: diagnostics, repairs, summaries.
use std::fmt;
use std::marker::PhantomData;

use serde::de::{self, Visitor};
use serde::{Deserialize, Serialize};

use crate::JsonObject;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DoctorReportSchema {
    #[serde(rename = "runx.doctor.v1")]
    V1,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DoctorStatus {
    Success,
    Failure,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DoctorDiagnosticSeverity {
    Error,
    Warning,
    Info,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DoctorRepairKind {
    CreateFile,
    ReplaceFile,
    EditYaml,
    EditJson,
    AddFixture,
    RunCommand,
    Manual,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DoctorRepairConfidence {
    Low,
    Medium,
    High,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DoctorRepairRisk {
    Low,
    Medium,
    High,
    Sensitive,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DoctorRepair {
    pub id: String,
    pub kind: DoctorRepairKind,
    pub confidence: DoctorRepairConfidence,
    pub risk: DoctorRepairRisk,
    #[serde(
        default,
        deserialize_with = "deserialize_optional_non_null",
        skip_serializing_if = "Option::is_none"
    )]
    pub path: Option<String>,
    #[serde(
        default,
        deserialize_with = "deserialize_optional_non_null",
        skip_serializing_if = "Option::is_none"
    )]
    pub json_pointer: Option<String>,
    #[serde(
        default,
        deserialize_with = "deserialize_optional_non_null",
        skip_serializing_if = "Option::is_none"
    )]
    pub contents: Option<String>,
    #[serde(
        default,
        deserialize_with = "deserialize_optional_non_null",
        skip_serializing_if = "Option::is_none"
    )]
    pub patch: Option<String>,
    #[serde(
        default,
        deserialize_with = "deserialize_optional_non_null",
        skip_serializing_if = "Option::is_none"
    )]
    pub command: Option<String>,
    pub requires_human_review: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DoctorLocation {
    pub path: String,
    #[serde(
        default,
        deserialize_with = "deserialize_optional_non_null",
        skip_serializing_if = "Option::is_none"
    )]
    pub json_pointer: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DoctorDiagnostic {
    pub id: String,
    pub instance_id: String,
    pub severity: DoctorDiagnosticSeverity,
    pub title: String,
    pub message: String,
    pub target: JsonObject,
    pub location: DoctorLocation,
    #[serde(
        default,
        deserialize_with = "deserialize_optional_non_null",
        skip_serializing_if = "Option::is_none"
    )]
    pub evidence: Option<JsonObject>,
    pub repairs: Vec<DoctorRepair>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DoctorSummary {
    pub errors: u64,
    pub warnings: u64,
    pub infos: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DoctorReport {
    pub schema: DoctorReportSchema,
    pub status: DoctorStatus,
    pub summary: DoctorSummary,
    pub diagnostics: Vec<DoctorDiagnostic>,
}

fn deserialize_optional_non_null<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    struct OptionalNonNull<T>(PhantomData<T>);

    impl<'de, T> Visitor<'de> for OptionalNonNull<T>
    where
        T: Deserialize<'de>,
    {
        type Value = Option<T>;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("an omitted field or a non-null value")
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Err(E::custom(
                "optional doctor fields must be omitted instead of null",
            ))
        }

        fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            T::deserialize(deserializer).map(Some)
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Err(E::custom(
                "optional doctor fields must be omitted instead of null",
            ))
        }
    }

    deserializer.deserialize_option(OptionalNonNull(PhantomData))
}
