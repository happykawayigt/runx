// rust-style-allow: large-file - parser parity mirrors the compact TypeScript skill parser surface.
use std::collections::BTreeMap;
use std::sync::OnceLock;

use regex::Regex;
use runx_contracts::{
    ExecutionSemantics, GovernedDisposition, InputContextCapture, JsonObject, JsonValue,
    OutcomeState, ReceiptOutcome, ReceiptSurfaceRef,
};
use runx_core::policy::{
    CwdPolicy, SandboxDeclaration, SandboxProfile, normalize_sandbox_declaration,
};
use serde::{Deserialize, Serialize};

use crate::graph::{RawGraphIr, validate_graph_document};
use crate::{ParseError, ValidationError};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawSkillIr {
    pub frontmatter: JsonObject,
    pub raw_frontmatter: String,
    pub body: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillInput {
    #[serde(rename = "type")]
    pub input_type: String,
    pub required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<JsonValue>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillRetryPolicy {
    pub max_attempts: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillIdempotencyPolicy {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
}

/// Closed set of built-in skill source kinds. The extension lane is the
/// `ExternalAdapter` variant; custom adapters are identified by the
/// external-adapter manifest, not by an open `source.type` string.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SourceKind {
    CliTool,
    Mcp,
    Catalog,
    A2a,
    Agent,
    AgentStep,
    HarnessHook,
    Graph,
    ExternalAdapter,
}

impl SourceKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            SourceKind::CliTool => "cli-tool",
            SourceKind::Mcp => "mcp",
            SourceKind::Catalog => "catalog",
            SourceKind::A2a => "a2a",
            SourceKind::Agent => "agent",
            SourceKind::AgentStep => "agent-step",
            SourceKind::HarnessHook => "harness-hook",
            SourceKind::Graph => "graph",
            SourceKind::ExternalAdapter => "external-adapter",
        }
    }
}

