// rust-style-allow: large-file because the native registry command keeps local
// and hosted registry routing, output envelopes, and install/publish wiring in
// one audited CLI boundary.
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use runx_runtime::registry::{
    AcquireOptions, FileRegistryStore, IngestSkillOptions, InstallCandidate,
    InstallLocalSkillOptions, LocalRegistryClient, PublishSkillMarkdownOptions, RegistryClient,
    RegistryResolveOptions, RegistrySearchOptions, RegistrySkillResolution, install_local_skill,
    publish_skill_markdown, read_registry_skill, resolve_registry_skill,
    search_registry_with_options,
};

#[derive(Debug, Eq, PartialEq)]
pub enum RegistryAction {
    Search,
    Read,
    Resolve,
    Install,
    Publish,
}

#[derive(Debug, Eq, PartialEq)]
pub struct RegistryPlan {
    pub action: RegistryAction,
    pub subject: String,
    pub registry: Option<String>,
    pub registry_dir: Option<PathBuf>,
    pub version: Option<String>,
    pub expected_digest: Option<String>,
    pub destination: Option<PathBuf>,
    pub installation_id: Option<String>,
    pub owner: Option<String>,
    pub profile: Option<PathBuf>,
    pub limit: Option<usize>,
    pub upsert: bool,
    pub json: bool,
}

pub fn run_native_registry(plan: RegistryPlan) -> ExitCode {
    match run_registry(plan) {
        Ok(output) => write_stdout(&output.stdout, output.exit_code),
        Err(error) => {
            let _ignored = write_stderr(&format!("\n  ✗  {}\n\n", error.message));
            ExitCode::from(error.exit_code)
        }
    }
}

struct RegistryCliOutput {
    stdout: String,
    exit_code: u8,
}

fn run_registry(plan: RegistryPlan) -> Result<RegistryCliOutput, RegistryCliError> {
    let env = env_map();
    let cwd = env::current_dir().map_err(|error| internal_error(error.to_string()))?;
    let target = resolve_registry_target(&plan, &env, &cwd);
    match plan.action {
        RegistryAction::Search => run_search(plan, target),
        RegistryAction::Read => run_read(plan, target),
        RegistryAction::Resolve => run_resolve(plan, target),
        RegistryAction::Install => run_install(plan, target, &env, &cwd),
        RegistryAction::Publish => run_publish(plan, target, &env, &cwd),
    }
}

fn run_search(
    plan: RegistryPlan,
    target: RegistryTarget,
) -> Result<RegistryCliOutput, RegistryCliError> {
    let source = target.label();
    let query = plan.subject;
    let results = match target {
        RegistryTarget::Remote { registry_url } => RegistryClient::new(&registry_url)?
            .search_with_limit(&query, plan.limit.unwrap_or(20))?,
        RegistryTarget::Local {
            registry_path,
            registry_url,
        } => search_registry_with_options(
            &FileRegistryStore::new(registry_path),
            &query,
            RegistrySearchOptions {
                limit: plan.limit,
                registry_url,
            },
        )?,
    };
    write_output(
        plan.json,
        &RegistryEnvelope {
            status: "success",
            registry: RegistryPayload::Search {
                source,
                query: query.clone(),
                results,
            },
        },
        || render_search(&query, source),
    )
}

fn run_read(
    plan: RegistryPlan,
    target: RegistryTarget,
) -> Result<RegistryCliOutput, RegistryCliError> {
    let source = target.label();
    let skill = match target {
        RegistryTarget::Remote { registry_url } => RegistryClient::new(&registry_url)?
            .read(&plan.subject, plan.version.as_deref())?
            .ok_or_else(|| not_found(&plan.subject))?,
        RegistryTarget::Local {
            registry_path,
            registry_url,
        } => read_registry_skill(
            &FileRegistryStore::new(registry_path),
            &plan.subject,
            plan.version.as_deref(),
            registry_url.as_deref(),
        )?
        .ok_or_else(|| not_found(&plan.subject))?,
    };
    write_output(
        plan.json,
        &RegistryEnvelope {
            status: "success",
            registry: RegistryPayload::Read {
                source,
                r#ref: plan.subject,
                skill: Box::new(skill),
            },
        },
        || "\n  registry read  success\n\n".to_owned(),
    )
}

