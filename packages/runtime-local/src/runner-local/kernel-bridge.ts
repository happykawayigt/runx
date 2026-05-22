import { spawn } from "node:child_process";
import process from "node:process";

export interface RetryAdmissionRequest {
  readonly stepId: string;
  readonly retry?: {
    readonly maxAttempts?: number;
  };
  readonly mutating: boolean;
  readonly idempotencyKey?: string;
}

export type RetryAdmissionDecision =
  | {
      readonly status: "allow";
      readonly reasons: readonly string[];
    }
  | {
      readonly status: "deny";
      readonly reasons: readonly string[];
    };

export type AdmissionDecision = RetryAdmissionDecision;

export interface LocalAdmissionGrant {
  readonly grant_id: string;
  readonly provider: string;
  readonly scopes: readonly string[];
  readonly status?: "active" | "revoked";
  readonly not_before?: string;
  readonly expires_at?: string;
  readonly scope_family?: string;
  readonly authority_kind?: "read_only" | "constructive" | "destructive";
  readonly target_repo?: string;
  readonly target_locator?: string;
}

export interface LocalAdmissionSkill {
  readonly name: string;
  readonly source: {
    readonly type: string;
    readonly command?: string;
    readonly args?: readonly string[];
    readonly timeoutSeconds?: number;
    readonly sandbox?: unknown;
  };
  readonly auth?: unknown;
  readonly runtime?: unknown;
}

export interface LocalAdmissionOptions {
  readonly allowedSourceTypes?: readonly string[];
  readonly maxTimeoutSeconds?: number;
  readonly connectedGrants?: readonly LocalAdmissionGrant[];
  readonly connectedAuthCheckedAt?: string;
  readonly skipConnectedAuth?: boolean;
  readonly approvedSandboxEscalation?: boolean;
  readonly skipSandboxEscalation?: boolean;
  readonly executionPolicy?: unknown;
}

export interface GraphScopeGrant {
  readonly grant_id?: string;
  readonly scopes: readonly string[];
}

export interface GraphScopeAdmissionRequest {
  readonly stepId: string;
  readonly requestedScopes: readonly string[];
  readonly grant: GraphScopeGrant;
}

export type GraphScopeAdmissionDecision =
  | {
      readonly status: "allow";
      readonly reasons: readonly string[];
      readonly stepId: string;
      readonly requestedScopes: readonly string[];
      readonly grantedScopes: readonly string[];
      readonly grantId?: string;
    }
  | {
      readonly status: "deny";
      readonly reasons: readonly string[];
      readonly stepId: string;
      readonly requestedScopes: readonly string[];
      readonly grantedScopes: readonly string[];
      readonly grantId?: string;
    };

export interface LocalScopeAdmission {
  readonly status: "allow" | "deny";
  readonly requested_scopes: readonly string[];
  readonly granted_scopes: readonly string[];
  readonly grant_id?: string;
  readonly reasons?: readonly string[];
  readonly decision_summary: string;
}

export interface LocalScopeAdmissionOptions {
  readonly deniedBeforeGrantResolution?: boolean;
  readonly connectedAuthCheckedAt?: string;
  readonly wildcardScopesTrusted?: boolean;
}

export interface CredentialBindingRequest {
  readonly auth?: unknown;
  readonly grants?: readonly LocalAdmissionGrant[];
  readonly scopeAdmission: LocalScopeAdmission;
  readonly credential?: unknown;
}

export interface BuildAuthorityProofMetadataRequest {
  readonly runId?: string;
  readonly skillName: string;
  readonly sourceType: string;
  readonly auth?: unknown;
  readonly grants?: readonly unknown[];
  readonly scopeAdmission?: unknown;
  readonly credential?: unknown;
  readonly sandboxDeclaration?: unknown;
  readonly sandboxMetadata?: unknown;
  readonly approval?: unknown;
  readonly mutating?: boolean;
}

export type GraphStatus = "pending" | "running" | "succeeded" | "failed" | "paused" | "escalated";
export type GraphStepStatus = "pending" | "running" | "succeeded" | "failed";
export type StepStatus = "pending" | "admitted" | "running" | "succeeded" | "failed";
export type FanoutSyncStrategy = "all" | "any" | "quorum";
export type FanoutBranchFailurePolicy = "halt" | "continue";
export type FanoutGateAction = "pause" | "escalate";

export interface SingleStepState {
  readonly stepId: string;
  readonly status: StepStatus;
  readonly startedAt?: string;
  readonly completedAt?: string;
  readonly error?: string;
}

