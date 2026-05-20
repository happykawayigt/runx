use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn native_skill_pauses_and_resumes_with_run_id() -> Result<(), Box<dyn std::error::Error>> {
    let root = temp_root("runx-skill");
    let skill_dir = write_agent_step_skill(&root)?;
    let receipt_dir = root.join("receipts");

    let pause = runx_command()
        .args([
            "skill",
            skill_dir.to_str().ok_or("non-utf8 skill dir")?,
            "--receipt-dir",
            receipt_dir.to_str().ok_or("non-utf8 receipt dir")?,
            "--json",
            "--non-interactive",
            "--thread-title",
            "Docs bug",
        ])
        .output()?;
    let pause_json = assert_json(&pause, Some(2))?;
    assert_eq!(pause_json["status"], "needs_agent");
    assert_eq!(pause_json["run_id"], "run_agent_step-issue-intake-output");
    assert_eq!(pause_json["requests"][0]["kind"], "agent_act");

    let answers_path = root.join("answers.json");
    fs::write(
        &answers_path,
        serde_json::json!({
            "answers": {
                "agent_step.issue-intake.output": {
                    "intake_report": {
                        "summary": "Docs bug is bounded."
                    }
                }
            }
        })
        .to_string(),
    )?;

    let resume = runx_command()
        .args([
            "skill",
            skill_dir.to_str().ok_or("non-utf8 skill dir")?,
            "--receipt-dir",
            receipt_dir.to_str().ok_or("non-utf8 receipt dir")?,
            "--run-id",
            "issue-intake-run",
            "--answers",
            answers_path.to_str().ok_or("non-utf8 answers path")?,
            "--json",
            "--non-interactive",
        ])
        .output()?;
    let resume_json = assert_json(&resume, Some(0))?;
    assert_eq!(resume_json["status"], "sealed");
    assert_eq!(resume_json["run_id"], "issue-intake-run");
    assert_eq!(resume_json["closure"]["disposition"], "closed");
    assert_eq!(resume_json["receipt"]["schema"], "runx.harness_receipt.v1");
    let receipt_id = resume_json["receipt_id"]
        .as_str()
        .ok_or("missing receipt_id")?;
    assert!(receipt_dir.join(format!("{receipt_id}.json")).exists());

    Ok(())
}

#[test]
fn native_skill_rejects_answers_without_run_id() -> Result<(), Box<dyn std::error::Error>> {
    let root = temp_root("runx-skill-reject-answers");
    let skill_dir = write_agent_step_skill(&root)?;
    let answers_path = root.join("answers.json");
    fs::write(&answers_path, "{}")?;
    let output = runx_command()
        .args([
            "skill",
            skill_dir.to_str().ok_or("non-utf8 skill dir")?,
            "--answers",
            answers_path.to_str().ok_or("non-utf8 answers path")?,
        ])
        .output()?;

    assert_eq!(output.status.code(), Some(64));
    assert!(String::from_utf8(output.stderr)?.contains("runx skill --answers requires --run-id"));
    assert_eq!(String::from_utf8(output.stdout)?, "");

    Ok(())
}

#[test]
fn native_skill_rejects_run_id_without_answers() -> Result<(), Box<dyn std::error::Error>> {
    let root = temp_root("runx-skill-reject-run-id");
    let skill_dir = write_agent_step_skill(&root)?;
    let output = runx_command()
        .args([
            "skill",
            skill_dir.to_str().ok_or("non-utf8 skill dir")?,
            "--run-id",
            "issue-intake-run",
        ])
        .output()?;

    assert_eq!(output.status.code(), Some(64));
    assert!(String::from_utf8(output.stderr)?.contains("runx skill --run-id requires --answers"));
    assert_eq!(String::from_utf8(output.stdout)?, "");

    Ok(())
}

#[test]
fn native_skill_rejects_retired_receipt_options() -> Result<(), Box<dyn std::error::Error>> {
    let root = temp_root("runx-skill-reject-retired-receipt");
    let skill_dir = write_agent_step_skill(&root)?;
    let receipt_dir = root.join("receipts");
    let retired_receipt = format!("--{}", "receipt");
    let retired_receipt_dir = format!("--{}", ["receipt", "Dir"].concat());
    let retired_receipt_dir_equals = format!(
        "{}={}",
        retired_receipt_dir,
        receipt_dir.to_str().ok_or("non-utf8 receipt dir")?
    );

    for args in [
        vec![
            "skill".to_owned(),
            skill_dir.to_str().ok_or("non-utf8 skill dir")?.to_owned(),
            retired_receipt,
            receipt_dir
                .to_str()
                .ok_or("non-utf8 receipt dir")?
                .to_owned(),
        ],
        vec![
            "skill".to_owned(),
            skill_dir.to_str().ok_or("non-utf8 skill dir")?.to_owned(),
            retired_receipt_dir,
            receipt_dir
                .to_str()
                .ok_or("non-utf8 receipt dir")?
                .to_owned(),
        ],
        vec![
            "skill".to_owned(),
            skill_dir.to_str().ok_or("non-utf8 skill dir")?.to_owned(),
            retired_receipt_dir_equals,
        ],
    ] {
        let output = runx_command().args(args).output()?;
        assert_eq!(output.status.code(), Some(64));
        assert!(String::from_utf8(output.stderr)?.contains("retired runx skill receipt option"));
        assert_eq!(String::from_utf8(output.stdout)?, "");
    }

    Ok(())
}

fn runx_command() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_runx"));
    command.env("NO_COLOR", "1");
    command
}

fn assert_json(
    output: &std::process::Output,
    expected_status: Option<i32>,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    if let Some(expected_status) = expected_status {
        assert_eq!(
            output.status.code(),
            Some(expected_status),
            "stderr={}\nstdout={}",
            String::from_utf8_lossy(&output.stderr),
            String::from_utf8_lossy(&output.stdout)
        );
    }
    assert!(
        output.status.success() || expected_status.is_some(),
        "status={:?}\nstderr={}\nstdout={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout)
    );
    assert_eq!(String::from_utf8(output.stderr.clone())?, "");
    Ok(serde_json::from_slice(&output.stdout)?)
}

fn write_agent_step_skill(root: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let skill_dir = root.join("issue-intake");
    fs::create_dir_all(&skill_dir)?;
    fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: issue-intake\n---\n# Issue Intake\n",
    )?;
    fs::write(
        skill_dir.join("X.yaml"),
        r#"
skill: issue-intake
runners:
  intake:
    default: true
    type: agent-step
    agent: builder
    task: issue-intake
    outputs:
      intake_report: object
    inputs:
      thread_title:
        type: string
        required: false
"#,
    )?;
    Ok(skill_dir)
}

fn temp_root(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    let root = std::env::temp_dir().join(format!("{name}-{}-{nanos}", std::process::id()));
    if root.exists() {
        let _ignored = fs::remove_dir_all(&root);
    }
    root
}