fn run_resolve(
    plan: RegistryPlan,
    target: RegistryTarget,
) -> Result<RegistryCliOutput, RegistryCliError> {
    let source = target.label();
    let resolution = match target {
        RegistryTarget::Remote { registry_url } => {
            let resolved = RegistryClient::new(&registry_url)?
                .resolve_ref(&plan.subject, plan.version.as_deref())?
                .ok_or_else(|| not_found(&plan.subject))?;
            RemoteOrLocalResolution::Remote(Box::new(resolved))
        }
        RegistryTarget::Local {
            registry_path,
            registry_url,
        } => RemoteOrLocalResolution::Local(Box::new(
            resolve_registry_skill(
                &FileRegistryStore::new(registry_path),
                &plan.subject,
                RegistryResolveOptions {
                    version: plan.version,
                    registry_url,
                },
            )?
            .ok_or_else(|| not_found(&plan.subject))?,
        )),
    };
    write_output(
        plan.json,
        &RegistryEnvelope {
            status: "success",
            registry: RegistryPayload::Resolve {
                source,
                r#ref: plan.subject,
                resolution,
            },
        },
        || "\n  registry resolve  success\n\n".to_owned(),
    )
}

fn run_install(
    plan: RegistryPlan,
    target: RegistryTarget,
    env: &BTreeMap<String, String>,
    cwd: &Path,
) -> Result<RegistryCliOutput, RegistryCliError> {
    let source = target.label();
    let (candidate, acquisition) = install_candidate(&plan, target, env)?;
    let install = install_local_skill(
        &candidate,
        &InstallLocalSkillOptions {
            destination_root: destination_root(&plan, env, cwd),
            expected_digest: plan.expected_digest,
        },
    )?;
    let receipt_metadata = runx_runtime::registry_install_receipt_metadata(
        runx_runtime::RegistryInstallMetadataInput {
            candidate: &candidate,
            install: &install,
            acquisition: acquisition.as_ref(),
        },
    );
    write_output(
        plan.json,
        &RegistryEnvelope {
            status: "success",
            registry: RegistryPayload::Install {
                source,
                r#ref: plan.subject,
                install: Box::new(install),
                receipt_metadata,
            },
        },
        || "\n  registry install  success\n\n".to_owned(),
    )
}

fn run_publish(
    plan: RegistryPlan,
    target: RegistryTarget,
    env: &BTreeMap<String, String>,
    cwd: &Path,
) -> Result<RegistryCliOutput, RegistryCliError> {
    let RegistryTarget::Local {
        registry_path,
        registry_url,
    } = target
    else {
        return Err(usage_error(
            "remote registry publish is not supported from the native OSS CLI",
        ));
    };
    let package = read_skill_package(&plan.subject, plan.profile.as_deref(), env, cwd)?;
    let result = publish_skill_markdown(
        &LocalRegistryClient::new(FileRegistryStore::new(registry_path)),
        &package.markdown,
        PublishSkillMarkdownOptions {
            ingest: IngestSkillOptions {
                owner: plan.owner,
                version: plan.version,
                profile_document: package.profile_document,
                upsert: plan.upsert,
                ..IngestSkillOptions::default()
            },
            registry_url,
        },
    )?;
    write_output(
        plan.json,
        &RegistryEnvelope {
            status: "success",
            registry: RegistryPayload::Publish {
                publish: Box::new(result),
            },
        },
        || "\n  registry publish  success\n\n".to_owned(),
    )
}

