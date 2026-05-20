use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[test]
fn doctor_empty_workspace_json_matches_fixture() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = doctor_fixture("empty-success")?;
    let output = runx_command()
        .args(["doctor", "--json"])
        .env("RUNX_CWD", fixture.join("workspace"))
        .output()?;

    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stderr)?, "");
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&output.stdout)?,
        expected_report(&fixture)?
    );
    Ok(())
}

#[test]
fn doctor_failure_json_exits_one_and_matches_fixture() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = doctor_fixture("removed-tool-yaml")?;
    let workspace = fixture.join("workspace");
    let output = runx_command()
        .args(["doctor", workspace.to_str().unwrap_or_default(), "--json"])
        .output()?;

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(String::from_utf8(output.stderr)?, "");
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&output.stdout)?,
        expected_report(&fixture)?
    );
    Ok(())
}

fn runx_command() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_runx"));
    command.env("NO_COLOR", "1");
    command
}

fn expected_report(fixture: &Path) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let expected_json = fs::read_to_string(fixture.join("expected.json"))?;
    Ok(serde_json::from_str(&expected_json)?)
}

fn doctor_fixture(name: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    Ok(repo_root()?.join("fixtures").join("doctor").join(name))
}

fn repo_root() -> Result<PathBuf, Box<dyn std::error::Error>> {
    Ok(Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()?)
}