export interface StepAdmissionWitness {
  readonly stepId: string;
  readonly receiptId: string;
  readonly authority?: unknown;
}

export type SingleStepEvent =
  | { readonly type: "admit" }
  | { readonly type: "start"; readonly at: string }
  | { readonly type: "succeed"; readonly at: string; readonly admissionWitness: StepAdmissionWitness }
  | { readonly type: "fail"; readonly at: string; readonly error: string };

export interface SequentialGraphStepDefinition {
  readonly id: string;
  readonly contextFrom?: readonly string[];
  readonly retry?: {
    readonly maxAttempts: number;
  };
  readonly fanoutGroup?: string;
}

export interface SequentialGraphStepState {
  readonly stepId: string;
  readonly status: GraphStepStatus;
  readonly attempts: number;
  readonly startedAt?: string;
  readonly completedAt?: string;
  readonly receiptId?: string;
  readonly outputs?: Readonly<Record<string, unknown>>;
  readonly error?: string;
}

export interface SequentialGraphState {
  readonly graphId: string;
  readonly status: GraphStatus;
  readonly steps: readonly SequentialGraphStepState[];
}

export interface FanoutThresholdGate {
  readonly step: string;
  readonly field: string;
  readonly above: number;
  readonly action: FanoutGateAction;
}

export interface FanoutConflictGate {
  readonly field: string;
  readonly steps: readonly string[];
  readonly action: FanoutGateAction;
}

export interface FanoutGroupPolicy {
  readonly groupId: string;
  readonly strategy: FanoutSyncStrategy;
  readonly minSuccess?: number;
  readonly onBranchFailure: FanoutBranchFailurePolicy;
  readonly thresholdGates?: readonly FanoutThresholdGate[];
  readonly conflictGates?: readonly FanoutConflictGate[];
}

export interface FanoutBranchResult {
  readonly stepId: string;
  readonly status: GraphStepStatus;
  readonly outputs?: Readonly<Record<string, unknown>>;
}

export interface FanoutSyncDecision {
  readonly groupId: string;
  readonly decision: "proceed" | "halt" | "pause" | "escalate";
  readonly strategy: FanoutSyncStrategy;
  readonly ruleFired: string;
  readonly reason: string;
  readonly branchCount: number;
  readonly successCount: number;
  readonly failureCount: number;
  readonly requiredSuccesses: number;
  readonly gate?: {
    readonly type: "threshold" | "conflict";
    readonly stepId?: string;
    readonly field: string;
    readonly value?: unknown;
    readonly comparedTo?: number;
    readonly values?: Readonly<Record<string, unknown>>;
    readonly action: FanoutGateAction;
  };
}

export type SequentialGraphEvent =
  | { readonly type: "start_step"; readonly stepId: string; readonly at: string }
  | {
      readonly type: "step_succeeded";
      readonly stepId: string;
      readonly at: string;
      readonly receiptId: string;
      readonly admissionWitness: StepAdmissionWitness;
      readonly outputs?: Readonly<Record<string, unknown>>;
    }
  | { readonly type: "step_failed"; readonly stepId: string; readonly at: string; readonly error: string }
  | { readonly type: "complete" }
  | { readonly type: "pause_graph"; readonly reason: string }
  | { readonly type: "escalate_graph"; readonly reason: string }
  | { readonly type: "fail_graph"; readonly error: string };

export type SequentialGraphPlan =
  | {
      readonly type: "run_step";
      readonly stepId: string;
      readonly attempt: number;
      readonly contextFrom: readonly string[];
    }
  | {
      readonly type: "run_fanout";
      readonly groupId: string;
      readonly stepIds: readonly string[];
      readonly attempts: Readonly<Record<string, number>>;
      readonly contextFrom: Readonly<Record<string, readonly string[]>>;
    }
  | { readonly type: "complete" }
  | { readonly type: "failed"; readonly stepId: string; readonly reason: string; readonly syncDecision?: FanoutSyncDecision }
  | { readonly type: "blocked"; readonly stepId: string; readonly reason: string; readonly syncDecision?: FanoutSyncDecision }
  | { readonly type: "paused"; readonly stepId: string; readonly reason: string; readonly syncDecision: FanoutSyncDecision }
  | { readonly type: "escalated"; readonly stepId: string; readonly reason: string; readonly syncDecision: FanoutSyncDecision };

export interface KernelBridgeOptions {
  readonly env?: NodeJS.ProcessEnv;
  readonly cwd?: string;
  readonly command?: string;
  readonly argsPrefix?: readonly string[];
  readonly timeoutMs?: number;
}