fn install_candidate(
    plan: &RegistryPlan,
    target: RegistryTarget,
    env: &BTreeMap<String, String>,
) -> Result<
    (
        InstallCandidate,
        Option<runx_runtime::registry::AcquiredRegistrySkill>,
    ),
    RegistryCliError,
> {
    match target {
        RegistryTarget::Remote { registry_url } => {
            let installation_id = plan
                .installation_id
                .as_deref()
                .or_else(|| env.get("RUNX_INSTALLATION_ID").map(String::as_str))
                .ok_or_else(|| usage_error("remote registry install requires --installation-id"))?;
            let acquired = RegistryClient::new(&registry_url)?.acquire(
                &plan.subject,
                AcquireOptions {
                    installation_id,
                    version: plan.version.as_deref(),
                    channel: Some("cli"),
                },
            )?;
            Ok((
                candidate_from_acquired(&plan.subject, &acquired),
                Some(acquired),
            ))
        }
        RegistryTarget::Local {
            registry_path,
            registry_url,
        } => {
            let resolution = resolve_registry_skill(
                &FileRegistryStore::new(registry_path),
                &plan.subject,
                RegistryResolveOptions {
                    version: plan.version.clone(),
                    registry_url,
                },
            )?
            .ok_or_else(|| not_found(&plan.subject))?;
            Ok((candidate_from_resolution(&plan.subject, resolution), None))
        }
    }
}

fn candidate_from_resolution(
    registry_ref: &str,
    resolution: RegistrySkillResolution,
) -> InstallCandidate {
    InstallCandidate {
        markdown: resolution.markdown,
        profile_document: resolution.profile_document,
        source: resolution.source,
        source_label: resolution.source_label,
        r#ref: registry_ref.to_owned(),
        skill_id: Some(resolution.skill_id),
        version: Some(resolution.version),
        digest: Some(resolution.digest),
        profile_digest: resolution.profile_digest,
        runner_names: resolution.runner_names,
        trust_tier: Some(resolution.trust_tier),
    }
}

fn candidate_from_acquired(
    registry_ref: &str,
    acquired: &runx_runtime::registry::AcquiredRegistrySkill,
) -> InstallCandidate {
    InstallCandidate {
        markdown: acquired.markdown.clone(),
        profile_document: acquired.profile_document.clone(),
        source: "runx-registry".to_owned(),
        source_label: "runx registry".to_owned(),
        r#ref: registry_ref.to_owned(),
        skill_id: Some(acquired.skill_id.clone()),
        version: Some(acquired.version.clone()),
        digest: Some(acquired.digest.clone()),
        profile_digest: acquired.profile_digest.clone(),
        runner_names: acquired.runner_names.clone(),
        trust_tier: Some(acquired.trust_tier.clone()),
    }
}

#[derive(Clone, Debug)]
enum RegistryTarget {
    Remote {
        registry_url: String,
    },
    Local {
        registry_path: PathBuf,
        registry_url: Option<String>,
    },
}

impl RegistryTarget {
    fn label(&self) -> &'static str {
        match self {
            Self::Remote { .. } => "remote",
            Self::Local { .. } => "local",
        }
    }
}

