use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn registry_local_publish_search_resolve_install_json() -> Result<(), Box<dyn std::error::Error>> {
    let root = temp_root("registry-local");
    let skill_dir = root.join("skill");
    let registry_dir = root.join("registry");
    let install_dir = root.join("installed");
    fs::create_dir_all(&skill_dir)?;
    fs::write(
        skill_dir.join("SKILL.md"),
        include_str!("../../../fixtures/registry/install/echo-SKILL.md"),
    )?;
    fs::write(
        skill_dir.join("X.yaml"),
        include_str!("../../../fixtures/registry/install/echo-X.yaml"),
    )?;

    let publish = runx_command()
        .args([
            "registry",
            "publish",
            skill_dir.to_str().ok_or("non-utf8 skill dir")?,
            "--registry-dir",
            registry_dir.to_str().ok_or("non-utf8 registry dir")?,
            "--owner",
            "acme",
            "--version",
            "1.0.0",
            "--json",
        ])
        .output()?;
    assert_success_contains(
        &publish,
        &["\"action\": \"publish\"", "\"skill_id\": \"acme/echo\""],
    )?;

    let search = runx_command()
        .args([
            "registry",
            "search",
            "echo",
            "--registry-dir",
            registry_dir.to_str().ok_or("non-utf8 registry dir")?,
            "--json",
        ])
        .output()?;
    assert_success_contains(
        &search,
        &["\"action\": \"search\"", "\"skill_id\": \"acme/echo\""],
    )?;

    let resolve = runx_command()
        .args([
            "registry",
            "resolve",
            "registry:echo",
            "--registry-dir",
            registry_dir.to_str().ok_or("non-utf8 registry dir")?,
            "--json",
        ])
        .output()?;
    assert_success_contains(
        &resolve,
        &[
            "\"action\": \"resolve\"",
            "\"kind\": \"local\"",
            "\"skill_id\": \"acme/echo\"",
        ],
    )?;

    let install = runx_command()
        .args([
            "registry",
            "install",
            "acme/echo@1.0.0",
            "--registry-dir",
            registry_dir.to_str().ok_or("non-utf8 registry dir")?,
            "--to",
            install_dir.to_str().ok_or("non-utf8 install dir")?,
            "--json",
        ])
        .output()?;
    assert_success_contains(
        &install,
        &[
            "\"action\": \"install\"",
            "\"skill_id\": \"acme/echo\"",
            "\"status\": \"installed\"",
        ],
    )?;
    assert!(
        install_dir
            .join("acme")
            .join("echo")
            .join("SKILL.md")
            .exists()
    );

    Ok(())
}

fn runx_command() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_runx"));
    command.env("NO_COLOR", "1");
    command
}

fn assert_success_contains(
    output: &std::process::Output,
    needles: &[&str],
) -> Result<(), Box<dyn std::error::Error>> {
    assert!(
        output.status.success(),
        "status={:?}\nstderr={}\nstdout={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout)
    );
    let stdout = String::from_utf8(output.stdout.clone())?;
    for needle in needles {
        assert!(
            stdout.contains(needle),
            "missing {needle} in stdout:\n{stdout}"
        );
    }
    assert_eq!(String::from_utf8(output.stderr.clone())?, "");
    Ok(())
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