interface KernelSuccessEnvelope {
  readonly status: "success";
  readonly result: {
    readonly kind: "output";
    readonly value: unknown;
  };
}

export async function createSingleStepStateViaKernel(
  stepId: string,
  bridgeOptions: KernelBridgeOptions = {},
): Promise<SingleStepState> {
  const result = await evaluateKernelDocument(
    {
      kind: "state-machine.createSingleStepState",
      stepId,
    },
    bridgeOptions,
  );
  return parseSingleStepState(result);
}

export async function transitionSingleStepViaKernel(
  state: SingleStepState,
  event: SingleStepEvent,
  bridgeOptions: KernelBridgeOptions = {},
): Promise<SingleStepState> {
  const result = await evaluateKernelDocument(
    {
      kind: "state-machine.transitionSingleStep",
      state,
      event,
    },
    bridgeOptions,
  );
  return parseSingleStepState(result);
}

export async function admitRetryPolicyViaKernel(
  request: RetryAdmissionRequest,
  options: KernelBridgeOptions = {},
): Promise<RetryAdmissionDecision> {
  if ((request.retry?.maxAttempts ?? 1) <= 1) {
    return {
      status: "allow",
      reasons: ["retry policy not requested"],
    };
  }

  const result = await evaluateKernelDocument(
    {
      kind: "policy.admitRetryPolicy",
      request,
    },
    options,
  );
  return parseRetryAdmissionDecision(result);
}

export async function admitGraphStepScopesViaKernel(
  request: GraphScopeAdmissionRequest,
  options: KernelBridgeOptions = {},
): Promise<GraphScopeAdmissionDecision> {
  if (request.requestedScopes.length === 0) {
    return {
      status: "allow",
      reasons: ["graph step requested no scopes"],
      stepId: request.stepId,
      requestedScopes: [],
      grantedScopes: uniqueStrings(request.grant.scopes),
      grantId: request.grant.grant_id,
    };
  }

  const result = await evaluateKernelDocument(
    {
      kind: "policy.admitGraphStepScopes",
      request,
    },
    options,
  );
  return parseGraphScopeAdmissionDecision(result);
}

export async function localSkillAdmissionViaKernel(
  skill: LocalAdmissionSkill,
  options: LocalAdmissionOptions = {},
  bridgeOptions: KernelBridgeOptions = {},
): Promise<AdmissionDecision> {
  const result = await evaluateKernelDocument(
    {
      kind: "policy.admitLocalSkill",
      skill,
      options,
    },
    bridgeOptions,
  );
  return parseAdmissionDecision(result, "local admission");
}

export async function localScopeAdmissionViaKernel(
  auth: unknown,
  grants: readonly LocalAdmissionGrant[] = [],
  options: LocalScopeAdmissionOptions = {},
  bridgeOptions: KernelBridgeOptions = {},
): Promise<LocalScopeAdmission> {
  const result = await evaluateKernelDocument(
    {
      kind: "policy.buildLocalScopeAdmission",
      auth,
      grants,
      options,
    },
    bridgeOptions,
  );
  return parseLocalScopeAdmission(result);
}

export async function credentialBindingViaKernel(
  request: CredentialBindingRequest,
  bridgeOptions: KernelBridgeOptions = {},
): Promise<AdmissionDecision> {
  const result = await evaluateKernelDocument(
    {
      kind: "policy.validateCredentialBinding",
      request,
    },
    bridgeOptions,
  );
  return parseAdmissionDecision(result, "credential binding");
}

export async function authorityProofMetadataViaKernel(
  options: BuildAuthorityProofMetadataRequest,
  bridgeOptions: KernelBridgeOptions = {},
): Promise<Readonly<Record<string, unknown>>> {
  const result = await evaluateKernelDocument(
    {
      kind: "policy.buildAuthorityProofMetadata",
      options,
    },
    bridgeOptions,
  );
  return parseAuthorityProofMetadata(result);
}

export async function createSequentialGraphStateViaKernel(
  graphId: string,
  steps: readonly SequentialGraphStepDefinition[],
  bridgeOptions: KernelBridgeOptions = {},
): Promise<SequentialGraphState> {
  const result = await evaluateKernelDocument(
    {
      kind: "state-machine.createSequentialGraphState",
      graphId,
      steps,
    },
    bridgeOptions,
  );
  return parseSequentialGraphState(result);
}

