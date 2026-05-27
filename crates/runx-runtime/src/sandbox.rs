// rust-style-allow: large-file because the sandbox root owns orchestration
// tests that exercise the split backend, command, env, metadata, and policy
// modules together.
mod backend;
mod command;
mod env;
mod metadata;
mod policy;
mod template;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use runx_contracts::JsonObject;
use runx_parser::{SkillMcpServer, SkillSource};

use crate::RuntimeError;

use self::backend::resolve_sandbox_runtime;
use self::command::{SandboxSpawnCommand, sandbox_network_enabled, sandbox_spawn_command};
use self::env::{
    child_base_env, child_env, cleanup_paths_quietly, prepare_sandbox_tmp_env,
    sandbox_private_tmp_enabled,
};
use self::metadata::sandbox_metadata_with_runtime;
use self::policy::{
    resolve_cwd, resolve_cwd_value, resolved_writable_paths, validate_sandbox,
    validate_writable_paths, workspace_cwd,
};
use self::template::resolve_template;

pub use self::metadata::sandbox_metadata;

#[derive(Clone, Debug, PartialEq)]
pub struct SandboxPlan {
    pub command: String,
    pub args: Vec<String>,
    pub cwd: PathBuf,
    pub env: BTreeMap<String, String>,
    pub metadata: JsonObject,
    pub cleanup_paths: Vec<PathBuf>,
}