impl std::fmt::Display for SourceKind {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputMode {
    Args,
    Stdin,
    None,
}

impl InputMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            InputMode::Args => "args",
            InputMode::Stdin => "stdin",
            InputMode::None => "none",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillSource {
    #[serde(rename = "type")]
    pub source_type: SourceKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    pub args: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_seconds: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_mode: Option<InputMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sandbox: Option<SkillSandbox>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server: Option<SkillMcpServer>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub catalog_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<JsonObject>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_card_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_identity: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hook: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outputs: Option<JsonObject>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graph: Option<crate::ExecutionGraph>,
    pub raw: JsonObject,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillMcpServer {
    pub command: String,
    pub args: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillSandbox {
    pub profile: SandboxProfile,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd_policy: Option<CwdPolicy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env_allowlist: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<bool>,
    pub writable_paths: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub require_enforcement: Option<bool>,
    #[serde(skip)]
    pub approved_escalation: Option<bool>,
    pub raw: JsonObject,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillArtifactContract {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emits: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub named_emits: Option<BTreeMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wrap_as: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillQualityProfile {
    pub heading: String,
    pub content: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValidateSkillMode {
    Strict,
    Lenient,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ValidateSkillOptions {
    pub mode: ValidateSkillMode,
}

impl Default for ValidateSkillOptions {
    fn default() -> Self {
        Self {
            mode: ValidateSkillMode::Strict,
        }
    }
}

impl ValidateSkillOptions {
    #[must_use]
    pub const fn strict() -> Self {
        Self {
            mode: ValidateSkillMode::Strict,
        }
    }

    #[must_use]
    pub const fn lenient() -> Self {
        Self {
            mode: ValidateSkillMode::Lenient,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidatedSkill {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub body: String,
    pub source: SkillSource,
    pub inputs: BTreeMap<String, SkillInput>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth: Option<JsonValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub risk: Option<JsonValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime: Option<JsonValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry: Option<SkillRetryPolicy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idempotency: Option<SkillIdempotencyPolicy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mutating: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifacts: Option<SkillArtifactContract>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality_profile: Option<SkillQualityProfile>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_tools: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution: Option<ExecutionSemantics>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runx: Option<JsonObject>,
    pub raw: RawSkillIr,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillRunnerDefinition {
    pub name: String,
    pub default: bool,
    pub source: SkillSource,
    pub inputs: BTreeMap<String, SkillInput>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth: Option<JsonValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub risk: Option<JsonValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime: Option<JsonValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry: Option<SkillRetryPolicy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idempotency: Option<SkillIdempotencyPolicy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mutating: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifacts: Option<SkillArtifactContract>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_tools: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution: Option<ExecutionSemantics>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runx: Option<JsonObject>,
    pub raw: JsonObject,
}

struct SkillGovernance {
    retry: Option<SkillRetryPolicy>,
    idempotency: Option<SkillIdempotencyPolicy>,
    mutating: Option<bool>,
    artifacts: Option<SkillArtifactContract>,
    allowed_tools: Option<Vec<String>>,
    execution: Option<ExecutionSemantics>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CatalogKind {
    Skill,
    Graph,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CatalogAudience {
    Public,
    Builder,
    Operator,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CatalogVisibility {
    Public,
    Private,
}

impl CatalogKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            CatalogKind::Skill => "skill",
            CatalogKind::Graph => "graph",
        }
    }
}

impl CatalogAudience {
    pub fn as_str(&self) -> &'static str {
        match self {
            CatalogAudience::Public => "public",
            CatalogAudience::Builder => "builder",
            CatalogAudience::Operator => "operator",
        }
    }
}

impl CatalogVisibility {
    pub fn as_str(&self) -> &'static str {
        match self {
            CatalogVisibility::Public => "public",
            CatalogVisibility::Private => "private",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogMetadata {
    pub kind: CatalogKind,
    pub audience: CatalogAudience,
    pub visibility: CatalogVisibility,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HarnessCallerFixture {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub answers: Option<JsonObject>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approvals: Option<BTreeMap<String, bool>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceiptExpectation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skill_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graph_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HarnessExpectation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receipt: Option<ReceiptExpectation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub steps: Option<Vec<String>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RunnerHarnessCase {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runner: Option<String>,
    pub inputs: JsonObject,
    pub env: BTreeMap<String, String>,
    pub caller: HarnessCallerFixture,
    pub expect: HarnessExpectation,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RunnerHarnessManifest {
    pub cases: Vec<RunnerHarnessCase>,
}

pub fn parse_skill_markdown(markdown: &str) -> Result<RawSkillIr, ParseError> {
    static SKILL_FRONTMATTER_PATTERN: OnceLock<Result<Regex, String>> = OnceLock::new();
    let pattern = match SKILL_FRONTMATTER_PATTERN.get_or_init(|| {
        Regex::new(r"(?s)^---\r?\n(.*?)\r?\n---\r?\n?(.*)$").map_err(|error| error.to_string())
    }) {
        Ok(pattern) => pattern,
        Err(message) => {
            return Err(ParseError::InvalidDocument {
                field: "skill".to_owned(),
                message: message.clone(),
            });
        }
    };
    let Some(captures) = pattern.captures(markdown) else {
        return Err(ParseError::InvalidDocument {
            field: "skill".to_owned(),
            message: "Skill markdown must start with YAML frontmatter delimited by ---.".to_owned(),
        });
    };
    let raw_frontmatter = capture_string(&captures, 1)?;
    let body = capture_string(&captures, 2)?;
    let frontmatter = parse_yaml_object(
        &raw_frontmatter,
        "Skill frontmatter must parse to an object.",
    )?;
    Ok(RawSkillIr {
        frontmatter,
        raw_frontmatter,
        body,
    })
}

pub fn validate_skill(raw: RawSkillIr) -> Result<ValidatedSkill, ValidationError> {
    validate_skill_with_options(raw, ValidateSkillOptions::default())
}

pub fn validate_skill_with_options(
    raw: RawSkillIr,
    options: ValidateSkillOptions,
) -> Result<ValidatedSkill, ValidationError> {
    let runx = validate_runx_metadata(raw.frontmatter.get("runx"), options.mode)?;
    let source = raw
        .frontmatter
        .get("source")
        .map(|value| optional_object(Some(value), "source"))
        .transpose()?
        .flatten()
        .unwrap_or_else(default_agent_source);
    let risk = raw.frontmatter.get("risk").cloned();
    let governance = validate_skill_governance(&raw, runx.as_ref(), risk.as_ref())?;

    Ok(ValidatedSkill {
        name: required_string(raw.frontmatter.get("name"), "name")?,
        description: optional_string(raw.frontmatter.get("description"), "description")?,
        body: raw.body.clone(),
        source: validate_source(&source, runx.as_ref())?,
        inputs: validate_inputs(
            optional_object(raw.frontmatter.get("inputs"), "inputs")?.unwrap_or_default(),
        )?,
        auth: raw.frontmatter.get("auth").cloned(),
        risk: risk.clone(),
        runtime: raw.frontmatter.get("runtime").cloned(),
        retry: governance.retry,
        idempotency: governance.idempotency,
        mutating: governance.mutating,
        artifacts: governance.artifacts,
        quality_profile: extract_skill_quality_profile(&raw.body),
        allowed_tools: governance.allowed_tools,
        execution: governance.execution,
        runx,
        raw,
    })
}

fn validate_runx_metadata(
    value: Option<&JsonValue>,
    mode: ValidateSkillMode,
) -> Result<Option<JsonObject>, ValidationError> {
    match value {
        None | Some(JsonValue::Null) => Ok(None),
        Some(JsonValue::Object(value)) => Ok(Some(value.clone())),
        Some(_) if mode == ValidateSkillMode::Lenient => Ok(None),
        Some(_) => Err(ValidationError::InvalidField {
            field: "runx".to_owned(),
            message: "runx must be an object when present.".to_owned(),
        }),
    }
}

fn validate_skill_governance(
    raw: &RawSkillIr,
    runx: Option<&JsonObject>,
    risk: Option<&JsonValue>,
) -> Result<SkillGovernance, ValidationError> {
    Ok(SkillGovernance {
        retry: validate_retry(
            first_value(raw.frontmatter.get("retry"), field_value(runx, "retry")),
            "retry",
        )?,
        idempotency: validate_idempotency(
            first_value(
                raw.frontmatter.get("idempotency"),
                field_value(runx, "idempotency"),
            ),
            "idempotency",
        )?,
        mutating: validate_mutating(
            first_value(
                first_value(
                    raw.frontmatter.get("mutating"),
                    nested_value(risk, "mutating"),
                ),
                field_value(runx, "mutating"),
            ),
            "mutating",
        )?,
        artifacts: validate_artifact_contract(field_value(runx, "artifacts"), "runx.artifacts")?,
        allowed_tools: validate_allowed_tools(
            field_value(runx, "allowed_tools"),
            "runx.allowed_tools",
        )?,
        execution: validate_execution_semantics(
            first_value(
                raw.frontmatter.get("execution"),
                field_value(runx, "execution"),
            ),
            "execution",
        )?,
    })
}

pub fn validate_skill_source(
    source: &JsonObject,
    runx: Option<&JsonObject>,
) -> Result<SkillSource, ValidationError> {
    validate_source(source, runx)
}

pub fn validate_skill_artifact_contract(
    value: Option<&JsonValue>,
    field: &str,
) -> Result<Option<SkillArtifactContract>, ValidationError> {
    validate_artifact_contract(value, field)
}

pub fn extract_skill_quality_profile(body: &str) -> Option<SkillQualityProfile> {
    extract_markdown_section(body, "Quality Profile", 2).map(|content| SkillQualityProfile {
        heading: "Quality Profile".to_owned(),
        content,
    })
}

pub(crate) fn validate_runner_definition(
    name: &str,
    runner: JsonObject,
) -> Result<SkillRunnerDefinition, ValidationError> {
    let runx = optional_object(runner.get("runx"), &format!("runners.{name}.runx"))?;
    super::runner::resolve_post_run_reflect_policy(runx.as_ref(), &format!("runners.{name}.runx"))?;
    let source_record = optional_object(runner.get("source"), &format!("runners.{name}.source"))?
        .unwrap_or_else(|| runner.clone());
    let risk = runner.get("risk").cloned();
    let governance = validate_runner_governance(name, &runner, runx.as_ref(), risk.as_ref())?;
    Ok(SkillRunnerDefinition {
        name: name.to_owned(),
        default: optional_bool(runner.get("default"), &format!("runners.{name}.default"))?
            .unwrap_or(false),
        source: validate_source(&source_record, runx.as_ref())?,
        inputs: validate_inputs(
            optional_object(runner.get("inputs"), &format!("runners.{name}.inputs"))?
                .unwrap_or_default(),
        )?,
        auth: runner.get("auth").cloned(),
        risk: risk.clone(),
        runtime: runner.get("runtime").cloned(),
        retry: governance.retry,
        idempotency: governance.idempotency,
        mutating: governance.mutating,
        artifacts: governance.artifacts,
        allowed_tools: governance.allowed_tools,
        execution: governance.execution,
        runx,
        raw: runner,
    })
}

fn validate_runner_governance(
    name: &str,
    runner: &JsonObject,
    runx: Option<&JsonObject>,
    risk: Option<&JsonValue>,
) -> Result<SkillGovernance, ValidationError> {
    Ok(SkillGovernance {
        retry: validate_retry(
            first_value(runner.get("retry"), field_value(runx, "retry")),
            &format!("runners.{name}.retry"),
        )?,
        idempotency: validate_idempotency(
            first_value(runner.get("idempotency"), field_value(runx, "idempotency")),
            &format!("runners.{name}.idempotency"),
        )?,
        mutating: validate_mutating(
            first_value(
                first_value(runner.get("mutating"), nested_value(risk, "mutating")),
                field_value(runx, "mutating"),
            ),
            &format!("runners.{name}.mutating"),
        )?,
        artifacts: validate_artifact_contract(
            first_value(runner.get("artifacts"), field_value(runx, "artifacts")),
            &format!("runners.{name}.artifacts"),
        )?,
        allowed_tools: validate_allowed_tools(
            field_value(runx, "allowed_tools"),
            &format!("runners.{name}.runx.allowed_tools"),
        )?,
        execution: validate_execution_semantics(
            first_value(runner.get("execution"), field_value(runx, "execution")),
            &format!("runners.{name}.execution"),
        )?,
    })
}

pub(crate) fn validate_catalog_metadata(
    value: Option<JsonObject>,
    label: &str,
) -> Result<Option<CatalogMetadata>, ValidationError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let kind = match required_string(value.get("kind"), &format!("{label}.kind"))?.as_str() {
        "skill" => CatalogKind::Skill,
        "graph" => CatalogKind::Graph,
        _ => {
            return Err(validation_error(format!(
                "{label}.kind must be skill or graph."
            )));
        }
    };
    let audience =
        match required_string(value.get("audience"), &format!("{label}.audience"))?.as_str() {
            "public" => CatalogAudience::Public,
            "builder" => CatalogAudience::Builder,
            "operator" => CatalogAudience::Operator,
            _ => {
                return Err(validation_error(format!(
                    "{label}.audience must be public, builder, or operator."
                )));
            }
        };
    let visibility = match optional_string(value.get("visibility"), &format!("{label}.visibility"))?
        .as_deref()
    {
        Some("public") | None => CatalogVisibility::Public,
        Some("private") => CatalogVisibility::Private,
        Some(_) => {
            return Err(validation_error(format!(
                "{label}.visibility must be public or private."
            )));
        }
    };
    Ok(Some(CatalogMetadata {
        kind,
        audience,
        visibility,
    }))
}

pub(crate) fn validate_harness_manifest(
    value: Option<JsonObject>,
    field: &str,
) -> Result<Option<RunnerHarnessManifest>, ValidationError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let cases = required_plain_array(value.get("cases"), &format!("{field}.cases"))?
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            validate_harness_case(
                required_object(Some(entry), &format!("{field}.cases[{index}]"))?,
                &format!("{field}.cases[{index}]"),
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Some(RunnerHarnessManifest { cases }))
}

fn validate_harness_case(
    value: &JsonObject,
    field: &str,
) -> Result<RunnerHarnessCase, ValidationError> {
    Ok(RunnerHarnessCase {
        name: required_string(value.get("name"), &format!("{field}.name"))?,
        runner: optional_non_empty_string(value.get("runner"), &format!("{field}.runner"))?,
        inputs: optional_object(value.get("inputs"), &format!("{field}.inputs"))?
            .unwrap_or_default(),
        env: validate_string_object(
            optional_object(value.get("env"), &format!("{field}.env"))?.unwrap_or_default(),
            &format!("{field}.env"),
        )?,
        caller: validate_harness_caller(
            optional_object(value.get("caller"), &format!("{field}.caller"))?.unwrap_or_default(),
            &format!("{field}.caller"),
        )?,
        expect: validate_harness_expectation(
            required_object(value.get("expect"), &format!("{field}.expect"))?,
            &format!("{field}.expect"),
        )?,
    })
}

fn validate_source(
    source: &JsonObject,
    runx: Option<&JsonObject>,
) -> Result<SkillSource, ValidationError> {
    let source_type = required_string(source.get("type"), "source.type")?;
    let args = optional_string_array(source.get("args"), "source.args")?.unwrap_or_default();
    let input_mode = optional_input_mode(source.get("input_mode"))?;
    let timeout_seconds = optional_u64(source.get("timeout_seconds"), "source.timeout_seconds")?;

    if source_type == "cli-tool" {
        required_string(source.get("command"), "source.command")?;
    }
    validate_agent_command_boundary(source, &source_type)?;
    let source_kind = parse_source_kind(&source_type, "source.type")?;

    Ok(SkillSource {
        command: optional_string(source.get("command"), "source.command")?,
        args,
        cwd: optional_string(source.get("cwd"), "source.cwd")?,
        timeout_seconds,
        input_mode,
        sandbox: validate_sandbox(first_value(
            source.get("sandbox"),
            field_value(runx, "sandbox"),
        ))?,
        server: validate_mcp_server(source, &source_type)?,
        catalog_ref: validate_catalog_ref(source, &source_type)?,
        tool: validate_mcp_tool(source, &source_type)?,
        arguments: optional_object(source.get("arguments"), "source.arguments")?,
        agent_card_url: validate_a2a_url(source, &source_type)?,
        agent_identity: optional_string(source.get("agent_identity"), "source.agent_identity")?,
        agent: validate_agent(source, &source_type)?,
        task: validate_task(source, &source_type)?,
        hook: validate_hook(source, &source_type)?,
        outputs: optional_object(source.get("outputs"), "source.outputs")?,
        graph: validate_graph_source(source, &source_type)?,
        raw: source.clone(),
        source_type: source_kind,
    })
}

fn parse_source_kind(value: &str, field: &str) -> Result<SourceKind, ValidationError> {
    match value {
        "cli-tool" => Ok(SourceKind::CliTool),
        "mcp" => Ok(SourceKind::Mcp),
        "catalog" => Ok(SourceKind::Catalog),
        "a2a" => Ok(SourceKind::A2a),
        "agent" => Ok(SourceKind::Agent),
        "agent-step" => Ok(SourceKind::AgentStep),
        "harness-hook" => Ok(SourceKind::HarnessHook),
        "graph" => Ok(SourceKind::Graph),
        "external-adapter" => Ok(SourceKind::ExternalAdapter),
        other => Err(validation_error(format!(
            "{field} {other} is not a supported source type."
        ))),
    }
}

fn validate_sandbox(value: Option<&JsonValue>) -> Result<Option<SkillSandbox>, ValidationError> {
    let Some(record) = value else {
        return Ok(None);
    };
    let record = required_object(Some(record), "sandbox")?;
    let profile = required_sandbox_profile(record.get("profile"), "sandbox.profile")?;
    let cwd_policy = optional_cwd_policy(record.get("cwd_policy"))?;
    let env_allowlist =
        optional_string_array(record.get("env_allowlist"), "sandbox.env_allowlist")?;
    let network = optional_bool(record.get("network"), "sandbox.network")?;
    let writable_paths =
        optional_string_array(record.get("writable_paths"), "sandbox.writable_paths")?
            .unwrap_or_default();
    let require_enforcement = optional_bool(
        record.get("require_enforcement"),
        "sandbox.require_enforcement",
    )?;
    let declaration = sandbox_declaration(
        &profile,
        cwd_policy.as_deref(),
        env_allowlist.clone(),
        network,
        Some(writable_paths.clone()),
        require_enforcement,
    )?;
    let normalized = normalize_sandbox_declaration(Some(&declaration));
    Ok(Some(SkillSandbox {
        profile: normalized.profile,
        cwd_policy: Some(normalized.cwd_policy),
        env_allowlist: normalized.env_allowlist,
        network: Some(normalized.network),
        writable_paths: normalized.writable_paths,
        require_enforcement: Some(normalized.require_enforcement),
        // TS currently preserves approvedEscalation only inside raw.
        approved_escalation: None,
        raw: record.clone(),
    }))
}

fn validate_execution_semantics(
    value: Option<&JsonValue>,
    field: &str,
) -> Result<Option<ExecutionSemantics>, ValidationError> {
    let Some(record) = optional_object(value, field)? else {
        return Ok(None);
    };
    Ok(Some(ExecutionSemantics {
        disposition: optional_disposition(
            record.get("disposition"),
            &format!("{field}.disposition"),
        )?,
        outcome_state: optional_outcome_state(
            record.get("outcome_state"),
            &format!("{field}.outcome_state"),
        )?,
        outcome: validate_outcome(record.get("outcome"), &format!("{field}.outcome"))?,
        input_context: validate_input_context(
            record.get("input_context"),
            &format!("{field}.input_context"),
        )?,
        surface_refs: validate_surface_refs(
            record.get("surface_refs"),
            &format!("{field}.surface_refs"),
        )?,
        evidence_refs: validate_surface_refs(
            record.get("evidence_refs"),
            &format!("{field}.evidence_refs"),
        )?,
    }))
}

fn validate_artifact_contract(
    value: Option<&JsonValue>,
    field: &str,
) -> Result<Option<SkillArtifactContract>, ValidationError> {
    let Some(record) = optional_object(value, field)? else {
        return Ok(None);
    };
    let emits = match record.get("emits") {
        Some(JsonValue::String(value)) => Some(vec![value.clone()]),
        value => optional_string_array(value, &format!("{field}.emits"))?,
    };
    let named_emits = validate_named_emits(
        first_value(record.get("named_emits"), record.get("namedEmits")),
        &format!("{field}.named_emits"),
    )?;
    let wrap_as = optional_non_empty_string(
        first_value(record.get("wrap_as"), record.get("wrapAs")),
        &format!("{field}.wrap_as"),
    )?;
    if emits.is_none() && named_emits.is_none() && wrap_as.is_none() {
        return Ok(None);
    }
    Ok(Some(SkillArtifactContract {
        emits,
        named_emits,
        wrap_as,
    }))
}

fn validate_inputs(inputs: JsonObject) -> Result<BTreeMap<String, SkillInput>, ValidationError> {
    inputs
        .into_iter()
        .map(|(name, value)| {
            let field = format!("inputs.{name}");
            let input = required_object(Some(&value), &field)?;
            Ok((
                name.clone(),
                SkillInput {
                    input_type: optional_string(input.get("type"), &format!("{field}.type"))?
                        .unwrap_or_else(|| "string".to_owned()),
                    required: optional_bool(input.get("required"), &format!("{field}.required"))?
                        .unwrap_or(false),
                    description: optional_string(
                        input.get("description"),
                        &format!("{field}.description"),
                    )?,
                    default: input.get("default").cloned(),
                },
            ))
        })
        .collect()
}

fn validate_retry(
    value: Option<&JsonValue>,
    field: &str,
) -> Result<Option<SkillRetryPolicy>, ValidationError> {
    let Some(retry) = optional_object(value, field)? else {
        return Ok(None);
    };
    let max_attempts =
        optional_u64(retry.get("max_attempts"), &format!("{field}.max_attempts"))?.unwrap_or(1);
    if max_attempts == 0 {
        return Err(validation_error(format!(
            "{field}.max_attempts must be a positive integer."
        )));
    }
    Ok(Some(SkillRetryPolicy { max_attempts }))
}

fn validate_idempotency(
    value: Option<&JsonValue>,
    field: &str,
) -> Result<Option<SkillIdempotencyPolicy>, ValidationError> {
    match value {
        None | Some(JsonValue::Null) => Ok(None),
        Some(JsonValue::String(value)) if value.trim().is_empty() => {
            Err(validation_error(format!("{field} must not be empty.")))
        }
        Some(JsonValue::String(value)) => Ok(Some(SkillIdempotencyPolicy {
            key: Some(value.clone()),
        })),
        Some(value) => {
            let record = required_object(Some(value), field)?;
            Ok(Some(SkillIdempotencyPolicy {
                key: optional_non_empty_string(record.get("key"), &format!("{field}.key"))?,
            }))
        }
    }
}

fn validate_mutating(
    value: Option<&JsonValue>,
    field: &str,
) -> Result<Option<bool>, ValidationError> {
    optional_bool(value, field)
}

fn parse_yaml_object(source: &str, object_error: &str) -> Result<JsonObject, ParseError> {
    let parsed: JsonValue =
        serde_norway::from_str(source).map_err(|error| ParseError::InvalidYaml {
            field: "skill_frontmatter".to_owned(),
            message: error.to_string(),
        })?;
    match parsed {
        JsonValue::Object(object) => Ok(object),
        _ => Err(ParseError::InvalidDocument {
            field: "skill_frontmatter".to_owned(),
            message: object_error.to_owned(),
        }),
    }
}

fn validation_error(message: impl Into<String>) -> ValidationError {
    ValidationError::InvalidField {
        field: "skill".to_owned(),
        message: message.into(),
    }
}

fn required_string(value: Option<&JsonValue>, field: &str) -> Result<String, ValidationError> {
    match optional_string(value, field)? {
        Some(value) if !value.is_empty() => Ok(value),
        _ => Err(ValidationError::MissingField {
            field: field.to_owned(),
        }),
    }
}

fn optional_string(
    value: Option<&JsonValue>,
    field: &str,
) -> Result<Option<String>, ValidationError> {
    match value {
        None | Some(JsonValue::Null) => Ok(None),
        Some(JsonValue::String(value)) => Ok(Some(value.clone())),
        Some(_) => Err(validation_error(format!("{field} must be a string."))),
    }
}

fn optional_non_empty_string(
    value: Option<&JsonValue>,
    field: &str,
) -> Result<Option<String>, ValidationError> {
    let Some(value) = optional_string(value, field)? else {
        return Ok(None);
    };
    if value.trim().is_empty() {
        return Err(validation_error(format!("{field} must not be empty.")));
    }
    Ok(Some(value))
}

fn required_object<'a>(
    value: Option<&'a JsonValue>,
    field: &str,
) -> Result<&'a JsonObject, ValidationError> {
    match value {
        Some(JsonValue::Object(value)) => Ok(value),
        None | Some(JsonValue::Null) => Err(validation_error(format!("{field} is required."))),
        Some(_) => Err(validation_error(format!("{field} must be an object."))),
    }
}

fn optional_object(
    value: Option<&JsonValue>,
    field: &str,
) -> Result<Option<JsonObject>, ValidationError> {
    match value {
        None | Some(JsonValue::Null) => Ok(None),
        Some(JsonValue::Object(value)) => Ok(Some(value.clone())),
        Some(_) => Err(validation_error(format!("{field} must be an object."))),
    }
}

fn required_plain_array<'a>(
    value: Option<&'a JsonValue>,
    field: &str,
) -> Result<&'a [JsonValue], ValidationError> {
    match value {
        Some(JsonValue::Array(values)) => Ok(values),
        None | Some(JsonValue::Null) => Err(validation_error(format!("{field} is required."))),
        Some(_) => Err(validation_error(format!("{field} must be an array."))),
    }
}

fn optional_string_array(
    value: Option<&JsonValue>,
    field: &str,
) -> Result<Option<Vec<String>>, ValidationError> {
    match value {
        None | Some(JsonValue::Null) => Ok(None),
        Some(JsonValue::Array(values)) => values
            .iter()
            .map(|value| match value {
                JsonValue::String(value) => Ok(value.clone()),
                _ => Err(validation_error(format!(
                    "{field} must be an array of strings."
                ))),
            })
            .collect::<Result<Vec<_>, _>>()
            .map(Some),
        Some(_) => Err(validation_error(format!(
            "{field} must be an array of strings."
        ))),
    }
}

fn optional_bool(value: Option<&JsonValue>, field: &str) -> Result<Option<bool>, ValidationError> {
    match value {
        None | Some(JsonValue::Null) => Ok(None),
        Some(JsonValue::Bool(value)) => Ok(Some(*value)),
        Some(_) => Err(validation_error(format!("{field} must be a boolean."))),
    }
}

fn optional_u64(value: Option<&JsonValue>, field: &str) -> Result<Option<u64>, ValidationError> {
    match value {
        None | Some(JsonValue::Null) => Ok(None),
        Some(JsonValue::Number(number)) => {
            let Some(value) = number.as_f64() else {
                return Err(validation_error(format!(
                    "{field} must be a finite number."
                )));
            };
            if value.fract() == 0.0 && value >= 0.0 && value <= u64::MAX as f64 {
                Ok(Some(value as u64))
            } else {
                Err(validation_error(format!(
                    "{field} must be a positive integer."
                )))
            }
        }
        Some(_) => Err(validation_error(format!(
            "{field} must be a finite number."
        ))),
    }
}

fn optional_input_mode(value: Option<&JsonValue>) -> Result<Option<InputMode>, ValidationError> {
    let Some(value) = optional_string(value, "source.input_mode")? else {
        return Ok(None);
    };
    match value.as_str() {
        "args" => Ok(Some(InputMode::Args)),
        "stdin" => Ok(Some(InputMode::Stdin)),
        "none" => Ok(Some(InputMode::None)),
        _ => Err(validation_error(
            "source.input_mode must be args, stdin, or none.",
        )),
    }
}

fn first_value<'a>(
    left: Option<&'a JsonValue>,
    right: Option<&'a JsonValue>,
) -> Option<&'a JsonValue> {
    match left {
        None | Some(JsonValue::Null) => right,
        Some(value) => Some(value),
    }
}

fn field_value<'a>(object: Option<&'a JsonObject>, field: &str) -> Option<&'a JsonValue> {
    object.and_then(|object| object.get(field))
}

fn nested_value<'a>(value: Option<&'a JsonValue>, field: &str) -> Option<&'a JsonValue> {
    match value {
        Some(JsonValue::Object(object)) => object.get(field),
        _ => None,
    }
}

fn default_agent_source() -> JsonObject {
    [("type".to_owned(), JsonValue::String("agent".to_owned()))]
        .into_iter()
        .collect()
}

fn capture_string(captures: &regex::Captures<'_>, index: usize) -> Result<String, ParseError> {
    captures
        .get(index)
        .map(|value| value.as_str().to_owned())
        .ok_or_else(|| ParseError::InvalidDocument {
            field: "skill".to_owned(),
            message: "Skill markdown must start with YAML frontmatter delimited by ---.".to_owned(),
        })
}

fn validate_mcp_server(
    source: &JsonObject,
    source_type: &str,
) -> Result<Option<SkillMcpServer>, ValidationError> {
    if source_type != "mcp" {
        return Ok(None);
    }
    let server = required_object(source.get("server"), "source.server")?;
    Ok(Some(SkillMcpServer {
        command: required_string(server.get("command"), "source.server.command")?,
        args: optional_string_array(server.get("args"), "source.server.args")?.unwrap_or_default(),
        cwd: optional_string(server.get("cwd"), "source.server.cwd")?,
    }))
}

fn validate_mcp_tool(
    source: &JsonObject,
    source_type: &str,
) -> Result<Option<String>, ValidationError> {
    if source_type == "mcp" {
        return Ok(Some(required_string(source.get("tool"), "source.tool")?));
    }
    optional_string(source.get("tool"), "source.tool")
}

fn validate_catalog_ref(
    source: &JsonObject,
    source_type: &str,
) -> Result<Option<String>, ValidationError> {
    if source_type == "catalog" {
        return Ok(Some(required_string(
            source.get("catalog_ref"),
            "source.catalog_ref",
        )?));
    }
    optional_string(source.get("catalog_ref"), "source.catalog_ref")
}

fn validate_a2a_url(
    source: &JsonObject,
    source_type: &str,
) -> Result<Option<String>, ValidationError> {
    if source_type == "a2a" {
        return Ok(Some(required_string(
            source.get("agent_card_url"),
            "source.agent_card_url",
        )?));
    }
    optional_string(source.get("agent_card_url"), "source.agent_card_url")
}

fn validate_agent(
    source: &JsonObject,
    source_type: &str,
) -> Result<Option<String>, ValidationError> {
    if source_type == "agent-step" {
        return Ok(Some(required_string(source.get("agent"), "source.agent")?));
    }
    optional_string(source.get("agent"), "source.agent")
}

fn validate_task(
    source: &JsonObject,
    source_type: &str,
) -> Result<Option<String>, ValidationError> {
    if matches!(source_type, "agent-step" | "a2a") {
        return Ok(Some(required_string(source.get("task"), "source.task")?));
    }
    optional_string(source.get("task"), "source.task")
}

fn validate_hook(
    source: &JsonObject,
    source_type: &str,
) -> Result<Option<String>, ValidationError> {
    if source_type == "harness-hook" {
        return Ok(Some(required_string(source.get("hook"), "source.hook")?));
    }
    optional_string(source.get("hook"), "source.hook")
}

fn validate_graph_source(
    source: &JsonObject,
    source_type: &str,
) -> Result<Option<crate::ExecutionGraph>, ValidationError> {
    if source_type != "graph" {
        return Ok(None);
    }
    let graph = required_object(source.get("graph"), "source.graph")?.clone();
    validate_graph_document(graph.clone(), Some(RawGraphIr { document: graph })).map(Some)
}

fn validate_agent_command_boundary(
    source: &JsonObject,
    source_type: &str,
) -> Result<(), ValidationError> {
    if matches!(source_type, "agent-step" | "harness-hook")
        && (source.contains_key("command") || source.contains_key("args"))
    {
        return Err(validation_error(format!(
            "{source_type} sources must not declare source.command or source.args."
        )));
    }
    Ok(())
}

fn required_sandbox_profile(
    value: Option<&JsonValue>,
    field: &str,
) -> Result<String, ValidationError> {
    let profile = required_string(value, field)?;
    if matches!(
        profile.as_str(),
        "readonly" | "workspace-write" | "network" | "unrestricted-local-dev"
    ) {
        return Ok(profile);
    }
    Err(validation_error(format!(
        "{field} must be readonly, workspace-write, network, or unrestricted-local-dev."
    )))
}

fn optional_cwd_policy(value: Option<&JsonValue>) -> Result<Option<String>, ValidationError> {
    let Some(value) = optional_string(value, "sandbox.cwd_policy")? else {
        return Ok(None);
    };
    if matches!(value.as_str(), "skill-directory" | "workspace" | "custom") {
        return Ok(Some(value));
    }
    Err(validation_error(
        "sandbox.cwd_policy must be skill-directory, workspace, or custom.",
    ))
}

fn sandbox_declaration(
    profile: &str,
    cwd_policy: Option<&str>,
    env_allowlist: Option<Vec<String>>,
    network: Option<bool>,
    writable_paths: Option<Vec<String>>,
    require_enforcement: Option<bool>,
) -> Result<SandboxDeclaration, ValidationError> {
    Ok(SandboxDeclaration {
        profile: match profile {
            "readonly" => SandboxProfile::Readonly,
            "workspace-write" => SandboxProfile::WorkspaceWrite,
            "network" => SandboxProfile::Network,
            "unrestricted-local-dev" => SandboxProfile::UnrestrictedLocalDev,
            _ => return Err(validation_error("sandbox.profile is invalid.")),
        },
        cwd_policy: match cwd_policy {
            None => None,
            Some("skill-directory") => Some(CwdPolicy::SkillDirectory),
            Some("workspace") => Some(CwdPolicy::Workspace),
            Some("custom") => Some(CwdPolicy::Custom),
            Some(_) => return Err(validation_error("sandbox.cwd_policy is invalid.")),
        },
        env_allowlist,
        network,
        writable_paths,
        require_enforcement,
    })
}

fn validate_outcome(
    value: Option<&JsonValue>,
    field: &str,
) -> Result<Option<ReceiptOutcome>, ValidationError> {
    let Some(record) = optional_object(value, field)? else {
        return Ok(None);
    };
    Ok(Some(ReceiptOutcome {
        code: optional_string(record.get("code"), &format!("{field}.code"))?,
        summary: optional_string(record.get("summary"), &format!("{field}.summary"))?,
        observed_at: optional_string(record.get("observed_at"), &format!("{field}.observed_at"))?,
        data: optional_object(record.get("data"), &format!("{field}.data"))?,
    }))
}

fn validate_input_context(
    value: Option<&JsonValue>,
    field: &str,
) -> Result<Option<InputContextCapture>, ValidationError> {
    let Some(record) = optional_object(value, field)? else {
        return Ok(None);
    };
    let max_bytes = optional_u64(record.get("max_bytes"), &format!("{field}.max_bytes"))?;
    if matches!(max_bytes, Some(0)) {
        return Err(validation_error(format!(
            "{field}.max_bytes must be a positive integer."
        )));
    }
    Ok(Some(InputContextCapture {
        capture: optional_bool(record.get("capture"), &format!("{field}.capture"))?,
        source: optional_string(record.get("source"), &format!("{field}.source"))?,
        max_bytes,
        snapshot: record.get("snapshot").cloned(),
    }))
}

fn validate_surface_refs(
    value: Option<&JsonValue>,
    field: &str,
) -> Result<Option<Vec<ReceiptSurfaceRef>>, ValidationError> {
    let Some(values) = optional_array(value, field)? else {
        return Ok(None);
    };
    values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let record = required_object(Some(value), &format!("{field}[{index}]"))?;
            Ok(ReceiptSurfaceRef {
                surface_type: required_string(
                    record.get("type"),
                    &format!("{field}[{index}].type"),
                )?,
                uri: required_string(record.get("uri"), &format!("{field}[{index}].uri"))?,
                label: optional_string(record.get("label"), &format!("{field}[{index}].label"))?,
            })
        })
        .collect::<Result<Vec<_>, _>>()
        .map(Some)
}

fn optional_array<'a>(
    value: Option<&'a JsonValue>,
    field: &str,
) -> Result<Option<&'a [JsonValue]>, ValidationError> {
    match value {
        None | Some(JsonValue::Null) => Ok(None),
        Some(JsonValue::Array(values)) => Ok(Some(values)),
        Some(_) => Err(validation_error(format!(
            "{field} must be an array when present."
        ))),
    }
}

