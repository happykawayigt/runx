use std::collections::BTreeMap;
use std::path::PathBuf;

use runx_contracts::{DoctorReport, JsonObject, JsonValue};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DevLoopOptions {
    pub root: PathBuf,
    pub unit_path: Option<PathBuf>,
    pub lane: DevLane,
}

impl DevLoopOptions {
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            unit_path: None,
            lane: DevLane::Deterministic,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DevLane {
    Deterministic,
    RepoIntegration,
    Agent,
    All,
    Other(String),
}

impl DevLane {
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Deterministic => "deterministic",
            Self::RepoIntegration => "repo-integration",
            Self::Agent => "agent",
            Self::All => "all",
            Self::Other(value) => value,
        }
    }
}

impl From<&str> for DevLane {
    fn from(value: &str) -> Self {
        match value {
            "deterministic" => Self::Deterministic,
            "repo-integration" => Self::RepoIntegration,
            "agent" => Self::Agent,
            "all" => Self::All,
            other => Self::Other(other.to_owned()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DevReportStatus {
    Success,
    Failure,
    Skipped,
    NeedsApproval,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DevFixtureStatus {
    Success,
    Failure,
    Skipped,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DevFixtureAssertionKind {
    SubsetMiss,
    ExactMismatch,
    PacketInvalid,
    StatusMismatch,
    TypeMismatch,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DevFixtureAssertion {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected: Option<JsonValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual: Option<JsonValue>,
    pub kind: DevFixtureAssertionKind,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DevFixtureResult {
    pub name: String,
    pub lane: String,
    pub target: JsonObject,
    pub status: DevFixtureStatus,
    pub duration_ms: u64,
    pub assertions: Vec<DevFixtureAssertion>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<JsonValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replay_path: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DevReport {
    pub schema: String,
    pub status: DevReportStatus,
    pub doctor: DoctorReport,
    pub fixtures: Vec<DevFixtureResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receipt_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ParsedDevFixture {
    pub path: PathBuf,
    pub name: String,
    pub lane: String,
    pub target: JsonObject,
    pub document: JsonObject,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DevFixtureExecutionRoots {
    pub cwd: PathBuf,
    pub repo_root: PathBuf,
}

#[derive(Clone, Debug)]
pub struct PreparedDevFixtureWorkspace {
    pub root: Option<PathBuf>,
    pub tokens: BTreeMap<String, String>,
}

#[derive(Debug, Error)]
pub enum DevError {
    #[error("failed to read dev fixture {path}: {source}")]
    ReadFixture {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse dev fixture {path}: {source}")]
    ParseFixture {
        path: PathBuf,
        #[source]
        source: serde_norway::Error,
    },
    #[error("failed to read {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse JSON at {path}: {source}")]
    Json {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("dev fixture workspace path must be relative: {path}")]
    AbsoluteWorkspacePath { path: String },
    #[error("dev fixture workspace path escapes root: {path}")]
    EscapingWorkspacePath { path: String },
    #[error("failed to run fixture command {command}: {source}")]
    Spawn {
        command: String,
        #[source]
        source: std::io::Error,
    },
    #[error(transparent)]
    Runtime(#[from] crate::RuntimeError),
}

pub trait DevFixtureExecutor {
    fn run_fixture(
        &self,
        root: &std::path::Path,
        fixture: &ParsedDevFixture,
    ) -> Result<DevFixtureResult, DevError>;
}

#[derive(Clone, Debug, Default)]
pub struct LocalDevFixtureExecutor;