fn resolve_registry_target(
    plan: &RegistryPlan,
    env: &BTreeMap<String, String>,
    cwd: &Path,
) -> RegistryTarget {
    let configured_registry = plan
        .registry
        .as_deref()
        .or_else(|| env.get("RUNX_REGISTRY_URL").map(String::as_str));
    if let Some(registry) = &plan.registry {
        if is_remote_registry_url(registry) {
            return RegistryTarget::Remote {
                registry_url: registry.clone(),
            };
        }
        return RegistryTarget::Local {
            registry_path: registry_path_from_value(registry, env, cwd),
            registry_url: env
                .get("RUNX_REGISTRY_URL")
                .filter(|value| is_remote_registry_url(value))
                .cloned(),
        };
    }
    if let Some(registry_dir) = &plan.registry_dir {
        return RegistryTarget::Local {
            registry_path: resolve_path(registry_dir, env, cwd, false),
            registry_url: configured_registry
                .filter(|value| is_remote_registry_url(value))
                .map(ToOwned::to_owned),
        };
    }
    if let Some(registry_dir) = env.get("RUNX_REGISTRY_DIR") {
        return RegistryTarget::Local {
            registry_path: runx_runtime::resolve_path_from_user_input(
                registry_dir,
                env,
                cwd,
                false,
            ),
            registry_url: configured_registry
                .filter(|value| is_remote_registry_url(value))
                .map(ToOwned::to_owned),
        };
    }
    if let Some(registry) = configured_registry.filter(|value| is_remote_registry_url(value)) {
        return RegistryTarget::Remote {
            registry_url: registry.to_owned(),
        };
    }
    RegistryTarget::Local {
        registry_path: runx_runtime::resolve_runx_global_home_dir(env, cwd).join("registry"),
        registry_url: configured_registry.map(ToOwned::to_owned),
    }
}

fn registry_path_from_value(value: &str, env: &BTreeMap<String, String>, cwd: &Path) -> PathBuf {
    if let Some(path) = value.strip_prefix("file://") {
        return PathBuf::from(path);
    }
    runx_runtime::resolve_path_from_user_input(value, env, cwd, false)
}

fn destination_root(plan: &RegistryPlan, env: &BTreeMap<String, String>, cwd: &Path) -> PathBuf {
    plan.destination
        .as_ref()
        .map(|path| resolve_path(path, env, cwd, false))
        .unwrap_or_else(|| workspace_base(env, cwd).join("skills"))
}

fn read_skill_package(
    subject: &str,
    profile: Option<&Path>,
    env: &BTreeMap<String, String>,
    cwd: &Path,
) -> Result<SkillPackage, RegistryCliError> {
    let subject_path = runx_runtime::resolve_path_from_user_input(subject, env, cwd, true);
    let metadata = fs::metadata(&subject_path).map_err(|error| RegistryCliError {
        message: format!(
            "failed to read skill package {}: {error}",
            subject_path.display()
        ),
        exit_code: 1,
    })?;
    let markdown_path = if metadata.is_dir() {
        subject_path.join("SKILL.md")
    } else {
        subject_path.clone()
    };
    let markdown = fs::read_to_string(&markdown_path).map_err(|error| RegistryCliError {
        message: format!(
            "failed to read skill markdown {}: {error}",
            markdown_path.display()
        ),
        exit_code: 1,
    })?;
    let profile_path = profile
        .map(|path| resolve_path(path, env, cwd, true))
        .or_else(|| {
            let candidate = markdown_path.parent()?.join("X.yaml");
            candidate.exists().then_some(candidate)
        });
    let profile_document = match profile_path {
        Some(path) => Some(fs::read_to_string(&path).map_err(|error| RegistryCliError {
            message: format!("failed to read skill profile {}: {error}", path.display()),
            exit_code: 1,
        })?),
        None => None,
    };
    Ok(SkillPackage {
        markdown,
        profile_document,
    })
}

struct SkillPackage {
    markdown: String,
    profile_document: Option<String>,
}

#[derive(serde::Serialize)]
struct RegistryEnvelope<T> {
    status: &'static str,
    registry: T,
}

#[derive(serde::Serialize)]
#[serde(tag = "action", rename_all = "snake_case")]
enum RegistryPayload {
    Search {
        source: &'static str,
        query: String,
        results: Vec<runx_runtime::registry::RegistrySearchResult>,
    },
    Read {
        source: &'static str,
        r#ref: String,
        skill: Box<runx_runtime::registry::RegistrySkillDetail>,
    },
    Resolve {
        source: &'static str,
        r#ref: String,
        resolution: RemoteOrLocalResolution,
    },
    Install {
        source: &'static str,
        r#ref: String,
        install: Box<runx_runtime::registry::InstallLocalSkillResult>,
        receipt_metadata: runx_contracts::JsonObject,
    },
    Publish {
        publish: Box<runx_runtime::registry::PublishSkillMarkdownResult>,
    },
}

