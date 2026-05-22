// rust-style-allow: large-file because sandbox planning owns both process and
// MCP environment/cwd policy plus the audit metadata emitted with each plan.
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use runx_contracts::{JsonObject, JsonValue};
use runx_parser::{SkillMcpServer, SkillSandbox, SkillSource};

use crate::RuntimeError;
use crate::receipts::paths::{INIT_CWD_ENV, RUNX_CWD_ENV};

const DEFAULT_ENV_ALLOWLIST: [&str; 9] = [
    "PATH",
    "HOME",
    "TMPDIR",
    "TMP",
    "TEMP",
    "SystemRoot",
    "WINDIR",
    "COMSPEC",
    "PATHEXT",
];
const MAX_INLINE_INPUTS_BYTES: usize = 48 * 1024;
const MAX_INLINE_INPUT_VALUE_BYTES: usize = 8 * 1024;

#[derive(Clone, Debug, PartialEq)]
pub struct SandboxPlan {
    pub command: String,
    pub args: Vec<String>,
    pub cwd: PathBuf,
    pub env: BTreeMap<String, String>,
    pub metadata: JsonObject,
}

pub fn prepare_process_sandbox(
    source: &SkillSource,
    skill_directory: &Path,
    inputs: &JsonObject,
    base_env: &BTreeMap<String, String>,
) -> Result<SandboxPlan, RuntimeError> {
    let command = source.command.clone().ok_or(RuntimeError::MissingCommand)?;
    let sandbox = source.sandbox.as_ref();
    validate_sandbox(sandbox)?;
    let workspace_cwd = workspace_cwd(base_env)?;
    let cwd = resolve_cwd(source, sandbox, skill_directory, workspace_cwd.as_deref())?;
    let env = child_env(sandbox, base_env, inputs)?;
    let args = source
        .args
        .iter()
        .map(|arg| resolve_template(arg, inputs))
        .collect();
    Ok(SandboxPlan {
        command,
        args,
        cwd,
        env,
        metadata: sandbox_metadata(sandbox),
    })
}

pub fn prepare_mcp_process_sandbox(
    source: &SkillSource,
    server: &SkillMcpServer,
    skill_directory: &Path,
    base_env: &BTreeMap<String, String>,
) -> Result<SandboxPlan, RuntimeError> {
    let sandbox = source.sandbox.as_ref();
    validate_sandbox(sandbox)?;
    let workspace_cwd = workspace_cwd(base_env)?;
    let cwd = resolve_cwd_value(
        server.cwd.as_deref(),
        sandbox,
        skill_directory,
        workspace_cwd.as_deref(),
    )?;
    let env = child_base_env(sandbox, base_env)?;
    Ok(SandboxPlan {
        command: server.command.clone(),
        args: server.args.clone(),
        cwd,
        env,
        metadata: sandbox_metadata(sandbox),
    })
}

fn resolve_cwd(
    source: &SkillSource,
    sandbox: Option<&SkillSandbox>,
    skill_directory: &Path,
    workspace_cwd: Option<&Path>,
) -> Result<PathBuf, RuntimeError> {
    resolve_cwd_value(
        source.cwd.as_deref(),
        sandbox,
        skill_directory,
        workspace_cwd,
    )
}

fn resolve_cwd_value(
    source_cwd: Option<&str>,
    sandbox: Option<&SkillSandbox>,
    skill_directory: &Path,
    workspace_cwd: Option<&Path>,
) -> Result<PathBuf, RuntimeError> {
    let policy = sandbox
        .and_then(|sandbox| sandbox.cwd_policy.as_deref())
        .unwrap_or("skill-directory");
    let profile = sandbox
        .map(|sandbox| sandbox.profile.as_str())
        .unwrap_or("readonly");
    let cwd = match (policy, source_cwd) {
        ("custom", Some(cwd)) => Ok(resolve_path(skill_directory, cwd)),
        ("workspace", Some(cwd)) => Ok(resolve_path(skill_directory, cwd)),
        ("workspace", None) => workspace_cwd.map(Path::to_path_buf).map_or_else(
            || {
                std::env::current_dir()
                    .map_err(|source| RuntimeError::io("resolving workspace cwd", source))
            },
            Ok,
        ),
        (_, Some(cwd)) => Ok(resolve_path(skill_directory, cwd)),
        _ => Ok(skill_directory.to_path_buf()),
    }?;
    validate_cwd_policy(policy, profile, &cwd, skill_directory, workspace_cwd)?;
    Ok(normalize_path(&cwd))
}

