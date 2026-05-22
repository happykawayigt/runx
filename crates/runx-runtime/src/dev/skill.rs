// rust-style-allow: large-file because native dev skill/graph replay keeps
// fixture preparation, expectation projection, and harness invocation together
// until the CLI watch cutover creates a stable module boundary.
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use runx_contracts::{ClosureDisposition, JsonNumber, JsonObject, JsonValue};

use super::r#loop::{
    assert_fixture_expectation, failed_fixture, prepare_fixture_workspace,
    resolve_fixture_execution_roots,
};
use super::tool::materialize_fixture_value;
use super::types::{
    DevError, DevFixtureAssertion, DevFixtureAssertionKind, DevFixtureResult, DevFixtureStatus,
    ParsedDevFixture, PreparedDevFixtureWorkspace,
};
use crate::harness::{HarnessExpectedStatus, HarnessReplayOutput, run_harness_fixture};

pub(super) fn run_skill_or_graph_fixture(
    root: &Path,
    fixture: &ParsedDevFixture,
) -> Result<DevFixtureResult, DevError> {
    let started = Instant::now();
    let Some(kind) = string_field(&fixture.target, "kind") else {
        return Ok(missing_target_kind(fixture, started));
    };
    let Some(reference) = string_field(&fixture.target, "ref") else {
        return Ok(missing_target_ref(fixture, started, kind));
    };
    let Some(target_path) = resolve_target_path(root, kind, reference) else {
        return Ok(unknown_target_ref(fixture, started, kind, reference));
    };
    let workspace = prepare_fixture_workspace(root, &fixture.path, &fixture.document)?;
    let result =
        run_skill_or_graph_fixture_inner(root, fixture, kind, &target_path, &workspace, started);
    if let Some(workspace_root) = &workspace.root {
        let _ = fs::remove_dir_all(workspace_root);
    }
    result
}

fn run_skill_or_graph_fixture_inner(
    root: &Path,
    fixture: &ParsedDevFixture,
    kind: &str,
    target_path: &Path,
    workspace: &PreparedDevFixtureWorkspace,
    started: Instant,
) -> Result<DevFixtureResult, DevError> {
    let Some(execution_roots) =
        resolve_fixture_execution_roots(root, &fixture.lane, workspace.root.as_deref())
    else {
        return Ok(missing_execution_roots(fixture, started));
    };
    let harness_fixture_path =
        write_harness_replay_fixture(fixture, kind, target_path, workspace, &execution_roots)?;
    let output = run_harness_fixture(&harness_fixture_path);
    let _ = fs::remove_file(&harness_fixture_path);
    if let Some(parent) = harness_fixture_path.parent() {
        let _ = fs::remove_dir(parent);
    }
    match output {
        Ok(output) => Ok(result_from_harness_output(fixture, started, output)),
        Err(error) => Ok(failed_fixture(
            fixture,
            started,
            vec![DevFixtureAssertion {
                path: "target.ref".to_owned(),
                expected: Some(JsonValue::String("native harness replay".to_owned())),
                actual: Some(JsonValue::String(error.to_string())),
                kind: DevFixtureAssertionKind::ExactMismatch,
                message: "Native skill or graph dev fixture execution failed.".to_owned(),
            }],
        )),
    }
}

fn write_harness_replay_fixture(
    fixture: &ParsedDevFixture,
    kind: &str,
    target_path: &Path,
    workspace: &PreparedDevFixtureWorkspace,
    roots: &super::types::DevFixtureExecutionRoots,
) -> Result<PathBuf, DevError> {
    let mut harness = JsonObject::new();
    harness.insert("name".to_owned(), JsonValue::String(fixture.name.clone()));
    harness.insert("kind".to_owned(), JsonValue::String(kind.to_owned()));
    harness.insert(
        "target".to_owned(),
        JsonValue::String(target_path.to_string_lossy().into_owned()),
    );
    harness.insert(
        "inputs".to_owned(),
        materialize_fixture_value(
            object_field(&fixture.document, "inputs")
                .map(|inputs| JsonValue::Object(inputs.clone()))
                .unwrap_or_else(|| JsonValue::Object(JsonObject::new())),
            &workspace.tokens,
        ),
    );
    let env = fixture_env(fixture, workspace, roots);
    if !env.is_empty() {
        harness.insert("env".to_owned(), JsonValue::Object(env));
    }
    if let Some(caller) = object_field(&fixture.document, "caller") {
        harness.insert("caller".to_owned(), JsonValue::Object(caller.clone()));
    }
    let path = unique_harness_fixture_path()?;
    let contents = serde_json::to_string_pretty(&JsonValue::Object(harness)).map_err(|source| {
        DevError::Json {
            path: path.clone(),
            source,
        }
    })?;
    fs::write(&path, format!("{contents}\n")).map_err(|source| DevError::Io {
        path: path.clone(),
        source,
    })?;
    Ok(path)
}

