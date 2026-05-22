// rust-style-allow: large-file because the managed-agent parity slice keeps
// agent and agent-step invocation, telemetry, and metadata together until live
// provider adapters create natural module boundaries.
use std::time::Instant;

use runx_contracts::{
    AgentActInvocation, JsonNumber, JsonObject, JsonValue, ResolutionRequest, ResolutionResponse,
    ResolutionResponseActor,
};

use crate::RuntimeError;
use crate::adapter::{InvocationStatus, SkillAdapter, SkillInvocation, SkillOutput};
use crate::agent_invocation::{
    AgentActInvocationSourceType, agent_act_resolution_request, build_agent_act_invocation,
};
use crate::config::{ManagedAgentConfig, ManagedAgentProvider};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AgentAdapterSourceType {
    Agent,
    AgentStep,
}

impl AgentAdapterSourceType {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Agent => "agent",
            Self::AgentStep => "agent-step",
        }
    }

    const fn invocation_source_type(self) -> AgentActInvocationSourceType {
        match self {
            Self::Agent => AgentActInvocationSourceType::Agent,
            Self::AgentStep => AgentActInvocationSourceType::AgentStep,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AgentExecutionTelemetry {
    pub rounds: Option<u64>,
    pub tool_calls: Option<u64>,
    pub tools: Option<Vec<String>>,
    pub tool_executions: Option<Vec<AgentToolExecutionTrace>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentToolExecutionTrace {
    pub tool: String,
    pub status: String,
    pub receipt_id: Option<String>,
    pub resolution_kind: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AgentResolution {
    pub response: ResolutionResponse,
    pub telemetry: Option<AgentExecutionTelemetry>,
}

impl AgentResolution {
    #[must_use]
    pub fn agent(payload: JsonValue, telemetry: Option<AgentExecutionTelemetry>) -> Self {
        Self {
            response: ResolutionResponse {
                actor: ResolutionResponseActor::Agent,
                payload,
            },
            telemetry,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentResolverError {
    sanitized_message: String,
}

impl AgentResolverError {
    #[must_use]
    pub fn provider_error(_message: impl Into<String>) -> Self {
        Self {
            sanitized_message: "Managed agent provider request failed.".to_owned(),
        }
    }

    #[must_use]
    pub fn sanitized(message: impl Into<String>) -> Self {
        Self {
            sanitized_message: message.into(),
        }
    }

    #[must_use]
    pub fn sanitized_message(&self) -> &str {
        &self.sanitized_message
    }
}

pub trait AgentResolver {
    fn resolve(&self, request: ResolutionRequest) -> Result<AgentResolution, AgentResolverError>;
}

#[derive(Clone, Debug)]
pub struct AgentAdapter<T> {
    source_type: AgentAdapterSourceType,
    config: ManagedAgentConfig,
    resolver: T,
}

impl<T> AgentAdapter<T> {
    #[must_use]
    pub fn new(
        source_type: AgentAdapterSourceType,
        config: ManagedAgentConfig,
        resolver: T,
    ) -> Self {
        Self {
            source_type,
            config,
            resolver,
        }
    }

    #[must_use]
    pub fn agent(config: ManagedAgentConfig, resolver: T) -> Self {
        Self::new(AgentAdapterSourceType::Agent, config, resolver)
    }

    #[must_use]
    pub fn agent_step(config: ManagedAgentConfig, resolver: T) -> Self {
        Self::new(AgentAdapterSourceType::AgentStep, config, resolver)
    }
}

impl<T> SkillAdapter for AgentAdapter<T>
where
    T: AgentResolver,
{
    fn adapter_type(&self) -> &'static str {
        self.source_type.as_str()
    }

    fn invoke(&self, request: SkillInvocation) -> Result<SkillOutput, RuntimeError> {
        let started = Instant::now();
        if request.source.source_type.as_str() != self.source_type.as_str() {
            return Err(RuntimeError::UnsupportedAdapter {
                adapter_type: request.source.source_type.as_str().to_owned(),
            });
        }

        let resolution_request =
            agent_act_resolution_request(&request, self.source_type.invocation_source_type());
        match self.resolver.resolve(resolution_request) {
            Ok(resolution) => {
                let metadata = native_agent_metadata(
                    self.source_type,
                    &request,
                    &self.config,
                    "success",
                    resolution.telemetry.as_ref(),
                );
                Ok(success_output(resolution, started, metadata)?)
            }
            Err(error) => Ok(failure_output(
                error.sanitized_message(),
                started,
                native_agent_metadata(self.source_type, &request, &self.config, "failure", None),
            )),
        }
    }
}

#[must_use]
pub fn build_managed_agent_act_invocation(
    request: &SkillInvocation,
    source_type: AgentAdapterSourceType,
) -> AgentActInvocation {
    build_agent_act_invocation(request, source_type.invocation_source_type())
}

fn skill_name(request: &SkillInvocation, source_type: AgentAdapterSourceType) -> String {
    if request.skill_name.is_empty() {
        return match source_type {
            AgentAdapterSourceType::Agent => "skill".to_owned(),
            AgentAdapterSourceType::AgentStep => "agent-step".to_owned(),
        };
    }
    request.skill_name.clone()
}

fn success_output(
    resolution: AgentResolution,
    started: Instant,
    metadata: JsonObject,
) -> Result<SkillOutput, RuntimeError> {
    Ok(SkillOutput {
        status: InvocationStatus::Success,
        stdout: stringify_payload(&resolution.response.payload)?,
        stderr: String::new(),
        exit_code: Some(0),
        duration_ms: duration_ms(started),
        metadata,
    })
}

fn failure_output(message: &str, started: Instant, metadata: JsonObject) -> SkillOutput {
    SkillOutput {
        status: InvocationStatus::Failure,
        stdout: String::new(),
        stderr: message.to_owned(),
        exit_code: None,
        duration_ms: duration_ms(started),
        metadata,
    }
}

fn stringify_payload(payload: &JsonValue) -> Result<String, RuntimeError> {
    match payload {
        JsonValue::String(value) => Ok(value.clone()),
        value => serde_json::to_string(value)
            .map_err(|source| RuntimeError::json("serializing agent response payload", source)),
    }
}

fn native_agent_metadata(
    source_type: AgentAdapterSourceType,
    request: &SkillInvocation,
    config: &ManagedAgentConfig,
    status: &str,
    telemetry: Option<&AgentExecutionTelemetry>,
) -> JsonObject {
    let mut root = JsonObject::new();
    let mut entry = JsonObject::new();
    match source_type {
        AgentAdapterSourceType::AgentStep => {
            entry.insert(
                "source_type".to_owned(),
                JsonValue::String("agent-step".to_owned()),
            );
            if let Some(agent) = &request.source.agent {
                entry.insert("agent".to_owned(), JsonValue::String(agent.clone()));
            }
            if let Some(task) = &request.source.task {
                entry.insert("task".to_owned(), JsonValue::String(task.clone()));
            }
            insert_common_metadata(&mut entry, config, status);
            insert_telemetry(&mut entry, telemetry);
            root.insert("agent_hook".to_owned(), JsonValue::Object(entry));
        }
        AgentAdapterSourceType::Agent => {
            entry.insert(
                "skill".to_owned(),
                JsonValue::String(skill_name(request, source_type)),
            );
            insert_common_metadata(&mut entry, config, status);
            insert_telemetry(&mut entry, telemetry);
            root.insert("agent_runner".to_owned(), JsonValue::Object(entry));
        }
    }
    root
}

fn insert_common_metadata(entry: &mut JsonObject, config: &ManagedAgentConfig, status: &str) {
    entry.insert("route".to_owned(), JsonValue::String("native".to_owned()));
    entry.insert(
        "provider".to_owned(),
        JsonValue::String(provider_name(&config.provider).to_owned()),
    );
    entry.insert("model".to_owned(), JsonValue::String(config.model.clone()));
    entry.insert("status".to_owned(), JsonValue::String(status.to_owned()));
}

fn insert_telemetry(entry: &mut JsonObject, telemetry: Option<&AgentExecutionTelemetry>) {
    let Some(telemetry) = telemetry else {
        return;
    };
    if let Some(rounds) = telemetry.rounds {
        entry.insert(
            "rounds".to_owned(),
            JsonValue::Number(JsonNumber::U64(rounds)),
        );
    }
    if let Some(tool_calls) = telemetry.tool_calls {
        entry.insert(
            "tool_calls".to_owned(),
            JsonValue::Number(JsonNumber::U64(tool_calls)),
        );
    }
    if let Some(tools) = &telemetry.tools {
        entry.insert(
            "tools".to_owned(),
            JsonValue::Array(tools.iter().cloned().map(JsonValue::String).collect()),
        );
    }
    if let Some(tool_executions) = &telemetry.tool_executions {
        entry.insert(
            "tool_executions".to_owned(),
            JsonValue::Array(
                tool_executions
                    .iter()
                    .map(tool_execution_trace)
                    .collect::<Vec<_>>(),
            ),
        );
    }
}

fn tool_execution_trace(trace: &AgentToolExecutionTrace) -> JsonValue {
    let mut object = JsonObject::new();
    object.insert("tool".to_owned(), JsonValue::String(trace.tool.clone()));
    object.insert("status".to_owned(), JsonValue::String(trace.status.clone()));
    if let Some(receipt_id) = &trace.receipt_id {
        object.insert(
            "receiptId".to_owned(),
            JsonValue::String(receipt_id.clone()),
        );
    }
    if let Some(resolution_kind) = &trace.resolution_kind {
        object.insert(
            "resolutionKind".to_owned(),
            JsonValue::String(resolution_kind.clone()),
        );
    }
    JsonValue::Object(object)
}

fn provider_name(provider: &ManagedAgentProvider) -> &'static str {
    match provider {
        ManagedAgentProvider::OpenAi => "openai",
        ManagedAgentProvider::Anthropic => "anthropic",
    }
}

fn duration_ms(started: Instant) -> u64 {
    let millis = started.elapsed().as_millis();
    u64::try_from(millis).unwrap_or(u64::MAX)
}
