import type { AgentActResolutionRequestContract as AgentActResolutionRequest } from "@runxhq/contracts";
import { errorMessage } from "@runxhq/core/util";

import {
  FINAL_RESULT_TOOL_NAME,
  maxManagedAgentRounds,
  type ManagedAgentExecutionDetails,
  type ManagedAgentPausedExecution,
  type ManagedRuntimeTool,
  type ManagedToolExecutionTrace,
  type OpenAiResponseBody,
  type OpenAiResponseInputItem,
  type OpenAiToolCall,
  type OpenAiToolDefinition,
  type ToolCallOutputItem,
} from "./types.js";
import {
  extractApiErrorMessage,
  isRecord,
  parseJsonObject,
  parseJsonValue,
} from "./helpers.js";
import {
  outputToJsonSchema,
  validateFinalPayload,
} from "./json-schema.js";
import { executeManagedToolCall } from "./runtime-tools.js";
import { buildManagedRuntimeInstructions } from "./agent-act-invocation.js";
import type { ManagedAgentConfig } from "./index.js";

export async function resolveWithOpenAi(
  config: ManagedAgentConfig,
  request: AgentActResolutionRequest,
  runtimeTools: readonly ManagedRuntimeTool[],
  signal: AbortSignal | undefined,
  allowPauseOnNestedResolution: boolean,
): Promise<ManagedAgentExecutionDetails | ManagedAgentPausedExecution> {
  const tools = buildOpenAiTools(request, runtimeTools);
  const toolByProviderName = new Map(runtimeTools.map((tool) => [tool.providerName, tool] as const));
  const history: OpenAiResponseInputItem[] = [buildOpenAiInitialRequestMessage(request)];
  let toolCalls = 0;
  const toolExecutions: ManagedToolExecutionTrace[] = [];

  for (let round = 1; round <= maxManagedAgentRounds; round += 1) {
    const response = await createOpenAiResponse(config, {
      instructions: buildManagedRuntimeInstructions(request),
      input: history,
      tools,
      signal,
    });
    const functionCalls = collectOpenAiFunctionCalls(response);

    if (functionCalls.length === 0) {
      const assistantText = extractOpenAiAssistantText(response);
      if (!request.invocation.envelope.output) {
        if (!assistantText.trim()) {
          throw new Error(`Managed agent resolution for ${request.id} returned no assistant text.`);
        }
        return {
          response: { actor: "agent", payload: assistantText },
          rounds: round,
          toolCalls,
          tools: runtimeTools.map((tool) => tool.runxName),
          toolExecutions,
        };
      }

      if (Array.isArray(response.output) && response.output.length > 0) {
        history.push(...response.output.filter(isRecord));
      }
      history.push(buildOpenAiCorrectionMessage("Return the final payload by calling submit_result. Do not answer in prose."));
      continue;
    }

    toolCalls += functionCalls.length;
    if (Array.isArray(response.output) && response.output.length > 0) {
      history.push(...response.output.filter(isRecord));
    }

    const toolOutputs: ToolCallOutputItem[] = [];
    for (const call of functionCalls) {
      if (call.name === FINAL_RESULT_TOOL_NAME) {
        try {
          const submittedPayload = parseJsonObject(call.arguments, `${call.name}.arguments`);
          const validationError = validateFinalPayload(submittedPayload, request.invocation.envelope.output);
          if (!validationError) {
            return {
              response: { actor: "agent", payload: submittedPayload },
              rounds: round,
              toolCalls,
              tools: runtimeTools.map((tool) => tool.runxName),
              toolExecutions,
            };
          }
          toolOutputs.push({
            type: "function_call_output",
            call_id: call.call_id,
            output: JSON.stringify({ error: validationError }),
          });
        } catch (error) {
          toolOutputs.push({
            type: "function_call_output",
            call_id: call.call_id,
            output: JSON.stringify({
              error: errorMessage(error),
            }),
          });
        }
        continue;
      }

      const tool = toolByProviderName.get(call.name);
      if (!tool) {
        toolOutputs.push({
          type: "function_call_output",
          call_id: call.call_id,
          output: JSON.stringify({ error: `Unknown tool '${call.name}'.` }),
        });
        continue;
      }

      const result = await executeManagedToolCall(tool, parseJsonValue(call.arguments, `${call.name}.arguments`));
      if (result.trace) {
        toolExecutions.push(result.trace);
      }
      if (result.request && allowPauseOnNestedResolution) {
        return {
          request: result.request,
          rounds: round,
          toolCalls,
          tools: runtimeTools.map((tool) => tool.runxName),
          toolExecutions,
        };
      }
      toolOutputs.push({
        type: "function_call_output",
        call_id: call.call_id,
        output: JSON.stringify(result.value),
      });
    }

    history.push(...toolOutputs);
  }

  throw new Error(`Managed OpenAI agent resolution for ${request.id} exceeded the maximum tool-call rounds.`);
}