fn fixture_env(
    fixture: &ParsedDevFixture,
    workspace: &PreparedDevFixtureWorkspace,
    roots: &super::types::DevFixtureExecutionRoots,
) -> JsonObject {
    let mut env = JsonObject::new();
    for (key, value) in materialized_string_map(fixture.document.get("env"), &workspace.tokens) {
        env.insert(key, JsonValue::String(value));
    }
    env.insert(
        "RUNX_CWD".to_owned(),
        JsonValue::String(roots.cwd.to_string_lossy().into_owned()),
    );
    env.insert(
        "RUNX_REPO_ROOT".to_owned(),
        JsonValue::String(roots.repo_root.to_string_lossy().into_owned()),
    );
    if let Some(workspace_root) = &workspace.root {
        env.insert(
            "RUNX_FIXTURE_ROOT".to_owned(),
            JsonValue::String(workspace_root.to_string_lossy().into_owned()),
        );
    }
    env
}

fn materialized_string_map(
    value: Option<&JsonValue>,
    tokens: &BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    let Some(JsonValue::Object(object)) = value else {
        return BTreeMap::new();
    };
    object
        .iter()
        .filter_map(|(key, value)| materialized_string_entry(key, value, tokens))
        .collect()
}

fn materialized_string_entry(
    key: &str,
    value: &JsonValue,
    tokens: &BTreeMap<String, String>,
) -> Option<(String, String)> {
    match materialize_fixture_value(value.clone(), tokens) {
        JsonValue::Null => None,
        JsonValue::String(value) => Some((key.to_owned(), value)),
        other => Some((
            key.to_owned(),
            serde_json::to_string(&other).unwrap_or_else(|_| "null".to_owned()),
        )),
    }
}

fn result_from_harness_output(
    fixture: &ParsedDevFixture,
    started: Instant,
    output: HarnessReplayOutput,
) -> DevFixtureResult {
    let fixture_output = dev_output_from_harness(&output);
    let exit_code = if output.status == HarnessExpectedStatus::Sealed {
        0
    } else {
        1
    };
    let assertions = assert_fixture_expectation(
        fixture.document.get("expect"),
        exit_code,
        Some(&fixture_output),
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
        output: Some(fixture_output),
        replay_path: None,
    }
}

fn dev_output_from_harness(output: &HarnessReplayOutput) -> JsonValue {
    if let Some(skill_output) = &output.skill_output {
        return parse_json_maybe(&skill_output.stdout);
    }
    let mut object = JsonObject::new();
    object.insert(
        "receipt_id".to_owned(),
        JsonValue::String(output.receipt.id.clone()),
    );
    object.insert(
        "harness_id".to_owned(),
        JsonValue::String(output.receipt.subject.reference.uri.clone()),
    );
    object.insert(
        "status".to_owned(),
        JsonValue::String(harness_status(&output.status).to_owned()),
    );
    object.insert(
        "disposition".to_owned(),
        JsonValue::String(disposition_name(&output.receipt.seal.disposition).to_owned()),
    );
    object.insert(
        "step_count".to_owned(),
        JsonValue::Number(JsonNumber::I64(
            i64::try_from(output.step_receipts.len()).unwrap_or(i64::MAX),
        )),
    );
    object.insert(
        "step_receipt_ids".to_owned(),
        JsonValue::Array(
            output
                .step_receipts
                .iter()
                .map(|receipt| JsonValue::String(receipt.id.clone()))
                .collect(),
        ),
    );
    JsonValue::Object(object)
}

