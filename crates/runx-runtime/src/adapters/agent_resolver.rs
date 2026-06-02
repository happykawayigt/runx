//! Production [`AgentResolver`]: the optional in-kernel managed-agent loop.
//!
//! Runs the agent loop in-process against a provider, tying together the
//! [`AnthropicModelCaller`], the [`RuntimeToolExecutor`], and [`run_agent_loop`].
//! This is the OPTIONAL governance path. The default shipped agent behavior stays
//! host-drives (the `needs_agent` yield in skill execution); this resolver is used
//! only when a provider key is configured (the opt-in branch in the agent path).

use std::collections::BTreeMap;
use std::path::PathBuf;

use runx_contracts::{JsonObject, JsonValue, ResolutionRequest};

use super::agent::{AgentResolution, AgentResolver, AgentResolverError};
use super::agent_anthropic::{AgentToolDefinition, AnthropicModelCaller};
use super::agent_loop::{AgentLoopConfig, run_agent_loop};
use super::agent_tools::RuntimeToolExecutor;
use crate::credentials::CredentialDelivery;
use crate::runtime_http::RuntimeHttpTransport;

const FINAL_RESULT_TOOL: &str = "runx_final_result";
const MAX_ROUNDS: u32 = 16;

/// Resolves a managed agent act by running the in-process tool-use loop against
/// the Anthropic provider, carrying the run context for governed tool execution.
pub struct AnthropicAgentResolver<T> {
    transport: T,
    api_key: String,
    model: String,
    env: BTreeMap<String, String>,
    skill_directory: PathBuf,
    credential_delivery: CredentialDelivery,
}

impl<T> AnthropicAgentResolver<T> {
    #[must_use]
    pub fn new(
        transport: T,
        api_key: String,
        model: String,
        env: BTreeMap<String, String>,
        skill_directory: PathBuf,
        credential_delivery: CredentialDelivery,
    ) -> Self {
        Self {
            transport,
            api_key,
            model,
            env,
            skill_directory,
            credential_delivery,
        }
    }
}

fn object_schema() -> JsonValue {
    let mut schema = JsonObject::new();
    schema.insert("type".to_owned(), JsonValue::String("object".to_owned()));
    JsonValue::Object(schema)
}

/// The skill's allowed tools plus the final-result tool the model calls to finish.
/// Input schemas are permissive for now; resolving each tool's manifest schema is
/// a refinement, not required for the loop to run governed.
fn tool_definitions<'a>(tool_names: impl Iterator<Item = &'a str>) -> Vec<AgentToolDefinition> {
    let mut tools: Vec<AgentToolDefinition> = tool_names
        .map(|name| AgentToolDefinition {
            name: name.to_owned(),
            description: format!("runx tool {name}"),
            input_schema: object_schema(),
        })
        .collect();
    tools.push(AgentToolDefinition {
        name: FINAL_RESULT_TOOL.to_owned(),
        description: "Submit the final structured payload for this runx agent act.".to_owned(),
        input_schema: object_schema(),
    });
    tools
}

fn build_prompt(instructions: &str, inputs: &JsonObject) -> String {
    let inputs = serde_json::to_string(inputs).unwrap_or_default();
    format!(
        "{instructions}\n\nInputs (JSON): {inputs}\n\nWhen the task is complete, call \
         {FINAL_RESULT_TOOL} exactly once with the final payload."
    )
}

impl<T: RuntimeHttpTransport + Clone> AgentResolver for AnthropicAgentResolver<T> {
    fn resolve(&self, request: ResolutionRequest) -> Result<AgentResolution, AgentResolverError> {
        let ResolutionRequest::AgentAct { invocation, .. } = request else {
            return Err(AgentResolverError::sanitized(
                "managed agent resolver handles agent acts only",
            ));
        };
        let envelope = invocation.envelope;
        let tools = tool_definitions(envelope.allowed_tools.iter().map(|name| name.as_str()));
        let prompt = build_prompt(envelope.instructions.as_str(), &envelope.inputs);

        let model = AnthropicModelCaller::new(
            self.transport.clone(),
            self.api_key.clone(),
            self.model.clone(),
            tools,
        );
        let executor = RuntimeToolExecutor::new(
            self.env.clone(),
            self.skill_directory.clone(),
            self.credential_delivery.clone(),
        );
        let config = AgentLoopConfig {
            max_rounds: MAX_ROUNDS,
            final_result_tool: FINAL_RESULT_TOOL.to_owned(),
        };
        run_agent_loop(&config, &model, &executor, prompt)
            .map_err(|error| AgentResolverError::sanitized(error.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_definitions_include_allowed_and_final_result() {
        let tools = tool_definitions(["pay", "read"].into_iter());
        let names: Vec<&str> = tools.iter().map(|tool| tool.name.as_str()).collect();
        assert!(
            names == ["pay", "read", FINAL_RESULT_TOOL],
            "tool defs should be the allowed tools plus the final-result tool; got: {names:?}"
        );
    }

    #[test]
    fn prompt_carries_instructions_and_final_result_directive() {
        let prompt = build_prompt("Do the thing", &JsonObject::new());
        assert!(
            prompt.contains("Do the thing") && prompt.contains(FINAL_RESULT_TOOL),
            "prompt should carry the instructions and the final-result directive; got: {prompt:?}"
        );
    }
}