export async function planSequentialGraphTransitionViaKernel(
  state: SequentialGraphState,
  steps: readonly SequentialGraphStepDefinition[],
  fanoutPolicies: Readonly<Record<string, FanoutGroupPolicy>> = {},
  options: { readonly resolvedFanoutGateKeys?: ReadonlySet<string> } = {},
  bridgeOptions: KernelBridgeOptions = {},
): Promise<SequentialGraphPlan> {
  const result = await evaluateKernelDocument(
    {
      kind: "state-machine.planSequentialGraphTransition",
      state,
      steps,
      fanoutPolicies,
      resolvedFanoutGateKeys: options.resolvedFanoutGateKeys
        ? Array.from(options.resolvedFanoutGateKeys)
        : undefined,
    },
    bridgeOptions,
  );
  return parseSequentialGraphPlan(result);
}

export async function transitionSequentialGraphViaKernel(
  state: SequentialGraphState,
  event: SequentialGraphEvent,
  bridgeOptions: KernelBridgeOptions = {},
): Promise<SequentialGraphState> {
  const result = await evaluateKernelDocument(
    {
      kind: "state-machine.transitionSequentialGraph",
      state,
      event,
    },
    bridgeOptions,
  );
  return parseSequentialGraphState(result);
}

export async function evaluateFanoutSyncViaKernel(
  policy: FanoutGroupPolicy,
  results: readonly FanoutBranchResult[],
  options: { readonly resolvedGateKeys?: ReadonlySet<string> } = {},
  bridgeOptions: KernelBridgeOptions = {},
): Promise<FanoutSyncDecision> {
  const result = await evaluateKernelDocument(
    {
      kind: "state-machine.evaluateFanoutSync",
      policy,
      results,
      resolvedGateKeys: options.resolvedGateKeys ? Array.from(options.resolvedGateKeys) : undefined,
    },
    bridgeOptions,
  );
  return parseFanoutSyncDecision(result);
}

export async function fanoutSyncDecisionKeyViaKernel(
  decision: Pick<FanoutSyncDecision, "groupId" | "ruleFired">,
  bridgeOptions: KernelBridgeOptions = {},
): Promise<string> {
  const result = await evaluateKernelDocument(
    {
      kind: "state-machine.fanoutSyncDecisionKey",
      decision,
    },
    bridgeOptions,
  );
  if (typeof result !== "string" || result.length === 0) {
    throw new Error("Rust kernel eval returned an invalid fanout sync decision key.");
  }
  return result;
}

export async function evaluateKernelDocument(
  input: unknown,
  options: KernelBridgeOptions = {},
): Promise<unknown> {
  const envelope = await runKernelEval(input, options);
  return envelope.result.value;
}

async function runKernelEval(
  input: unknown,
  options: KernelBridgeOptions,
): Promise<KernelSuccessEnvelope> {
  const command = resolveKernelCommand(options);
  const args = [...(options.argsPrefix ?? []), "kernel", "eval", "--input", "-", "--json"];
  const env = {
    ...process.env,
    ...(options.env ?? {}),
    NO_COLOR: "1",
    RUNX_RUST_CLI: "1",
  };

  const result = await spawnKernelProcess({
    command,
    args,
    cwd: options.cwd ?? process.cwd(),
    env,
    stdin: JSON.stringify(input),
    timeoutMs: options.timeoutMs ?? 10_000,
  });

  if (result.status !== 0) {
    throw new Error(
      `Rust kernel eval failed with exit ${result.status}: ${firstNonEmpty(result.stderr, result.stdout, "no output")}`,
    );
  }

  let parsed: unknown;
  try {
    parsed = JSON.parse(result.stdout);
  } catch (error) {
    throw new Error(`Rust kernel eval returned invalid JSON: ${errorMessage(error)}`);
  }

  if (!isKernelSuccessEnvelope(parsed)) {
    throw new Error("Rust kernel eval returned an invalid success envelope.");
  }
  return parsed;
}

interface SpawnKernelProcessOptions {
  readonly command: string;
  readonly args: readonly string[];
  readonly cwd: string;
  readonly env: NodeJS.ProcessEnv;
  readonly stdin: string;
  readonly timeoutMs: number;
}

interface SpawnKernelProcessResult {
  readonly status: number | null;
  readonly stdout: string;
  readonly stderr: string;
}

