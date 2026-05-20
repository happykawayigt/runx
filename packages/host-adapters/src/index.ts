import type {
  ResolutionRequestContract as ResolutionRequest,
  ResolutionResponseContract as ResolutionResponse,
} from "@runxhq/contracts";

export interface HostExecutionEvent {
  readonly type:
    | "skill_loaded"
    | "inputs_resolved"
    | "auth_resolved"
    | "resolution_requested"
    | "resolution_resolved"
    | "admitted"
    | "executing"
    | "step_started"
    | "step_waiting_resolution"
    | "step_completed"
    | "warning"
    | "completed";
  readonly message: string;
  readonly data?: unknown;
}

export interface HostCaller {
  readonly resolve: (request: ResolutionRequest) => Promise<ResolutionResponse | undefined>;
  readonly report: (event: HostExecutionEvent) => void | Promise<void>;
}

export interface HostAuthResolver {
  readonly resolveGrants: (request: any) => Promise<any>;
  readonly resolveCredential: (request: any) => Promise<any>;
}

export interface HostRunOptions {
  readonly skillPath: string;
  readonly inputs?: Readonly<Record<string, unknown>>;
  readonly answersPath?: string;
  readonly runner?: string;
  readonly receiptDir?: string;
  readonly runxHome?: string;
  readonly parentReceipt?: string;
  readonly contextFrom?: readonly string[];
  readonly caller?: HostCaller;
  readonly authResolver?: HostAuthResolver;
  readonly allowedSourceTypes?: readonly string[];
  readonly resumeFromRunId?: string;
}

export interface HostBoundaryContext {
  readonly request: ResolutionRequest;
  readonly events: readonly HostExecutionEvent[];
}

export type HostBoundaryReply =
  | ResolutionResponse
  | {
      readonly actor?: "agent" | "human";
      readonly payload: unknown;
    }
  | boolean
  | string
  | number
  | Readonly<Record<string, unknown>>
  | undefined;

export type HostBoundaryResolver = (
  context: HostBoundaryContext,
) => Promise<HostBoundaryReply> | HostBoundaryReply;

export interface HostPausedResult {
  readonly status: "paused";
  readonly skillName: string;
  readonly runId: string;
  readonly requests: readonly ResolutionRequest[];
  readonly stepIds?: readonly string[];
  readonly stepLabels?: readonly string[];
  readonly events: readonly HostExecutionEvent[];
}

export interface HostCompletedResult {
  readonly status: "completed";
  readonly skillName: string;
  readonly receiptId: string;
  readonly output: string;
  readonly events: readonly HostExecutionEvent[];
}

export interface HostFailedResult {
  readonly status: "failed";
  readonly skillName: string;
  readonly receiptId?: string;
  readonly error: string;
  readonly events: readonly HostExecutionEvent[];
}

export interface HostEscalatedResult {
  readonly status: "escalated";
  readonly skillName: string;
  readonly receiptId: string;
  readonly error: string;
  readonly events: readonly HostExecutionEvent[];
}

export interface HostDeniedResult {
  readonly status: "denied";
  readonly skillName: string;
  readonly reasons: readonly string[];
  readonly receiptId?: string;
  readonly events: readonly HostExecutionEvent[];
}

export type HostRunResult =
  | HostPausedResult
  | HostCompletedResult
  | HostFailedResult
  | HostEscalatedResult
  | HostDeniedResult;

export interface HostRunVerification {
  readonly status: "verified" | "unverified" | "invalid";
  readonly reason?: string;
}

export interface HostRunLineage {
  readonly kind: "rerun";
  readonly sourceRunId: string;
  readonly sourceReceiptId?: string;
}

export interface HostRunApproval {
  readonly gateId?: string;
  readonly gateType?: string;
  readonly decision?: "approved" | "denied";
  readonly reason?: string;
}

export interface HostInspectOptions {
  readonly receiptDir?: string;
  readonly runxHome?: string;
}

export interface HostPausedState {
  readonly status: "paused";
  readonly skillName: string;
  readonly runId: string;
  readonly requestedPath?: string;
  readonly resolvedPath?: string;
  readonly selectedRunner?: string;
  readonly requests: readonly ResolutionRequest[];
  readonly stepIds?: readonly string[];
  readonly stepLabels?: readonly string[];
  readonly lineage?: HostRunLineage;
}

interface HostTerminalState {
  readonly skillName: string;
  readonly runId: string;
  readonly receiptId: string;
  readonly verification: HostRunVerification;
  readonly sourceType?: string;
  readonly startedAt?: string;
  readonly completedAt?: string;
  readonly disposition?: string;
  readonly outcomeState?: string;
  readonly actors?: readonly string[];
  readonly artifactTypes?: readonly string[];
  readonly runnerProvider?: string;
  readonly approval?: HostRunApproval;
  readonly lineage?: HostRunLineage;
}

export interface HostCompletedState extends HostTerminalState {
  readonly status: "completed";
}

export interface HostFailedState extends HostTerminalState {
  readonly status: "failed";
}

export interface HostEscalatedState extends HostTerminalState {
  readonly status: "escalated";
}

export interface HostDeniedState extends HostTerminalState {
  readonly status: "denied";
}

export type HostRunState =
  | HostPausedState
  | HostCompletedState
  | HostFailedState
  | HostEscalatedState
  | HostDeniedState;