fn workspace_cwd(env: &BTreeMap<String, String>) -> Result<Option<PathBuf>, RuntimeError> {
    let Some(path) = env.get(RUNX_CWD_ENV).or_else(|| env.get(INIT_CWD_ENV)) else {
        return Ok(None);
    };
    let path = PathBuf::from(path);
    if path.is_absolute() {
        Ok(Some(path))
    } else {
        std::env::current_dir()
            .map(|cwd| Some(cwd.join(path)))
            .map_err(|source| RuntimeError::io("resolving relative workspace cwd", source))
    }
}

fn resolve_path(base: &Path, path: &str) -> PathBuf {
    let candidate = PathBuf::from(path);
    if candidate.is_absolute() {
        candidate
    } else {
        base.join(candidate)
    }
}

fn validate_cwd_policy(
    policy: &str,
    profile: &str,
    cwd: &Path,
    skill_directory: &Path,
    workspace_cwd: Option<&Path>,
) -> Result<(), RuntimeError> {
    if profile == "unrestricted-local-dev" {
        return Ok(());
    }
    let cwd = normalize_path(cwd);
    let skill_directory = normalize_path(skill_directory);
    let workspace_root = match workspace_cwd {
        Some(workspace_cwd) => normalize_path(workspace_cwd),
        None => normalize_path(&std::env::current_dir().map_err(|source| {
            RuntimeError::io("resolving workspace cwd for sandbox policy", source)
        })?),
    };
    match policy {
        "unrestricted-local-dev" => Ok(()),
        "custom"
            if is_within_path(&cwd, &skill_directory) || is_within_path(&cwd, &workspace_root) =>
        {
            Ok(())
        }
        "custom" => Err(sandbox_violation(format!(
            "sandbox custom cwd '{}' is outside skill directory '{}' and workspace '{}'",
            cwd.display(),
            skill_directory.display(),
            workspace_root.display()
        ))),
        "skill-directory" if is_within_path(&cwd, &skill_directory) => Ok(()),
        "skill-directory" => Err(sandbox_violation(format!(
            "sandbox cwd '{}' is outside skill directory '{}'",
            cwd.display(),
            skill_directory.display()
        ))),
        "workspace" if is_within_path(&cwd, &workspace_root) => Ok(()),
        "workspace" => Err(sandbox_violation(format!(
            "sandbox cwd '{}' is outside workspace '{}'",
            cwd.display(),
            workspace_root.display()
        ))),
        _ => Ok(()),
    }
}

fn is_within_path(candidate: &Path, root: &Path) -> bool {
    candidate == root || candidate.starts_with(root)
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(component.as_os_str()),
            Component::CurDir => {}
            Component::Normal(part) => normalized.push(part),
            Component::ParentDir => {
                if normalized.as_os_str().is_empty()
                    || normalized
                        .components()
                        .next_back()
                        .is_some_and(|component| component == Component::ParentDir)
                {
                    normalized.push("..");
                } else {
                    normalized.pop();
                }
            }
        }
    }
    normalized
}

fn child_env(
    sandbox: Option<&SkillSandbox>,
    base_env: &BTreeMap<String, String>,
    inputs: &JsonObject,
) -> Result<BTreeMap<String, String>, RuntimeError> {
    let mut env = child_base_env(sandbox, base_env)?;
    let serialized = serde_json::to_string(inputs)
        .map_err(|source| RuntimeError::json("serializing runtime inputs", source))?;
    if serialized.len() > MAX_INLINE_INPUTS_BYTES {
        env.insert(
            "RUNX_INPUTS_PATH".to_owned(),
            write_inputs_file(base_env, &serialized)?,
        );
    } else {
        env.insert("RUNX_INPUTS_JSON".to_owned(), serialized);
    }
    let mut input_env_names = BTreeMap::new();
    for (key, value) in inputs {
        let serialized = json_value_env(value)?;
        if serialized.len() <= MAX_INLINE_INPUT_VALUE_BYTES {
            let env_name = input_env_name(key);
            if let Some(prior_key) = input_env_names.insert(env_name.clone(), key) {
                return Err(sandbox_violation(format!(
                    "input keys {prior_key:?} and {key:?} collide on environment variable {env_name}"
                )));
            }
            env.insert(env_name, serialized);
        }
    }
    Ok(env)
}