function spawnKernelProcess(options: SpawnKernelProcessOptions): Promise<SpawnKernelProcessResult> {
  return new Promise((resolve, reject) => {
    const child = spawn(options.command, options.args, {
      cwd: options.cwd,
      env: options.env,
      stdio: ["pipe", "pipe", "pipe"],
    });
    let settled = false;
    let timedOut = false;
    let stdout = "";
    let stderr = "";
    let killTimer: NodeJS.Timeout | undefined;

    const timer = setTimeout(() => {
      if (settled) {
        return;
      }
      timedOut = true;
      child.kill("SIGTERM");
      killTimer = setTimeout(() => {
        child.kill("SIGKILL");
        if (settled) {
          return;
        }
        settled = true;
        reject(new Error(`Rust kernel eval timed out after ${options.timeoutMs}ms.`));
      }, 1_000);
    }, options.timeoutMs);

    const clearTimers = () => {
      clearTimeout(timer);
      if (killTimer) {
        clearTimeout(killTimer);
      }
    };

    child.stdout.setEncoding("utf8");
    child.stderr.setEncoding("utf8");
    child.stdout.on("data", (chunk: string) => {
      stdout += chunk;
    });
    child.stderr.on("data", (chunk: string) => {
      stderr += chunk;
    });
    child.on("error", (error) => {
      if (settled) {
        return;
      }
      settled = true;
      clearTimers();
      reject(new Error(`Failed to spawn Rust kernel eval command '${options.command}': ${error.message}`));
    });
    child.on("close", (status) => {
      if (settled) {
        return;
      }
      settled = true;
      clearTimers();
      if (timedOut) {
        reject(new Error(`Rust kernel eval timed out after ${options.timeoutMs}ms.`));
        return;
      }
      resolve({ status, stdout, stderr });
    });
    child.stdin.on("error", () => {
      // The child may exit before consuming stdin. The close handler reports
      // the kernel process status with captured stdout/stderr.
    });
    child.stdin.end(options.stdin);
  });
}

function resolveKernelCommand(options: KernelBridgeOptions): string {
  const command = options.command ?? options.env?.RUNX_KERNEL_EVAL_BIN ?? process.env.RUNX_KERNEL_EVAL_BIN;
  if (!command) {
    throw new Error("Rust kernel eval requires RUNX_KERNEL_EVAL_BIN or an explicit command.");
  }
  return command;
}

function parseAdmissionDecision(value: unknown, label: string): AdmissionDecision {
  if (!isRecord(value)) {
    throw new Error(`Rust kernel eval returned a non-object ${label} decision.`);
  }
  const status = value.status;
  if (status !== "allow" && status !== "deny") {
    throw new Error(`Rust kernel eval returned an invalid ${label} status.`);
  }
  if (!Array.isArray(value.reasons) || !value.reasons.every((reason) => typeof reason === "string")) {
    throw new Error(`Rust kernel eval returned invalid ${label} reasons.`);
  }
  return {
    status,
    reasons: value.reasons,
  };
}

function parseRetryAdmissionDecision(value: unknown): RetryAdmissionDecision {
  if (!isRecord(value)) {
    throw new Error("Rust kernel eval returned a non-object retry admission decision.");
  }
  const status = value.status;
  if (status !== "allow" && status !== "deny") {
    throw new Error("Rust kernel eval returned an invalid retry admission status.");
  }
  if (!Array.isArray(value.reasons) || !value.reasons.every((reason) => typeof reason === "string")) {
    throw new Error("Rust kernel eval returned invalid retry admission reasons.");
  }
  return {
    status,
    reasons: value.reasons,
  };
}

function parseLocalScopeAdmission(value: unknown): LocalScopeAdmission {
  if (!isRecord(value)) {
    throw new Error("Rust kernel eval returned a non-object local scope admission.");
  }
  const status = value.status;
  if (status !== "allow" && status !== "deny") {
    throw new Error("Rust kernel eval returned an invalid local scope admission status.");
  }
  if (!Array.isArray(value.requested_scopes) || !value.requested_scopes.every((scope) => typeof scope === "string")) {
    throw new Error("Rust kernel eval returned invalid local scope admission requested_scopes.");
  }
  if (!Array.isArray(value.granted_scopes) || !value.granted_scopes.every((scope) => typeof scope === "string")) {
    throw new Error("Rust kernel eval returned invalid local scope admission granted_scopes.");
  }
  if (value.grant_id !== undefined && typeof value.grant_id !== "string") {
    throw new Error("Rust kernel eval returned invalid local scope admission grant_id.");
  }
  if (
    value.reasons !== undefined
    && (!Array.isArray(value.reasons) || !value.reasons.every((reason) => typeof reason === "string"))
  ) {
    throw new Error("Rust kernel eval returned invalid local scope admission reasons.");
  }
  if (typeof value.decision_summary !== "string") {
    throw new Error("Rust kernel eval returned invalid local scope admission decision_summary.");
  }
  return {
    status,
    requested_scopes: value.requested_scopes,
    granted_scopes: value.granted_scopes,
    grant_id: value.grant_id,
    reasons: value.reasons,
    decision_summary: value.decision_summary,
  };
}