fn optional_disposition(
    value: Option<&JsonValue>,
    field: &str,
) -> Result<Option<GovernedDisposition>, ValidationError> {
    match optional_string(value, field)?.as_deref() {
        None => Ok(None),
        Some("completed") => Ok(Some(GovernedDisposition::Completed)),
        Some("needs_agent") => Ok(Some(GovernedDisposition::NeedsAgent)),
        Some("policy_denied") => Ok(Some(GovernedDisposition::PolicyDenied)),
        Some("approval_required") => Ok(Some(GovernedDisposition::ApprovalRequired)),
        Some("observing") => Ok(Some(GovernedDisposition::Observing)),
        Some("escalated") => Ok(Some(GovernedDisposition::Escalated)),
        Some(_) => Err(validation_error(format!(
            "{field} must be one of completed, needs_agent, policy_denied, approval_required, observing, escalated."
        ))),
    }
}

fn optional_outcome_state(
    value: Option<&JsonValue>,
    field: &str,
) -> Result<Option<OutcomeState>, ValidationError> {
    match optional_string(value, field)?.as_deref() {
        None => Ok(None),
        Some("pending") => Ok(Some(OutcomeState::Pending)),
        Some("complete") => Ok(Some(OutcomeState::Complete)),
        Some("expired") => Ok(Some(OutcomeState::Expired)),
        Some(_) => Err(validation_error(format!(
            "{field} must be one of pending, complete, or expired."
        ))),
    }
}

