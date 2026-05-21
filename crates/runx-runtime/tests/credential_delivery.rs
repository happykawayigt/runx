#![cfg(all(feature = "cli-tool", any(feature = "mcp", feature = "mcp-rmcp")))]

use std::collections::BTreeMap;
use std::path::PathBuf;

use runx_contracts::{CredentialDeliveryMode, CredentialDeliveryPurpose, CredentialMaterialRole};
use runx_core::policy::{CredentialBindingDecision, CredentialEnvelope};
use runx_parser::{SkillSandbox, SkillSource};
use runx_runtime::adapters::cli_tool::CliToolAdapter;
use runx_runtime::adapters::mcp::{FixtureMcpTransport, McpAdapter, ProcessMcpTransport};
use runx_runtime::{
    CredentialDelivery, CredentialDeliveryError, CredentialDeliveryProfile,
    InMemoryMaterialResolver, InvocationStatus, ResolvedCredentialMaterial, RuntimeError,
    SkillAdapter, SkillInvocation,
};

#[test]
fn delivery_profile_requires_allowed_binding() -> Result<(), Box<dyn std::error::Error>> {
    let result = CredentialDelivery::from_allowed_binding(
        &CredentialBindingDecision::Deny {
            reasons: vec!["grant mismatch".to_owned()],
        },
        &credential(),
        &github_profile()?,
        &resolver(),
    );

    match result {
        Err(CredentialDeliveryError::BindingDenied { reasons }) => {
            assert_eq!(reasons, vec!["grant mismatch"]);
        }
        Ok(_) => {
            return Err(std::io::Error::other(
                "credential delivery must fail closed on denied binding",
            )
            .into());
        }
        Err(error) => {
            return Err(std::io::Error::other(format!("unexpected error: {error}")).into());
        }
    }
    Ok(())
}

#[test]
fn delivery_profile_rejects_provider_mismatch() -> Result<(), Box<dyn std::error::Error>> {
    let result = CredentialDelivery::from_allowed_binding(
        &CredentialBindingDecision::Allow {
            reasons: vec!["allowed".to_owned()],
        },
        &credential(),
        &CredentialDeliveryProfile::env_token("slack", "oauth_bearer", "SLACK_TOKEN")?,
        &resolver(),
    );

    match result {
        Err(CredentialDeliveryError::ProviderMismatch {
            credential_provider,
            profile_provider,
        }) => {
            assert_eq!(credential_provider, "github");
            assert_eq!(profile_provider, "slack");
        }
        Ok(_) => {
            return Err(std::io::Error::other(
                "credential delivery must reject mismatched providers",
            )
            .into());
        }
        Err(error) => {
            return Err(std::io::Error::other(format!("unexpected error: {error}")).into());
        }
    }
    Ok(())
}

#[test]
fn delivery_profile_maps_process_env_contract_profile() -> Result<(), Box<dyn std::error::Error>> {
    let profile = CredentialDeliveryProfile::from_contract_profile(&contract_profile(
        vec![CredentialMaterialRole::AccessToken],
        "GITHUB_TOKEN",
    ))?;
    let delivery = CredentialDelivery::from_allowed_binding(
        &CredentialBindingDecision::Allow {
            reasons: vec!["allowed".to_owned()],
        },
        &credential(),
        &profile,
        &resolver(),
    )?;

    assert_eq!(profile.provider(), "github");
    assert_eq!(profile.auth_mode(), "oauth_bearer");
    assert_eq!(
        delivery.secret_env().get("GITHUB_TOKEN"),
        Some("ghs_secret_token")
    );
    Ok(())
}

#[test]
fn delivery_profile_skips_optional_unsupported_contract_binding()
-> Result<(), Box<dyn std::error::Error>> {
    let mut contract = contract_profile(vec![CredentialMaterialRole::AccessToken], "GITHUB_TOKEN");
    contract
        .material_roles
        .push(CredentialMaterialRole::RefreshToken);
    contract
        .env_bindings
        .push(runx_contracts::CredentialDeliveryEnvBinding {
            role: CredentialMaterialRole::RefreshToken,
            env_var: "GITHUB_REFRESH_TOKEN".to_owned(),
            required: false,
        });
    let profile = CredentialDeliveryProfile::from_contract_profile(&contract)?;
    let delivery = CredentialDelivery::from_allowed_binding(
        &CredentialBindingDecision::Allow {
            reasons: vec!["allowed".to_owned()],
        },
        &credential(),
        &profile,
        &resolver(),
    )?;

    assert_eq!(
        delivery.secret_env().get("GITHUB_TOKEN"),
        Some("ghs_secret_token")
    );
    assert_eq!(delivery.secret_env().get("GITHUB_REFRESH_TOKEN"), None);
    Ok(())
}

#[test]
fn delivery_profile_rejects_unsupported_contract_role() {
    let result = CredentialDeliveryProfile::from_contract_profile(&contract_profile(
        vec![CredentialMaterialRole::RefreshToken],
        "GITHUB_REFRESH_TOKEN",
    ));

    assert!(matches!(
        result,
        Err(CredentialDeliveryError::UnsupportedMaterialRole { .. })
    ));
}

