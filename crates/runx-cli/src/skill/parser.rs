use std::collections::BTreeMap;
use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use runx_contracts::JsonValue;
use runx_runtime::orchestrator::LocalCredentialDescriptor;

use super::SkillPlan;
use super::inputs::{parse_direct_input_arg, parse_input_arg};

pub fn parse_skill_plan(args: &[OsString]) -> Result<SkillPlan, String> {
    let mut state = SkillParseState::default();
    let mut index = 1;

    while index < args.len() {
        index = parse_skill_arg(args, index, &mut state)?;
        index += 1;
    }

    let local_credential = finalize_local_credential(&state)?;

    let Some(skill_path) = state.skill_path.as_ref() else {
        return Err("runx skill requires a skill package path".to_owned());
    };
    reject_resolver_flags_for_skill_management_action(skill_path, &state)?;
    let skill_path = skill_path.clone();
    if state.answers.is_some() && state.run_id.is_none() {
        return Err("runx skill --answers requires --run-id".to_owned());
    }
    if state.run_id.is_some() && state.answers.is_none() {
        return Err("runx skill --run-id requires --answers".to_owned());
    }

    Ok(SkillPlan {
        skill_path,
        runner: state.runner,
        receipt_dir: state.receipt_dir,
        run_id: state.run_id,
        answers: state.answers,
        registry: state.registry,
        expected_digest: state.expected_digest,
        json: state.json,
        inputs: state.inputs,
        local_credential,
    })
}

#[derive(Default)]
struct SkillParseState {
    skill_path: Option<PathBuf>,
    runner: Option<String>,
    receipt_dir: Option<PathBuf>,
    run_id: Option<String>,
    answers: Option<PathBuf>,
    registry: Option<String>,
    expected_digest: Option<String>,
    json: bool,
    inputs: BTreeMap<String, JsonValue>,
    credential: Option<CredentialBinding>,
    secret_env: Option<(String, String)>,
}

struct CredentialBinding {
    provider: String,
    auth_mode: String,
    material_ref: String,
    scopes: Vec<String>,
}

