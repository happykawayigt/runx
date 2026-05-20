// rust-style-allow: large-file because this first dev-mode slice keeps fixture
// discovery, workspace materialization, and result projection together until
// native skill/graph dev execution creates the next durable module boundary.
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use runx_contracts::{DoctorStatus, JsonObject, JsonValue};

use super::tool::{materialize_fixture_string, materialize_fixture_value, run_tool_fixture};
use super::types::{
    DevError, DevFixtureAssertion, DevFixtureAssertionKind, DevFixtureExecutionRoots,
    DevFixtureExecutor, DevFixtureResult, DevFixtureStatus, DevLoopOptions, DevReport,
    DevReportStatus, LocalDevFixtureExecutor, ParsedDevFixture, PreparedDevFixtureWorkspace,
};
use crate::doctor::{default_doctor_options, run_doctor};

pub fn run_dev_once(options: &DevLoopOptions) -> Result<DevReport, DevError> {
    run_dev_once_with_executor(options, &LocalDevFixtureExecutor)
}

pub fn run_dev_once_with_executor(
    options: &DevLoopOptions,
    executor: &impl DevFixtureExecutor,
) -> Result<DevReport, DevError> {
    let root = normalize_path(&options.root);
    let doctor = run_doctor(&root, &default_doctor_options())?;
    if doctor.status == DoctorStatus::Failure {
        return Ok(DevReport {
            schema: "runx.dev.v1".to_owned(),
            status: DevReportStatus::Failure,
            doctor,
            fixtures: Vec::new(),
            receipt_id: None,
        });
    }

    let fixture_paths = discover_fixture_paths(
        options.unit_path.as_deref().unwrap_or(root.as_path()),
        &root,
    )?;
    let mut fixtures = Vec::new();
    for fixture_path in fixture_paths {
        let parsed = parse_dev_fixture_file(&fixture_path)?;
        fixtures.push(run_or_skip_fixture(
            &root,
            &parsed,
            options.lane.as_str(),
            executor,
        )?);
    }

    Ok(DevReport {
        schema: "runx.dev.v1".to_owned(),
        status: report_status(&fixtures),
        doctor,
        fixtures,
        receipt_id: None,
    })
}

pub fn discover_fixture_paths(unit_path: &Path, root: &Path) -> Result<Vec<PathBuf>, DevError> {
    let stat_path = if unit_path.exists() { unit_path } else { root };
    let mut paths = yaml_files_in(&stat_path.join("fixtures"))?;
    if !paths.is_empty() && stat_path != root {
        paths.sort();
        return Ok(paths);
    }
    for tool_dir in discover_tool_directories(root)? {
        paths.extend(yaml_files_in(&tool_dir.join("fixtures"))?);
    }
    paths.sort();
    Ok(paths)
}

#[must_use]
pub fn dev_receipt_metadata(lane: &str, fixture_path: Option<&Path>) -> JsonObject {
    let mut dev = JsonObject::new();
    dev.insert("mode".to_owned(), JsonValue::String("dev".to_owned()));
    dev.insert("dev_mode".to_owned(), JsonValue::Bool(true));
    dev.insert("lane".to_owned(), JsonValue::String(lane.to_owned()));
    if let Some(path) = fixture_path {
        dev.insert(
            "fixture_path".to_owned(),
            JsonValue::String(path.to_string_lossy().into_owned()),
        );
    }
    let mut metadata = JsonObject::new();
    metadata.insert("runx".to_owned(), JsonValue::Object(dev));
    metadata
}

impl DevFixtureExecutor for LocalDevFixtureExecutor {
    fn run_fixture(
        &self,
        root: &Path,
        fixture: &ParsedDevFixture,
    ) -> Result<DevFixtureResult, DevError> {
        match string_field(&fixture.target, "kind") {
            Some("tool") => run_tool_fixture(root, fixture),
            Some("skill") | Some("graph") => Ok(failed_fixture(
                fixture,
                Instant::now(),
                vec![DevFixtureAssertion {
                    path: "target.kind".to_owned(),
                    expected: Some(JsonValue::String("native skill/graph dev executor".to_owned())),
                    actual: string_field(&fixture.target, "kind")
                        .map(|value| JsonValue::String(value.to_owned())),
                    kind: DevFixtureAssertionKind::ExactMismatch,
                    message: "Native skill and graph dev fixture execution is not wired in this runtime slice.".to_owned(),
                }],
            )),
            Some(_) | None => Ok(failed_fixture(
                fixture,
                Instant::now(),
                vec![DevFixtureAssertion {
                    path: "target.kind".to_owned(),
                    expected: Some(JsonValue::String("tool | skill | graph".to_owned())),
                    actual: fixture.target.get("kind").cloned(),
                    kind: DevFixtureAssertionKind::ExactMismatch,
                    message: "Fixture target.kind must be tool, skill, or graph.".to_owned(),
                }],
            )),
        }
    }
}