fn resolve_target_path(root: &Path, kind: &str, reference: &str) -> Option<PathBuf> {
    match kind {
        "skill" => resolve_skill_dir_from_ref(root, reference),
        "graph" => resolve_graph_path_from_ref(root, reference),
        _ => None,
    }
}

fn resolve_skill_dir_from_ref(root: &Path, reference: &str) -> Option<PathBuf> {
    let candidates = [root.join("skills").join(reference), root.join(reference)];
    candidates
        .into_iter()
        .find(|candidate| candidate.join("SKILL.md").exists())
        .and_then(|candidate| fs::canonicalize(candidate).ok())
}

fn resolve_graph_path_from_ref(root: &Path, reference: &str) -> Option<PathBuf> {
    let reference_path = Path::new(reference);
    let mut candidates = vec![root.join(reference_path)];
    if reference_path.extension().is_none() {
        candidates.push(root.join("graphs").join(format!("{reference}.yaml")));
        candidates.push(root.join("graphs").join(reference).join("graph.yaml"));
    }
    candidates
        .into_iter()
        .find(|candidate| candidate.is_file())
        .and_then(|candidate| fs::canonicalize(candidate).ok())
}

fn missing_target_kind(fixture: &ParsedDevFixture, started: Instant) -> DevFixtureResult {
    failed_fixture(
        fixture,
        started,
        vec![DevFixtureAssertion {
            path: "target.kind".to_owned(),
            expected: Some(JsonValue::String("skill | graph".to_owned())),
            actual: fixture.target.get("kind").cloned(),
            kind: DevFixtureAssertionKind::ExactMismatch,
            message: "Native fixture target.kind must be skill or graph.".to_owned(),
        }],
    )
}

fn missing_target_ref(
    fixture: &ParsedDevFixture,
    started: Instant,
    kind: &str,
) -> DevFixtureResult {
    failed_fixture(
        fixture,
        started,
        vec![DevFixtureAssertion {
            path: "target.ref".to_owned(),
            expected: Some(JsonValue::String(format!("existing {kind}"))),
            actual: fixture.target.get("ref").cloned(),
            kind: DevFixtureAssertionKind::ExactMismatch,
            message: format!("{kind} reference is required."),
        }],
    )
}

fn unknown_target_ref(
    fixture: &ParsedDevFixture,
    started: Instant,
    kind: &str,
    reference: &str,
) -> DevFixtureResult {
    failed_fixture(
        fixture,
        started,
        vec![DevFixtureAssertion {
            path: "target.ref".to_owned(),
            expected: Some(JsonValue::String(format!("existing {kind}"))),
            actual: Some(JsonValue::String(reference.to_owned())),
            kind: DevFixtureAssertionKind::ExactMismatch,
            message: format!("{kind} {reference} was not found."),
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

fn parse_json_maybe(value: &str) -> JsonValue {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return JsonValue::String(String::new());
    }
    serde_json::from_str(trimmed).unwrap_or_else(|_| JsonValue::String(trimmed.to_owned()))
}

fn unique_harness_fixture_path() -> Result<PathBuf, DevError> {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    let directory =
        std::env::temp_dir().join(format!("runx-dev-harness-{}-{nanos}", std::process::id()));
    fs::create_dir_all(&directory).map_err(|source| DevError::Io {
        path: directory.clone(),
        source,
    })?;
    Ok(directory.join("fixture.yaml"))
}

fn harness_status(status: &HarnessExpectedStatus) -> &'static str {
    match status {
        HarnessExpectedStatus::Sealed => "sealed",
        HarnessExpectedStatus::Failure => "failure",
        HarnessExpectedStatus::NeedsAgent => "needs_agent",
        HarnessExpectedStatus::PolicyDenied => "policy_denied",
        HarnessExpectedStatus::Escalated => "escalated",
    }
}

fn disposition_name(disposition: &ClosureDisposition) -> &'static str {
    match disposition {
        ClosureDisposition::Closed => "closed",
        ClosureDisposition::Deferred => "deferred",
        ClosureDisposition::Superseded => "superseded",
        ClosureDisposition::Declined => "declined",
        ClosureDisposition::Blocked => "blocked",
        ClosureDisposition::Failed => "failed",
        ClosureDisposition::Killed => "killed",
        ClosureDisposition::TimedOut => "timed_out",
    }
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

fn elapsed_ms(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
}