fn validate_named_emits(
    value: Option<&JsonValue>,
    field: &str,
) -> Result<Option<BTreeMap<String, String>>, ValidationError> {
    let Some(record) = optional_object(value, field)? else {
        return Ok(None);
    };
    record
        .into_iter()
        .map(|(key, value)| {
            let JsonValue::String(value) = value else {
                return Err(validation_error(format!(
                    "{field}.{key} must be a non-empty string."
                )));
            };
            if value.trim().is_empty() {
                return Err(validation_error(format!(
                    "{field}.{key} must be a non-empty string."
                )));
            }
            Ok((key, value))
        })
        .collect::<Result<BTreeMap<_, _>, _>>()
        .map(Some)
}

fn validate_allowed_tools(
    value: Option<&JsonValue>,
    field: &str,
) -> Result<Option<Vec<String>>, ValidationError> {
    let Some(values) = optional_string_array(value, field)? else {
        return Ok(None);
    };
    for value in &values {
        if value.trim().is_empty() {
            return Err(validation_error(format!(
                "{field} entries must not be empty."
            )));
        }
    }
    Ok(Some(values))
}

fn validate_string_object(
    value: JsonObject,
    field: &str,
) -> Result<BTreeMap<String, String>, ValidationError> {
    value
        .into_iter()
        .map(|(key, value)| match value {
            JsonValue::String(value) => Ok((key, value)),
            _ => Err(validation_error(format!("{field}.{key} must be a string."))),
        })
        .collect()
}