fn run_or_skip_fixture(
    root: &Path,
    fixture: &ParsedDevFixture,
    selected_lane: &str,
    executor: &impl DevFixtureExecutor,
) -> Result<DevFixtureResult, DevError> {
    let started = Instant::now();
    if selected_lane != "all" && fixture.lane != selected_lane {
        return Ok(DevFixtureResult {
            name: fixture.name.clone(),
            lane: fixture.lane.clone(),
            target: fixture.target.clone(),
            status: DevFixtureStatus::Skipped,
            duration_ms: elapsed_ms(started),
            assertions: Vec::new(),
            skip_reason: Some(format!(
                "lane {} excluded by --lane {}",
                fixture.lane, selected_lane
            )),
            output: None,
            replay_path: None,
        });
    }
    if fixture.lane != "deterministic" && fixture.lane != "repo-integration" {
        return Ok(DevFixtureResult {
            name: fixture.name.clone(),
            lane: fixture.lane.clone(),
            target: fixture.target.clone(),
            status: DevFixtureStatus::Skipped,
            duration_ms: elapsed_ms(started),
            assertions: Vec::new(),
            skip_reason: Some(format!(
                "{} fixtures are parsed but not executed in dev v1",
                fixture.lane
            )),
            output: None,
            replay_path: None,
        });
    }
    executor.run_fixture(root, fixture)
}

fn parse_dev_fixture_file(path: &Path) -> Result<ParsedDevFixture, DevError> {
    let contents = fs::read_to_string(path).map_err(|source| DevError::ReadFixture {
        path: path.to_path_buf(),
        source,
    })?;
    let document: JsonValue =
        serde_norway::from_str(&contents).map_err(|source| DevError::ParseFixture {
            path: path.to_path_buf(),
            source,
        })?;
    let JsonValue::Object(document) = document else {
        return Ok(ParsedDevFixture {
            path: path.to_path_buf(),
            name: path_stem(path),
            lane: "unknown".to_owned(),
            target: JsonObject::new(),
            document: JsonObject::new(),
        });
    };
    let name = string_field(&document, "name")
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| path_stem(path));
    let lane = string_field(&document, "lane")
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| "deterministic".to_owned());
    let target = object_field(&document, "target")
        .cloned()
        .unwrap_or_default();
    Ok(ParsedDevFixture {
        path: path.to_path_buf(),
        name,
        lane,
        target,
        document,
    })
}

pub(super) fn prepare_fixture_workspace(
    root: &Path,
    fixture_path: &Path,
    fixture: &JsonObject,
) -> Result<PreparedDevFixtureWorkspace, DevError> {
    let fixture_dir = fixture_path.parent().unwrap_or(root);
    let mut tokens = BTreeMap::from([
        (
            "RUNX_REPO_ROOT".to_owned(),
            root.to_string_lossy().into_owned(),
        ),
        (
            "RUNX_FIXTURE_FILE".to_owned(),
            fixture_path.to_string_lossy().into_owned(),
        ),
        (
            "RUNX_FIXTURE_DIR".to_owned(),
            fixture_dir.to_string_lossy().into_owned(),
        ),
    ]);
    let workspace = object_field(fixture, "workspace").or_else(|| object_field(fixture, "repo"));
    let Some(workspace) = workspace else {
        return Ok(PreparedDevFixtureWorkspace { root: None, tokens });
    };
    let fixture_root = unique_temp_dir()?;
    tokens.insert(
        "RUNX_FIXTURE_ROOT".to_owned(),
        fixture_root.to_string_lossy().into_owned(),
    );
    write_fixture_file_map(
        &fixture_root,
        object_field(workspace, "files"),
        &tokens,
        false,
    )?;
    write_fixture_file_map(
        &fixture_root,
        object_field(workspace, "json_files"),
        &tokens,
        true,
    )?;
    write_fixture_file_map(
        &fixture_root,
        object_field(workspace, "executable_files"),
        &tokens,
        false,
    )?;
    Ok(PreparedDevFixtureWorkspace {
        root: Some(fixture_root),
        tokens,
    })
}