fn child_base_env(
    sandbox: Option<&SkillSandbox>,
    base_env: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, String>, RuntimeError> {
    let mut env = allowed_base_env(sandbox, base_env);
    env.insert(
        RUNX_CWD_ENV.to_owned(),
        workspace_root(base_env)?.to_string_lossy().into_owned(),
    );
    Ok(env)
}

fn workspace_root(base_env: &BTreeMap<String, String>) -> Result<PathBuf, RuntimeError> {
    workspace_cwd(base_env)?.map_or_else(
        || {
            std::env::current_dir()
                .map_err(|source| RuntimeError::io("resolving workspace cwd", source))
        },
        Ok,
    )
}

fn write_inputs_file(
    base_env: &BTreeMap<String, String>,
    serialized: &str,
) -> Result<String, RuntimeError> {
    let temp_root = base_env
        .get("TMPDIR")
        .or_else(|| base_env.get("TMP"))
        .or_else(|| base_env.get("TEMP"))
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    let dir = temp_root.join(format!("runx-cli-inputs-{}-{nanos}", std::process::id()));
    fs::create_dir_all(&dir)
        .map_err(|source| RuntimeError::io("creating inputs temp dir", source))?;
    let path = dir.join("inputs.json");
    let mut file = fs::File::create(&path)
        .map_err(|source| RuntimeError::io("creating inputs temp file", source))?;
    file.write_all(serialized.as_bytes())
        .map_err(|source| RuntimeError::io("writing inputs temp file", source))?;
    Ok(path.to_string_lossy().into_owned())
}

fn allowed_base_env(
    sandbox: Option<&SkillSandbox>,
    base_env: &BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    let mut allowed = DEFAULT_ENV_ALLOWLIST
        .iter()
        .filter_map(|key| {
            base_env
                .get(*key)
                .cloned()
                .map(|value| ((*key).to_owned(), value))
        })
        .collect::<BTreeMap<_, _>>();
    if let Some(env_allowlist) = sandbox.and_then(|sandbox| sandbox.env_allowlist.as_ref()) {
        for key in env_allowlist {
            if let Some(value) = base_env.get(key) {
                allowed.insert(key.clone(), value.clone());
            }
        }
    }
    allowed
}

fn validate_sandbox(sandbox: Option<&SkillSandbox>) -> Result<(), RuntimeError> {
    let Some(sandbox) = sandbox else {
        return Ok(());
    };
    if sandbox.require_enforcement == Some(true) {
        return Err(sandbox_violation(
            "platform isolation helpers are not available in the runtime skeleton",
        ));
    }
    match sandbox.profile.as_str() {
        "readonly" => validate_readonly_sandbox(sandbox),
        "workspace-write" | "network" => Ok(()),
        "unrestricted-local-dev" => validate_unrestricted_sandbox(sandbox),
        profile => Err(sandbox_violation(format!(
            "unsupported sandbox profile '{profile}'"
        ))),
    }
}

fn validate_readonly_sandbox(sandbox: &SkillSandbox) -> Result<(), RuntimeError> {
    if sandbox.network == Some(true) {
        return Err(sandbox_violation("readonly sandbox cannot request network"));
    }
    if !sandbox.writable_paths.is_empty() {
        return Err(sandbox_violation(
            "readonly sandbox cannot declare writable paths",
        ));
    }
    Ok(())
}

fn validate_unrestricted_sandbox(sandbox: &SkillSandbox) -> Result<(), RuntimeError> {
    if sandbox.approved_escalation == Some(true) {
        Ok(())
    } else {
        Err(sandbox_violation(
            "unrestricted-local-dev requires approved escalation",
        ))
    }
}

fn sandbox_violation(message: impl Into<String>) -> RuntimeError {
    RuntimeError::SandboxViolation {
        message: message.into(),
    }
}

fn input_env_name(key: &str) -> String {
    let mut suffix = String::new();
    let mut pending_separator = false;
    for ch in key.chars() {
        if ch.is_ascii_alphanumeric() {
            if pending_separator && !suffix.is_empty() {
                suffix.push('_');
            }
            suffix.push(ch.to_ascii_uppercase());
            pending_separator = false;
        } else {
            pending_separator = true;
        }
    }
    format!("RUNX_INPUT_{suffix}")
}

fn json_value_env(value: &JsonValue) -> Result<String, RuntimeError> {
    match value {
        JsonValue::Null => Ok(String::new()),
        JsonValue::Bool(value) => Ok(value.to_string()),
        JsonValue::Number(value) => serde_json::to_string(value)
            .map_err(|source| RuntimeError::json("serializing input number", source)),
        JsonValue::String(value) => Ok(value.clone()),
        JsonValue::Array(_) | JsonValue::Object(_) => serde_json::to_string(value)
            .map_err(|source| RuntimeError::json("serializing structured input", source)),
    }
}

fn resolve_template(template: &str, inputs: &JsonObject) -> String {
    let mut resolved = template.to_owned();
    for (key, value) in inputs {
        if let Ok(value) = json_value_env(value) {
            resolved = resolved.replace(&format!("{{{{{key}}}}}"), &value);
            resolved = resolved.replace(&format!("{{{{ {key} }}}}"), &value);
        }
    }
    resolved
}

pub fn sandbox_metadata(sandbox: Option<&SkillSandbox>) -> JsonObject {
    let mut metadata = JsonObject::new();
    if let Some(sandbox) = sandbox {
        metadata.insert(
            "profile".to_owned(),
            JsonValue::String(sandbox.profile.clone()),
        );
        if let Some(cwd_policy) = &sandbox.cwd_policy {
            metadata.insert(
                "cwd_policy".to_owned(),
                JsonValue::String(cwd_policy.clone()),
            );
        }
        metadata.insert(
            "env".to_owned(),
            JsonValue::Object(sandbox_env_metadata(sandbox)),
        );
        insert_network_metadata(&mut metadata, sandbox);
        insert_writable_paths_metadata(&mut metadata, sandbox);
        metadata.insert(
            "require_enforcement".to_owned(),
            JsonValue::Bool(sandbox.require_enforcement.unwrap_or(false)),
        );
        insert_filesystem_metadata(&mut metadata, sandbox);
        insert_approval_metadata(&mut metadata, sandbox);
        insert_runtime_metadata(&mut metadata);
    }
    metadata
}

fn sandbox_env_metadata(sandbox: &SkillSandbox) -> JsonObject {
    let allowlist = sandbox.env_allowlist.clone().unwrap_or_else(|| {
        DEFAULT_ENV_ALLOWLIST
            .into_iter()
            .map(str::to_owned)
            .collect()
    });
    [
        (
            "mode".to_owned(),
            JsonValue::String(if sandbox.env_allowlist.is_some() {
                "allowlist".to_owned()
            } else {
                "default-allowlist".to_owned()
            }),
        ),
        (
            "allowlist".to_owned(),
            JsonValue::Array(allowlist.into_iter().map(JsonValue::String).collect()),
        ),
    ]
    .into()
}

fn insert_network_metadata(metadata: &mut JsonObject, sandbox: &SkillSandbox) {
    metadata.insert(
        "network".to_owned(),
        JsonValue::Object(
            [
                (
                    "declared".to_owned(),
                    JsonValue::Bool(sandbox.network.unwrap_or(false)),
                ),
                (
                    "enforcement".to_owned(),
                    JsonValue::String("not-enforced-local".to_owned()),
                ),
            ]
            .into(),
        ),
    );
}

fn insert_writable_paths_metadata(metadata: &mut JsonObject, sandbox: &SkillSandbox) {
    metadata.insert(
        "writable_paths".to_owned(),
        JsonValue::Array(
            sandbox
                .writable_paths
                .iter()
                .cloned()
                .map(JsonValue::String)
                .collect(),
        ),
    );
}

fn insert_filesystem_metadata(metadata: &mut JsonObject, sandbox: &SkillSandbox) {
    metadata.insert(
        "filesystem".to_owned(),
        JsonValue::Object(
            [
                (
                    "enforcement".to_owned(),
                    JsonValue::String("not-enforced-local".to_owned()),
                ),
                (
                    "readonly_paths".to_owned(),
                    JsonValue::Bool(sandbox.profile != "unrestricted-local-dev"),
                ),
                ("writable_paths_enforced".to_owned(), JsonValue::Bool(false)),
                ("private_tmp".to_owned(), JsonValue::Bool(false)),
            ]
            .into(),
        ),
    );
}

fn insert_approval_metadata(metadata: &mut JsonObject, sandbox: &SkillSandbox) {
    metadata.insert(
        "approval".to_owned(),
        JsonValue::Object(
            [
                (
                    "required".to_owned(),
                    JsonValue::Bool(sandbox.profile == "unrestricted-local-dev"),
                ),
                (
                    "approved".to_owned(),
                    JsonValue::Bool(sandbox.approved_escalation.unwrap_or(false)),
                ),
            ]
            .into(),
        ),
    );
}

fn insert_runtime_metadata(metadata: &mut JsonObject) {
    metadata.insert(
        "runtime".to_owned(),
        JsonValue::Object(
            [
                (
                    "enforcer".to_owned(),
                    JsonValue::String("declared-policy-only".to_owned()),
                ),
                (
                    "reason".to_owned(),
                    JsonValue::String(
                        "local sandbox isolation is not enforced by the runtime skeleton"
                            .to_owned(),
                    ),
                ),
            ]
            .into(),
        ),
    );
}