fn validate_harness_caller(
    value: JsonObject,
    field: &str,
) -> Result<HarnessCallerFixture, ValidationError> {
    Ok(HarnessCallerFixture {
        answers: optional_object(value.get("answers"), &format!("{field}.answers"))?,
        approvals: Some(validate_bool_object(
            optional_object(value.get("approvals"), &format!("{field}.approvals"))?
                .unwrap_or_default(),
            &format!("{field}.approvals"),
        )?),
    })
}

fn validate_bool_object(
    value: JsonObject,
    field: &str,
) -> Result<BTreeMap<String, bool>, ValidationError> {
    value
        .into_iter()
        .map(|(key, value)| match value {
            JsonValue::Bool(value) => Ok((key, value)),
            _ => Err(validation_error(format!(
                "{field}.{key} must be a boolean."
            ))),
        })
        .collect()
}

fn validate_harness_expectation(
    value: &JsonObject,
    field: &str,
) -> Result<HarnessExpectation, ValidationError> {
    Ok(HarnessExpectation {
        status: optional_harness_status(value.get("status"), &format!("{field}.status"))?,
        receipt: validate_receipt_expectation(
            optional_object(value.get("receipt"), &format!("{field}.receipt"))?,
            &format!("{field}.receipt"),
        )?,
        steps: optional_string_array(value.get("steps"), &format!("{field}.steps"))?,
    })
}

