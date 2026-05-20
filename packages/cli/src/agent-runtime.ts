import {
  executeManagedAgentResolution,
  formatManagedAgentLabel,
  loadManagedAgentConfig,
  type ManagedAgentConfig,
} from "@runxhq/adapters/agent";
import type {
  ResolutionRequestContract as ResolutionRequest,
  ResolutionResponseContract as ResolutionResponse,
} from "@runxhq/contracts";

type AgentActResolutionRequest = Extract<ResolutionRequest, { readonly kind: "agent_act" }>;

export interface CliAgentRuntime {
  readonly label: string;
  readonly resolve: (request: AgentActResolutionRequest) => Promise<ResolutionResponse>;
}

export async function loadCliAgentRuntime(
  env: NodeJS.ProcessEnv = process.env,
): Promise<CliAgentRuntime | undefined> {
  const config = await loadManagedAgentConfig(env);
  if (!config) {
    return undefined;
  }
  return createCliAgentRuntime(config, env);
}

function createCliAgentRuntime(
  config: ManagedAgentConfig,
  env: NodeJS.ProcessEnv,
): CliAgentRuntime {
  return {
    label: formatManagedAgentLabel(config),
    resolve: async (request) =>
      await executeManagedAgentResolution(config, request, {
        env,
      }),
  };
}
