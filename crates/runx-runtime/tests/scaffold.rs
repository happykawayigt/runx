use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use serde::Deserialize;

use runx_runtime::scaffold::{
    InitAction, InitGeneratedValues, RunxInitOptions, RunxNewOptions, ScaffoldError, runx_init,
    scaffold_runx_package,
};

static NEXT_TEST_DIR: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Deserialize)]
struct ScaffoldFixtureManifest {
    name: String,
    files: Vec<String>,
    next_steps: Vec<String>,
}

#[test]
fn new_scaffold_matches_native_fixture() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TestDir::create("new-byte-parity")?;
    let target = temp.path().join("docs-demo");
    let options = RunxNewOptions {
        name: "docs-demo".to_owned(),
        directory: target.clone(),
    };

    let result = scaffold_runx_package(&options)?;
    let manifest = scaffold_fixture_manifest()?;

    assert_eq!(result.name, manifest.name);
    assert_eq!(result.files, manifest.files);
    assert_eq!(
        normalize_next_steps(&target, &result.next_steps),
        manifest.next_steps
    );
    assert_scaffold_files_match(&target, &manifest.files)?;
    Ok(())
}

#[test]
fn new_refuses_non_empty_targets() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TestDir::create("new-non-empty")?;
    let target = temp.path().join("occupied");
    fs::create_dir_all(&target)?;
    fs::write(target.join("README.md"), "keep me\n")?;
    let options = RunxNewOptions {
        name: "docs-demo".to_owned(),
        directory: target.clone(),
    };

    match scaffold_runx_package(&options) {
        Err(ScaffoldError::NonEmptyTarget { path }) => assert_eq!(path, target),
        Err(error) => return Err(format!("expected non-empty target error, got {error}").into()),
        Ok(_) => return Err("expected non-empty target error".into()),
    }
    assert!(!target.join("SKILL.md").exists());
    Ok(())
}

#[test]
fn init_project_state_is_reused() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TestDir::create("init-project")?;
    let project_dir = temp.path().join(".runx");
    let options = init_options(InitAction::Project, &temp);

    let created = runx_init(&RunxInitOptions {
        project_dir: project_dir.clone(),
        ..options.clone()
    })?;
    let reused = runx_init(&RunxInitOptions {
        project_dir: project_dir.clone(),
        generated: generated("proj_other", "inst_other", "2026-05-19T01:02:03.004Z"),
        ..options
    })?;

    assert!(created.created);
    assert!(!reused.created);
    assert_eq!(created.project_id, reused.project_id);
    assert!(project_dir.join("project.json").exists());
    assert!(project_dir.join("skills").is_dir());
    assert!(project_dir.join("tools").is_dir());
    Ok(())
}

#[test]
fn init_global_prefetches_official_cache() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TestDir::create("init-global")?;
    let home = temp.path().join("home");
    let official = temp.path().join("official");
    let result = runx_init(&RunxInitOptions {
        action: InitAction::Global,
        project_dir: temp.path().join(".runx"),
        global_home_dir: home.clone(),
        official_cache_dir: official.clone(),
        prefetch_official: true,
        generated: generated("proj_fixture", "inst_fixture", "2026-05-19T01:02:03.004Z"),
    })?;

    assert!(result.created);
    assert_eq!(result.global_home_dir, Some(home.clone()));
    assert_eq!(result.official_cache_dir, Some(official.clone()));
    assert!(home.join("install.json").exists());
    assert!(official.is_dir());
    Ok(())
}

fn init_options(action: InitAction, temp: &TestDir) -> RunxInitOptions {
    RunxInitOptions {
        action,
        project_dir: temp.path().join(".runx"),
        global_home_dir: temp.path().join("home"),
        official_cache_dir: temp.path().join("official"),
        prefetch_official: false,
        generated: generated("proj_fixture", "inst_fixture", "2026-05-19T01:02:03.004Z"),
    }
}

fn generated(project_id: &str, installation_id: &str, created_at: &str) -> InitGeneratedValues {
    InitGeneratedValues {
        project_id: project_id.to_owned(),
        installation_id: installation_id.to_owned(),
        created_at: created_at.to_owned(),
    }
}

fn scaffold_fixture_manifest() -> Result<ScaffoldFixtureManifest, Box<dyn std::error::Error>> {
    let source = fs::read_to_string(scaffold_fixture_root().join("manifest.json"))?;
    let manifest = serde_json::from_str(&source)?;
    Ok(manifest)
}

fn assert_scaffold_files_match(
    generated_root: &Path,
    expected_files: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    for relative_path in expected_files {
        let generated = fs::read_to_string(generated_root.join(relative_path))?;
        let expected =
            fs::read_to_string(scaffold_fixture_root().join("files").join(relative_path))?;
        assert_eq!(generated, expected, "{relative_path}");
    }
    Ok(())
}

fn normalize_next_steps(target: &Path, next_steps: &[String]) -> Vec<String> {
    next_steps
        .iter()
        .map(|step| {
            if step == &format!("cd {}", target.display()) {
                "cd <target>".to_owned()
            } else {
                step.clone()
            }
        })
        .collect()
}

fn scaffold_fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/scaffold/new-docs-demo")
        .lexically_normalized()
}

struct TestDir {
    path: PathBuf,
}

impl TestDir {
    fn create(label: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let id = NEXT_TEST_DIR.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "runx-runtime-scaffold-{label}-{}-{id}",
            std::process::id()
        ));
        if path.exists() {
            fs::remove_dir_all(&path)?;
        }
        fs::create_dir_all(&path)?;
        Ok(Self { path })
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ignored = fs::remove_dir_all(&self.path);
    }
}

trait LexicallyNormalized {
    fn lexically_normalized(self) -> Self;
}

impl LexicallyNormalized for PathBuf {
    fn lexically_normalized(self) -> Self {
        let mut normalized = PathBuf::new();
        for component in self.components() {
            match component {
                std::path::Component::ParentDir => {
                    normalized.pop();
                }
                std::path::Component::CurDir => {}
                other => normalized.push(other.as_os_str()),
            }
        }
        normalized
    }
}