fn validate_receipt_expectation(
    value: Option<JsonObject>,
    field: &str,
) -> Result<Option<ReceiptExpectation>, ValidationError> {
    let Some(value) = value else {
        return Ok(None);
    };
    Ok(Some(ReceiptExpectation {
        kind: optional_receipt_kind(value.get("kind"), &format!("{field}.kind"))?,
        status: optional_receipt_status(value.get("status"), &format!("{field}.status"))?,
        skill_name: optional_string(value.get("skill_name"), &format!("{field}.skill_name"))?,
        source_type: optional_string(value.get("source_type"), &format!("{field}.source_type"))?,
        graph_name: optional_string(value.get("graph_name"), &format!("{field}.graph_name"))?,
        owner: optional_string(value.get("owner"), &format!("{field}.owner"))?,
    }))
}

fn optional_harness_status(
    value: Option<&JsonValue>,
    field: &str,
) -> Result<Option<String>, ValidationError> {
    validate_enum(
        value,
        field,
        &[
            "sealed",
            "failure",
            "needs_agent",
            "policy_denied",
            "escalated",
        ],
    )
}

fn optional_receipt_status(
    value: Option<&JsonValue>,
    field: &str,
) -> Result<Option<String>, ValidationError> {
    validate_enum(value, field, &["sealed", "failure"])
}

