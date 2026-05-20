import {
  validateCredentialEnvelopeContract,
  type AgentContextProvenanceContract,
  type ArtifactEnvelopeContract,
} from "@runxhq/contracts";
import type { ToolCatalogAdapter } from "@runxhq/runtime-local/tool-catalogs";

import type {
  ActReceiptEnvelope,
  Context,
  ContextDocument,
  CredentialEnvelope,
  NestedSkillInvoker,
  QualityProfileContext,
  SkillAdapter,
} from "./adapter-types.js";
import type { ValidatedSkill } from "../parser-types.js";

export interface ExecuteSkillOptions {
  readonly skill: ValidatedSkill;
  readonly inputs: Readonly<Record<string, unknown>>;
  readonly resolvedInputs?: Readonly<Record<string, string>>;
  readonly skillDirectory: string;
  readonly adapters: readonly SkillAdapter[];
  readonly env?: NodeJS.ProcessEnv;
  readonly credential?: CredentialEnvelope;
  readonly signal?: AbortSignal;
  readonly allowedTools?: readonly string[];
  readonly runId?: string;
  readonly stepId?: string;
  readonly currentContext?: readonly ArtifactEnvelopeContract[];
  readonly historicalContext?: readonly ArtifactEnvelopeContract[];
  readonly contextProvenance?: readonly AgentContextProvenanceContract[];
  readonly context?: Context;
  readonly voiceProfile?: ContextDocument;
  readonly qualityProfile?: QualityProfileContext;
  readonly nestedSkillInvoker?: NestedSkillInvoker;
  readonly toolCatalogAdapters?: readonly ToolCatalogAdapter[];
}

export async function executeSkill(options: ExecuteSkillOptions): Promise<ActReceiptEnvelope> {
  const adapter = options.adapters.find((candidate) => candidate.type === options.skill.source.type);

  if (!adapter) {
    return {
      status: "failure",
      stdout: "",
      stderr: "",
      exitCode: null,
      signal: null,
      durationMs: 0,
      errorMessage: `No adapter registered for source type '${options.skill.source.type}'.`,
    };
  }

  return await adapter.invoke({
    skillName: options.skill.name,
    skillBody: options.skill.body,
    allowedTools: options.allowedTools ?? options.skill.allowedTools,
    source: options.skill.source,
    inputs: options.inputs,
    resolvedInputs: options.resolvedInputs,
    skillDirectory: options.skillDirectory,
    env: options.env,
    credential: options.credential ? validateCredentialEnvelopeContract(options.credential, "credential") : undefined,
    signal: options.signal,
    runId: options.runId,
    stepId: options.stepId,
    currentContext: options.currentContext,
    historicalContext: options.historicalContext,
    contextProvenance: options.contextProvenance,
    context: options.context,
    voiceProfile: options.voiceProfile,
    qualityProfile: options.qualityProfile,
    nestedSkillInvoker: options.nestedSkillInvoker,
    toolCatalogAdapters: options.toolCatalogAdapters,
  });
}