fn write_fixture_file_map(
    root: &Path,
    files: Option<&JsonObject>,
    tokens: &BTreeMap<String, String>,
    force_json: bool,
) -> Result<(), DevError> {
    let Some(files) = files else {
        return Ok(());
    };
    for (relative_path, raw_contents) in files {
        let target_path = resolve_inside_fixture_root(root, relative_path)?;
        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent).map_err(|source| DevError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        let contents = if force_json {
            format!(
                "{}\n",
                serde_json::to_string_pretty(&materialize_fixture_value(
                    raw_contents.clone(),
                    tokens
                ))
                .map_err(|source| DevError::Json {
                    path: target_path.clone(),
                    source,
                })?
            )
        } else if let JsonValue::String(value) = raw_contents {
            materialize_fixture_string(value, tokens)
        } else {
            format!(
                "{}\n",
                serde_json::to_string_pretty(&materialize_fixture_value(
                    raw_contents.clone(),
                    tokens
                ))
                .map_err(|source| DevError::Json {
                    path: target_path.clone(),
                    source,
                })?
            )
        };
        fs::write(&target_path, contents).map_err(|source| DevError::Io {
            path: target_path,
            source,
        })?;
    }
    Ok(())
}

fn resolve_inside_fixture_root(root: &Path, relative_path: &str) -> Result<PathBuf, DevError> {
    let relative = Path::new(relative_path);
    if relative.is_absolute() {
        return Err(DevError::AbsoluteWorkspacePath {
            path: relative_path.to_owned(),
        });
    }
    let resolved = normalize_path(&root.join(relative));
    if !resolved.starts_with(root) {
        return Err(DevError::EscapingWorkspacePath {
            path: relative_path.to_owned(),
        });
    }
    Ok(resolved)
}

pub(super) fn resolve_fixture_execution_roots(
    root: &Path,
    lane: &str,
    workspace_root: Option<&Path>,
) -> Option<DevFixtureExecutionRoots> {
    if lane == "repo-integration" {
        let workspace_root = workspace_root?;
        return Some(DevFixtureExecutionRoots {
            cwd: workspace_root.to_path_buf(),
            repo_root: workspace_root.to_path_buf(),
        });
    }
    Some(DevFixtureExecutionRoots {
        cwd: workspace_root.unwrap_or(root).to_path_buf(),
        repo_root: root.to_path_buf(),
    })
}

pub(super) fn assert_fixture_expectation(
    expectation: Option<&JsonValue>,
    exit_code: i32,
    output: Option<&JsonValue>,
) -> Vec<DevFixtureAssertion> {
    let mut assertions = Vec::new();
    let expect = match expectation {
        Some(JsonValue::Object(value)) => value,
        _ => return assertions,
    };
    let expected_status = string_field(expect, "status").unwrap_or("success");
    let actual_status = if exit_code == 0 { "success" } else { "failure" };
    if expected_status != actual_status {
        assertions.push(DevFixtureAssertion {
            path: "expect.status".to_owned(),
            expected: Some(JsonValue::String(expected_status.to_owned())),
            actual: Some(JsonValue::String(actual_status.to_owned())),
            kind: DevFixtureAssertionKind::StatusMismatch,
            message: format!("Expected status {expected_status}, got {actual_status}."),
        });
    }
    if let Some(output_expectation) = object_field(expect, "output") {
        assertions.extend(assert_output_expectation(
            output_expectation,
            output.unwrap_or(&JsonValue::String(String::new())),
            "expect.output",
        ));
    }
    assertions
}