fn optional_receipt_kind(
    value: Option<&JsonValue>,
    field: &str,
) -> Result<Option<String>, ValidationError> {
    validate_enum(value, field, &["receipt"])
}

fn validate_enum(
    value: Option<&JsonValue>,
    field: &str,
    allowed: &[&str],
) -> Result<Option<String>, ValidationError> {
    let Some(value) = optional_string(value, field)? else {
        return Ok(None);
    };
    if allowed.iter().any(|allowed| *allowed == value) {
        return Ok(Some(value));
    }
    Err(validation_error(format!(
        "{field} must be {}.",
        allowed.join(", ")
    )))
}

fn extract_markdown_section(body: &str, heading: &str, level: usize) -> Option<String> {
    let heading_prefix = "#".repeat(level);
    let boundary = "#".repeat(level + 1);
    let lines = body.lines().collect::<Vec<_>>();
    let start = lines.iter().position(|line| {
        line.trim()
            .eq_ignore_ascii_case(&format!("{heading_prefix} {heading}"))
    })?;
    let mut collected = Vec::new();
    for line in lines.iter().skip(start + 1) {
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') && !trimmed.starts_with(&boundary) {
            break;
        }
        collected.push(*line);
    }
    let content = trim_blank_lines(&collected).join("\n").trim().to_owned();
    if content.is_empty() {
        None
    } else {
        Some(content)
    }
}

fn trim_blank_lines<'a>(lines: &'a [&'a str]) -> Vec<&'a str> {
    let mut start = 0;
    let mut end = lines.len();
    while start < end && lines[start].trim().is_empty() {
        start += 1;
    }
    while end > start && lines[end - 1].trim().is_empty() {
        end -= 1;
    }
    lines[start..end].to_vec()
}
