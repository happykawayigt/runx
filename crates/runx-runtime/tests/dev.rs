use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use runx_contracts::{DoctorReport, DoctorReportSchema, DoctorStatus, DoctorSummary, JsonValue};
use runx_runtime::{
    DevFixtureResult, DevFixtureStatus, DevLoopOptions, DevReport, DevReportStatus,
    DevWatchOptions, DevWatchTrigger, discover_fixture_paths, render_dev_result, run_dev_once,
    should_ignore_dev_watch_path,
};

#[test]
fn dev_discovers_direct_unit_fixtures_before_workspace_tool_fixtures()
-> Result<(), Box<dyn std::error::Error>> {
    let root = fixture_root()?;
    let direct = root.join("units/direct");

    let paths = discover_fixture_paths(&direct, &root)?;

    assert_eq!(paths, vec![direct.join("fixtures/direct.yaml")]);
    Ok(())
}

#[test]
fn dev_runs_deterministic_tool_fixtures_and_skips_excluded_lanes()
-> Result<(), Box<dyn std::error::Error>> {
    let root = fixture_root()?;

    let report = run_dev_once(&DevLoopOptions::new(&root))?;

    assert_eq!(report.schema, "runx.dev.v1");
    assert_eq!(report.status, DevReportStatus::Success);
    assert_eq!(report.fixtures.len(), 2);
    assert_eq!(report.fixtures[0].name, "echo-agent");
    assert_eq!(report.fixtures[0].status, DevFixtureStatus::Skipped);
    assert_eq!(
        report.fixtures[0].skip_reason.as_deref(),
        Some("lane agent excluded by --lane deterministic")
    );
    assert_eq!(report.fixtures[1].name, "echo-success");
    assert_eq!(report.fixtures[1].status, DevFixtureStatus::Success);
    assert!(report.fixtures[1].assertions.is_empty());
    assert_eq!(
        nested_string(report.fixtures[1].output.as_ref(), &["message"]),
        Some("hello")
    );
    Ok(())
}

#[test]
fn dev_presentation_matches_terminal_shape() {
    let report = DevReport {
        schema: "runx.dev.v1".to_owned(),
        status: DevReportStatus::Success,
        doctor: DoctorReport {
            schema: DoctorReportSchema::V1,
            status: DoctorStatus::Success,
            summary: DoctorSummary {
                errors: 0,
                warnings: 0,
                infos: 0,
            },
            diagnostics: Vec::new(),
        },
        fixtures: vec![DevFixtureResult {
            name: "echo-success".to_owned(),
            lane: "deterministic".to_owned(),
            target: Default::default(),
            status: DevFixtureStatus::Success,
            duration_ms: 7,
            assertions: Vec::new(),
            skip_reason: None,
            output: None,
            replay_path: None,
        }],
        receipt_id: Some("receipt-dev-1".to_owned()),
    };

    assert_eq!(
        render_dev_result(&report),
        "\n  ok  dev  1 fixture(s)\n  ok  deterministic  echo-success  7ms\n  receipt  receipt-dev-1\n"
    );
}

#[test]
fn dev_watch_ignores_generated_paths_and_debounces_changes()
-> Result<(), Box<dyn std::error::Error>> {
    let root = unique_temp_dir()?;
    fs::create_dir_all(root.join("src"))?;
    fs::write(root.join("src/input.txt"), "one")?;
    assert!(should_ignore_dev_watch_path(
        &root.join("node_modules/pkg/index.js"),
        &[]
    ));
    assert!(!should_ignore_dev_watch_path(
        &root.join("src/input.txt"),
        &[]
    ));

    let mut options = DevWatchOptions::new(&root);
    options.debounce = Duration::from_millis(0);
    let mut watcher = runx_runtime::PollingDevWatcher::new(options)?;
    fs::write(root.join("src/input.txt"), "two")?;

    assert!(watcher.poll()?.is_none());
    let DevWatchTrigger { events } = watcher
        .poll()?
        .ok_or("expected debounced watch trigger after file change")?;
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].path, root.join("src/input.txt"));
    let _ = fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn dev_receipt_metadata_marks_dev_mode_without_secret_material()
-> Result<(), Box<dyn std::error::Error>> {
    let metadata =
        runx_runtime::dev_receipt_metadata("deterministic", Some(Path::new("fixtures/a.yaml")));
    let Some(JsonValue::Object(runx)) = metadata.get("runx") else {
        return Err("metadata.runx should be an object".into());
    };

    assert_eq!(runx.get("dev_mode"), Some(&JsonValue::Bool(true)));
    assert_eq!(
        runx.get("lane"),
        Some(&JsonValue::String("deterministic".to_owned()))
    );
    assert_eq!(
        runx.get("fixture_path"),
        Some(&JsonValue::String("fixtures/a.yaml".to_owned()))
    );
    Ok(())
}

fn fixture_root() -> Result<PathBuf, Box<dyn std::error::Error>> {
    Ok(Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/dev/simple")
        .canonicalize()?)
}

fn nested_string<'a>(value: Option<&'a JsonValue>, path: &[&str]) -> Option<&'a str> {
    let mut current = value?;
    for segment in path {
        let JsonValue::Object(object) = current else {
            return None;
        };
        current = object.get(*segment)?;
    }
    match current {
        JsonValue::String(value) => Some(value),
        _ => None,
    }
}

fn unique_temp_dir() -> Result<PathBuf, std::io::Error> {
    let root = std::env::temp_dir().join(format!(
        "runx-dev-watch-test-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |duration| duration.as_nanos())
    ));
    fs::create_dir_all(&root)?;
    thread::sleep(Duration::from_millis(2));
    Ok(root)
}