#[test]
fn delivery_profile_rejects_empty_material() -> Result<(), Box<dyn std::error::Error>> {
    let resolver = InMemoryMaterialResolver::with_material(
        "secret://github/main",
        ResolvedCredentialMaterial::access_token("secret://github/main", "  "),
    );
    let result = CredentialDelivery::from_allowed_binding(
        &CredentialBindingDecision::Allow {
            reasons: vec!["allowed".to_owned()],
        },
        &credential(),
        &github_profile()?,
        &resolver,
    );

    assert!(matches!(
        result,
        Err(CredentialDeliveryError::EmptyMaterial { role }) if role == "access_token"
    ));
    Ok(())
}

#[test]
fn cli_tool_injects_secret_env_and_redacts_process_output() -> Result<(), Box<dyn std::error::Error>>
{
    let delivery = allowed_delivery()?;
    let output = CliToolAdapter.invoke(SkillInvocation {
        skill_name: "credential.echo".to_owned(),
        source: cli_source(),
        inputs: Default::default(),
        resolved_inputs: Default::default(),
        skill_directory: std::env::current_dir()?,
        env: process_env(),
        credential_delivery: delivery,
    })?;

    assert_eq!(output.status, InvocationStatus::Success);
    assert_eq!(output.stdout.trim(), "[redacted-credential]");
    assert!(!output.stdout.contains("ghs_secret_token"));
    assert!(
        !serde_json::to_string(&output.metadata)?.contains("ghs_secret_token"),
        "credential material must not enter sandbox metadata"
    );
    Ok(())
}

#[test]
fn cli_tool_omits_truncated_output_before_redaction() -> Result<(), Box<dyn std::error::Error>> {
    let output = CliToolAdapter.invoke(SkillInvocation {
        skill_name: "credential.large-output".to_owned(),
        source: large_output_cli_source(),
        inputs: Default::default(),
        resolved_inputs: Default::default(),
        skill_directory: std::env::current_dir()?,
        env: process_env(),
        credential_delivery: allowed_delivery()?,
    })?;

    assert_eq!(output.status, InvocationStatus::Failure);
    assert_eq!(output.stdout, "");
    assert!(output.stderr.contains("stdout/stderr omitted"));
    assert!(!output.stdout.contains("ghs_secret_token"));
    assert!(!output.stderr.contains("ghs_secret_token"));
    Ok(())
}

#[test]
fn credential_delivery_redacts_before_truncating() -> Result<(), Box<dyn std::error::Error>> {
    let output = allowed_delivery()?.redact_bytes_to_string(
        b"prefix ghs_secret_token suffix".to_vec(),
        "prefix [redacted-credential]".len(),
    );

    assert_eq!(output, "prefix [redacted-credential]");
    assert!(!output.contains("ghs_secret_token"));
    Ok(())
}

#[test]
fn mcp_adapter_delivers_secret_env_and_redacts_tool_result()
-> Result<(), Box<dyn std::error::Error>> {
    let mut inputs = runx_contracts::JsonObject::new();
    inputs.insert(
        "name".to_owned(),
        runx_contracts::JsonValue::String("GITHUB_TOKEN".to_owned()),
    );
    let output = McpAdapter::new(FixtureMcpTransport).invoke(SkillInvocation {
        skill_name: "credential.mcp".to_owned(),
        source: mcp_source(),
        inputs,
        resolved_inputs: Default::default(),
        skill_directory: std::env::current_dir()?,
        env: process_env(),
        credential_delivery: allowed_delivery()?,
    })?;

    assert_eq!(output.status, InvocationStatus::Success);
    assert_eq!(output.stdout.trim(), "[redacted-credential]");
    assert!(!output.stdout.contains("ghs_secret_token"));
    assert!(!serde_json::to_string(&output.metadata)?.contains("ghs_secret_token"));
    Ok(())
}

#[test]
fn mcp_process_transport_delivers_secret_env_and_redacts_tool_result()
-> Result<(), Box<dyn std::error::Error>> {
    let mut inputs = runx_contracts::JsonObject::new();
    inputs.insert(
        "name".to_owned(),
        runx_contracts::JsonValue::String("GITHUB_TOKEN".to_owned()),
    );
    let output = McpAdapter::new(ProcessMcpTransport).invoke(SkillInvocation {
        skill_name: "credential.mcp.process".to_owned(),
        source: mcp_process_source()?,
        inputs,
        resolved_inputs: Default::default(),
        skill_directory: repo_root()?,
        env: process_env(),
        credential_delivery: allowed_delivery()?,
    })?;

    assert_eq!(output.status, InvocationStatus::Success);
    assert_eq!(output.stdout.trim(), "[redacted-credential]");
    assert!(!output.stdout.contains("ghs_secret_token"));
    assert!(!serde_json::to_string(&output.metadata)?.contains("ghs_secret_token"));
    Ok(())
}