impl Drop for SandboxPlan {
    fn drop(&mut self) {
        cleanup_paths_quietly(&self.cleanup_paths);
    }
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
    let args = source
        .args
        .iter()
        .map(|arg| resolve_template(arg, inputs, base_env))
        .collect();
    let writable_paths = resolved_writable_paths(sandbox, inputs, base_env);
    validate_writable_paths(sandbox, &writable_paths, &cwd, workspace_cwd.as_deref())?;
    let runtime = resolve_sandbox_runtime(sandbox, base_env)?;
    let private_tmp_enabled = sandbox_private_tmp_enabled(sandbox, runtime.as_ref());
    let mut cleanup_paths = Vec::new();
    let mut sandbox_base_env = base_env.clone();
    prepare_sandbox_tmp_env(sandbox, &runtime, &mut sandbox_base_env, &mut cleanup_paths)?;
    let env = match child_env(sandbox, &sandbox_base_env, inputs, &mut cleanup_paths) {
        Ok(env) => env,
        Err(error) => {
            cleanup_paths_quietly(&cleanup_paths);
            return Err(error);
        }
    };
    let (command, args) = sandbox_spawn_command(SandboxSpawnCommand {
        runtime: runtime.as_ref(),
        command,
        args,
        cwd: &cwd,
        skill_directory,
        workspace_cwd: workspace_cwd.as_deref(),
        writable_paths: &writable_paths,
        network: sandbox_network_enabled(sandbox),
        private_tmp: cleanup_paths.first().map(PathBuf::as_path),
    });
    Ok(SandboxPlan {
        command,
        args,
        cwd,
        env,
        metadata: sandbox_metadata_with_runtime(
            sandbox,
            &writable_paths,
            runtime.as_ref(),
            private_tmp_enabled,
        ),
        cleanup_paths,
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
    let writable_paths = resolved_writable_paths(sandbox, &JsonObject::new(), base_env);
    validate_writable_paths(sandbox, &writable_paths, &cwd, workspace_cwd.as_deref())?;
    let runtime = resolve_sandbox_runtime(sandbox, base_env)?;
    let private_tmp_enabled = sandbox_private_tmp_enabled(sandbox, runtime.as_ref());
    let mut cleanup_paths = Vec::new();
    let mut sandbox_base_env = base_env.clone();
    prepare_sandbox_tmp_env(sandbox, &runtime, &mut sandbox_base_env, &mut cleanup_paths)?;
    let env = match child_base_env(sandbox, &sandbox_base_env) {
        Ok(env) => env,
        Err(error) => {
            cleanup_paths_quietly(&cleanup_paths);
            return Err(error);
        }
    };
    let (command, args) = sandbox_spawn_command(SandboxSpawnCommand {
        runtime: runtime.as_ref(),
        command: server.command.clone(),
        args: server.args.clone(),
        cwd: &cwd,
        skill_directory,
        workspace_cwd: workspace_cwd.as_deref(),
        writable_paths: &writable_paths,
        network: sandbox_network_enabled(sandbox),
        private_tmp: cleanup_paths.first().map(PathBuf::as_path),
    });
    Ok(SandboxPlan {
        command,
        args,
        cwd,
        env,
        metadata: sandbox_metadata_with_runtime(
            sandbox,
            &writable_paths,
            runtime.as_ref(),
            private_tmp_enabled,
        ),
        cleanup_paths,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::{Path, PathBuf};

    use runx_contracts::{JsonObject, JsonValue};
    use runx_core::policy::SandboxProfile;
    use runx_parser::SkillSandbox;

    use super::backend::{SandboxRuntime, find_trusted_executable};
    use super::command::{
        sandbox_exec_path_filter_path, sandbox_exec_profile, sandbox_profile_string,
    };
    use super::env::{cleanup_paths_quietly, prepare_sandbox_tmp_env};
    use super::policy::{resolved_writable_paths, validate_writable_paths};

    #[test]
    fn writable_paths_omit_unresolved_optional_templates() {
        let sandbox = SkillSandbox {
            profile: SandboxProfile::WorkspaceWrite,
            cwd_policy: None,
            env_allowlist: None,
            network: None,
            writable_paths: vec![
                "{{workspace_path}}".to_owned(),
                "{{ fixture }}".to_owned(),
                "{{ env.RUNX_RAIL_COUNT_PATH }}".to_owned(),
                "logs".to_owned(),
            ],
            require_enforcement: None,
            approved_escalation: None,
            raw: JsonObject::new(),
        };
        let inputs = [(
            "fixture".to_owned(),
            JsonValue::String("/tmp/runx-fixture".to_owned()),
        )]
        .into_iter()
        .collect();
        let env = [(
            "RUNX_RAIL_COUNT_PATH".to_owned(),
            "/tmp/runx-rail-count.txt".to_owned(),
        )]
        .into_iter()
        .collect();

        assert_eq!(
            resolved_writable_paths(Some(&sandbox), &inputs, &env),
            vec![
                "/tmp/runx-fixture".to_owned(),
                "/tmp/runx-rail-count.txt".to_owned(),
                "logs".to_owned()
            ]
        );
    }

    #[test]
    fn trusted_enforcer_lookup_ignores_caller_path() {
        let trusted = find_trusted_executable("runx-test-enforcer-that-should-not-exist");
        assert!(trusted.is_none());
    }

    #[test]
    fn sandbox_exec_runtime_gets_private_writable_tmp_env() -> Result<(), String> {
        let sandbox = readonly_sandbox();
        let runtime = Some(SandboxRuntime::SandboxExec {
            path: PathBuf::from("/usr/bin/sandbox-exec"),
        });
        let mut env = BTreeMap::new();
        let mut cleanup_paths = Vec::new();
        prepare_sandbox_tmp_env(Some(&sandbox), &runtime, &mut env, &mut cleanup_paths)
            .map_err(|source| source.to_string())?;

        let tmpdir = env
            .get("TMPDIR")
            .ok_or_else(|| "TMPDIR was not set".to_owned())?;
        assert_eq!(env.get("TMP"), Some(tmpdir));
        assert_eq!(env.get("TEMP"), Some(tmpdir));
        assert_eq!(cleanup_paths, vec![PathBuf::from(tmpdir)]);
        assert!(Path::new(tmpdir).is_dir());

        let profile =
            sandbox_exec_profile(Path::new("/workspace"), &[], true, Some(Path::new(tmpdir)));
        assert!(profile.contains("(allow file-write* (literal \"/dev/null\"))"));
        assert!(profile.contains("(allow mach-lookup)"));
        let tmp_filter_path = sandbox_exec_path_filter_path(Path::new(tmpdir));
        assert!(profile.contains(&format!(
            "(subpath \"{}\")",
            sandbox_profile_string(&tmp_filter_path)
        )));
        cleanup_paths_quietly(&cleanup_paths);
        Ok(())
    }

    #[test]
    fn sandbox_exec_profile_keeps_legitimate_writable_path() {
        let profile = sandbox_exec_profile(
            Path::new("/workspace"),
            &["logs/output".to_owned()],
            false,
            None,
        );

        assert!(profile.contains("(allow file-write* (literal \"/workspace/logs/output\"))"));
        assert!(!profile.contains("(allow network*)"));
    }

    #[test]
    fn declared_policy_runtime_gets_private_tmp_env() -> Result<(), String> {
        let sandbox = readonly_sandbox();
        let runtime = Some(SandboxRuntime::DeclaredPolicyOnly {
            reason: "missing test backend".to_owned(),
        });
        let mut env = BTreeMap::new();
        let mut cleanup_paths = Vec::new();
        prepare_sandbox_tmp_env(Some(&sandbox), &runtime, &mut env, &mut cleanup_paths)
            .map_err(|source| source.to_string())?;

        let tmpdir = env
            .get("TMPDIR")
            .ok_or_else(|| "TMPDIR was not set".to_owned())?;
        assert_eq!(env.get("TMP"), Some(tmpdir));
        assert_eq!(env.get("TEMP"), Some(tmpdir));
        assert_eq!(cleanup_paths, vec![PathBuf::from(tmpdir)]);
        assert!(Path::new(tmpdir).is_dir());

        cleanup_paths_quietly(&cleanup_paths);
        Ok(())
    }

    fn readonly_sandbox() -> SkillSandbox {
        SkillSandbox {
            profile: SandboxProfile::Readonly,
            cwd_policy: None,
            env_allowlist: None,
            network: None,
            writable_paths: Vec::new(),
            require_enforcement: None,
            approved_escalation: None,
            raw: JsonObject::new(),
        }
    }

    #[test]
    fn writable_path_rejects_sexpr_metacharacters() -> Result<(), String> {
        let temp = tempfile::tempdir().map_err(|source| source.to_string())?;
        let workspace = temp.path().join("workspace");
        fs::create_dir_all(&workspace).map_err(|source| source.to_string())?;
        let sandbox = SkillSandbox {
            profile: SandboxProfile::WorkspaceWrite,
            cwd_policy: None,
            env_allowlist: None,
            network: None,
            writable_paths: Vec::new(),
            require_enforcement: None,
            approved_escalation: None,
            raw: JsonObject::new(),
        };

        let error = validate_writable_paths(
            Some(&sandbox),
            &["safe\")) (allow network*)".to_owned()],
            &workspace,
            Some(&workspace),
        )
        .err()
        .ok_or_else(|| "sexpr metacharacter path unexpectedly passed".to_owned())?;

        assert!(
            error.to_string().contains("profile metacharacters"),
            "unexpected error: {error}"
        );
        Ok(())
    }

    #[test]
    #[cfg(unix)]
    fn workspace_write_rejects_symlink_escape() -> Result<(), String> {
        let temp = tempfile::tempdir().map_err(|source| source.to_string())?;
        let workspace = temp.path().join("workspace");
        let outside = temp.path().join("outside");
        fs::create_dir_all(&workspace).map_err(|source| source.to_string())?;
        fs::create_dir_all(&outside).map_err(|source| source.to_string())?;
        std::os::unix::fs::symlink(&outside, workspace.join("link"))
            .map_err(|source| source.to_string())?;
        let sandbox = SkillSandbox {
            profile: SandboxProfile::WorkspaceWrite,
            cwd_policy: None,
            env_allowlist: None,
            network: None,
            writable_paths: Vec::new(),
            require_enforcement: None,
            approved_escalation: None,
            raw: JsonObject::new(),
        };

        let error = validate_writable_paths(
            Some(&sandbox),
            &["link/escape.txt".to_owned()],
            &workspace,
            Some(&workspace),
        )
        .err()
        .ok_or_else(|| "symlink escape unexpectedly passed".to_owned())?;

        assert!(
            error.to_string().contains("outside workspace"),
            "unexpected error: {error}"
        );
        Ok(())
    }
}
