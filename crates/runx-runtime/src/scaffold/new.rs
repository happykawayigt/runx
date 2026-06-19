use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use super::ScaffoldError;
use super::templates::scaffold_package_files;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunxNewOptions {
    pub name: String,
    pub directory: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RunxNewResult {
    pub name: String,
    pub directory: PathBuf,
    pub files: Vec<String>,
    pub next_steps: Vec<String>,
}

pub fn scaffold_runx_package(options: &RunxNewOptions) -> Result<RunxNewResult, ScaffoldError> {
    let name = sanitize_runx_package_name(&options.name);
    let root = lexical_absolute(&options.directory)?;
    assert_writable_scaffold_target(&root)?;

    let writes = scaffold_package_files(&name);

    fs::create_dir_all(&root)
        .map_err(|source| ScaffoldError::io("creating scaffold root", &root, source))?;
    for file in &writes {
        write_file(&root, &file.relative_path, &file.contents)?;
    }

    Ok(RunxNewResult {
        name,
        directory: root.clone(),
        files: writes.into_iter().map(|file| file.relative_path).collect(),
        next_steps: vec![
            format!("cd {}", root.display()),
            "runx harness . --json".to_owned(),
            "runx skill . --input message=hello --json".to_owned(),
        ],
    })
}

#[must_use]
pub fn sanitize_runx_package_name(value: &str) -> String {
    let sanitized = trim_boundary_separators(&replace_runs(
        &value.trim().to_lowercase(),
        |character| {
            character.is_ascii_lowercase()
                || character.is_ascii_digit()
                || matches!(character, '_' | '.' | '-')
        },
        '-',
    ));
    if sanitized.is_empty() {
        "runx-package".to_owned()
    } else {
        sanitized
    }
}

fn assert_writable_scaffold_target(root: &Path) -> Result<(), ScaffoldError> {
    match fs::read_dir(root) {
        Ok(mut entries) => match entries.next() {
            Some(Ok(_)) => Err(ScaffoldError::NonEmptyTarget {
                path: root.to_path_buf(),
            }),
            Some(Err(source)) => Err(ScaffoldError::io("reading scaffold target", root, source)),
            None => Ok(()),
        },
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(ScaffoldError::io("reading scaffold target", root, source)),
    }
}

fn write_file(root: &Path, relative_path: &str, contents: &str) -> Result<(), ScaffoldError> {
    let file_path = root.join(relative_path);
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|source| ScaffoldError::io("creating scaffold directory", parent, source))?;
    }
    let mut writable = contents.to_owned();
    if !writable.ends_with('\n') {
        writable.push('\n');
    }
    fs::write(&file_path, writable)
        .map_err(|source| ScaffoldError::io("writing scaffold file", file_path, source))
}

fn lexical_absolute(path: &Path) -> Result<PathBuf, ScaffoldError> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(path))
            .map_err(|source| ScaffoldError::io("resolving current directory", ".", source))
    }
}

fn replace_runs(value: &str, keep: impl Fn(char) -> bool, replacement: char) -> String {
    let mut output = String::with_capacity(value.len());
    let mut replacing = false;
    for character in value.chars() {
        if keep(character) {
            output.push(character);
            replacing = false;
        } else if !replacing {
            output.push(replacement);
            replacing = true;
        }
    }
    output
}

fn trim_boundary_separators(value: &str) -> String {
    value
        .trim_matches(|character| matches!(character, '.' | '_' | '-'))
        .to_owned()
}