fn allowed_delivery() -> Result<CredentialDelivery, CredentialDeliveryError> {
    CredentialDelivery::from_allowed_binding(
        &CredentialBindingDecision::Allow {
            reasons: vec!["credential material matches admitted grant".to_owned()],
        },
        &credential(),
        &github_profile()?,
        &resolver(),
    )
}

fn resolver() -> InMemoryMaterialResolver {
    InMemoryMaterialResolver::with_material(
        "secret://github/main",
        ResolvedCredentialMaterial::access_token("secret://github/main", "ghs_secret_token"),
    )
}

fn github_profile() -> Result<CredentialDeliveryProfile, CredentialDeliveryError> {
    CredentialDeliveryProfile::env_token("github", "oauth_bearer", "GITHUB_TOKEN")
}

fn credential() -> CredentialEnvelope {
    CredentialEnvelope {
        kind: "runx.credential-envelope.v1".to_owned(),
        grant_id: "grant_github_main".to_owned(),
        provider: "github".to_owned(),
        auth_mode: "oauth_bearer".to_owned(),
        material_kind: "access_token".to_owned(),
        connection_id: Some("conn_github_main".to_owned()),
        scopes: vec!["repo".to_owned()],
        grant_reference: None,
        material_ref: "secret://github/main".to_owned(),
    }
}

fn contract_profile(
    roles: Vec<CredentialMaterialRole>,
    env_var: &str,
) -> runx_contracts::CredentialDeliveryProfile {
    runx_contracts::CredentialDeliveryProfile {
        schema: "runx.credential_delivery.profile.v1".to_owned(),
        profile_id: "github-provider-api-env".to_owned(),
        provider: "github".to_owned(),
        auth_mode: "oauth_bearer".to_owned(),
        purpose: CredentialDeliveryPurpose::ProviderApi,
        delivery_mode: CredentialDeliveryMode::ProcessEnv,
        material_roles: roles.clone(),
        env_bindings: roles
            .into_iter()
            .map(|role| runx_contracts::CredentialDeliveryEnvBinding {
                role,
                env_var: env_var.to_owned(),
                required: true,
            })
            .collect(),
        redaction_policy_ref: runx_contracts::Reference {
            reference_type: runx_contracts::ReferenceType::RedactionPolicy,
            uri: "runx:redaction-policy:credentials-v1".to_owned(),
            provider: None,
            locator: None,
            label: None,
            observed_at: None,
            proof_kind: None,
        },
    }
}

fn cli_source() -> SkillSource {
    SkillSource {
        source_type: "cli-tool".to_owned(),
        command: Some("sh".to_owned()),
        args: vec![
            "-c".to_owned(),
            "printf '%s\\n' \"$GITHUB_TOKEN\"".to_owned(),
        ],
        cwd: None,
        timeout_seconds: Some(5),
        input_mode: None,
        sandbox: Some(readonly_sandbox()),
        server: None,
        catalog_ref: None,
        tool: None,
        arguments: None,
        agent_card_url: None,
        agent_identity: None,
        agent: None,
        task: None,
        hook: None,
        outputs: None,
        graph: None,
        raw: Default::default(),
    }
}

fn large_output_cli_source() -> SkillSource {
    let mut source = cli_source();
    source.command = Some("node".to_owned());
    source.args = vec![
        "-e".to_owned(),
        "process.stdout.write('x'.repeat(1024 * 1024 - 4)); process.stdout.write(process.env.GITHUB_TOKEN || '');"
            .to_owned(),
    ];
    source
}

fn mcp_source() -> SkillSource {
    let mut source = cli_source();
    source.source_type = "mcp".to_owned();
    source.command = None;
    source.args = Vec::new();
    source.server = Some(runx_parser::SkillMcpServer {
        command: "fixture".to_owned(),
        args: Vec::new(),
        cwd: None,
    });
    source.tool = Some("env".to_owned());
    source
}

fn mcp_process_source() -> Result<SkillSource, RuntimeError> {
    let root = repo_root()?;
    let mut source = cli_source();
    source.source_type = "mcp".to_owned();
    source.command = None;
    source.args = Vec::new();
    source.server = Some(runx_parser::SkillMcpServer {
        command: "node".to_owned(),
        args: vec![
            root.join("fixtures/runtime/adapters/mcp/stdio-server.mjs")
                .to_string_lossy()
                .into_owned(),
        ],
        cwd: Some(root.to_string_lossy().into_owned()),
    });
    source.tool = Some("env".to_owned());
    Ok(source)
}

fn readonly_sandbox() -> SkillSandbox {
    SkillSandbox {
        profile: "readonly".to_owned(),
        cwd_policy: None,
        env_allowlist: Some(vec!["PATH".to_owned()]),
        network: None,
        writable_paths: Vec::new(),
        require_enforcement: None,
        approved_escalation: None,
        raw: Default::default(),
    }
}

fn process_env() -> BTreeMap<String, String> {
    std::env::vars().collect()
}

fn repo_root() -> Result<PathBuf, RuntimeError> {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .map_err(|error| RuntimeError::Io {
            context: "repository root is available".to_owned(),
            source: error,
        })
}
