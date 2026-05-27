use runx_contracts::{JsonObject, JsonValue};
use runx_core::policy::SandboxProfile;
use runx_parser::SkillSandbox;

use super::backend::SandboxRuntime;
use super::command::sandbox_network_enabled;
use super::env::DEFAULT_ENV_ALLOWLIST;

pub fn sandbox_metadata(sandbox: Option<&SkillSandbox>) -> JsonObject {
    let writable_paths = sandbox
        .map(|sandbox| sandbox.writable_paths.clone())
        .unwrap_or_default();
    sandbox_metadata_with_runtime(sandbox, &writable_paths, None, false)
}

pub(super) fn sandbox_metadata_with_runtime(
    sandbox: Option<&SkillSandbox>,
    writable_paths: &[String],
    runtime: Option<&SandboxRuntime>,
    private_tmp_enabled: bool,
) -> JsonObject {
    let mut metadata = JsonObject::new();
    if let Some(sandbox) = sandbox {
        metadata.insert(
            "profile".to_owned(),
            JsonValue::String(sandbox.profile.as_str().to_owned()),
        );
        if let Some(cwd_policy) = &sandbox.cwd_policy {
            metadata.insert(
                "cwd_policy".to_owned(),
                JsonValue::String(cwd_policy.as_str().to_owned()),
            );
        }
        metadata.insert(
            "env".to_owned(),
            JsonValue::Object(sandbox_env_metadata(sandbox)),
        );
        insert_network_metadata(&mut metadata, sandbox, runtime);
        insert_writable_paths_metadata(&mut metadata, writable_paths);
        metadata.insert(
            "require_enforcement".to_owned(),
            JsonValue::Bool(sandbox.require_enforcement.unwrap_or(false)),
        );
        insert_filesystem_metadata(&mut metadata, sandbox, runtime, private_tmp_enabled);
        insert_approval_metadata(&mut metadata, sandbox);
        insert_runtime_metadata(&mut metadata, sandbox, runtime);
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

fn insert_network_metadata(
    metadata: &mut JsonObject,
    sandbox: &SkillSandbox,
    runtime: Option<&SandboxRuntime>,
) {
    metadata.insert(
        "network".to_owned(),
        JsonValue::Object(
            [
                (
                    "declared".to_owned(),
                    JsonValue::Bool(sandbox_network_enabled(Some(sandbox))),
                ),
                (
                    "enforcement".to_owned(),
                    JsonValue::String(network_enforcement(sandbox, runtime).to_owned()),
                ),
            ]
            .into(),
        ),
    );
}

fn insert_writable_paths_metadata(metadata: &mut JsonObject, writable_paths: &[String]) {
    metadata.insert(
        "writable_paths".to_owned(),
        JsonValue::Array(
            writable_paths
                .iter()
                .cloned()
                .map(JsonValue::String)
                .collect(),
        ),
    );
}

fn insert_filesystem_metadata(
    metadata: &mut JsonObject,
    sandbox: &SkillSandbox,
    runtime: Option<&SandboxRuntime>,
    private_tmp_enabled: bool,
) {
    metadata.insert(
        "filesystem".to_owned(),
        JsonValue::Object(
            [
                (
                    "enforcement".to_owned(),
                    JsonValue::String(filesystem_enforcement(sandbox, runtime).to_owned()),
                ),
                (
                    "readonly_paths".to_owned(),
                    JsonValue::Bool(sandbox.profile != SandboxProfile::UnrestrictedLocalDev),
                ),
                (
                    "writable_paths_enforced".to_owned(),
                    JsonValue::Bool(
                        runtime.is_some_and(SandboxRuntime::enforces)
                            && sandbox.profile == SandboxProfile::WorkspaceWrite,
                    ),
                ),
                (
                    "private_tmp".to_owned(),
                    JsonValue::Bool(private_tmp_enabled),
                ),
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
                    JsonValue::Bool(sandbox.profile == SandboxProfile::UnrestrictedLocalDev),
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

fn insert_runtime_metadata(
    metadata: &mut JsonObject,
    sandbox: &SkillSandbox,
    runtime: Option<&SandboxRuntime>,
) {
    metadata.insert(
        "runtime".to_owned(),
        JsonValue::Object(runtime_metadata(sandbox, runtime)),
    );
}

fn network_enforcement(sandbox: &SkillSandbox, runtime: Option<&SandboxRuntime>) -> &'static str {
    match runtime {
        Some(SandboxRuntime::Bubblewrap { .. } | SandboxRuntime::SandboxExec { .. }) => {
            if sandbox_network_enabled(Some(sandbox)) {
                "host-network-shared"
            } else {
                "isolated-namespace"
            }
        }
        Some(SandboxRuntime::Direct) if sandbox.profile == SandboxProfile::UnrestrictedLocalDev => {
            "host-ambient"
        }
        Some(SandboxRuntime::DeclaredPolicyOnly { .. }) | None => "not-enforced-local",
        Some(SandboxRuntime::Direct) => "host-ambient",
    }
}

fn filesystem_enforcement(
    sandbox: &SkillSandbox,
    runtime: Option<&SandboxRuntime>,
) -> &'static str {
    match runtime {
        Some(SandboxRuntime::Bubblewrap { .. }) => "bubblewrap-mount-namespace",
        Some(SandboxRuntime::SandboxExec { .. }) => "sandbox-exec-seatbelt",
        Some(SandboxRuntime::Direct) if sandbox.profile == SandboxProfile::UnrestrictedLocalDev => {
            "host-ambient"
        }
        Some(SandboxRuntime::DeclaredPolicyOnly { .. }) | None => "not-enforced-local",
        Some(SandboxRuntime::Direct) => "host-ambient",
    }
}

fn runtime_metadata(sandbox: &SkillSandbox, runtime: Option<&SandboxRuntime>) -> JsonObject {
    match runtime {
        Some(SandboxRuntime::Bubblewrap { path }) => [
            (
                "enforcer".to_owned(),
                JsonValue::String("bubblewrap".to_owned()),
            ),
            (
                "command".to_owned(),
                JsonValue::String(path.to_string_lossy().into_owned()),
            ),
        ]
        .into(),
        Some(SandboxRuntime::SandboxExec { path }) => [
            (
                "enforcer".to_owned(),
                JsonValue::String("sandbox-exec".to_owned()),
            ),
            (
                "command".to_owned(),
                JsonValue::String(path.to_string_lossy().into_owned()),
            ),
        ]
        .into(),
        Some(SandboxRuntime::Direct) => [(
            "enforcer".to_owned(),
            JsonValue::String("direct".to_owned()),
        )]
        .into(),
        Some(SandboxRuntime::DeclaredPolicyOnly { reason }) => [
            (
                "enforcer".to_owned(),
                JsonValue::String("declared-policy-only".to_owned()),
            ),
            ("reason".to_owned(), JsonValue::String(reason.clone())),
        ]
        .into(),
        None => [
            (
                "enforcer".to_owned(),
                JsonValue::String("declared-policy-only".to_owned()),
            ),
            (
                "reason".to_owned(),
                JsonValue::String(format!(
                    "local sandbox profile '{}' requires Linux bubblewrap or macOS sandbox-exec for filesystem and network enforcement",
                    sandbox.profile.as_str()
                )),
            ),
        ]
        .into(),
    }
}