function buildOpenAiTools(
  request: AgentActResolutionRequest,
  runtimeTools: readonly ManagedRuntimeTool[],
): readonly OpenAiToolDefinition[] {
  const tools = runtimeTools.map((tool) => ({
    type: "function" as const,
    name: tool.providerName,
    description: tool.description,
    parameters: tool.parameters,
    strict: false,
  }));
  if (!request.invocation.envelope.output) {
    return tools;
  }
  return [
    ...tools,
    {
      type: "function",
      name: FINAL_RESULT_TOOL_NAME,
      description: "Submit the final structured payload for this runx agent_act request.",
      strict: false,
      parameters: outputToJsonSchema(request.invocation.envelope.output),
    },
  ];
}

function buildOpenAiInitialRequestMessage(request: AgentActResolutionRequest): Readonly<Record<string, unknown>> {
  return {
    role: "user",
    content: [
      {
        type: "input_text",
        text: [
          "Resolve this runx agent_act request.",
          JSON.stringify({
            request_id: request.id,
            source_type: request.invocation.source_type,
            agent: request.invocation.agent,
            task: request.invocation.task,
            envelope: request.invocation.envelope,
          }, null, 2),
        ].join("\n\n"),
      },
    ],
  };
}

function buildOpenAiCorrectionMessage(message: string): Readonly<Record<string, unknown>> {
  return {
    role: "user",
    content: [
      {
        type: "input_text",
        text: message,
      },
    ],
  };
}

async function createOpenAiResponse(
  config: ManagedAgentConfig,
  request: {
    readonly instructions: string;
    readonly input: readonly OpenAiResponseInputItem[];
    readonly tools: readonly OpenAiToolDefinition[];
    readonly signal?: AbortSignal;
  },
): Promise<OpenAiResponseBody> {
  const response = await fetch("https://api.openai.com/v1/responses", {
    method: "POST",
    headers: {
      "Authorization": `Bearer ${config.apiKey}`,
      "Content-Type": "application/json",
    },
    body: JSON.stringify({
      model: config.model,
      store: false,
      parallel_tool_calls: false,
      instructions: request.instructions,
      input: request.input,
      tools: request.tools,
    }),
    signal: request.signal,
  });

  if (!response.ok) {
    const bodyText = await response.text();
    throw new Error(`OpenAI Responses API ${response.status}: ${extractApiErrorMessage(bodyText)}`);
  }

  return await response.json() as OpenAiResponseBody;
}

function collectOpenAiFunctionCalls(response: OpenAiResponseBody): readonly OpenAiToolCall[] {
  return Array.isArray(response.output)
    ? response.output
      .filter((item): item is OpenAiToolCall =>
        isRecord(item)
        && item.type === "function_call"
        && typeof item.call_id === "string"
        && typeof item.name === "string"
        && typeof item.arguments === "string")
    : [];
}

function extractOpenAiAssistantText(response: OpenAiResponseBody): string {
  if (!Array.isArray(response.output)) {
    return "";
  }
  const parts: string[] = [];
  for (const item of response.output) {
    if (!isRecord(item) || item.type !== "message" || item.role !== "assistant" || !Array.isArray(item.content)) {
      continue;
    }
    for (const content of item.content) {
      if (!isRecord(content)) {
        continue;
      }
      if ((content.type === "output_text" || content.type === "text") && typeof content.text === "string") {
        parts.push(content.text);
        continue;
      }
      if (content.type === "refusal" && typeof content.refusal === "string") {
        throw new Error(`OpenAI agent refused the request: ${content.refusal}`);
      }
    }
  }
  return parts.join("").trim();
}