fn assert_output_expectation(
    expectation: &JsonObject,
    output: &JsonValue,
    base_path: &str,
) -> Vec<DevFixtureAssertion> {
    let mut assertions = Vec::new();
    if let Some(exact) = expectation.get("exact") {
        if output != exact {
            assertions.push(DevFixtureAssertion {
                path: format!("{base_path}.exact"),
                expected: Some(exact.clone()),
                actual: Some(output.clone()),
                kind: DevFixtureAssertionKind::ExactMismatch,
                message: "Output did not exactly match.".to_owned(),
            });
        }
    }
    if let Some(subset) = expectation.get("subset") {
        assertions.extend(assert_subset(subset, output, ""));
    }
    assertions
}

fn assert_subset(
    expected: &JsonValue,
    actual: &JsonValue,
    base_path: &str,
) -> Vec<DevFixtureAssertion> {
    let JsonValue::Object(expected_object) = expected else {
        return if expected == actual {
            Vec::new()
        } else {
            vec![DevFixtureAssertion {
                path: base_path.to_owned(),
                expected: Some(expected.clone()),
                actual: Some(actual.clone()),
                kind: DevFixtureAssertionKind::SubsetMiss,
                message: "Subset value did not match.".to_owned(),
            }]
        };
    };
    let mut assertions = Vec::new();
    for (key, value) in expected_object {
        let path = if base_path.is_empty() {
            key.clone()
        } else {
            format!("{base_path}.{key}")
        };
        let actual_value = match actual {
            JsonValue::Object(object) => object.get(key).unwrap_or(&JsonValue::Null),
            _ => &JsonValue::Null,
        };
        assertions.extend(assert_subset(value, actual_value, &path));
    }
    assertions
}

pub(super) fn failed_fixture(
    fixture: &ParsedDevFixture,
    started: Instant,
    assertions: Vec<DevFixtureAssertion>,
) -> DevFixtureResult {
    DevFixtureResult {
        name: fixture.name.clone(),
        lane: fixture.lane.clone(),
        target: fixture.target.clone(),
        status: DevFixtureStatus::Failure,
        duration_ms: elapsed_ms(started),
        assertions,
        skip_reason: None,
        output: None,
        replay_path: None,
    }
}

fn report_status(fixtures: &[DevFixtureResult]) -> DevReportStatus {
    if fixtures
        .iter()
        .any(|fixture| fixture.status == DevFixtureStatus::Failure)
    {
        DevReportStatus::Failure
    } else if fixtures
        .iter()
        .any(|fixture| fixture.status == DevFixtureStatus::Success)
    {
        DevReportStatus::Success
    } else {
        DevReportStatus::Skipped
    }
}

fn discover_tool_directories(root: &Path) -> Result<Vec<PathBuf>, DevError> {
    let tools_root = root.join("tools");
    let mut directories = Vec::new();
    for namespace in safe_read_dir(&tools_root)? {
        let namespace_path = namespace.path();
        if !namespace_path.is_dir() {
            continue;
        }
        for tool in safe_read_dir(&namespace_path)? {
            let tool_path = tool.path();
            if tool_path.is_dir() {
                directories.push(tool_path);
            }
        }
    }
    directories.sort();
    Ok(directories)
}

fn yaml_files_in(directory: &Path) -> Result<Vec<PathBuf>, DevError> {
    Ok(safe_read_dir(directory)?
        .into_iter()
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && is_yaml_file(path))
        .collect())
}

fn safe_read_dir(directory: &Path) -> Result<Vec<fs::DirEntry>, DevError> {
    match fs::read_dir(directory) {
        Ok(entries) => entries
            .collect::<Result<Vec<_>, _>>()
            .map_err(|source| DevError::Io {
                path: directory.to_path_buf(),
                source,
            }),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(source) => Err(DevError::Io {
            path: directory.to_path_buf(),
            source,
        }),
    }
}

fn unique_temp_dir() -> Result<PathBuf, DevError> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    let path = env::temp_dir().join(format!("runx-fixture-{}-{nanos}", std::process::id()));
    fs::create_dir_all(&path).map_err(|source| DevError::Io {
        path: path.clone(),
        source,
    })?;
    Ok(path)
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    normalized
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

fn path_stem(path: &Path) -> String {
    path.file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("fixture")
        .to_owned()
}

fn is_yaml_file(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .is_some_and(|extension| {
            extension.eq_ignore_ascii_case("yaml") || extension.eq_ignore_ascii_case("yml")
        })
}

fn elapsed_ms(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
}