export interface HostBridge {
  readonly run: (
    options: HostRunOptions & {
      readonly resolver?: HostBoundaryResolver;
    },
  ) => Promise<HostRunResult>;
  readonly resume: (
    runId: string,
    options: Omit<HostRunOptions, "resumeFromRunId" | "skillPath"> & {
      readonly skillPath?: string;
      readonly resolver?: HostBoundaryResolver;
    },
  ) => Promise<HostRunResult>;
  readonly inspect: (
    referenceId: string,
    options?: HostInspectOptions,
  ) => Promise<HostRunState>;
}

export interface OpenAIHostResponse {
  readonly role: "tool";
  readonly content: readonly [{ readonly type: "text"; readonly text: string }];
  readonly structuredContent: {
    readonly runx: HostRunResult;
  };
}

export interface AnthropicHostResponse {
  readonly content: readonly [{ readonly type: "text"; readonly text: string }];
  readonly metadata: {
    readonly runx: HostRunResult;
  };
}

export interface VercelAiHostResponse {
  readonly messages: readonly [{ readonly role: "assistant"; readonly content: string }];
  readonly data: {
    readonly runx: HostRunResult;
  };
}

export interface LangChainHostResponse {
  readonly content: string;
  readonly additional_kwargs: {
    readonly runx: HostRunResult;
  };
}

export interface CrewAiHostResponse {
  readonly raw: string;
  readonly json_dict: {
    readonly runx: HostRunResult;
  };
}

export interface ProviderHostAdapter<TResponse> {
  readonly run: (
    options: HostRunOptions & {
      readonly resolver?: HostBoundaryResolver;
    },
  ) => Promise<TResponse>;
  readonly resume: (
    runId: string,
    options: Omit<HostRunOptions, "resumeFromRunId" | "skillPath"> & {
      readonly skillPath?: string;
      readonly resolver?: HostBoundaryResolver;
    },
  ) => Promise<TResponse>;
}

export function createOpenAiHostAdapter(bridge: HostBridge): ProviderHostAdapter<OpenAIHostResponse> {
  return {
    run: async (options) => toOpenAiResponse(await bridge.run(options)),
    resume: async (runId, options) => toOpenAiResponse(await bridge.resume(runId, options)),
  };
}

export function createAnthropicHostAdapter(bridge: HostBridge): ProviderHostAdapter<AnthropicHostResponse> {
  return {
    run: async (options) => toAnthropicResponse(await bridge.run(options)),
    resume: async (runId, options) => toAnthropicResponse(await bridge.resume(runId, options)),
  };
}

export function createVercelAiHostAdapter(bridge: HostBridge): ProviderHostAdapter<VercelAiHostResponse> {
  return {
    run: async (options) => toVercelAiResponse(await bridge.run(options)),
    resume: async (runId, options) => toVercelAiResponse(await bridge.resume(runId, options)),
  };
}

export function createLangChainHostAdapter(bridge: HostBridge): ProviderHostAdapter<LangChainHostResponse> {
  return {
    run: async (options) => toLangChainResponse(await bridge.run(options)),
    resume: async (runId, options) => toLangChainResponse(await bridge.resume(runId, options)),
  };
}

export function createCrewAiHostAdapter(bridge: HostBridge): ProviderHostAdapter<CrewAiHostResponse> {
  return {
    run: async (options) => toCrewAiResponse(await bridge.run(options)),
    resume: async (runId, options) => toCrewAiResponse(await bridge.resume(runId, options)),
  };
}

function toOpenAiResponse(result: HostRunResult): OpenAIHostResponse {
  return {
    role: "tool",
    content: [{ type: "text", text: summarizeHostResult(result) }],
    structuredContent: { runx: result },
  };
}

function toAnthropicResponse(result: HostRunResult): AnthropicHostResponse {
  return {
    content: [{ type: "text", text: summarizeHostResult(result) }],
    metadata: { runx: result },
  };
}

function toVercelAiResponse(result: HostRunResult): VercelAiHostResponse {
  return {
    messages: [{ role: "assistant", content: summarizeHostResult(result) }],
    data: { runx: result },
  };
}

function toLangChainResponse(result: HostRunResult): LangChainHostResponse {
  return {
    content: summarizeHostResult(result),
    additional_kwargs: { runx: result },
  };
}

function toCrewAiResponse(result: HostRunResult): CrewAiHostResponse {
  return {
    raw: summarizeHostResult(result),
    json_dict: { runx: result },
  };
}

function summarizeHostResult(result: HostRunResult): string {
  switch (result.status) {
    case "completed":
      return `${result.skillName} completed. Inspect receipt ${result.receiptId}.`;
    case "paused":
      return `${result.skillName} paused at ${result.runId}. Resume after resolving ${result.requests.length} request(s).`;
    case "denied":
      return `${result.skillName} was denied by policy.`;
    case "escalated":
      return `${result.skillName} escalated. Inspect receipt ${result.receiptId}.`;
    case "failed":
      return `${result.skillName} failed. Inspect receipt ${result.receiptId ?? "n/a"}.`;
    default:
      return assertNever(result);
  }
}

function assertNever(value: never): never {
  throw new Error(`Unhandled host run result: ${JSON.stringify(value)}`);
}