#[derive(serde::Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum RemoteOrLocalResolution {
    Remote(Box<runx_runtime::registry::ResolvedRegistryRef>),
    Local(Box<RegistrySkillResolution>),
}

fn write_output<T: serde::Serialize>(
    json: bool,
    value: &T,
    human: impl FnOnce() -> String,
) -> Result<RegistryCliOutput, RegistryCliError> {
    let stdout = if json {
        serde_json::to_string_pretty(value)
            .map(|json| format!("{json}\n"))
            .map_err(|error| internal_error(error.to_string()))?
    } else {
        human()
    };
    Ok(RegistryCliOutput {
        stdout,
        exit_code: 0,
    })
}

fn render_search(query: &str, source: &str) -> String {
    format!("\n  registry search  {query}\n  source           {source}\n\n")
}

fn resolve_path(
    path: &Path,
    env: &BTreeMap<String, String>,
    cwd: &Path,
    prefer_existing: bool,
) -> PathBuf {
    if path.is_absolute() {
        return path.to_path_buf();
    }
    runx_runtime::resolve_path_from_user_input(
        &path.display().to_string(),
        env,
        cwd,
        prefer_existing,
    )
}

fn workspace_base(env: &BTreeMap<String, String>, cwd: &Path) -> PathBuf {
    env.get("RUNX_CWD")
        .map(PathBuf::from)
        .or_else(|| find_workspace_root(cwd))
        .or_else(|| env.get("INIT_CWD").map(PathBuf::from))
        .unwrap_or_else(|| cwd.to_path_buf())
}

fn find_workspace_root(start: &Path) -> Option<PathBuf> {
    let mut current = start.to_path_buf();
    loop {
        if current.join("pnpm-workspace.yaml").exists() {
            return Some(current);
        }
        if !current.pop() {
            return None;
        }
    }
}

fn is_remote_registry_url(value: &str) -> bool {
    value.starts_with("https://") || value.starts_with("http://")
}

fn env_map() -> BTreeMap<String, String> {
    env::vars().collect()
}

fn write_stdout(value: &str, code: u8) -> ExitCode {
    match io::stdout().write_all(value.as_bytes()) {
        Ok(()) => ExitCode::from(code),
        Err(_) => ExitCode::from(1),
    }
}

fn write_stderr(value: &str) -> io::Result<()> {
    io::stderr().write_all(value.as_bytes())
}

struct RegistryCliError {
    message: String,
    exit_code: u8,
}

fn usage_error(message: impl Into<String>) -> RegistryCliError {
    RegistryCliError {
        message: message.into(),
        exit_code: 64,
    }
}

fn internal_error(message: impl Into<String>) -> RegistryCliError {
    RegistryCliError {
        message: message.into(),
        exit_code: 1,
    }
}

fn not_found(registry_ref: &str) -> RegistryCliError {
    RegistryCliError {
        message: format!("registry skill not found: {registry_ref}"),
        exit_code: 1,
    }
}

impl From<runx_runtime::registry::RegistryClientError> for RegistryCliError {
    fn from(error: runx_runtime::registry::RegistryClientError) -> Self {
        internal_error(error.to_string())
    }
}

impl From<runx_runtime::registry::RegistryResolveError> for RegistryCliError {
    fn from(error: runx_runtime::registry::RegistryResolveError) -> Self {
        internal_error(error.to_string())
    }
}

impl From<runx_runtime::registry::LocalRegistryError> for RegistryCliError {
    fn from(error: runx_runtime::registry::LocalRegistryError) -> Self {
        internal_error(error.to_string())
    }
}

impl From<runx_runtime::registry::InstallError> for RegistryCliError {
    fn from(error: runx_runtime::registry::InstallError) -> Self {
        internal_error(error.to_string())
    }
}