function parseGraphScopeAdmissionDecision(value: unknown): GraphScopeAdmissionDecision {
  if (!isRecord(value)) {
    throw new Error("Rust kernel eval returned a non-object graph scope admission decision.");
  }
  const status = value.status;
  if (status !== "allow" && status !== "deny") {
    throw new Error("Rust kernel eval returned an invalid graph scope admission status.");
  }
  if (!Array.isArray(value.reasons) || !value.reasons.every((reason) => typeof reason === "string")) {
    throw new Error("Rust kernel eval returned invalid graph scope admission reasons.");
  }
  if (typeof value.stepId !== "string") {
    throw new Error("Rust kernel eval returned invalid graph scope admission stepId.");
  }
  if (!Array.isArray(value.requestedScopes) || !value.requestedScopes.every((scope) => typeof scope === "string")) {
    throw new Error("Rust kernel eval returned invalid graph scope requestedScopes.");
  }
  if (!Array.isArray(value.grantedScopes) || !value.grantedScopes.every((scope) => typeof scope === "string")) {
    throw new Error("Rust kernel eval returned invalid graph scope grantedScopes.");
  }
  if (value.grantId !== undefined && typeof value.grantId !== "string") {
    throw new Error("Rust kernel eval returned invalid graph scope grantId.");
  }
  return {
    status,
    reasons: value.reasons,
    stepId: value.stepId,
    requestedScopes: value.requestedScopes,
    grantedScopes: value.grantedScopes,
    grantId: value.grantId,
  };
}

function parseAuthorityProofMetadata(value: unknown): Readonly<Record<string, unknown>> {
  if (!isRecord(value)) {
    throw new Error("Rust kernel eval returned non-object authority proof metadata.");
  }
  if (!isRecord(value.authority_proof)) {
    throw new Error("Rust kernel eval returned authority proof metadata without authority_proof.");
  }
  return value;
}

function parseSingleStepState(value: unknown): SingleStepState {
  if (!isRecord(value)) {
    throw new Error("Rust kernel eval returned a non-object single step state.");
  }
  if (typeof value.stepId !== "string") {
    throw new Error("Rust kernel eval returned invalid single step state stepId.");
  }
  if (!isStepStatus(value.status)) {
    throw new Error("Rust kernel eval returned invalid single step state status.");
  }
  if (value.startedAt !== undefined && typeof value.startedAt !== "string") {
    throw new Error("Rust kernel eval returned invalid single step state startedAt.");
  }
  if (value.completedAt !== undefined && typeof value.completedAt !== "string") {
    throw new Error("Rust kernel eval returned invalid single step state completedAt.");
  }
  if (value.error !== undefined && typeof value.error !== "string") {
    throw new Error("Rust kernel eval returned invalid single step state error.");
  }
  return {
    stepId: value.stepId,
    status: value.status,
    startedAt: value.startedAt,
    completedAt: value.completedAt,
    error: value.error,
  };
}

function parseSequentialGraphState(value: unknown): SequentialGraphState {
  if (!isRecord(value)) {
    throw new Error("Rust kernel eval returned a non-object sequential graph state.");
  }
  if (typeof value.graphId !== "string") {
    throw new Error("Rust kernel eval returned invalid sequential graph state graphId.");
  }
  if (!isGraphStatus(value.status)) {
    throw new Error("Rust kernel eval returned invalid sequential graph state status.");
  }
  if (!Array.isArray(value.steps)) {
    throw new Error("Rust kernel eval returned invalid sequential graph state steps.");
  }
  return {
    graphId: value.graphId,
    status: value.status,
    steps: value.steps.map(parseSequentialGraphStepState),
  };
}

