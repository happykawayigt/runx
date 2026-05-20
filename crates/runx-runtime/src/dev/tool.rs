// rust-style-allow: large-file because deterministic dev tool execution keeps
// manifest parsing, process environment construction, and expectation mapping
// in one fixture-runner boundary.
use std::collections::BTreeMap;
use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use runx_contracts::{JsonObject, JsonValue};
use serde::Deserialize;

use super::r#loop::{
    assert_fixture_expectation, failed_fixture, prepare_fixture_workspace,
    resolve_fixture_execution_roots,
};
use super::types::{
    DevError, DevFixtureAssertion, DevFixtureAssertionKind, DevFixtureExecutionRoots,
    DevFixtureResult, DevFixtureStatus, ParsedDevFixture, PreparedDevFixtureWorkspace,
};

pub(super) fn run_tool_fixture(
    root: &Path,
    fixture: &ParsedDevFixture,
) -> Result<DevFixtureResult, DevError> {
    let started = Instant::now();
    let Some(reference) = string_field(&fixture.target, "ref") else {
        return Ok(missing_tool_ref(fixture, started));
    };
    let Some(tool_dir) = resolve_tool_dir_from_ref(root, reference) else {
        return Ok(unknown_tool_ref(fixture, started, reference));
    };
    let manifest = read_tool_manifest(&tool_dir.join("manifest.json"))?;
    let workspace = prepare_fixture_workspace(root, &fixture.path, &fixture.document)?;
    let result = run_tool_fixture_inner(root, fixture, &tool_dir, &manifest, &workspace, started);
    if let Some(workspace_root) = &workspace.root {
        let _ = fs::remove_dir_all(workspace_root);
    }
    result
}

fn run_tool_fixture_inner(
    root: &Path,
    fixture: &ParsedDevFixture,
    tool_dir: &Path,
    manifest: &RawToolManifest,
    workspace: &PreparedDevFixtureWorkspace,
    started: Instant,
) -> Result<DevFixtureResult, DevError> {
    let Some(execution_roots) =
        resolve_fixture_execution_roots(root, &fixture.lane, workspace.root.as_deref())
    else {
        return Ok(missing_execution_roots(fixture, started));
    };
    let execution = run_process(
        &manifest.command(),
        &manifest.args(),
        tool_dir,
        tool_process_env(fixture, workspace, &execution_roots)?,
    )?;
    Ok(tool_result_from_execution(fixture, started, execution))
}

fn tool_process_env(
    fixture: &ParsedDevFixture,
    workspace: &PreparedDevFixtureWorkspace,
    roots: &DevFixtureExecutionRoots,
) -> Result<BTreeMap<OsString, OsString>, DevError> {
    let fixture_env = materialize_fixture_env(fixture.document.get("env"), &workspace.tokens);
    let inputs = materialize_fixture_value(
        object_field(&fixture.document, "inputs")
            .map(|inputs| JsonValue::Object(inputs.clone()))
            .unwrap_or_else(|| JsonValue::Object(JsonObject::new())),
        &workspace.tokens,
    );
    process_env(&fixture_env, &inputs, roots, workspace.root.as_deref())
}

fn tool_result_from_execution(
    fixture: &ParsedDevFixture,
    started: Instant,
    execution: ProcessResult,
) -> DevFixtureResult {
    let output = parse_json_maybe(&execution.stdout);
    let assertions = assert_fixture_expectation(
        fixture.document.get("expect"),
        execution.exit_code,
        output.as_ref(),
    );
    DevFixtureResult {
        name: fixture.name.clone(),
        lane: fixture.lane.clone(),
        target: fixture.target.clone(),
        status: if assertions.is_empty() {
            DevFixtureStatus::Success
        } else {
            DevFixtureStatus::Failure
        },
        duration_ms: elapsed_ms(started),
        assertions,
        skip_reason: None,
        output,
        replay_path: None,
    }
}

