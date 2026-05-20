import type {
  ActReceiptEnvelopeContract,
  AgentContextProvenanceContract,
  ArtifactEnvelopeContract,
  ContextContract,
  ContextDocumentContract,
  CredentialEnvelopeContract,
  QualityProfileContextContract,
  ResolutionRequestContract,
} from "@runxhq/contracts";

import type { ToolCatalogAdapter } from "../tool-catalogs/index.js";
import type { ValidatedSkill } from "../parser-types.js";

export type ActReceiptEnvelope = ActReceiptEnvelopeContract;
export type AgentContextProvenance = AgentContextProvenanceContract;
export type Context = ContextContract;
export type ContextDocument = ContextDocumentContract;
export type CredentialEnvelope = CredentialEnvelopeContract;
export type QualityProfileContext = QualityProfileContextContract;
export type ResolutionRequest = ResolutionRequestContract;

export interface AdapterActInvocation {
  readonly skillName?: string;
  readonly skillBody?: string;
  readonly allowedTools?: readonly string[];
  readonly source: ValidatedSkill["source"];
  readonly inputs: Readonly<Record<string, unknown>>;
  readonly resolvedInputs?: Readonly<Record<string, string>>;
  readonly skillDirectory: string;
  readonly env?: NodeJS.ProcessEnv;
  readonly credential?: CredentialEnvelope;
  readonly signal?: AbortSignal;
  readonly runId?: string;
  readonly stepId?: string;
  readonly currentContext?: readonly ArtifactEnvelopeContract[];
  readonly historicalContext?: readonly ArtifactEnvelopeContract[];
  readonly contextProvenance?: readonly AgentContextProvenance[];
  readonly context?: Context;
  readonly voiceProfile?: ContextDocument;
  readonly qualityProfile?: QualityProfileContext;
  readonly nestedSkillInvoker?: NestedSkillInvoker;
  readonly toolCatalogAdapters?: readonly ToolCatalogAdapter[];
}

export interface NestedSkillInvocation {
  readonly skill: ValidatedSkill;
  readonly skillDirectory: string;
  readonly requestedSkillPath: string;
  readonly inputs: Readonly<Record<string, unknown>>;
  readonly receiptMetadata?: Readonly<Record<string, unknown>>;
}

export type NestedSkillInvocationResult =
  | {
      readonly status: "needs_resolution";
      readonly request: ResolutionRequest;
      readonly receiptId?: string;
    }
  | {
      readonly status: "policy_denied";
      readonly reasons: readonly string[];
      readonly receiptId?: string;
      readonly errorMessage?: string;
    }
  | {
      readonly status: "success" | "failure";
      readonly stdout: string;
      readonly stderr: string;
      readonly exitCode: number | null;
      readonly signal: NodeJS.Signals | null;
      readonly durationMs: number;
      readonly errorMessage?: string;
      readonly receiptId?: string;
    };

export type NestedSkillInvoker = (
  options: NestedSkillInvocation,
) => Promise<NestedSkillInvocationResult>;

export interface SkillAdapter {
  // Execution adapters do work for one source type. They do not own
  // approvals, receipts, or host interaction; the kernel mediates those
  // boundaries and surfaces resolve them.
  readonly type: string;
  readonly invoke: (request: AdapterActInvocation) => Promise<ActReceiptEnvelope>;
}