function parseSequentialGraphStepState(value: unknown): SequentialGraphStepState {
  if (!isRecord(value)) {
    throw new Error("Rust kernel eval returned a non-object sequential graph step state.");
  }
  if (typeof value.stepId !== "string") {
    throw new Error("Rust kernel eval returned invalid sequential graph step state stepId.");
  }
  if (!isGraphStepStatus(value.status)) {
    throw new Error("Rust kernel eval returned invalid sequential graph step state status.");
  }
  const attempts = value.attempts;
  if (typeof attempts !== "number" || !Number.isInteger(attempts) || attempts < 0) {
    throw new Error("Rust kernel eval returned invalid sequential graph step state attempts.");
  }
  if (value.startedAt !== undefined && typeof value.startedAt !== "string") {
    throw new Error("Rust kernel eval returned invalid sequential graph step state startedAt.");
  }
  if (value.completedAt !== undefined && typeof value.completedAt !== "string") {
    throw new Error("Rust kernel eval returned invalid sequential graph step state completedAt.");
  }
  if (value.receiptId !== undefined && typeof value.receiptId !== "string") {
    throw new Error("Rust kernel eval returned invalid sequential graph step state receiptId.");
  }
  if (value.outputs !== undefined && !isRecord(value.outputs)) {
    throw new Error("Rust kernel eval returned invalid sequential graph step state outputs.");
  }
  if (value.error !== undefined && typeof value.error !== "string") {
    throw new Error("Rust kernel eval returned invalid sequential graph step state error.");
  }
  return {
    stepId: value.stepId,
    status: value.status,
    attempts,
    startedAt: value.startedAt,
    completedAt: value.completedAt,
    receiptId: value.receiptId,
    outputs: value.outputs,
    error: value.error,
  };
}

function parseSequentialGraphPlan(value: unknown): SequentialGraphPlan {
  if (!isRecord(value)) {
    throw new Error("Rust kernel eval returned a non-object sequential graph plan.");
  }
  const type = value.type;
  switch (type) {
    case "run_step": {
      if (typeof value.stepId !== "string") {
        throw new Error("Rust kernel eval returned invalid run_step plan stepId.");
      }
      if (!isNonNegativeInteger(value.attempt) || value.attempt < 1) {
        throw new Error("Rust kernel eval returned invalid run_step plan attempt.");
      }
      if (!isStringArray(value.contextFrom)) {
        throw new Error("Rust kernel eval returned invalid run_step plan contextFrom.");
      }
      return {
        type,
        stepId: value.stepId,
        attempt: value.attempt,
        contextFrom: value.contextFrom,
      };
    }
    case "run_fanout": {
      if (typeof value.groupId !== "string") {
        throw new Error("Rust kernel eval returned invalid run_fanout plan groupId.");
      }
      if (!isStringArray(value.stepIds)) {
        throw new Error("Rust kernel eval returned invalid run_fanout plan stepIds.");
      }
      if (!isNumberRecord(value.attempts)) {
        throw new Error("Rust kernel eval returned invalid run_fanout plan attempts.");
      }
      if (!isStringArrayRecord(value.contextFrom)) {
        throw new Error("Rust kernel eval returned invalid run_fanout plan contextFrom.");
      }
      return {
        type,
        groupId: value.groupId,
        stepIds: value.stepIds,
        attempts: value.attempts,
        contextFrom: value.contextFrom,
      };
    }
    case "complete":
      return { type };
    case "failed":
    case "blocked": {
      if (typeof value.stepId !== "string") {
        throw new Error(`Rust kernel eval returned invalid ${type} plan stepId.`);
      }
      if (typeof value.reason !== "string") {
        throw new Error(`Rust kernel eval returned invalid ${type} plan reason.`);
      }
      return {
        type,
        stepId: value.stepId,
        reason: value.reason,
        syncDecision: value.syncDecision === undefined ? undefined : parseFanoutSyncDecision(value.syncDecision),
      };
    }
    case "paused":
    case "escalated": {
      if (typeof value.stepId !== "string") {
        throw new Error(`Rust kernel eval returned invalid ${type} plan stepId.`);
      }
      if (typeof value.reason !== "string") {
        throw new Error(`Rust kernel eval returned invalid ${type} plan reason.`);
      }
      return {
        type,
        stepId: value.stepId,
        reason: value.reason,
        syncDecision: parseFanoutSyncDecision(value.syncDecision),
      };
    }
    default:
      throw new Error("Rust kernel eval returned an invalid sequential graph plan type.");
  }
}