fn parse_credential_binding(value: &str) -> Result<CredentialBinding, String> {
    let mut parts = value.splitn(4, ':');
    let provider = parts
        .next()
        .filter(|part| !part.is_empty())
        .ok_or_else(|| {
            "runx skill --credential requires <provider>:<auth_mode>:<material_ref>".to_owned()
        })?;
    let auth_mode = parts
        .next()
        .filter(|part| !part.is_empty())
        .ok_or_else(|| {
            "runx skill --credential requires <provider>:<auth_mode>:<material_ref>".to_owned()
        })?;
    let material_ref = parts
        .next()
        .filter(|part| !part.is_empty())
        .ok_or_else(|| {
            "runx skill --credential requires <provider>:<auth_mode>:<material_ref>".to_owned()
        })?;
    let scopes = parts
        .next()
        .map(|raw| {
            raw.split(',')
                .map(str::trim)
                .filter(|scope| !scope.is_empty())
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default();
    Ok(CredentialBinding {
        provider: provider.to_owned(),
        auth_mode: auth_mode.to_owned(),
        material_ref: material_ref.to_owned(),
        scopes,
    })
}

fn parse_secret_env(value: &str) -> Result<(String, String), String> {
    if value.contains('=') {
        return Err(
            "runx skill --secret-env accepts an environment variable name, not an inline value"
                .to_owned(),
        );
    }
    let name = value.trim();
    if name.is_empty() {
        return Err("runx skill --secret-env requires a non-empty env var name".to_owned());
    }
    let secret = env::var(name)
        .map_err(|_| format!("runx skill --secret-env env var '{name}' is not set"))?;
    if secret.trim().is_empty() {
        return Err("runx skill --secret-env requires a non-empty secret value".to_owned());
    }
    Ok((name.to_owned(), secret.to_owned()))
}

fn finalize_local_credential(
    state: &SkillParseState,
) -> Result<Option<LocalCredentialDescriptor>, String> {
    match (&state.credential, &state.secret_env) {
        (None, None) => Ok(None),
        (Some(_), None) => {
            Err("runx skill --credential requires --secret-env <ENV_VAR>".to_owned())
        }
        (binding, Some((env_var, secret))) => {
            let binding = binding.as_ref().ok_or_else(|| {
                "runx skill --secret-env requires --credential <provider>:<auth_mode>:<material_ref>"
                    .to_owned()
            })?;
            Ok(Some(LocalCredentialDescriptor {
                provider: binding.provider.clone(),
                auth_mode: binding.auth_mode.clone(),
                env_var: env_var.clone(),
                material_ref: binding.material_ref.clone(),
                scopes: binding.scopes.clone(),
                secret: secret.clone(),
            }))
        }
    }
}

fn reject_resolver_flags_for_skill_management_action(
    skill_path: &Path,
    state: &SkillParseState,
) -> Result<(), String> {
    if state.registry.is_none() && state.expected_digest.is_none() {
        return Ok(());
    }
    if !is_skill_management_action(skill_path) {
        return Ok(());
    }
    Err("runx skill --registry and --digest are only supported when running a skill ref".to_owned())
}

fn is_skill_management_action(skill_path: &Path) -> bool {
    if skill_path.components().count() != 1 {
        return false;
    }
    matches!(
        skill_path.to_str(),
        Some("add" | "inspect" | "publish" | "search" | "validate")
    )
}

// rust-style-allow: long-function because this is the single skill-flag dispatch
// match (--receipt-dir/--run-id/--answers/--json/--credential and positionals);
// splitting the arms would scatter the CLI parse contract.
fn parse_skill_arg(
    args: &[OsString],
    mut index: usize,
    state: &mut SkillParseState,
) -> Result<usize, String> {
    let token = string_arg(args, index)?;
    if is_retired_skill_option(&token) {
        return Err(
            "retired runx skill receipt option is not supported; use --receipt-dir".to_owned(),
        );
    }
    match token.as_str() {
        value if value.starts_with("--receipt-dir=") => {
            state.receipt_dir = Some(PathBuf::from(value.trim_start_matches("--receipt-dir=")));
        }
        "--receipt-dir" => {
            index += 1;
            state.receipt_dir = Some(PathBuf::from(string_arg(args, index)?));
        }
        value if value.starts_with("--run-id=") => {
            state.run_id = Some(value.trim_start_matches("--run-id=").to_owned());
        }
        "--run-id" => {
            index += 1;
            state.run_id = Some(string_arg(args, index)?);
        }
        value if value.starts_with("--answers=") => {
            state.answers = Some(PathBuf::from(value.trim_start_matches("--answers=")));
        }
        "--answers" => {
            index += 1;
            state.answers = Some(PathBuf::from(string_arg(args, index)?));
        }
        value if value.starts_with("--runner=") => {
            state.runner = Some(non_empty_flag_value(
                "--runner",
                value.trim_start_matches("--runner="),
            )?);
        }
        "--runner" => {
            index += 1;
            state.runner = Some(non_empty_flag_value("--runner", &string_arg(args, index)?)?);
        }
        value if value.starts_with("--registry=") => {
            state.registry = Some(non_empty_flag_value(
                "--registry",
                value.trim_start_matches("--registry="),
            )?);
        }
        "--registry" => {
            index += 1;
            state.registry = Some(non_empty_flag_value(
                "--registry",
                &string_arg(args, index)?,
            )?);
        }
        value if value.starts_with("--digest=") => {
            state.expected_digest = Some(non_empty_flag_value(
                "--digest",
                value.trim_start_matches("--digest="),
            )?);
        }
        "--digest" => {
            index += 1;
            state.expected_digest =
                Some(non_empty_flag_value("--digest", &string_arg(args, index)?)?);
        }
        value if value.starts_with("--input=") => {
            index = parse_input_arg(
                args,
                index,
                Some(value.trim_start_matches("--input=")),
                &mut state.inputs,
            )?;
        }
        "--input" => index = parse_input_arg(args, index, None, &mut state.inputs)?,
        value if value.starts_with("--credential=") => {
            state.credential = Some(parse_credential_binding(
                value.trim_start_matches("--credential="),
            )?);
        }
        "--credential" => {
            index += 1;
            state.credential = Some(parse_credential_binding(&string_arg(args, index)?)?);
        }
        value if value.starts_with("--secret-env=") => {
            state.secret_env = Some(parse_secret_env(value.trim_start_matches("--secret-env="))?);
        }
        "--secret-env" => {
            index += 1;
            state.secret_env = Some(parse_secret_env(&string_arg(args, index)?)?);
        }
        "--json" => state.json = true,
        "--non-interactive" => {}
        value if value.starts_with("--") => {
            index = parse_direct_input_arg(args, index, value, &mut state.inputs)?;
        }
        value => {
            if state.skill_path.is_some() {
                return Err(format!("unexpected runx skill argument {value}"));
            }
            state.skill_path = Some(PathBuf::from(value));
        }
    }
    Ok(index)
}

fn non_empty_flag_value(flag: &str, value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(format!("runx skill {flag} requires a non-empty value"));
    }
    Ok(value.to_owned())
}

fn is_retired_skill_option(token: &str) -> bool {
    let Some(flag) = token.strip_prefix("--") else {
        return false;
    };
    let name = flag.split_once('=').map_or(flag, |(name, _value)| name);
    name == "receipt" || name == retired_receipt_dir_option_name()
}

fn retired_receipt_dir_option_name() -> String {
    ["receipt", "Dir"].concat()
}

fn string_arg(args: &[OsString], index: usize) -> Result<String, String> {
    let value = args
        .get(index)
        .ok_or_else(|| "missing value for runx skill argument".to_owned())?;
    value
        .to_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| "runx skill arguments must be UTF-8".to_owned())
}