fn missing_tool_ref(fixture: &ParsedDevFixture, started: Instant) -> DevFixtureResult {
    failed_fixture(
        fixture,
        started,
        vec![DevFixtureAssertion {
            path: "target.ref".to_owned(),
            expected: Some(JsonValue::String("existing tool".to_owned())),
            actual: fixture.target.get("ref").cloned(),
            kind: DevFixtureAssertionKind::ExactMismatch,
            message: "Tool reference is required.".to_owned(),
        }],
    )
}

fn unknown_tool_ref(
    fixture: &ParsedDevFixture,
    started: Instant,
    reference: &str,
) -> DevFixtureResult {
    failed_fixture(
        fixture,
        started,
        vec![DevFixtureAssertion {
            path: "target.ref".to_owned(),
            expected: Some(JsonValue::String("existing tool".to_owned())),
            actual: Some(JsonValue::String(reference.to_owned())),
            kind: DevFixtureAssertionKind::ExactMismatch,
            message: format!("Tool {reference} was not found."),
        }],
    )
}

fn missing_execution_roots(fixture: &ParsedDevFixture, started: Instant) -> DevFixtureResult {
    failed_fixture(
        fixture,
        started,
        vec![DevFixtureAssertion {
            path: "repo".to_owned(),
            expected: Some(JsonValue::String("repo or workspace fixture".to_owned())),
            actual: Some(JsonValue::String("missing".to_owned())),
            kind: DevFixtureAssertionKind::ExactMismatch,
            message: "repo-integration fixtures must declare repo or workspace contents."
                .to_owned(),
        }],
    )
}

fn resolve_tool_dir_from_ref(root: &Path, reference: &str) -> Option<PathBuf> {
    let parts = reference.split('.').filter(|part| !part.is_empty());
    let mut candidate = root.join("tools");
    let mut count = 0;
    for part in parts {
        candidate.push(part);
        count += 1;
    }
    (count >= 2 && candidate.join("manifest.json").exists()).then_some(candidate)
}