function parseFanoutSyncDecision(value: unknown): FanoutSyncDecision {
  if (!isRecord(value)) {
    throw new Error("Rust kernel eval returned a non-object fanout sync decision.");
  }
  if (typeof value.groupId !== "string") {
    throw new Error("Rust kernel eval returned invalid fanout sync decision groupId.");
  }
  if (!isFanoutSyncOutcome(value.decision)) {
    throw new Error("Rust kernel eval returned invalid fanout sync decision outcome.");
  }
  if (!isFanoutSyncStrategy(value.strategy)) {
    throw new Error("Rust kernel eval returned invalid fanout sync decision strategy.");
  }
  if (typeof value.ruleFired !== "string") {
    throw new Error("Rust kernel eval returned invalid fanout sync decision ruleFired.");
  }
  if (typeof value.reason !== "string") {
    throw new Error("Rust kernel eval returned invalid fanout sync decision reason.");
  }
  if (
    !isNonNegativeInteger(value.branchCount)
    || !isNonNegativeInteger(value.successCount)
    || !isNonNegativeInteger(value.failureCount)
    || !isNonNegativeInteger(value.requiredSuccesses)
  ) {
    throw new Error("Rust kernel eval returned invalid fanout sync decision counts.");
  }
  return {
    groupId: value.groupId,
    decision: value.decision,
    strategy: value.strategy,
    ruleFired: value.ruleFired,
    reason: value.reason,
    branchCount: value.branchCount,
    successCount: value.successCount,
    failureCount: value.failureCount,
    requiredSuccesses: value.requiredSuccesses,
    gate: value.gate === undefined ? undefined : parseFanoutGate(value.gate),
  };
}

function parseFanoutGate(value: unknown): FanoutSyncDecision["gate"] {
  if (!isRecord(value)) {
    throw new Error("Rust kernel eval returned invalid fanout gate.");
  }
  const type = value.type;
  if (type !== "threshold" && type !== "conflict") {
    throw new Error("Rust kernel eval returned invalid fanout gate type.");
  }
  if (typeof value.field !== "string") {
    throw new Error("Rust kernel eval returned invalid fanout gate field.");
  }
  if (!isFanoutGateAction(value.action)) {
    throw new Error("Rust kernel eval returned invalid fanout gate action.");
  }
  if (value.stepId !== undefined && typeof value.stepId !== "string") {
    throw new Error("Rust kernel eval returned invalid fanout gate stepId.");
  }
  if (value.comparedTo !== undefined && typeof value.comparedTo !== "number") {
    throw new Error("Rust kernel eval returned invalid fanout gate comparedTo.");
  }
  if (value.values !== undefined && !isRecord(value.values)) {
    throw new Error("Rust kernel eval returned invalid fanout gate values.");
  }
  return {
    type,
    field: value.field,
    action: value.action,
    stepId: value.stepId,
    value: value.value,
    comparedTo: value.comparedTo,
    values: value.values,
  };
}

function isStepStatus(value: unknown): value is StepStatus {
  return value === "pending"
    || value === "admitted"
    || value === "running"
    || value === "succeeded"
    || value === "failed";
}

function isGraphStatus(value: unknown): value is GraphStatus {
  return value === "pending"
    || value === "running"
    || value === "succeeded"
    || value === "failed"
    || value === "paused"
    || value === "escalated";
}

function isGraphStepStatus(value: unknown): value is GraphStepStatus {
  return value === "pending" || value === "running" || value === "succeeded" || value === "failed";
}

function isFanoutSyncStrategy(value: unknown): value is FanoutSyncStrategy {
  return value === "all" || value === "any" || value === "quorum";
}

function isFanoutSyncOutcome(value: unknown): value is FanoutSyncDecision["decision"] {
  return value === "proceed" || value === "halt" || value === "pause" || value === "escalate";
}

function isFanoutGateAction(value: unknown): value is FanoutGateAction {
  return value === "pause" || value === "escalate";
}

function isKernelSuccessEnvelope(value: unknown): value is KernelSuccessEnvelope {
  if (!isRecord(value) || value.status !== "success" || !isRecord(value.result)) {
    return false;
  }
  return value.result.kind === "output" && "value" in value.result;
}

function isRecord(value: unknown): value is Readonly<Record<string, unknown>> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isStringArray(value: unknown): value is readonly string[] {
  return Array.isArray(value) && value.every((entry) => typeof entry === "string");
}

function isNonNegativeInteger(value: unknown): value is number {
  return typeof value === "number" && Number.isInteger(value) && value >= 0;
}

function isNumberRecord(value: unknown): value is Readonly<Record<string, number>> {
  return isRecord(value)
    && Object.values(value).every((entry) => typeof entry === "number" && Number.isInteger(entry) && entry >= 0);
}

function isStringArrayRecord(value: unknown): value is Readonly<Record<string, readonly string[]>> {
  return isRecord(value) && Object.values(value).every(isStringArray);
}

function firstNonEmpty(...values: readonly string[]): string {
  return values.find((value) => value.trim().length > 0)?.trim() ?? "";
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function uniqueStrings(values: readonly string[]): readonly string[] {
  const seen = new Set<string>();
  const unique: string[] = [];
  for (const value of values) {
    if (seen.has(value)) {
      continue;
    }
    seen.add(value);
    unique.push(value);
  }
  return unique;
}
