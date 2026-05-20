// rust-style-allow: large-file because local registry installs keep digest
// validation, binding checks, conflict planning, and atomic writes in one
// transaction module.
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use runx_parser::{
    SkillInstallOrigin, ValidatedSkillInstall, parse_runner_manifest_yaml,
    validate_runner_manifest, validate_skill_install,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use super::refs::safe_skill_package_parts;
use super::types::TrustTier;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstallCandidate {
    pub markdown: String,
    pub profile_document: Option<String>,
    pub source: String,
    pub source_label: String,
    pub r#ref: String,
    pub skill_id: Option<String>,
    pub version: Option<String>,
    pub digest: Option<String>,
    pub profile_digest: Option<String>,
    pub runner_names: Vec<String>,
    pub trust_tier: Option<TrustTier>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstallLocalSkillOptions {
    pub destination_root: PathBuf,
    pub expected_digest: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallStatus {
    Installed,
    Unchanged,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct InstallLocalSkillResult {
    pub status: InstallStatus,
    pub destination: PathBuf,
    pub skill_name: String,
    pub source: String,
    pub source_label: String,
    pub skill_id: Option<String>,
    pub version: Option<String>,
    pub digest: String,
    pub profile_digest: Option<String>,
    pub profile_state_path: Option<PathBuf>,
    pub runner_names: Vec<String>,
    pub trust_tier: Option<TrustTier>,
}

#[derive(Debug, thiserror::Error)]
pub enum InstallError {
    #[error("{0}")]
    Parser(#[from] runx_parser::SkillInstallError),
    #[error("{0}")]
    Manifest(#[from] runx_parser::ValidationError),
    #[error("{0}")]
    ManifestParse(#[from] runx_parser::ParseError),
    #[error("digest mismatch for {ref_name}: expected {expected}, received {actual}")]
    DigestMismatch {
        ref_name: String,
        expected: String,
        actual: String,
    },
    #[error("binding digest mismatch for {ref_name}: expected {expected}, received {actual}")]
    ProfileDigestMismatch {
        ref_name: String,
        expected: String,
        actual: String,
    },
    #[error("runner manifest skill '{manifest_skill}' does not match skill '{skill_name}'")]
    ManifestSkillMismatch {
        manifest_skill: String,
        skill_name: String,
    },
    #[error("runner manifest runners do not match advertised runner metadata for skill '{0}'")]
    RunnerMetadataMismatch(String),
    #[error("skill install destination already exists with different content: {0}")]
    ConflictingSkill(PathBuf),
    #[error("skill install profile state already exists with different content: {0}")]
    ConflictingProfile(PathBuf),
    #[error("io error at {path}: {source}")]
    Io { path: PathBuf, source: io::Error },
    #[error("failed to serialize profile state: {0}")]
    Serialize(#[from] serde_json::Error),
}

pub fn install_local_skill(
    candidate: &InstallCandidate,
    options: &InstallLocalSkillOptions,
) -> Result<InstallLocalSkillResult, InstallError> {
    let validated = validate_install_candidate(candidate, options)?;
    let paths = install_paths(candidate, options, &validated.install.skill.name);
    let write_plan = prepare_install_write_plan(
        &paths,
        &validated.install.markdown,
        validated.next_profile_state.as_deref(),
    )?;
    commit_install_write_plan(
        &paths,
        &write_plan,
        &validated.install.markdown,
        validated.next_profile_state.as_deref(),
    )?;

    Ok(InstallLocalSkillResult {
        status: if write_plan.writes_skill || write_plan.writes_profile_state {
            InstallStatus::Installed
        } else {
            InstallStatus::Unchanged
        },
        destination: paths.destination,
        skill_name: validated.install.skill.name,
        source: validated.install.origin.source,
        source_label: validated.install.origin.source_label,
        skill_id: validated.install.origin.skill_id,
        version: validated.install.origin.version,
        digest: validated.actual_digest,
        profile_digest: validated.profile_digest,
        profile_state_path: paths.profile_state_path,
        runner_names: validated.runner_names,
        trust_tier: candidate.trust_tier.clone(),
    })
}

struct ValidatedLocalInstall {
    actual_digest: String,
    profile_digest: Option<String>,
    runner_names: Vec<String>,
    install: ValidatedSkillInstall,
    next_profile_state: Option<String>,
}

struct InstallPaths {
    package_root: PathBuf,
    destination: PathBuf,
    profile_state_path: Option<PathBuf>,
}

struct InstallWritePlan {
    writes_skill: bool,
    writes_profile_state: bool,
}

fn validate_install_candidate(
    candidate: &InstallCandidate,
    options: &InstallLocalSkillOptions,
) -> Result<ValidatedLocalInstall, InstallError> {
    let actual_digest = validate_candidate_digest(candidate, options)?;
    let origin = install_origin(candidate, &actual_digest);
    let install = validate_skill_install(&candidate.markdown, origin)?;
    let profile_digest = validate_candidate_profile_digest(candidate)?;
    let runner_names = validate_install_binding_manifest(
        &install.skill.name,
        candidate.profile_document.as_deref(),
        &candidate.runner_names,
    )?;
    let next_profile_state = next_profile_state(
        candidate,
        &install,
        &actual_digest,
        profile_digest.as_deref(),
        &runner_names,
    )?;
    Ok(ValidatedLocalInstall {
        actual_digest,
        profile_digest,
        runner_names,
        install,
        next_profile_state,
    })
}

fn validate_candidate_digest(
    candidate: &InstallCandidate,
    options: &InstallLocalSkillOptions,
) -> Result<String, InstallError> {
    let actual_digest = sha256_prefixed(&candidate.markdown);
    let expected_digest = options
        .expected_digest
        .as_ref()
        .or(candidate.digest.as_ref());
    if let Some(expected) =
        expected_digest.filter(|expected| !digest_matches(expected, &actual_digest))
    {
        return Err(InstallError::DigestMismatch {
            ref_name: candidate.r#ref.clone(),
            expected: expected.clone(),
            actual: actual_digest,
        });
    }
    Ok(actual_digest)
}

fn validate_candidate_profile_digest(
    candidate: &InstallCandidate,
) -> Result<Option<String>, InstallError> {
    let profile_digest = candidate
        .profile_document
        .as_ref()
        .map(|document| sha256_prefixed(document));
    if let Some(expected) = &candidate.profile_digest {
        let matches = profile_digest
            .as_ref()
            .is_some_and(|actual| digest_matches(expected, actual));
        if !matches {
            return Err(InstallError::ProfileDigestMismatch {
                ref_name: candidate.r#ref.clone(),
                expected: expected.clone(),
                actual: profile_digest.clone().unwrap_or_else(|| "none".to_owned()),
            });
        }
    }
    Ok(profile_digest)
}

fn next_profile_state(
    candidate: &InstallCandidate,
    install: &ValidatedSkillInstall,
    actual_digest: &str,
    profile_digest: Option<&str>,
    runner_names: &[String],
) -> Result<Option<String>, InstallError> {
    let Some(document) = &candidate.profile_document else {
        return Ok(None);
    };
    Ok(Some(profile_state(
        &install.skill.name,
        actual_digest,
        document,
        profile_digest,
        runner_names,
        &serde_json::to_value(&install.origin)?,
    )?))
}

fn install_origin(candidate: &InstallCandidate, actual_digest: &str) -> SkillInstallOrigin {
    SkillInstallOrigin {
        source: candidate.source.clone(),
        source_label: candidate.source_label.clone(),
        r#ref: candidate.r#ref.clone(),
        skill_id: candidate.skill_id.clone(),
        version: candidate.version.clone(),
        digest: Some(actual_digest.to_owned()),
        profile_digest: candidate.profile_digest.clone(),
        runner_names: Some(candidate.runner_names.clone()),
        trust_tier: candidate.trust_tier.as_ref().map(trust_tier_string),
    }
}

fn install_paths(
    candidate: &InstallCandidate,
    options: &InstallLocalSkillOptions,
    skill_name: &str,
) -> InstallPaths {
    let package_parts = safe_skill_package_parts(&candidate.r#ref, skill_name);
    let package_root = package_parts
        .iter()
        .fold(options.destination_root.clone(), |path, part| {
            path.join(part)
        });
    let destination = package_root.join("SKILL.md");
    let profile_state_path = candidate
        .profile_document
        .as_ref()
        .map(|_| package_root.join(".runx").join("profile.json"));
    InstallPaths {
        package_root,
        destination,
        profile_state_path,
    }
}

fn prepare_install_write_plan(
    paths: &InstallPaths,
    markdown: &str,
    next_profile_state: Option<&str>,
) -> Result<InstallWritePlan, InstallError> {
    let existing = read_optional(&paths.destination)?;
    let existing_profile = match &paths.profile_state_path {
        Some(path) => read_optional(path)?,
        None => None,
    };
    if let Some(existing) = &existing {
        if sha256_prefixed(existing) != sha256_prefixed(markdown) {
            return Err(InstallError::ConflictingSkill(paths.destination.clone()));
        }
    }
    if let (Some(path), Some(existing), Some(next)) = (
        &paths.profile_state_path,
        &existing_profile,
        next_profile_state,
    ) {
        if existing != next {
            return Err(InstallError::ConflictingProfile(path.clone()));
        }
    }
    Ok(InstallWritePlan {
        writes_skill: existing.is_none(),
        writes_profile_state: paths.profile_state_path.is_some() && existing_profile.is_none(),
    })
}

fn commit_install_write_plan(
    paths: &InstallPaths,
    write_plan: &InstallWritePlan,
    markdown: &str,
    next_profile_state: Option<&str>,
) -> Result<(), InstallError> {
    fs::create_dir_all(&paths.package_root).map_err(|source| InstallError::Io {
        path: paths.package_root.clone(),
        source,
    })?;
    if write_plan.writes_skill {
        write_atomic(&paths.destination, markdown)?;
    }
    if let (Some(path), true, Some(next)) = (
        &paths.profile_state_path,
        write_plan.writes_profile_state,
        next_profile_state,
    ) {
        let parent = path.parent().unwrap_or(&paths.package_root);
        fs::create_dir_all(parent).map_err(|source| InstallError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
        write_atomic(path, next)?;
    }
    Ok(())
}

fn validate_install_binding_manifest(
    skill_name: &str,
    profile_document: Option<&str>,
    advertised_runner_names: &[String],
) -> Result<Vec<String>, InstallError> {
    let Some(profile_document) = profile_document else {
        return Ok(advertised_runner_names.to_vec());
    };
    let manifest = validate_runner_manifest(parse_runner_manifest_yaml(profile_document)?)?;
    if let Some(manifest_skill) = manifest.skill {
        if manifest_skill != skill_name {
            return Err(InstallError::ManifestSkillMismatch {
                manifest_skill,
                skill_name: skill_name.to_owned(),
            });
        }
    }
    let runner_names = manifest.runners.keys().cloned().collect::<Vec<_>>();
    if !advertised_runner_names.is_empty() && advertised_runner_names != runner_names {
        return Err(InstallError::RunnerMetadataMismatch(skill_name.to_owned()));
    }
    Ok(runner_names)
}

fn profile_state(
    skill_name: &str,
    digest: &str,
    profile_document: &str,
    profile_digest: Option<&str>,
    runner_names: &[String],
    origin: &Value,
) -> Result<String, serde_json::Error> {
    let value = json!({
        "schema_version": "runx.skill-profile.v1",
        "skill": {
            "name": skill_name,
            "path": "SKILL.md",
            "digest": digest,
        },
        "profile": {
            "document": profile_document,
            "digest": profile_digest,
            "runner_names": runner_names,
        },
        "origin": origin,
    });
    serde_json::to_string_pretty(&value).map(|mut contents| {
        contents.push('\n');
        contents
    })
}

fn write_atomic(destination: &Path, contents: &str) -> Result<(), InstallError> {
    let temp_path = destination.with_extension(format!("tmp-{}", unique_suffix()));
    fs::write(&temp_path, contents).map_err(|source| InstallError::Io {
        path: temp_path.clone(),
        source,
    })?;
    if destination.exists() {
        let _ = fs::remove_file(&temp_path);
        return Err(InstallError::Io {
            path: destination.to_path_buf(),
            source: io::Error::new(io::ErrorKind::AlreadyExists, "destination exists"),
        });
    }
    fs::rename(&temp_path, destination).map_err(|source| {
        let _ = fs::remove_file(&temp_path);
        InstallError::Io {
            path: destination.to_path_buf(),
            source,
        }
    })
}

fn read_optional(path: &Path) -> Result<Option<String>, InstallError> {
    match fs::read_to_string(path) {
        Ok(contents) => Ok(Some(contents)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(InstallError::Io {
            path: path.to_path_buf(),
            source,
        }),
    }
}

fn sha256_prefixed(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        hex.push_str(&format!("{byte:02x}"));
    }
    format!("sha256:{hex}")
}

fn digest_matches(expected: &str, actual_prefixed: &str) -> bool {
    expected == actual_prefixed
        || actual_prefixed
            .strip_prefix("sha256:")
            .is_some_and(|actual_hex| expected == actual_hex)
}

fn trust_tier_string(value: &TrustTier) -> String {
    match value {
        TrustTier::FirstParty => "first_party",
        TrustTier::Verified => "verified",
        TrustTier::Community => "community",
    }
    .to_owned()
}

fn unique_suffix() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    format!("{}-{nanos}", std::process::id())
}