fn read_tool_manifest(path: &Path) -> Result<RawToolManifest, DevError> {
    let contents = fs::read_to_string(path).map_err(|source| DevError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    serde_json::from_str(&contents).map_err(|source| DevError::Json {
        path: path.to_path_buf(),
        source,
    })
}

fn run_process(
    command: &str,
    args: &[String],
    cwd: &Path,
    envs: BTreeMap<OsString, OsString>,
) -> Result<ProcessResult, DevError> {
    let output = Command::new(command)
        .args(args)
        .current_dir(cwd)
        .env_clear()
        .envs(envs)
        .output()
        .map_err(|source| DevError::Spawn {
            command: command.to_owned(),
            source,
        })?;
    Ok(ProcessResult {
        exit_code: output.status.code().unwrap_or(1),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
    })
}

fn process_env(
    fixture_env: &BTreeMap<String, String>,
    inputs: &JsonValue,
    roots: &DevFixtureExecutionRoots,
    workspace_root: Option<&Path>,
) -> Result<BTreeMap<OsString, OsString>, DevError> {
    let mut envs: BTreeMap<OsString, OsString> = env::vars_os().collect();
    for (key, value) in fixture_env {
        envs.insert(OsString::from(key), OsString::from(value));
    }
    envs.insert(
        OsString::from("RUNX_INPUTS_JSON"),
        OsString::from(
            serde_json::to_string(inputs).map_err(|source| DevError::Json {
                path: PathBuf::from("RUNX_INPUTS_JSON"),
                source,
            })?,
        ),
    );
    envs.insert(
        OsString::from("RUNX_CWD"),
        roots.cwd.as_os_str().to_os_string(),
    );
    envs.insert(
        OsString::from("RUNX_REPO_ROOT"),
        roots.repo_root.as_os_str().to_os_string(),
    );
    if let Some(workspace_root) = workspace_root {
        envs.insert(
            OsString::from("RUNX_FIXTURE_ROOT"),
            workspace_root.as_os_str().to_os_string(),
        );
    }
    Ok(envs)
}

fn materialize_fixture_env(
    value: Option<&JsonValue>,
    tokens: &BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    let Some(JsonValue::Object(object)) = value else {
        return BTreeMap::new();
    };
    object
        .iter()
        .filter_map(|(key, value)| materialize_env_entry(key, value, tokens))
        .collect()
}

fn materialize_env_entry(
    key: &str,
    value: &JsonValue,
    tokens: &BTreeMap<String, String>,
) -> Option<(String, String)> {
    match value {
        JsonValue::Null => None,
        JsonValue::String(value) => {
            Some((key.to_owned(), materialize_fixture_string(value, tokens)))
        }
        other => Some((
            key.to_owned(),
            materialize_fixture_string(&json_display(other), tokens),
        )),
    }
}

pub(super) fn materialize_fixture_value(
    value: JsonValue,
    tokens: &BTreeMap<String, String>,
) -> JsonValue {
    match value {
        JsonValue::String(value) => JsonValue::String(materialize_fixture_string(&value, tokens)),
        JsonValue::Array(values) => JsonValue::Array(
            values
                .into_iter()
                .map(|value| materialize_fixture_value(value, tokens))
                .collect(),
        ),
        JsonValue::Object(object) => JsonValue::Object(
            object
                .into_iter()
                .map(|(key, value)| (key, materialize_fixture_value(value, tokens)))
                .collect(),
        ),
        other => other,
    }
}

pub(super) fn materialize_fixture_string(value: &str, tokens: &BTreeMap<String, String>) -> String {
    let mut resolved = value.to_owned();
    for (key, replacement) in tokens {
        resolved = resolved.replace(&format!("${key}"), replacement);
        resolved = resolved.replace(&format!("${{{key}}}"), replacement);
    }
    resolved
}

fn parse_json_maybe(value: &str) -> Option<JsonValue> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Some(JsonValue::String(String::new()));
    }
    serde_json::from_str(trimmed)
        .ok()
        .or_else(|| Some(JsonValue::String(trimmed.to_owned())))
}

fn string_field<'a>(object: &'a JsonObject, field: &str) -> Option<&'a str> {
    match object.get(field) {
        Some(JsonValue::String(value)) => Some(value),
        _ => None,
    }
}

fn object_field<'a>(object: &'a JsonObject, field: &str) -> Option<&'a JsonObject> {
    match object.get(field) {
        Some(JsonValue::Object(value)) => Some(value),
        _ => None,
    }
}

fn json_display(value: &JsonValue) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "null".to_owned())
}

fn elapsed_ms(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
}

#[derive(Debug, Deserialize)]
struct RawToolManifest {
    #[serde(default)]
    source: Option<RawToolCommand>,
    #[serde(default)]
    runtime: Option<RawToolCommand>,
}

impl RawToolManifest {
    fn command(&self) -> String {
        self.source
            .as_ref()
            .and_then(|source| source.command.clone())
            .or_else(|| {
                self.runtime
                    .as_ref()
                    .and_then(|runtime| runtime.command.clone())
            })
            .unwrap_or_else(|| "node".to_owned())
    }

    fn args(&self) -> Vec<String> {
        self.source
            .as_ref()
            .and_then(|source| (!source.args.is_empty()).then(|| source.args.clone()))
            .or_else(|| {
                self.runtime
                    .as_ref()
                    .and_then(|runtime| (!runtime.args.is_empty()).then(|| runtime.args.clone()))
            })
            .unwrap_or_else(|| vec!["./run.mjs".to_owned()])
    }
}

#[derive(Debug, Deserialize)]
struct RawToolCommand {
    command: Option<String>,
    #[serde(default)]
    args: Vec<String>,
}

struct ProcessResult {
    exit_code: i32,
    stdout: String,
}
