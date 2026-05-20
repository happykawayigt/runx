// rust-style-allow: large-file because this untracked registry file is under
// active parallel work; keep the module stable while extracting blockers here.
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use runx_contracts::{JsonObject, JsonValue};
use runx_parser::{
    SkillRunnerManifest, ValidatedSkill, parse_runner_manifest_yaml, parse_skill_markdown,
    validate_runner_manifest, validate_skill,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};

use super::refs::parse_registry_ref;
use super::types::{
    ProfileMode, PublishSkillMarkdownResult, PublishStatus, RegistryAttestation,
    RegistryLinkResolution, RegistryPublisher, RegistrySearchResult, RegistrySkill,
    RegistrySkillDetail, RegistrySkillResolution, RegistrySkillVersion, RegistrySourceMetadata,
    TrustSignal, TrustTier,
};

#[derive(Clone, Debug)]
pub struct FileRegistryStore {
    root: PathBuf,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PutVersionOptions {
    pub upsert: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct IngestSkillOptions {
    pub owner: Option<String>,
    pub version: Option<String>,
    pub created_at: Option<String>,
    pub profile_document: Option<String>,
    pub publisher: Option<RegistryPublisher>,
    pub trust_tier: Option<TrustTier>,
    pub attestations: Vec<RegistryAttestation>,
    pub source_metadata: Option<RegistrySourceMetadata>,
    pub upsert: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CreateRegistrySkillVersionResult {
    pub record: RegistrySkillVersion,
    pub created: bool,
}

#[derive(Clone, Debug)]
pub struct LocalRegistryClient {
    store: FileRegistryStore,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PublishSkillMarkdownOptions {
    pub ingest: IngestSkillOptions,
    pub registry_url: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RegistrySearchOptions {
    pub limit: Option<usize>,
    pub registry_url: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RegistryResolveOptions {
    pub version: Option<String>,
    pub registry_url: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum LocalRegistryError {
    #[error("{0}")]
    Parse(#[from] runx_parser::ParseError),
    #[error("{0}")]
    Validation(#[from] runx_parser::ValidationError),
    #[error("io error while {action} {path}: {source}")]
    Io {
        action: &'static str,
        path: PathBuf,
        source: io::Error,
    },
    #[error("invalid registry JSON at {path}: {source}")]
    JsonRead {
        path: PathBuf,
        source: serde_json::Error,
    },
    #[error("failed to serialize registry JSON at {path}: {source}")]
    JsonWrite {
        path: PathBuf,
        source: serde_json::Error,
    },
    #[error("invalid registry version payload at {field}: {message}")]
    InvalidVersionPayload { field: String, message: String },
    #[error("invalid registry skill id '{0}'. Expected '<owner>/<name>'.")]
    InvalidSkillId(String),
    #[error("registry slugs cannot be empty")]
    EmptySlug,
    #[error("registry path component '{0}' is not allowed")]
    UnsafePathComponent(String),
    #[error("registry version {skill_id}@{version} already exists with a different digest")]
    VersionConflict { skill_id: String, version: String },
    #[error("Registry ref '{0}' is ambiguous. Use '<owner>/<name>' instead.")]
    Ambiguous(String),
}

impl FileRegistryStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn put_version(
        &self,
        version: RegistrySkillVersion,
        options: PutVersionOptions,
    ) -> Result<RegistrySkillVersion, LocalRegistryError> {
        let version_path = self.version_path(&version.skill_id, &version.version)?;
        if let Some(parent) = version_path.parent() {
            fs::create_dir_all(parent).map_err(|source| io_error("creating", parent, source))?;
        }

        if let Some(existing) = self.get_version(&version.skill_id, Some(&version.version))? {
            if existing.digest != version.digest
                || existing.profile_digest != version.profile_digest
            {
                if !options.upsert {
                    return Err(LocalRegistryError::VersionConflict {
                        skill_id: version.skill_id,
                        version: version.version,
                    });
                }
                let mut upserted = version;
                upserted.updated_at = now_iso8601();
                write_registry_json(&version_path, &upserted, false)?;
                return Ok(upserted);
            }

            let mut refreshed = version;
            refreshed.created_at = existing.created_at.clone();
            refreshed.updated_at = now_iso8601();
            if existing != refreshed {
                write_registry_json(&version_path, &refreshed, false)?;
            }
            return Ok(refreshed);
        }

        write_registry_json(&version_path, &version, true)?;
        Ok(version)
    }

    pub fn get_version(
        &self,
        skill_id: &str,
        version: Option<&str>,
    ) -> Result<Option<RegistrySkillVersion>, LocalRegistryError> {
        let versions = self.list_versions(skill_id)?;
        if versions.is_empty() {
            return Ok(None);
        }
        let Some(version) = version else {
            return Ok(versions.last().cloned());
        };
        Ok(versions
            .into_iter()
            .find(|candidate| candidate.version == version))
    }

    pub fn list_versions(
        &self,
        skill_id: &str,
    ) -> Result<Vec<RegistrySkillVersion>, LocalRegistryError> {
        let skill_dir = self.skill_dir(skill_id)?;
        let mut files = safe_read_dir_names(&skill_dir)?;
        files.sort();

        let mut versions = Vec::new();
        for file in files.into_iter().filter(|file| file.ends_with(".json")) {
            let path = skill_dir.join(file);
            let contents =
                fs::read_to_string(&path).map_err(|source| io_error("reading", &path, source))?;
            let payload = serde_json::from_str::<RegistrySkillVersionPayload>(&contents).map_err(
                |source| LocalRegistryError::JsonRead {
                    path: path.clone(),
                    source,
                },
            )?;
            versions.push(normalize_registry_skill_version(payload)?);
        }
        versions.sort_by(|left, right| {
            left.created_at
                .cmp(&right.created_at)
                .then_with(|| left.version.cmp(&right.version))
        });
        Ok(versions)
    }

    pub fn list_skills(&self) -> Result<Vec<RegistrySkill>, LocalRegistryError> {
        let owners = safe_read_dir_names(&self.root)?;
        let mut skills = Vec::new();
        for owner in owners {
            let owner_dir = self.root.join(&owner);
            for name in safe_read_dir_names(&owner_dir)? {
                let skill_id = format!("{}/{}", decode_part(&owner)?, decode_part(&name)?);
                let versions = self.list_versions(&skill_id)?;
                let Some(latest) = versions.last() else {
                    continue;
                };
                skills.push(RegistrySkill {
                    skill_id,
                    owner: latest.owner.clone(),
                    name: latest.name.clone(),
                    description: latest.description.clone(),
                    latest_version: latest.version.clone(),
                    latest_digest: latest.digest.clone(),
                    versions,
                });
            }
        }
        skills.sort_by(|left, right| left.skill_id.cmp(&right.skill_id));
        Ok(skills)
    }

    fn version_path(&self, skill_id: &str, version: &str) -> Result<PathBuf, LocalRegistryError> {
        Ok(self
            .skill_dir(skill_id)?
            .join(format!("{}.json", encode_part(version))))
    }

    fn skill_dir(&self, skill_id: &str) -> Result<PathBuf, LocalRegistryError> {
        let (owner, name) = split_skill_id(skill_id)?;
        Ok(self.root.join(encode_part(owner)).join(encode_part(name)))
    }
}

impl LocalRegistryClient {
    pub fn new(store: FileRegistryStore) -> Self {
        Self { store }
    }

    pub fn create_skill_version(
        &self,
        markdown: &str,
        options: IngestSkillOptions,
    ) -> Result<CreateRegistrySkillVersionResult, LocalRegistryError> {
        create_registry_skill_version(&self.store, markdown, options)
    }
}

pub fn create_file_registry_store(root: impl Into<PathBuf>) -> FileRegistryStore {
    FileRegistryStore::new(root)
}

pub fn create_local_registry_client(store: FileRegistryStore) -> LocalRegistryClient {
    LocalRegistryClient::new(store)
}

pub fn ingest_skill_markdown(
    store: &FileRegistryStore,
    markdown: &str,
    options: IngestSkillOptions,
) -> Result<RegistrySkillVersion, LocalRegistryError> {
    Ok(create_registry_skill_version(store, markdown, options)?.record)
}

pub fn create_registry_skill_version(
    store: &FileRegistryStore,
    markdown: &str,
    options: IngestSkillOptions,
) -> Result<CreateRegistrySkillVersionResult, LocalRegistryError> {
    let record = build_registry_skill_version(markdown, &options)?;
    let existing = store.get_version(&record.skill_id, Some(&record.version))?;
    if let Some(existing) = existing {
        if existing.digest != record.digest || existing.profile_digest != record.profile_digest {
            if !options.upsert {
                return Err(LocalRegistryError::VersionConflict {
                    skill_id: record.skill_id,
                    version: record.version,
                });
            }
            return Ok(CreateRegistrySkillVersionResult {
                record: store.put_version(record, PutVersionOptions { upsert: true })?,
                created: false,
            });
        }
        let mut refreshed = record;
        refreshed.created_at = existing.created_at;
        return Ok(CreateRegistrySkillVersionResult {
            record: store.put_version(refreshed, PutVersionOptions::default())?,
            created: false,
        });
    }

    Ok(CreateRegistrySkillVersionResult {
        record: store.put_version(record, PutVersionOptions::default())?,
        created: true,
    })
}

pub fn build_registry_skill_version(
    markdown: &str,
    options: &IngestSkillOptions,
) -> Result<RegistrySkillVersion, LocalRegistryError> {
    let raw = parse_skill_markdown(markdown)?;
    let skill = validate_skill(raw)?;
    let digest = sha256_hex(markdown);
    let binding = build_binding_artifact(&skill, options.profile_document.as_deref())?;
    let catalog = registry_catalog(binding.manifest.as_ref());
    let defaults = registry_version_defaults(&digest, binding.digest.as_deref(), options);
    let manifest = binding.manifest.as_ref();
    let skill_id = build_skill_id(&defaults.owner, &skill.name)?;
    Ok(RegistrySkillVersion {
        skill_id,
        owner: defaults.owner,
        name: skill.name.clone(),
        description: skill.description.clone(),
        version: defaults.version,
        digest,
        markdown: markdown.to_owned(),
        profile_document: options.profile_document.clone(),
        profile_digest: binding.digest,
        runner_names: binding.runner_names,
        source_type: skill.source.source_type.clone(),
        trust_tier: defaults.trust_tier,
        catalog_kind: Some(catalog.kind),
        catalog_audience: Some(catalog.audience),
        catalog_visibility: Some(catalog.visibility),
        source_metadata: defaults.source_metadata,
        attestations: defaults.attestations,
        required_scopes: registry_required_scopes(&skill, manifest),
        runtime: registry_runtime(&skill, manifest),
        auth: skill.auth.clone(),
        risk: registry_risk(&skill),
        runx: skill.runx.clone(),
        tags: registry_tags(&skill, manifest),
        publisher: defaults.publisher,
        created_at: defaults.created_at,
        updated_at: now_iso8601(),
    })
}

struct RegistryVersionDefaults {
    owner: String,
    created_at: String,
    publisher: RegistryPublisher,
    trust_tier: TrustTier,
    version: String,
    source_metadata: Option<RegistrySourceMetadata>,
    attestations: Vec<RegistryAttestation>,
}

fn registry_version_defaults(
    digest: &str,
    profile_digest: Option<&str>,
    options: &IngestSkillOptions,
) -> RegistryVersionDefaults {
    let owner = options.owner.clone().unwrap_or_else(|| "local".to_owned());
    let created_at = options.created_at.clone().unwrap_or_else(now_iso8601);
    let publisher = options
        .publisher
        .clone()
        .unwrap_or_else(|| default_registry_publisher(&owner));
    let trust_tier = options
        .trust_tier
        .clone()
        .unwrap_or_else(|| derive_registry_trust_tier(&owner, None));
    let version = options.version.clone().unwrap_or_else(|| {
        let seed = default_registry_version_seed(digest, profile_digest);
        format!("sha-{}", seed.chars().take(12).collect::<String>())
    });
    let source_metadata = options.source_metadata.clone();
    let attestations = merge_registry_attestations(vec![
        build_publisher_attestations(&publisher, &trust_tier, &created_at),
        build_source_attestations(source_metadata.as_ref(), &created_at),
        options.attestations.clone(),
    ]);
    RegistryVersionDefaults {
        owner,
        created_at,
        publisher,
        trust_tier,
        version,
        source_metadata,
        attestations,
    }
}

fn registry_catalog(manifest: Option<&SkillRunnerManifest>) -> runx_parser::CatalogMetadata {
    manifest
        .and_then(|manifest| manifest.catalog.clone())
        .unwrap_or(runx_parser::CatalogMetadata {
            kind: "skill".to_owned(),
            audience: "public".to_owned(),
            visibility: "public".to_owned(),
        })
}

fn registry_required_scopes(
    skill: &ValidatedSkill,
    manifest: Option<&SkillRunnerManifest>,
) -> Vec<String> {
    unique(
        extract_scopes(skill)
            .into_iter()
            .chain(extract_runner_scopes(manifest))
            .collect(),
    )
}

fn registry_runtime(
    skill: &ValidatedSkill,
    manifest: Option<&SkillRunnerManifest>,
) -> Option<JsonValue> {
    skill
        .runtime
        .clone()
        .or_else(|| record_field(skill.runx.as_ref(), "runtime"))
        .or_else(|| extract_runner_runtime(manifest))
}

fn registry_risk(skill: &ValidatedSkill) -> Option<JsonValue> {
    skill
        .risk
        .clone()
        .or_else(|| record_field(skill.runx.as_ref(), "risk"))
}

fn registry_tags(skill: &ValidatedSkill, manifest: Option<&SkillRunnerManifest>) -> Vec<String> {
    unique(
        extract_tags(skill)
            .into_iter()
            .chain(extract_runner_tags(manifest))
            .collect(),
    )
}

pub fn publish_skill_markdown(
    client: &LocalRegistryClient,
    markdown: &str,
    options: PublishSkillMarkdownOptions,
) -> Result<PublishSkillMarkdownResult, LocalRegistryError> {
    let result = client.create_skill_version(markdown, options.ingest)?;
    let link = runx_link_for_version(&result.record, options.registry_url.as_deref());
    Ok(PublishSkillMarkdownResult {
        status: if result.created {
            PublishStatus::Published
        } else {
            PublishStatus::Unchanged
        },
        skill_id: result.record.skill_id.clone(),
        name: result.record.name.clone(),
        version: result.record.version.clone(),
        digest: result.record.digest.clone(),
        profile_digest: result.record.profile_digest.clone(),
        runner_names: result.record.runner_names.clone(),
        source_type: result.record.source_type.clone(),
        registry_url: options.registry_url,
        link,
        record: result.record,
    })
}

pub fn search_registry(
    store: &FileRegistryStore,
    query: &str,
) -> Result<Vec<RegistrySearchResult>, LocalRegistryError> {
    search_registry_with_options(store, query, RegistrySearchOptions::default())
}

pub fn search_registry_with_options(
    store: &FileRegistryStore,
    query: &str,
    options: RegistrySearchOptions,
) -> Result<Vec<RegistrySearchResult>, LocalRegistryError> {
    let normalized_query = normalize(query);
    let mut matches = store
        .list_skills()?
        .into_iter()
        .filter_map(|skill| skill.versions.last().cloned())
        .filter(|version| {
            normalized_query.is_empty() || searchable_text(version).contains(&normalized_query)
        })
        .collect::<Vec<_>>();
    matches.sort_by(|left, right| left.skill_id.cmp(&right.skill_id));
    matches.truncate(options.limit.unwrap_or(20));
    Ok(matches
        .iter()
        .map(|version| search_result_for_version(version, options.registry_url.as_deref()))
        .collect())
}

pub fn resolve_registry_skill(
    store: &FileRegistryStore,
    registry_ref: &str,
    options: RegistryResolveOptions,
) -> Result<Option<RegistrySkillResolution>, LocalRegistryError> {
    let parsed = parse_registry_ref(registry_ref);
    let version = options.version.as_deref().or(parsed.version.as_deref());
    let record = if parsed.skill_id.contains('/') {
        store.get_version(&parsed.skill_id, version)?
    } else {
        resolve_by_name(store, &parsed.skill_id, version)?
    };
    Ok(record.map(|record| {
        let link = runx_link_for_version(&record, options.registry_url.as_deref());
        RegistrySkillResolution {
            markdown: record.markdown,
            profile_document: record.profile_document,
            profile_digest: record.profile_digest,
            runner_names: record.runner_names,
            skill_id: record.skill_id,
            name: record.name,
            version: record.version,
            digest: record.digest,
            source: "runx-registry".to_owned(),
            source_label: "runx registry".to_owned(),
            source_type: record.source_type,
            trust_tier: record.trust_tier,
            registry_url: options.registry_url,
            install_command: link.install_command,
            run_command: link.run_command,
        }
    }))
}

pub fn read_registry_skill(
    store: &FileRegistryStore,
    skill_id: &str,
    version: Option<&str>,
    registry_url: Option<&str>,
) -> Result<Option<RegistrySkillDetail>, LocalRegistryError> {
    Ok(store
        .get_version(skill_id, version)?
        .map(|record| detail_for_version(&record, registry_url)))
}

pub fn resolve_runx_link(
    store: &FileRegistryStore,
    skill_id: &str,
    version: Option<&str>,
    registry_url: Option<&str>,
) -> Result<Option<RegistryLinkResolution>, LocalRegistryError> {
    Ok(store
        .get_version(skill_id, version)?
        .map(|record| runx_link_for_version(&record, registry_url)))
}

pub fn runx_link_for_version(
    record: &RegistrySkillVersion,
    registry_url: Option<&str>,
) -> RegistryLinkResolution {
    let registry_ref = format!("{}@{}", record.skill_id, record.version);
    let registry_flag = registry_url.map_or_else(String::new, |url| format!(" --registry {url}"));
    RegistryLinkResolution {
        link: format!(
            "runx://skill/{}@{}",
            encode_uri_component(&record.skill_id),
            encode_uri_component(&record.version)
        ),
        skill_id: record.skill_id.clone(),
        version: record.version.clone(),
        digest: record.digest.clone(),
        registry_url: registry_url.map(ToOwned::to_owned),
        install_command: format!("runx skill add {registry_ref}{registry_flag}"),
        run_command: format!("runx skill {}", record.name),
    }
}

pub fn build_skill_id(owner: &str, name: &str) -> Result<String, LocalRegistryError> {
    Ok(format!("{}/{}", slugify(owner)?, slugify(name)?))
}

pub fn split_skill_id(skill_id: &str) -> Result<(&str, &str), LocalRegistryError> {
    let mut parts = skill_id.split('/');
    let owner = parts.next().unwrap_or_default();
    let name = parts.next().unwrap_or_default();
    if owner.is_empty() || name.is_empty() || parts.next().is_some() {
        return Err(LocalRegistryError::InvalidSkillId(skill_id.to_owned()));
    }
    reject_unsafe_path_component(owner)?;
    reject_unsafe_path_component(name)?;
    Ok((owner, name))
}

pub fn slugify(value: &str) -> Result<String, LocalRegistryError> {
    let mut slug = String::new();
    let mut last_dash = false;
    for ch in value.trim().to_lowercase().chars() {
        let keep = ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-');
        if keep {
            slug.push(ch);
            last_dash = false;
        } else if !last_dash {
            slug.push('-');
            last_dash = true;
        }
    }
    let slug = slug.trim_matches('-').to_owned();
    if slug.is_empty() {
        Err(LocalRegistryError::EmptySlug)
    } else if is_unsafe_path_component(&slug) {
        Err(LocalRegistryError::UnsafePathComponent(slug))
    } else {
        Ok(slug)
    }
}

pub fn normalize_registry_skill_version(
    payload: RegistrySkillVersionPayload,
) -> Result<RegistrySkillVersion, LocalRegistryError> {
    let owner = required_string(payload.owner, "registry_version.owner")?;
    let created_at = required_string(payload.created_at, "registry_version.created_at")?;
    let publisher = validate_publisher(
        payload
            .publisher
            .ok_or_else(|| missing_field("registry_version.publisher"))?,
        "registry_version.publisher",
    )?;
    let trust_tier = payload.trust_tier.unwrap_or(TrustTier::Community);
    let source_metadata = normalize_source_metadata(payload.source_metadata)?;
    let attestations = normalize_attestations(
        payload.attestations.unwrap_or_default(),
        source_metadata.as_ref(),
        &publisher,
        &trust_tier,
        &created_at,
    );
    let catalog = normalize_registry_catalog(
        payload.catalog_kind.as_deref(),
        payload.catalog_audience.as_deref(),
        payload.catalog_visibility.as_deref(),
    );
    let updated_at = payload
        .updated_at
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| created_at.clone());
    Ok(RegistrySkillVersion {
        skill_id: required_string(payload.skill_id, "registry_version.skill_id")?,
        owner,
        name: required_string(payload.name, "registry_version.name")?,
        description: payload.description,
        version: required_string(payload.version, "registry_version.version")?,
        digest: required_string(payload.digest, "registry_version.digest")?,
        markdown: required_string(payload.markdown, "registry_version.markdown")?,
        profile_document: payload.profile_document,
        profile_digest: payload.profile_digest,
        runner_names: payload.runner_names.unwrap_or_default(),
        source_type: required_string(payload.source_type, "registry_version.source_type")?,
        trust_tier,
        catalog_kind: Some(catalog.kind),
        catalog_audience: Some(catalog.audience),
        catalog_visibility: Some(catalog.visibility),
        source_metadata,
        attestations,
        required_scopes: payload.required_scopes.unwrap_or_default(),
        runtime: payload.runtime,
        auth: payload.auth,
        risk: payload.risk,
        runx: payload.runx,
        tags: payload.tags.unwrap_or_default(),
        publisher,
        updated_at,
        created_at,
    })
}

fn normalize_source_metadata(
    source_metadata: Option<RegistrySourceMetadata>,
) -> Result<Option<RegistrySourceMetadata>, LocalRegistryError> {
    source_metadata.map(validate_source_metadata).transpose()
}

fn normalize_registry_catalog(
    kind: Option<&str>,
    audience: Option<&str>,
    visibility: Option<&str>,
) -> runx_parser::CatalogMetadata {
    runx_parser::CatalogMetadata {
        kind: match kind {
            Some("graph") => "graph".to_owned(),
            _ => "skill".to_owned(),
        },
        audience: match audience {
            Some("builder") => "builder".to_owned(),
            Some("operator") => "operator".to_owned(),
            _ => "public".to_owned(),
        },
        visibility: match visibility {
            Some("private") => "private".to_owned(),
            _ => "public".to_owned(),
        },
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct RegistrySkillVersionPayload {
    skill_id: Option<String>,
    owner: Option<String>,
    name: Option<String>,
    description: Option<String>,
    version: Option<String>,
    digest: Option<String>,
    markdown: Option<String>,
    profile_document: Option<String>,
    profile_digest: Option<String>,
    runner_names: Option<Vec<String>>,
    source_type: Option<String>,
    trust_tier: Option<TrustTier>,
    catalog_kind: Option<String>,
    catalog_audience: Option<String>,
    catalog_visibility: Option<String>,
    source_metadata: Option<RegistrySourceMetadata>,
    attestations: Option<Vec<RegistryAttestation>>,
    required_scopes: Option<Vec<String>>,
    runtime: Option<JsonValue>,
    auth: Option<JsonValue>,
    risk: Option<JsonValue>,
    runx: Option<JsonObject>,
    tags: Option<Vec<String>>,
    publisher: Option<RegistryPublisher>,
    created_at: Option<String>,
    updated_at: Option<String>,
}

struct BindingArtifact {
    digest: Option<String>,
    runner_names: Vec<String>,
    manifest: Option<SkillRunnerManifest>,
}

fn build_binding_artifact(
    skill: &ValidatedSkill,
    profile_document: Option<&str>,
) -> Result<BindingArtifact, LocalRegistryError> {
    let Some(profile_document) = profile_document else {
        return Ok(BindingArtifact {
            digest: None,
            runner_names: Vec::new(),
            manifest: None,
        });
    };
    let manifest = validate_runner_manifest(parse_runner_manifest_yaml(profile_document)?)?;
    if let Some(manifest_skill) = &manifest.skill {
        if manifest_skill != &skill.name {
            return Err(LocalRegistryError::InvalidVersionPayload {
                field: "profile_document.skill".to_owned(),
                message: format!(
                    "runner manifest skill '{manifest_skill}' does not match skill '{}'",
                    skill.name
                ),
            });
        }
    }
    Ok(BindingArtifact {
        digest: Some(sha256_hex(profile_document)),
        runner_names: manifest.runners.keys().cloned().collect(),
        manifest: Some(manifest),
    })
}

fn default_registry_version_seed(markdown_digest: &str, profile_digest: Option<&str>) -> String {
    match profile_digest {
        Some(profile_digest) => sha256_hex(&format!(
            "{{\"markdown_digest\":\"{markdown_digest}\",\"profile_digest\":\"{profile_digest}\"}}"
        )),
        None => markdown_digest.to_owned(),
    }
}

fn default_registry_publisher(owner: &str) -> RegistryPublisher {
    RegistryPublisher {
        kind: if owner == "runx" {
            "organization".to_owned()
        } else {
            "publisher".to_owned()
        },
        id: owner.to_owned(),
        handle: Some(owner.to_owned()),
        display_name: None,
    }
}

fn derive_registry_trust_tier(owner: &str, trust_tier: Option<&TrustTier>) -> TrustTier {
    trust_tier.cloned().unwrap_or(if owner == "runx" {
        TrustTier::FirstParty
    } else {
        TrustTier::Community
    })
}

fn build_publisher_attestations(
    publisher: &RegistryPublisher,
    trust_tier: &TrustTier,
    issued_at: &str,
) -> Vec<RegistryAttestation> {
    let label = publisher
        .display_name
        .as_ref()
        .or(publisher.handle.as_ref())
        .unwrap_or(&publisher.id);
    let mut metadata = JsonObject::new();
    metadata.insert(
        "publisher_id".to_owned(),
        JsonValue::String(publisher.id.clone()),
    );
    metadata.insert(
        "publisher_kind".to_owned(),
        JsonValue::String(publisher.kind.clone()),
    );
    if let Some(handle) = &publisher.handle {
        metadata.insert(
            "publisher_handle".to_owned(),
            JsonValue::String(handle.clone()),
        );
    }
    if let Some(display_name) = &publisher.display_name {
        metadata.insert(
            "publisher_display_name".to_owned(),
            JsonValue::String(display_name.clone()),
        );
    }
    metadata.insert(
        "trust_tier".to_owned(),
        JsonValue::String(trust_tier_string(trust_tier).to_owned()),
    );
    vec![RegistryAttestation {
        kind: "publisher".to_owned(),
        id: format!("publisher:{}", publisher.id),
        status: if *trust_tier == TrustTier::Community {
            "declared".to_owned()
        } else {
            "verified".to_owned()
        },
        summary: label.clone(),
        source: None,
        issued_at: Some(issued_at.to_owned()),
        metadata: Some(JsonValue::Object(metadata)),
    }]
}

fn build_source_attestations(
    source_metadata: Option<&RegistrySourceMetadata>,
    issued_at: &str,
) -> Vec<RegistryAttestation> {
    let Some(source_metadata) = source_metadata else {
        return Vec::new();
    };
    let mut metadata = JsonObject::new();
    metadata.insert(
        "repo".to_owned(),
        JsonValue::String(source_metadata.repo.clone()),
    );
    metadata.insert(
        "ref".to_owned(),
        JsonValue::String(source_metadata.r#ref.clone()),
    );
    metadata.insert(
        "sha".to_owned(),
        JsonValue::String(source_metadata.sha.clone()),
    );
    metadata.insert(
        "event".to_owned(),
        JsonValue::String(source_metadata.event.clone()),
    );
    metadata.insert(
        "skill_path".to_owned(),
        JsonValue::String(source_metadata.skill_path.clone()),
    );
    if let Some(profile_path) = &source_metadata.profile_path {
        metadata.insert(
            "profile_path".to_owned(),
            JsonValue::String(profile_path.clone()),
        );
    }
    vec![RegistryAttestation {
        kind: "source".to_owned(),
        id: format!("{}_source", source_metadata.provider),
        status: "verified".to_owned(),
        summary: format!(
            "{}:{}@{}",
            source_metadata.provider, source_metadata.repo, source_metadata.sha
        ),
        source: Some(source_metadata.repo_url.clone()),
        issued_at: Some(issued_at.to_owned()),
        metadata: Some(JsonValue::Object(metadata)),
    }]
}

fn merge_registry_attestations(groups: Vec<Vec<RegistryAttestation>>) -> Vec<RegistryAttestation> {
    let mut keys: Vec<String> = Vec::new();
    let mut merged: Vec<RegistryAttestation> = Vec::new();
    for attestation in groups.into_iter().flatten() {
        let key = format!("{}:{}", attestation.kind, attestation.id);
        if let Some(index) = keys.iter().position(|candidate| candidate == &key) {
            merged[index] = attestation;
        } else {
            keys.push(key);
            merged.push(attestation);
        }
    }
    merged
}

fn normalize_attestations(
    attestations: Vec<RegistryAttestation>,
    source_metadata: Option<&RegistrySourceMetadata>,
    publisher: &RegistryPublisher,
    trust_tier: &TrustTier,
    created_at: &str,
) -> Vec<RegistryAttestation> {
    merge_registry_attestations(vec![
        build_publisher_attestations(publisher, trust_tier, created_at),
        build_source_attestations(source_metadata, created_at),
        attestations,
    ])
}

fn derive_trust_signals(version: &RegistrySkillVersion) -> Vec<TrustSignal> {
    vec![
        digest_trust_signal(version),
        trust_tier_signal(version),
        publisher_trust_signal(version),
        provenance_trust_signal(version),
        source_type_trust_signal(version),
        scopes_trust_signal(version),
        runtime_trust_signal(version),
        runner_metadata_trust_signal(version),
    ]
}

fn digest_trust_signal(version: &RegistrySkillVersion) -> TrustSignal {
    TrustSignal {
        id: "digest".to_owned(),
        label: "Immutable digest".to_owned(),
        status: "verified".to_owned(),
        value: display_sha256(&version.digest),
    }
}

fn trust_tier_signal(version: &RegistrySkillVersion) -> TrustSignal {
    TrustSignal {
        id: "trust_tier".to_owned(),
        label: "Trust tier".to_owned(),
        status: if version.trust_tier == TrustTier::Community {
            "declared".to_owned()
        } else {
            "verified".to_owned()
        },
        value: trust_tier_string(&version.trust_tier).to_owned(),
    }
}

fn publisher_trust_signal(version: &RegistrySkillVersion) -> TrustSignal {
    let attestation = version
        .attestations
        .iter()
        .find(|attestation| attestation.kind == "publisher");
    TrustSignal {
        id: "publisher".to_owned(),
        label: "Publisher identity".to_owned(),
        status: attestation
            .map_or("not_declared", |attestation| attestation.status.as_str())
            .to_owned(),
        value: publisher_label(&version.publisher).to_owned(),
    }
}

fn provenance_trust_signal(version: &RegistrySkillVersion) -> TrustSignal {
    let provenance = source_provenance(version);
    TrustSignal {
        id: "provenance".to_owned(),
        label: "Source provenance".to_owned(),
        status: if provenance.is_some() {
            "verified".to_owned()
        } else {
            "not_declared".to_owned()
        },
        value: provenance.unwrap_or_else(|| "no source attestation".to_owned()),
    }
}

fn source_type_trust_signal(version: &RegistrySkillVersion) -> TrustSignal {
    TrustSignal {
        id: "source_type".to_owned(),
        label: "Execution source".to_owned(),
        status: "declared".to_owned(),
        value: version.source_type.clone(),
    }
}

fn scopes_trust_signal(version: &RegistrySkillVersion) -> TrustSignal {
    TrustSignal {
        id: "scopes".to_owned(),
        label: "Required scopes".to_owned(),
        status: declared_status(!version.required_scopes.is_empty()).to_owned(),
        value: if version.required_scopes.is_empty() {
            "none declared".to_owned()
        } else {
            version.required_scopes.join(", ")
        },
    }
}

fn runtime_trust_signal(version: &RegistrySkillVersion) -> TrustSignal {
    TrustSignal {
        id: "runtime".to_owned(),
        label: "Runtime requirements".to_owned(),
        status: declared_status(version.runtime.is_some()).to_owned(),
        value: if version.runtime.is_some() {
            "declared in skill metadata".to_owned()
        } else {
            "none declared".to_owned()
        },
    }
}

fn runner_metadata_trust_signal(version: &RegistrySkillVersion) -> TrustSignal {
    TrustSignal {
        id: "runner_metadata".to_owned(),
        label: "Materialized binding".to_owned(),
        status: if version.profile_digest.is_some() {
            "verified".to_owned()
        } else {
            "not_declared".to_owned()
        },
        value: runner_metadata_value(version),
    }
}

fn publisher_label(publisher: &RegistryPublisher) -> &str {
    publisher
        .display_name
        .as_ref()
        .or(publisher.handle.as_ref())
        .unwrap_or(&publisher.id)
}

fn runner_metadata_value(version: &RegistrySkillVersion) -> String {
    version.profile_digest.as_ref().map_or_else(
        || "portable agent runner".to_owned(),
        |digest| {
            format!(
                "{} runner(s), binding {}",
                version.runner_names.len(),
                display_sha256(digest)
            )
        },
    )
}

fn declared_status(is_declared: bool) -> &'static str {
    if is_declared {
        "declared"
    } else {
        "not_declared"
    }
}

fn source_provenance(version: &RegistrySkillVersion) -> Option<String> {
    if let Some(source_metadata) = &version.source_metadata {
        return Some(format!(
            "{}:{}@{}",
            source_metadata.provider, source_metadata.repo, source_metadata.sha
        ));
    }
    version
        .attestations
        .iter()
        .find(|attestation| attestation.kind == "source")
        .map(|attestation| attestation.summary.clone())
}

fn search_result_for_version(
    version: &RegistrySkillVersion,
    registry_url: Option<&str>,
) -> RegistrySearchResult {
    let link = runx_link_for_version(version, registry_url);
    RegistrySearchResult {
        skill_id: version.skill_id.clone(),
        name: version.name.clone(),
        summary: version.description.clone(),
        owner: version.owner.clone(),
        version: Some(version.version.clone()),
        digest: Some(version.digest.clone()),
        source: Some("runx-registry".to_owned()),
        source_label: Some("runx registry".to_owned()),
        source_type: version.source_type.clone(),
        profile_mode: if version.profile_document.is_some() {
            ProfileMode::Profiled
        } else {
            ProfileMode::Portable
        },
        runner_names: version.runner_names.clone(),
        profile_digest: version.profile_digest.clone(),
        profile_trust_tier: version
            .profile_document
            .as_ref()
            .map(|_| version.trust_tier.clone()),
        required_scopes: version.required_scopes.clone(),
        tags: version.tags.clone(),
        trust_tier: version.trust_tier.clone(),
        trust_signals: derive_trust_signals(version),
        install_command: link.install_command,
        run_command: link.run_command,
    }
}

fn detail_for_version(
    version: &RegistrySkillVersion,
    registry_url: Option<&str>,
) -> RegistrySkillDetail {
    let link = runx_link_for_version(version, registry_url);
    RegistrySkillDetail {
        skill_id: version.skill_id.clone(),
        owner: version.owner.clone(),
        name: version.name.clone(),
        description: version.description.clone(),
        version: version.version.clone(),
        digest: version.digest.clone(),
        markdown: version.markdown.clone(),
        profile_digest: version.profile_digest.clone(),
        runner_names: version.runner_names.clone(),
        source_type: version.source_type.clone(),
        trust_tier: version.trust_tier.clone(),
        required_scopes: version.required_scopes.clone(),
        tags: version.tags.clone(),
        publisher: version.publisher.clone(),
        source_metadata: version.source_metadata.clone(),
        attestations: version.attestations.clone(),
        install_command: link.install_command,
        run_command: link.run_command,
    }
}

fn resolve_by_name(
    store: &FileRegistryStore,
    name: &str,
    version: Option<&str>,
) -> Result<Option<RegistrySkillVersion>, LocalRegistryError> {
    let normalized = slugify(name)?;
    let matches = store
        .list_skills()?
        .into_iter()
        .filter(|skill| {
            skill.name == normalized || skill.skill_id.ends_with(&format!("/{normalized}"))
        })
        .collect::<Vec<_>>();
    match matches.len() {
        0 => Ok(None),
        1 => store.get_version(&matches[0].skill_id, version),
        _ => Err(LocalRegistryError::Ambiguous(name.to_owned())),
    }
}

fn searchable_text(version: &RegistrySkillVersion) -> String {
    let mut parts = vec![
        version.skill_id.clone(),
        version.name.clone(),
        version.owner.clone(),
        version.source_type.clone(),
    ];
    if let Some(description) = &version.description {
        parts.push(description.clone());
    }
    parts.extend(version.runner_names.clone());
    parts.extend(version.tags.clone());
    normalize(&parts.join(" "))
}

fn normalize(value: &str) -> String {
    value.trim().to_lowercase()
}

fn extract_scopes(skill: &ValidatedSkill) -> Vec<String> {
    unique(
        record_array_field(skill.auth.as_ref(), "scopes")
            .into_iter()
            .chain(record_array_field_from_object(
                skill.runx.as_ref(),
                "scopes",
            ))
            .collect(),
    )
}

fn extract_runner_scopes(manifest: Option<&SkillRunnerManifest>) -> Vec<String> {
    let Some(manifest) = manifest else {
        return Vec::new();
    };
    unique(
        manifest
            .runners
            .values()
            .flat_map(|runner| {
                record_array_field(runner.auth.as_ref(), "scopes")
                    .into_iter()
                    .chain(record_array_field_from_object(
                        runner.runx.as_ref(),
                        "scopes",
                    ))
            })
            .collect(),
    )
}

fn extract_runner_runtime(manifest: Option<&SkillRunnerManifest>) -> Option<JsonValue> {
    let manifest = manifest?;
    let runners = manifest
        .runners
        .values()
        .filter(|runner| runner.runtime.is_some())
        .map(|runner| JsonValue::String(runner.name.clone()))
        .collect::<Vec<_>>();
    if runners.is_empty() {
        None
    } else {
        Some(JsonValue::Object(
            [("runners".to_owned(), JsonValue::Array(runners))].into(),
        ))
    }
}

fn extract_runner_tags(manifest: Option<&SkillRunnerManifest>) -> Vec<String> {
    let Some(manifest) = manifest else {
        return Vec::new();
    };
    unique(
        manifest
            .runners
            .values()
            .flat_map(|runner| record_array_field_from_object(runner.runx.as_ref(), "tags"))
            .collect(),
    )
}

fn extract_tags(skill: &ValidatedSkill) -> Vec<String> {
    unique(record_array_field_from_object(skill.runx.as_ref(), "tags"))
}

fn record_array_field(value: Option<&JsonValue>, field: &str) -> Vec<String> {
    let Some(JsonValue::Object(record)) = value else {
        return Vec::new();
    };
    record_array_field_from_object(Some(record), field)
}

fn record_array_field_from_object(value: Option<&JsonObject>, field: &str) -> Vec<String> {
    let Some(record) = value else {
        return Vec::new();
    };
    let Some(JsonValue::Array(values)) = record.get(field) else {
        return Vec::new();
    };
    values
        .iter()
        .filter_map(|value| match value {
            JsonValue::String(value) if !value.is_empty() => Some(value.clone()),
            _ => None,
        })
        .collect()
}

fn record_field(value: Option<&JsonObject>, field: &str) -> Option<JsonValue> {
    value.and_then(|record| record.get(field).cloned())
}

fn unique(values: Vec<String>) -> Vec<String> {
    let mut unique_values = Vec::new();
    for value in values {
        if !unique_values.contains(&value) {
            unique_values.push(value);
        }
    }
    unique_values
}

fn validate_publisher(
    publisher: RegistryPublisher,
    label: &str,
) -> Result<RegistryPublisher, LocalRegistryError> {
    if !matches!(
        publisher.kind.as_str(),
        "organization" | "user" | "team" | "service" | "publisher"
    ) {
        return Err(LocalRegistryError::InvalidVersionPayload {
            field: format!("{label}.kind"),
            message: "must be one of organization, user, team, service, or publisher".to_owned(),
        });
    }
    if publisher.id.is_empty() {
        return Err(LocalRegistryError::InvalidVersionPayload {
            field: format!("{label}.id"),
            message: "must be a non-empty string".to_owned(),
        });
    }
    Ok(publisher)
}

fn validate_source_metadata(
    source_metadata: RegistrySourceMetadata,
) -> Result<RegistrySourceMetadata, LocalRegistryError> {
    if source_metadata.provider != "github" {
        return Err(LocalRegistryError::InvalidVersionPayload {
            field: "registry_version.source_metadata.provider".to_owned(),
            message: "must be github".to_owned(),
        });
    }
    if !matches!(
        source_metadata.event.as_str(),
        "enrollment" | "push" | "tag" | "tombstone"
    ) {
        return Err(LocalRegistryError::InvalidVersionPayload {
            field: "registry_version.source_metadata.event".to_owned(),
            message: "must be one of enrollment, push, tag, or tombstone".to_owned(),
        });
    }
    Ok(source_metadata)
}

fn required_string(value: Option<String>, field: &str) -> Result<String, LocalRegistryError> {
    match value {
        Some(value) if !value.is_empty() => Ok(value),
        _ => Err(missing_field(field)),
    }
}

fn missing_field(field: &str) -> LocalRegistryError {
    LocalRegistryError::InvalidVersionPayload {
        field: field.to_owned(),
        message: "missing required field".to_owned(),
    }
}

fn safe_read_dir_names(path: &Path) -> Result<Vec<String>, LocalRegistryError> {
    match fs::read_dir(path) {
        Ok(entries) => entries
            .map(|entry| {
                let entry = entry.map_err(|source| io_error("reading", path, source))?;
                Ok(entry.file_name().to_string_lossy().into_owned())
            })
            .collect(),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(source) => Err(io_error("reading", path, source)),
    }
}

fn write_registry_json(
    path: &Path,
    version: &RegistrySkillVersion,
    create_new: bool,
) -> Result<(), LocalRegistryError> {
    let mut contents =
        serde_json::to_string_pretty(version).map_err(|source| LocalRegistryError::JsonWrite {
            path: path.to_path_buf(),
            source,
        })?;
    contents.push('\n');

    let mut options = fs::OpenOptions::new();
    options.write(true);
    if create_new {
        options.create_new(true);
    } else {
        options.create(true).truncate(true);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(path)
        .map_err(|source| io_error("writing", path, source))?;
    file.write_all(contents.as_bytes())
        .map_err(|source| io_error("writing", path, source))
}

fn io_error(action: &'static str, path: &Path, source: io::Error) -> LocalRegistryError {
    LocalRegistryError::Io {
        action,
        path: path.to_path_buf(),
        source,
    }
}

fn encode_part(value: &str) -> String {
    encode_uri_component(value)
}

fn decode_part(value: &str) -> Result<String, LocalRegistryError> {
    let decoded =
        percent_decode(value).map_err(|message| LocalRegistryError::InvalidVersionPayload {
            field: "registry_path".to_owned(),
            message,
        })?;
    if is_unsafe_path_component(&decoded) {
        return Err(LocalRegistryError::UnsafePathComponent(decoded));
    }
    Ok(decoded)
}

fn is_unsafe_path_component(value: &str) -> bool {
    matches!(value, "." | "..") || value.contains('/') || value.contains('\\')
}

fn reject_unsafe_path_component(value: &str) -> Result<(), LocalRegistryError> {
    if is_unsafe_path_component(value) {
        Err(LocalRegistryError::UnsafePathComponent(value.to_owned()))
    } else {
        Ok(())
    }
}

fn encode_uri_component(value: &str) -> String {
    let mut output = String::new();
    for byte in value.bytes() {
        let keep = byte.is_ascii_alphanumeric()
            || matches!(
                byte,
                b'-' | b'_' | b'.' | b'!' | b'~' | b'*' | b'\'' | b'(' | b')'
            );
        if keep {
            output.push(char::from(byte));
        } else {
            output.push_str(&format!("%{byte:02X}"));
        }
    }
    output
}

fn percent_decode(value: &str) -> Result<String, String> {
    let mut decoded = Vec::with_capacity(value.len());
    let bytes = value.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            if index + 2 >= bytes.len() {
                return Err(format!("invalid percent encoding in '{value}'"));
            }
            let Some(high) = hex_value(bytes[index + 1]) else {
                return Err(format!("invalid percent encoding in '{value}'"));
            };
            let Some(low) = hex_value(bytes[index + 2]) else {
                return Err(format!("invalid percent encoding in '{value}'"));
            };
            decoded.push((high << 4) | low);
            index += 3;
            continue;
        }
        decoded.push(bytes[index]);
        index += 1;
    }
    String::from_utf8(decoded).map_err(|error| error.to_string())
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn sha256_hex(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        hex.push_str(&format!("{byte:02x}"));
    }
    hex
}

fn display_sha256(digest: &str) -> String {
    if digest.starts_with("sha256:") {
        digest.to_owned()
    } else {
        format!("sha256:{digest}")
    }
}

fn trust_tier_string(value: &TrustTier) -> &'static str {
    match value {
        TrustTier::FirstParty => "first_party",
        TrustTier::Verified => "verified",
        TrustTier::Community => "community",
    }
}

fn now_iso8601() -> String {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let seconds = i64::try_from(duration.as_secs()).unwrap_or(i64::MAX);
    let millis = duration.subsec_millis();
    let (year, month, day, hour, minute, second) = civil_from_unix_seconds(seconds);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{millis:03}Z")
}

fn civil_from_unix_seconds(seconds: i64) -> (i32, u32, u32, u32, u32, u32) {
    let days = seconds.div_euclid(86_400);
    let day_seconds = seconds.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    let hour = u32::try_from(day_seconds / 3_600).unwrap_or(0);
    let minute = u32::try_from((day_seconds % 3_600) / 60).unwrap_or(0);
    let second = u32::try_from(day_seconds % 60).unwrap_or(0);
    (year, month, day, hour, minute, second)
}

fn civil_from_days(days_since_epoch: i64) -> (i32, u32, u32) {
    let z = days_since_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 }.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096).div_euclid(365);
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2).div_euclid(153);
    let day = doy - (153 * mp + 2).div_euclid(5) + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    let year = y + i64::from(month <= 2);
    (
        i32::try_from(year).unwrap_or(i32::MAX),
        u32::try_from(month).unwrap_or(1),
        u32::try_from(day).unwrap_or(1),
    )
}
