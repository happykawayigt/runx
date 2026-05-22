import {
  createHash,
  createPrivateKey,
  createPublicKey,
  generateKeyPairSync,
  randomUUID,
  sign,
  type KeyObject,
} from "node:crypto";
import { mkdir, readFile, stat, writeFile } from "node:fs/promises";
import path from "node:path";

import { resolveRunxGlobalHomeDir } from "@runxhq/core/config";
import {
  errorMessage,
  hashStable,
  hashString,
  isRecord,
  isNodeError,
  stableStringify,
} from "@runxhq/core/util";
import {
  RUNX_LOGICAL_SCHEMAS,
  validateHarnessReceiptContract,
  validateScopeAdmissionContract,
  type ActContract,
  type ClosureRecordContract,
  type HarnessAuthorityContract,
  type HarnessContract,
  type HarnessReceiptContract,
  type HarnessReceiptIssuerContract,
  type HarnessReceiptSignatureContract,
  type HarnessSealContract,
  type HarnessSealDispositionContract,
  type ReferenceContract,
  type ScopeAdmissionContract,
} from "@runxhq/contracts";

import {
  admitGraphStepScopesViaKernel,
  authorityProofMetadataViaKernel,
  type GraphScopeGrant,
  type FanoutSyncDecision,
  type KernelBridgeOptions,
} from "./kernel-bridge.js";
import { graphStepReference, graphStepRunner } from "./graph-reporting.js";
import type { GraphStepRun, MaterializedContextEdge } from "./index.js";
import type { NormalizedExecutionSemantics } from "./execution-semantics.js";
import type { ExecutionGraph, GraphStep, ValidatedSkill } from "../parser-types.js";

export interface RuntimeReceiptExecution {
  readonly status: "sealed" | "failure";
  readonly exitCode: number | null;
  readonly signal: NodeJS.Signals | null;
  readonly durationMs: number;
  readonly errorMessage?: string;
  readonly metadata?: Readonly<Record<string, unknown>>;
}

export type RunnerSkillReceipt = HarnessReceiptContract;
export type RunnerGraphReceipt = HarnessReceiptContract;
export type RunnerReceipt = HarnessReceiptContract;
type RunnerReceiptIssuer = HarnessReceiptIssuerContract;
type RunnerReceiptSignature = HarnessReceiptSignatureContract;

interface RunnerReceiptKeyPair {
  readonly privateKey: KeyObject;
  readonly publicKey: KeyObject;
  readonly kid: string;
  readonly publicKeySha256: string;
}

export interface WriteRunnerSkillReceiptOptions {
  readonly receiptId?: string;
  readonly receiptDir: string;
  readonly runxHome?: string;
  readonly skillName: string;
  readonly sourceType: string;
  readonly inputs: Readonly<Record<string, unknown>>;
  readonly stdout: string;
  readonly stderr: string;
  readonly execution: RuntimeReceiptExecution;
  readonly startedAt?: string;
  readonly completedAt?: string;
  readonly parentReceipt?: string;
  readonly contextFrom?: readonly string[];
  readonly artifactIds?: readonly string[];
  readonly disposition?: NormalizedExecutionSemantics["disposition"];
  readonly inputContext?: NormalizedExecutionSemantics["inputContext"];
  readonly outcomeState?: NormalizedExecutionSemantics["outcomeState"];
  readonly outcome?: NormalizedExecutionSemantics["outcome"];
  readonly surfaceRefs?: NormalizedExecutionSemantics["surfaceRefs"];
  readonly evidenceRefs?: NormalizedExecutionSemantics["evidenceRefs"];
  readonly verificationRefs?: readonly ReferenceContract[];
}

export interface WriteRunnerGraphReceiptOptions {
  readonly receiptDir: string;
  readonly runxHome?: string;
  readonly graphId: string;
  readonly graphName: string;
  readonly owner?: string;
  readonly status: "sealed" | "failure";
  readonly inputs: Readonly<Record<string, unknown>>;
  readonly output: string;
  readonly steps: readonly GraphReceiptStep[];
  readonly syncPoints?: readonly GraphReceiptSyncPoint[];
  readonly startedAt?: string;
  readonly completedAt?: string;
  readonly durationMs: number;
  readonly errorMessage?: string;
  readonly disposition?: NormalizedExecutionSemantics["disposition"];
  readonly inputContext?: NormalizedExecutionSemantics["inputContext"];
  readonly outcomeState?: NormalizedExecutionSemantics["outcomeState"];
  readonly outcome?: NormalizedExecutionSemantics["outcome"];
  readonly surfaceRefs?: NormalizedExecutionSemantics["surfaceRefs"];
  readonly evidenceRefs?: NormalizedExecutionSemantics["evidenceRefs"];
  readonly metadata?: Readonly<Record<string, unknown>>;
}

export async function writeRunnerSkillReceipt(
  options: WriteRunnerSkillReceiptOptions,
): Promise<RunnerSkillReceipt> {
  const keyPair = await loadOrCreateRunnerReceiptKey(options.runxHome);
  assertNonEmptyReceiptIdentity(options.skillName, "skillName", options.sourceType);
  assertNonEmptyReceiptIdentity(options.sourceType, "sourceType", options.skillName);
  const id = options.receiptId ?? uniqueRunnerReceiptId("rx");
  const completedAt = options.completedAt ?? new Date().toISOString();
  const disposition = sealDisposition(options.execution.status, options.disposition);
  const closure = closureRecord({
    disposition,
    reasonCode: options.execution.status === "sealed" ? "process_closed" : "process_failed",
    summary: skillClosureSummary(options.skillName, options.execution),
    closedAt: completedAt,
  });
  const sourceRefs = toReferences(options.evidenceRefs);
  const surfaceRefs = toReferences(options.surfaceRefs);
  const artifactRefs = artifactReferences(options.artifactIds);
  const verificationRefs = [
    ...verificationReferences(options.outcome),
    ...(options.verificationRefs ?? []),
  ];
  const act = actRecord({
    actId: `act_${safeRefSegment(options.skillName)}`,
    purpose: `Run skill ${options.skillName}`,
    legitimacy: "Local runtime admitted this skill harness.",
    summary: skillClosureSummary(options.skillName, options.execution),
    closure,
    sourceRefs,
    surfaceRefs,
    artifactRefs,
    verificationRefs,
    performedAt: completedAt,
  });
  const seal = sealRecord({
    disposition,
    reasonCode: closure.reason_code,
    summary: closure.summary,
    closedAt: closure.closed_at,
    criteria: act.criterion_bindings.map((binding) => ({
      criterion_id: binding.criterion_id,
      status: binding.status,
      act_id: act.act_id,
      verification_refs: binding.verification_refs,
      evidence_refs: binding.evidence_refs,
      summary: binding.summary,
    })),
    artifactRefs,
    stdout: options.stdout,
    stderr: options.stderr,
    inputs: options.inputs,
  });
  const receipt = signHarnessReceipt({
    id,
    createdAt: completedAt,
    issuer: runnerReceiptIssuer(keyPair),
    signatureKey: keyPair.privateKey,
    harness: harnessRecord({
      id,
      name: options.skillName,
      parentReceipt: options.parentReceipt,
      sourceType: options.sourceType,
      inputs: options.inputs,
      startedAt: options.startedAt,
      completedAt,
      acts: [act],
      childReceiptRefs: [],
      artifactRefs,
      signalRefs: sourceRefs,
      seal,
      metadata: options.execution.metadata,
    }),
    seal,
    metadata: runtimeReceiptMetadata({
      kind: "skill",
      name: options.skillName,
      sourceType: options.sourceType,
      status: options.execution.status,
      startedAt: options.startedAt,
      completedAt,
      durationMs: options.execution.durationMs,
      disposition: options.disposition,
      inputHash: hashStable(options.inputs),
      outputHash: hashString(options.stdout),
      stderrHash: options.stderr ? hashString(options.stderr) : undefined,
      execution: {
        exit_code: options.execution.exitCode,
        signal: options.execution.signal,
        error_hash: options.execution.errorMessage ? hashString(options.execution.errorMessage) : undefined,
      },
      contextFrom: options.contextFrom,
      outcomeState: options.outcomeState,
      outcome: options.outcome,
      inputContext: options.inputContext,
      surfaceRefs,
      evidenceRefs: sourceRefs,
      extra: options.execution.metadata,
    }),
  });
  validateHarnessReceiptContract(receipt);
  await persistRunnerReceipt(options.receiptDir, receipt);
  return receipt;
}

export async function writeRunnerGraphReceipt(
  options: WriteRunnerGraphReceiptOptions,
): Promise<RunnerGraphReceipt> {
  const keyPair = await loadOrCreateRunnerReceiptKey(options.runxHome);
  assertNonEmptyReceiptIdentity(options.graphName, "graphName", options.graphId);
  const normalizedSteps = options.steps.map((step, index) => ({
    ...step,
    governance: validateGraphReceiptGovernance(step.governance, `steps[${index}].governance`),
  }));
  const completedAt = options.completedAt ?? new Date().toISOString();
  const disposition = sealDisposition(options.status, options.disposition);
  const sourceRefs = toReferences(options.evidenceRefs);
  const surfaceRefs = toReferences(options.surfaceRefs);
  const childReceiptRefs = normalizedSteps
    .map((step) => step.receipt_id)
    .filter((receiptId): receiptId is string => typeof receiptId === "string" && receiptId.trim().length > 0)
    .map(harnessReceiptReference);
  const seal = sealRecord({
    disposition,
    reasonCode: options.status === "sealed" ? "graph_closed" : "graph_failed",
    summary: graphClosureSummary(options.graphName, options.status, normalizedSteps.length),
    closedAt: completedAt,
    criteria: [],
    artifactRefs: [],
    stdout: options.output,
    stderr: options.errorMessage ?? "",
    inputs: options.inputs,
  });
  const receipt = signHarnessReceipt({
    id: options.graphId,
    createdAt: completedAt,
    issuer: runnerReceiptIssuer(keyPair),
    signatureKey: keyPair.privateKey,
    harness: harnessRecord({
      id: options.graphId,
      name: options.graphName,
      owner: options.owner,
      parentReceipt: undefined,
      sourceType: "graph",
      inputs: options.inputs,
      startedAt: options.startedAt,
      completedAt,
      acts: [],
      childReceiptRefs,
      artifactRefs: [],
      signalRefs: sourceRefs,
      surfaceRefs,
      seal,
      metadata: options.metadata,
    }),
    seal,
    syncPoints: options.syncPoints,
    metadata: runtimeReceiptMetadata({
      kind: "graph",
      name: options.graphName,
      owner: options.owner,
      sourceType: "graph",
      status: options.status,
      startedAt: options.startedAt,
      completedAt,
      durationMs: options.durationMs,
      disposition: options.disposition,
      inputHash: hashStable(options.inputs),
      outputHash: hashString(options.output),
      errorHash: options.errorMessage ? hashString(options.errorMessage) : undefined,
      outcomeState: options.outcomeState,
      outcome: options.outcome,
      inputContext: options.inputContext,
      surfaceRefs,
      evidenceRefs: sourceRefs,
      steps: normalizedSteps,
      extra: options.metadata,
    }),
  });
  validateHarnessReceiptContract(receipt);
  await persistRunnerReceipt(options.receiptDir, receipt);
  return receipt;
}

export function uniqueRunnerReceiptId(prefix: "rx" | "gx"): string {
  return `hrn_rcpt_${prefix}_${randomUUID().replace(/-/g, "")}`;
}

export function runnerReceiptStatus(receipt: RunnerReceipt): "sealed" | "failure" {
  return receipt.seal.disposition === "closed" ? "sealed" : "failure";
}

export function runnerReceiptCategory(receipt: RunnerReceipt): "skill" | "graph" | "harness" {
  return runtimeMetadata(receipt).category ?? "harness";
}

export function runnerReceiptDisplayName(receipt: RunnerReceipt): string {
  return runtimeMetadata(receipt).name ?? receipt.harness.harness_ref.label ?? receipt.harness.harness_id;
}

export function runnerReceiptSource(receipt: RunnerReceipt): string | undefined {
  return runtimeMetadata(receipt).source;
}

export function runnerReceiptStartedAt(receipt: RunnerReceipt): string | undefined {
  return runtimeMetadata(receipt).started_at;
}

export function runnerReceiptCompletedAt(receipt: RunnerReceipt): string | undefined {
  return runtimeMetadata(receipt).completed_at ?? receipt.seal.closed_at;
}

export function runnerReceiptDurationMs(receipt: RunnerReceipt): number | undefined {
  return runtimeMetadata(receipt).duration_ms;
}

export function runnerReceiptDisposition(receipt: RunnerReceipt): HarnessSealDispositionContract {
  return receipt.seal.disposition;
}

export function runnerReceiptOutcomeState(receipt: RunnerReceipt): string | undefined {
  return runtimeMetadata(receipt).outcome_state;
}

export function runnerReceiptOutcome(receipt: RunnerReceipt): NormalizedExecutionSemantics["outcome"] {
  return runtimeMetadata(receipt).outcome as NormalizedExecutionSemantics["outcome"];
}

export function runnerReceiptInputContext(receipt: RunnerReceipt): NormalizedExecutionSemantics["inputContext"] {
  return runtimeMetadata(receipt).input_context as NormalizedExecutionSemantics["inputContext"];
}

export function runnerReceiptSurfaceRefs(receipt: RunnerReceipt): NormalizedExecutionSemantics["surfaceRefs"] {
  return runtimeMetadata(receipt).surface_refs as NormalizedExecutionSemantics["surfaceRefs"];
}

export function runnerReceiptEvidenceRefs(receipt: RunnerReceipt): NormalizedExecutionSemantics["evidenceRefs"] {
  return runtimeMetadata(receipt).evidence_refs as NormalizedExecutionSemantics["evidenceRefs"];
}

export function runnerReceiptGraphSteps(receipt: RunnerReceipt): readonly GraphReceiptStep[] {
  return runtimeMetadata(receipt).steps ?? [];
}

export interface GraphStepGovernance {
  readonly scopeAdmission: {
    readonly status: "allow" | "deny";
    readonly requestedScopes: readonly string[];
    readonly grantedScopes: readonly string[];
    readonly grantId?: string;
    readonly reasons?: readonly string[];
  };
}

export interface GraphReceiptGovernance {
  readonly scope_admission?: ScopeAdmissionContract;
}

export interface GraphReceiptStep {
  readonly step_id: string;
  readonly attempt: number;
  readonly skill: string;
  readonly runner?: string;
  readonly status: "sealed" | "failure";
  readonly receipt_id?: string;
  readonly parent_receipt?: string;
  readonly fanout_group?: string;
  readonly retry?: {
    readonly attempt: number;
    readonly max_attempts: number;
    readonly rule_fired: string;
    readonly idempotency_key_hash?: string;
  };
  readonly context_from: readonly {
    readonly input: string;
    readonly from_step: string;
    readonly output: string;
    readonly receipt_id?: string;
  }[];
  readonly governance?: GraphReceiptGovernance;
  readonly artifact_ids?: readonly string[];
  readonly disposition?: NormalizedExecutionSemantics["disposition"];
  readonly input_context?: NormalizedExecutionSemantics["inputContext"];
  readonly outcome_state?: NormalizedExecutionSemantics["outcomeState"];
  readonly outcome?: NormalizedExecutionSemantics["outcome"];
  readonly surface_refs?: NormalizedExecutionSemantics["surfaceRefs"];
  readonly evidence_refs?: NormalizedExecutionSemantics["evidenceRefs"];
}

export interface GraphReceiptSyncPoint {
  readonly group_id: string;
  readonly strategy: "all" | "any" | "quorum";
  readonly decision: "proceed" | "halt" | "pause" | "escalate";
  readonly rule_fired: string;
  readonly reason: string;
  readonly branch_count: number;
  readonly success_count: number;
  readonly failure_count: number;
  readonly required_successes: number;
  readonly branch_receipts: readonly string[];
  readonly gate?: Readonly<Record<string, unknown>>;
}

export async function buildGraphStepGovernance(
  step: GraphStep,
  graphGrant: GraphScopeGrant,
  options: KernelBridgeOptions = {},
): Promise<GraphStepGovernance> {
  let decision;
  try {
    decision = await admitGraphStepScopesViaKernel({
      stepId: step.id,
      requestedScopes: step.scopes,
      grant: graphGrant,
    }, options);
  } catch (error) {
    return {
      scopeAdmission: {
        status: "deny",
        requestedScopes: [...step.scopes],
        grantedScopes: [...graphGrant.scopes],
        grantId: graphGrant.grant_id,
        reasons: [`graph step scope admission failed closed: ${errorMessage(error)}`],
      },
    };
  }
  return {
    scopeAdmission: {
      status: decision.status,
      requestedScopes: decision.requestedScopes,
      grantedScopes: decision.grantedScopes,
      grantId: decision.grantId,
      reasons: decision.status === "deny" ? decision.reasons : undefined,
    },
  };
}

export function governanceReceiptMetadata(
  step: GraphStep,
  governance: GraphStepGovernance,
): Readonly<Record<string, unknown>> {
  return {
    graph_governance: {
      step_id: step.id,
      selected_runner: graphStepRunner(step) ?? "default",
      scope_admission: {
        status: governance.scopeAdmission.status,
        requested_scopes: governance.scopeAdmission.requestedScopes,
        granted_scopes: governance.scopeAdmission.grantedScopes,
        grant_id: governance.scopeAdmission.grantId,
        reasons: governance.scopeAdmission.reasons,
      },
    },
  };
}

export function graphStepScopeAdmission(governance: GraphStepGovernance): ScopeAdmissionContract {
  return {
    status: governance.scopeAdmission.status,
    requested_scopes: [...governance.scopeAdmission.requestedScopes],
    granted_scopes: [...governance.scopeAdmission.grantedScopes],
    grant_id: governance.scopeAdmission.grantId,
    reasons: governance.scopeAdmission.reasons ? [...governance.scopeAdmission.reasons] : undefined,
    decision_summary: governance.scopeAdmission.status === "allow"
      ? "graph step scope admission allowed"
      : "graph step scope admission denied",
  };
}

export function graphStepAuthorityProofMetadata(options: {
  readonly graphId: string;
  readonly step: GraphStep;
  readonly stepSkill: ValidatedSkill;
  readonly governance: GraphStepGovernance;
  readonly env?: NodeJS.ProcessEnv;
}): Promise<Readonly<Record<string, unknown>>> {
  return authorityProofMetadataViaKernel({
    runId: options.graphId,
    skillName: options.stepSkill.name,
    sourceType: options.stepSkill.source.type,
    auth: options.stepSkill.auth,
    grants: [],
    scopeAdmission: graphStepScopeAdmission(options.governance),
    sandboxDeclaration: options.stepSkill.source.sandbox,
    mutating: options.step.mutating || options.stepSkill.mutating === true,
  }, { env: options.env }).catch((error: unknown) => ({
    authority_proof_error: {
      status: "failed_closed",
      reason: errorMessage(error),
    },
  }));
}

export function buildDeniedGraphStepRun(options: {
  readonly step: GraphStep;
  readonly stepSkillPath: string;
  readonly attempt: number;
  readonly parentReceipt?: string;
  readonly fanoutGroup?: string;
  readonly governance: GraphStepGovernance;
  readonly context: readonly MaterializedContextEdge[];
  readonly stderr?: string;
}): GraphStepRun {
  return {
    stepId: options.step.id,
    skill: graphStepReference(options.step),
    skillPath: options.stepSkillPath,
    runner: graphStepRunner(options.step),
    attempt: options.attempt,
    status: "failure",
    stdout: "",
    stderr: options.stderr ?? options.governance.scopeAdmission.reasons?.join("; ") ?? "graph step scope denied",
    parentReceipt: options.parentReceipt,
    fanoutGroup: options.fanoutGroup,
    governance: options.governance,
    artifactIds: [],
    disposition: "policy_denied",
    outcomeState: "complete",
    contextFrom: options.context.map((edge) => ({
      input: edge.input,
      fromStep: edge.fromStep,
      output: edge.output,
      receiptId: edge.receiptId,
    })),
  };
}

export async function writePolicyDeniedGraphReceipt(options: {
  readonly receiptDir: string;
  readonly runxHome?: string;
  readonly graph: ExecutionGraph;
  readonly graphId: string;
  readonly startedAt: string;
  readonly startedAtMs: number;
  readonly inputs: Readonly<Record<string, unknown>>;
  readonly stepRuns: readonly GraphStepRun[];
  readonly errorMessage: string;
  readonly executionSemantics: NormalizedExecutionSemantics;
  readonly receiptMetadata?: Readonly<Record<string, unknown>>;
}): Promise<RunnerGraphReceipt> {
  return await writeRunnerGraphReceipt({
    receiptDir: options.receiptDir,
    runxHome: options.runxHome,
    graphId: options.graphId,
    graphName: options.graph.name,
    owner: options.graph.owner,
    status: "failure",
    inputs: options.inputs,
    output: "",
    steps: options.stepRuns.map(toGraphReceiptStep),
    startedAt: options.startedAt,
    completedAt: new Date().toISOString(),
    durationMs: Date.now() - options.startedAtMs,
    errorMessage: options.errorMessage,
    disposition: "policy_denied",
    inputContext: options.executionSemantics.inputContext,
    outcomeState: options.executionSemantics.outcomeState,
    outcome: options.executionSemantics.outcome,
    surfaceRefs: options.executionSemantics.surfaceRefs,
    evidenceRefs: options.executionSemantics.evidenceRefs,
    metadata: options.receiptMetadata,
  });
}

export function toGraphReceiptStep(step: GraphStepRun): GraphReceiptStep {
  return {
    step_id: step.stepId,
    attempt: step.attempt,
    skill: step.skill,
    runner: step.runner,
    status: step.status,
    receipt_id: step.receiptId,
    parent_receipt: step.parentReceipt,
    fanout_group: step.fanoutGroup,
    retry: step.retry
      ? {
        attempt: step.retry.attempt,
        max_attempts: step.retry.maxAttempts,
        rule_fired: step.retry.ruleFired,
        idempotency_key_hash: step.retry.idempotencyKeyHash,
      }
      : undefined,
    context_from: step.contextFrom.map((edge) => ({
      input: edge.input,
      from_step: edge.fromStep,
      output: edge.output,
      receipt_id: edge.receiptId,
    })),
    governance: step.governance ? toReceiptGovernance(step.governance) : undefined,
    artifact_ids: step.artifactIds && step.artifactIds.length > 0 ? step.artifactIds : undefined,
    disposition: step.disposition,
    input_context: step.inputContext,
    outcome_state: step.outcomeState,
    outcome: step.outcome,
    surface_refs: step.surfaceRefs,
    evidence_refs: step.evidenceRefs,
  };
}

function toReceiptGovernance(governance: GraphStepGovernance): GraphReceiptStep["governance"] {
  return {
    scope_admission: {
      status: governance.scopeAdmission.status,
      requested_scopes: [...governance.scopeAdmission.requestedScopes],
      granted_scopes: [...governance.scopeAdmission.grantedScopes],
      grant_id: governance.scopeAdmission.grantId,
      reasons: governance.scopeAdmission.reasons ? [...governance.scopeAdmission.reasons] : undefined,
    },
  };
}

export function toGraphReceiptSyncPoint(
  decision: FanoutSyncDecision,
  branchReceipts: readonly string[],
): GraphReceiptSyncPoint {
  return {
    group_id: decision.groupId,
    strategy: decision.strategy,
    decision: decision.decision,
    rule_fired: decision.ruleFired,
    reason: decision.reason,
    branch_count: decision.branchCount,
    success_count: decision.successCount,
    failure_count: decision.failureCount,
    required_successes: decision.requiredSuccesses,
    branch_receipts: branchReceipts,
    gate: decision.gate,
  };
}

export function latestFanoutReceiptIds(stepRuns: readonly GraphStepRun[], groupId: string): readonly string[] {
  const latest = new Map<string, string>();
  for (const stepRun of stepRuns) {
    if (stepRun.fanoutGroup === groupId && stepRun.receiptId) {
      latest.set(stepRun.stepId, stepRun.receiptId);
    }
  }
  return Array.from(latest.values());
}

async function persistRunnerReceipt(
  receiptDir: string,
  receipt: RunnerReceipt,
): Promise<void> {
  await mkdir(receiptDir, { recursive: true });
  await writeFile(path.join(receiptDir, `${receipt.id}.json`), `${JSON.stringify(receipt, null, 2)}\n`, {
    flag: "wx",
    mode: 0o600,
  });
}

async function loadOrCreateRunnerReceiptKey(runxHome?: string): Promise<RunnerReceiptKeyPair> {
  const keyDir = path.join(runxHome ?? resolveRunxGlobalHomeDir(process.env), "keys");
  const privateKeyPath = path.join(keyDir, "local-ed25519-private.pem");
  const publicKeyPath = path.join(keyDir, "local-ed25519-public.pem");

  const loaded = await tryLoadRunnerReceiptKeyPair(privateKeyPath, publicKeyPath);
  if (loaded) {
    return loaded;
  }

  await mkdir(keyDir, { recursive: true });
  const { privateKey, publicKey } = generateKeyPairSync("ed25519");
  const privatePem = privateKey.export({ format: "pem", type: "pkcs8" }).toString();
  const publicPem = publicKey.export({ format: "pem", type: "spki" }).toString();

  try {
    await writeFile(privateKeyPath, privatePem, { flag: "wx", mode: 0o600 });
  } catch (writeError: unknown) {
    if (isNodeError(writeError) && writeError.code === "EEXIST") {
      const retried = await tryLoadRunnerReceiptKeyPair(privateKeyPath, publicKeyPath);
      if (retried) {
        return retried;
      }
    }
    throw new Error(
      `runx signing key creation failed at ${privateKeyPath}: ${errorMessage(writeError)}`,
      { cause: writeError },
    );
  }

  try {
    await writeFile(publicKeyPath, publicPem, { flag: "wx", mode: 0o644 });
  } catch (writeError: unknown) {
    if (isNodeError(writeError) && writeError.code === "EEXIST") {
      await writeFile(publicKeyPath, publicPem, { mode: 0o644 });
    } else {
      throw new Error(
        `runx signing key creation failed at ${publicKeyPath}: ${errorMessage(writeError)}`,
        { cause: writeError },
      );
    }
  }

  return runnerReceiptKeyPairFromPem(privatePem, publicPem);
}

async function tryLoadRunnerReceiptKeyPair(
  privatePath: string,
  publicPath: string,
  retries = 2,
): Promise<RunnerReceiptKeyPair | null> {
  try {
    const [privatePem, publicPem] = await Promise.all([readFile(privatePath, "utf8"), readFile(publicPath, "utf8")]);

    if (process.platform !== "win32") {
      const info = await stat(privatePath);
      const mode = info.mode & 0o777;
      if (mode !== 0o600) {
        process.stderr.write(`warning: ${privatePath} has permissions ${mode.toString(8)}, expected 600\n`);
      }
    }

    return runnerReceiptKeyPairFromPem(privatePem, publicPem);
  } catch (error: unknown) {
    if (isNodeError(error) && error.code === "ENOENT") {
      if (retries > 0) {
        await new Promise((resolve) => setTimeout(resolve, 10));
        return tryLoadRunnerReceiptKeyPair(privatePath, publicPath, retries - 1);
      }
      return null;
    }
    throw new Error(
      `runx signing key unreadable at ${privatePath}: ${errorMessage(error)}`,
      { cause: error },
    );
  }
}

function runnerReceiptKeyPairFromPem(privatePem: string, publicPem: string): RunnerReceiptKeyPair {
  const privateKey = createPrivateKey(privatePem);
  const publicKey = createPublicKey(publicPem);
  const publicDer = publicKey.export({ format: "der", type: "spki" });
  const publicKeySha256 = createHash("sha256").update(publicDer).digest("hex");

  return {
    privateKey,
    publicKey,
    kid: `local_${publicKeySha256.slice(0, 16)}`,
    publicKeySha256,
  };
}

function runnerReceiptIssuer(keyPair: Pick<RunnerReceiptKeyPair, "kid" | "publicKeySha256">): RunnerReceiptIssuer {
  return {
    type: "local",
    kid: keyPair.kid,
    public_key_sha256: keyPair.publicKeySha256,
  };
}

function signPayloadString(payload: string, privateKey: KeyObject): RunnerReceiptSignature {
  return {
    alg: "Ed25519",
    value: Buffer.from(sign(null, Buffer.from(payload), privateKey)).toString("base64url"),
  };
}

type RuntimeReceiptMetadataKind = "skill" | "graph";

interface RuntimeReceiptMetadata {
  readonly category?: RuntimeReceiptMetadataKind;
  readonly name?: string;
  readonly owner?: string;
  readonly source?: string;
  readonly status?: "sealed" | "failure";
  readonly started_at?: string;
  readonly completed_at?: string;
  readonly duration_ms?: number;
  readonly disposition?: string;
  readonly input_hash?: string;
  readonly output_hash?: string;
  readonly stderr_hash?: string;
  readonly error_hash?: string;
  readonly execution?: Readonly<Record<string, unknown>>;
  readonly context_from?: readonly string[];
  readonly outcome_state?: string;
  readonly outcome?: unknown;
  readonly input_context?: unknown;
  readonly surface_refs?: readonly ReferenceContract[];
  readonly evidence_refs?: readonly ReferenceContract[];
  readonly steps?: readonly GraphReceiptStep[];
}

function signHarnessReceipt(options: {
  readonly id: string;
  readonly createdAt: string;
  readonly issuer: RunnerReceiptIssuer;
  readonly signatureKey: KeyObject;
  readonly harness: HarnessContract;
  readonly seal: HarnessSealContract;
  readonly syncPoints?: readonly GraphReceiptSyncPoint[];
  readonly metadata?: Readonly<Record<string, unknown>>;
}): RunnerReceipt {
  const unsigned = {
    schema: RUNX_LOGICAL_SCHEMAS.harnessReceipt,
    id: options.id,
    created_at: options.createdAt,
    issuer: options.issuer,
    harness: options.harness,
    seal: options.seal,
    sync_points: options.syncPoints && options.syncPoints.length > 0 ? options.syncPoints : undefined,
    metadata: options.metadata,
  };
  return {
    ...unsigned,
    signature: signPayloadString(stableStringify(unsigned), options.signatureKey),
  };
}

function harnessRecord(options: {
  readonly id: string;
  readonly name: string;
  readonly owner?: string;
  readonly parentReceipt?: string;
  readonly sourceType: string;
  readonly inputs: Readonly<Record<string, unknown>>;
  readonly startedAt?: string;
  readonly completedAt: string;
  readonly acts: readonly ActContract[];
  readonly childReceiptRefs: readonly ReferenceContract[];
  readonly artifactRefs: readonly ReferenceContract[];
  readonly signalRefs: readonly ReferenceContract[];
  readonly surfaceRefs?: readonly ReferenceContract[];
  readonly seal: HarnessSealContract;
  readonly metadata?: Readonly<Record<string, unknown>>;
}): HarnessContract {
  const harnessRef = harnessReference(options.id, options.name);
  return {
    harness_id: `hrn_${harnessIdentitySegment(options.id)}`,
    parent_harness_ref: options.parentReceipt ? harnessReceiptReference(options.parentReceipt) : null,
    state: "sealed",
    host_ref: { type: "host", uri: "runx:host:local-runtime", label: "local runtime" },
    harness_ref: harnessRef,
    authority: localHarnessAuthority(),
    enforcement: {
      harness_ref: harnessRef,
      version: "runtime-local.ts",
      enforcement_profile_hash: `sha256:${hashStable({
        source: options.sourceType,
        owner: options.owner,
        metadata: options.metadata,
      })}`,
      sandbox: {
        profile: "local-process",
        cwd_policy: "workspace",
        network: "custom",
        filesystem: "custom",
      },
      redaction_refs: [],
      stdout_hash: undefined,
      stderr_hash: undefined,
    },
    idempotency: {
      intent_key: `sha256:${hashStable({ id: options.id, name: options.name, input: options.inputs })}`,
      trigger_fingerprint: `sha256:${hashStable(options.signalRefs.length > 0 ? options.signalRefs : options.inputs)}`,
      content_hash: `sha256:${hashStable({ acts: options.acts, children: options.childReceiptRefs })}`,
    },
    revision: {
      sequence: 1,
      previous_ref: null,
    },
    signal_refs: options.signalRefs,
    decisions: [{
      decision_id: `dec_${safeRefSegment(options.id)}`,
      choice: "open",
      inputs: {
        signal_refs: options.signalRefs,
        target_ref: null,
        opportunity_refs: [],
        selection_ref: null,
      },
      proposed_intent: {
        purpose: `Open local harness for ${options.name}`,
        legitimacy: "The local runtime admitted this execution boundary.",
        success_criteria: [],
        constraints: [],
        derived_from: options.signalRefs,
      },
      selected_act_id: options.acts[0]?.act_id ?? null,
      selected_harness_ref: null,
      justification: {
        summary: "Local runtime selected this harness node.",
        evidence_refs: options.signalRefs,
      },
      closure: null,
      artifact_refs: options.artifactRefs,
    }],
    acts: options.acts,
    child_harness_receipt_refs: options.childReceiptRefs,
    artifact_refs: options.artifactRefs,
    seal: options.seal,
  };
}

function actRecord(options: {
  readonly actId: string;
  readonly purpose: string;
  readonly legitimacy: string;
  readonly summary: string;
  readonly closure: ClosureRecordContract;
  readonly sourceRefs: readonly ReferenceContract[];
  readonly surfaceRefs: readonly ReferenceContract[];
  readonly artifactRefs: readonly ReferenceContract[];
  readonly verificationRefs: readonly ReferenceContract[];
  readonly performedAt: string;
}): ActContract {
  const criterionStatus = options.closure.disposition === "closed" ? "verified" : "failed";
  return {
    act_id: options.actId,
    form: "observation",
    intent: {
      purpose: options.purpose,
      legitimacy: options.legitimacy,
      success_criteria: [{
        criterion_id: "process_closed",
        statement: "The runtime process reaches a terminal closure.",
        required: true,
      }],
      constraints: [],
      derived_from: options.sourceRefs,
    },
    summary: options.summary,
    closure: options.closure,
    criterion_bindings: [{
      criterion_id: "process_closed",
      status: criterionStatus,
      evidence_refs: options.sourceRefs,
      verification_refs: options.verificationRefs,
      summary: options.summary,
    }],
    source_refs: options.sourceRefs,
    target_refs: [],
    surface_refs: options.surfaceRefs,
    artifact_refs: options.artifactRefs,
    verification_refs: options.verificationRefs,
    harness_refs: [],
    performed_at: options.performedAt,
  };
}

function closureRecord(options: {
  readonly disposition: HarnessSealDispositionContract;
  readonly reasonCode: string;
  readonly summary: string;
  readonly closedAt: string;
}): ClosureRecordContract {
  return {
    disposition: options.disposition,
    reason_code: options.reasonCode,
    summary: options.summary,
    closed_at: options.closedAt,
  };
}

function sealRecord(options: {
  readonly disposition: HarnessSealDispositionContract;
  readonly reasonCode: string;
  readonly summary: string;
  readonly closedAt: string;
  readonly criteria: HarnessSealContract["criteria"];
  readonly artifactRefs: readonly ReferenceContract[];
  readonly stdout: string;
  readonly stderr: string;
  readonly inputs: Readonly<Record<string, unknown>>;
}): HarnessSealContract {
  return {
    disposition: options.disposition,
    reason_code: options.reasonCode,
    summary: options.summary,
    closed_at: options.closedAt,
    last_observed_at: options.closedAt,
    canonicalization: "runx.harness-receipt.c14n.v1",
    digest: `sha256:${hashStable({
      disposition: options.disposition,
      stdout: options.stdout,
      stderr: options.stderr,
      inputs: options.inputs,
      artifact_refs: options.artifactRefs,
    })}`,
    criteria: options.criteria,
    verification_summary: {
      signature_valid: true,
      hash_commitments_valid: true,
      authority_attenuation_valid: true,
      criteria_bound: options.criteria.length > 0,
      redaction_valid: true,
      external_attestations_present: false,
    },
    redaction_refs: [],
    artifact_refs: options.artifactRefs,
    hash_commitments: [
      {
        algorithm: "sha256",
        value: `sha256:${hashString(options.stdout)}`,
        canonicalization: "runx.stdout-hash.v1",
      },
      {
        algorithm: "sha256",
        value: `sha256:${hashString(options.stderr)}`,
        canonicalization: "runx.stderr-hash.v1",
      },
      {
        algorithm: "sha256",
        value: `sha256:${hashStable(options.inputs)}`,
        canonicalization: "runx.input-hash.v1",
      },
    ],
  };
}

function localHarnessAuthority(): HarnessAuthorityContract {
  return {
    actor_ref: { type: "principal", uri: "runx:principal:local-runtime", label: "local runtime" },
    authority_proof_refs: [],
    grant_refs: [],
    scope_refs: [],
    policy_refs: [],
    terms: [],
    attenuation: {
      parent_authority_ref: null,
      subset_proof: null,
    },
  };
}

function runtimeReceiptMetadata(options: {
  readonly kind: RuntimeReceiptMetadataKind;
  readonly name: string;
  readonly owner?: string;
  readonly sourceType: string;
  readonly status: "sealed" | "failure";
  readonly startedAt?: string;
  readonly completedAt: string;
  readonly durationMs: number;
  readonly disposition?: string;
  readonly inputHash: string;
  readonly outputHash: string;
  readonly stderrHash?: string;
  readonly errorHash?: string;
  readonly execution?: Readonly<Record<string, unknown>>;
  readonly contextFrom?: readonly string[];
  readonly outcomeState?: string;
  readonly outcome?: unknown;
  readonly inputContext?: unknown;
  readonly surfaceRefs?: readonly ReferenceContract[];
  readonly evidenceRefs?: readonly ReferenceContract[];
  readonly steps?: readonly GraphReceiptStep[];
  readonly extra?: Readonly<Record<string, unknown>>;
}): Readonly<Record<string, unknown>> {
  const extra = redactReceiptMetadata(options.extra ?? {});
  const extraRunx = isRecord(extra.runx) ? extra.runx : {};
  const { runx: _discardedRunx, ...extraMetadata } = extra;
  return redactReceiptMetadata({
    ...extraMetadata,
    runx: {
      ...extraRunx,
      category: options.kind,
      name: options.name,
      owner: options.owner,
      source: options.sourceType,
      status: options.status,
      started_at: options.startedAt,
      completed_at: options.completedAt,
      duration_ms: options.durationMs,
      disposition: options.disposition,
      input_hash: options.inputHash,
      output_hash: options.outputHash,
      stderr_hash: options.stderrHash,
      error_hash: options.errorHash,
      execution: options.execution,
      context_from: options.contextFrom,
      outcome_state: options.outcomeState,
      outcome: options.outcome,
      input_context: options.inputContext,
      surface_refs: options.surfaceRefs,
      evidence_refs: options.evidenceRefs,
      steps: options.steps,
    },
  });
}

function runtimeMetadata(receipt: RunnerReceipt): RuntimeReceiptMetadata {
  const metadata = receipt.metadata;
  const runx = metadata && typeof metadata.runx === "object" && metadata.runx !== null
    ? metadata.runx as RuntimeReceiptMetadata
    : {};
  return runx;
}

function sealDisposition(
  status: "sealed" | "failure",
  disposition: NormalizedExecutionSemantics["disposition"] | undefined,
): HarnessSealDispositionContract {
  if (disposition === "policy_denied") return "declined";
  if (disposition === "needs_agent" || disposition === "approval_required") return "deferred";
  if (disposition === "observing") return "deferred";
  if (disposition === "escalated") return "blocked";
  if (status === "failure") return "failed";
  return "closed";
}

function skillClosureSummary(skillName: string, execution: RuntimeReceiptExecution): string {
  return execution.status === "sealed"
    ? `${skillName} closed successfully.`
    : `${skillName} closed with failure.`;
}

function graphClosureSummary(graphName: string, status: "sealed" | "failure", stepCount: number): string {
  return status === "sealed"
    ? `${graphName} closed successfully with ${stepCount} step(s).`
    : `${graphName} closed with failure after ${stepCount} step(s).`;
}

function toReferences(refs: readonly { readonly type: string; readonly uri: string; readonly label?: string }[] | undefined): readonly ReferenceContract[] {
  return (refs ?? []).map((ref) => {
    const normalized: ReferenceContract = {
      type: normalizeReferenceType(ref.type),
      uri: ref.uri,
    };
    return ref.label && ref.label.trim().length > 0
      ? { ...normalized, label: ref.label }
      : normalized;
  });
}

function artifactReferences(ids: readonly string[] | undefined): readonly ReferenceContract[] {
  return (ids ?? []).map((id) => ({
    type: "artifact",
    uri: id.startsWith("runx:artifact:") ? id : `runx:artifact:${id}`,
  }));
}

function verificationReferences(outcome: NormalizedExecutionSemantics["outcome"] | undefined): readonly ReferenceContract[] {
  if (!outcome?.code) return [];
  return [{
    type: "verification",
    uri: `runx:verification:${safeRefSegment(outcome.code)}`,
    label: outcome.summary,
    observed_at: outcome.observed_at,
  }];
}

function harnessReceiptReference(id: string): ReferenceContract {
  return {
    type: "harness_receipt",
    uri: id.startsWith("runx:harness_receipt:") ? id : `runx:harness_receipt:${id}`,
  };
}

function harnessReference(id: string, label: string): ReferenceContract {
  return {
    type: "harness",
    uri: `runx:harness:${harnessIdentitySegment(id)}`,
    label,
  };
}

function harnessIdentitySegment(id: string): string {
  return safeRefSegment(id.startsWith("hrn_rcpt_") ? id.slice("hrn_rcpt_".length) : id);
}

function normalizeReferenceType(type: string): ReferenceContract["type"] {
  if (type === "issue") return "github_issue";
  const allowed = new Set<ReferenceContract["type"]>([
    "github_issue",
    "github_pull_request",
    "github_repo",
    "slack_thread",
    "sentry_event",
    "signal",
    "act",
    "receipt",
    "graph_receipt",
    "harness_receipt",
    "artifact",
    "verification",
    "harness",
    "host",
    "deployment",
    "surface",
    "target",
    "opportunity",
    "thesis_assessment",
    "selection",
    "skill_binding",
    "target_transition_entry",
    "selection_cycle",
    "decision",
    "reflection_entry",
    "feed_entry",
    "principal",
    "authority_proof",
    "scope_admission",
    "grant",
    "mandate",
    "credential",
    "webhook_delivery",
    "redaction_policy",
    "external_url",
  ]);
  return allowed.has(type as ReferenceContract["type"]) ? type as ReferenceContract["type"] : "external_url";
}

function safeRefSegment(value: string): string {
  return value.replace(/[^A-Za-z0-9_:-]+/g, "_").replace(/^_+|_+$/g, "") || "local";
}

function assertNonEmptyReceiptIdentity(
  value: string | null | undefined,
  fieldName: string,
  context: string | null | undefined,
): asserts value is string {
  if (typeof value !== "string" || value.trim().length === 0) {
    const ctx = typeof context === "string" && context.trim().length > 0 ? ` (context: ${context})` : "";
    throw new Error(`Receipt ${fieldName} must be a non-empty string${ctx}.`);
  }
}

function validateGraphReceiptGovernance(
  value: GraphReceiptGovernance | undefined,
  label = "governance",
): GraphReceiptGovernance | undefined {
  if (value === undefined) {
    return undefined;
  }

  return {
    scope_admission: value.scope_admission
      ? validateScopeAdmissionContract(value.scope_admission, `${label}.scope_admission`)
      : undefined,
  };
}

function redactReceiptMetadata(value: Readonly<Record<string, unknown>>): Readonly<Record<string, unknown>> {
  return redactReceiptValue(value) as Readonly<Record<string, unknown>>;
}

function redactReceiptValue(value: unknown): unknown {
  if (Array.isArray(value)) {
    return value.map((item) => redactReceiptValue(item));
  }
  if (typeof value === "string") {
    return redactSecretString(value);
  }
  if (value === null || typeof value !== "object") {
    return value;
  }

  return Object.fromEntries(
    Object.entries(value as Record<string, unknown>).map(([key, entryValue]) => [
      key,
      isSecretKey(key) ? "[redacted]" : redactReceiptValue(entryValue),
    ]),
  );
}

function isSecretKey(key: string): boolean {
  const normalized = key.toLowerCase();
  if (/(material[_-]?ref[_-]?hash|materialrefhash)/i.test(normalized)) {
    return false;
  }
  return /(access[_-]?token|refresh[_-]?token|api[_-]?key|client[_-]?secret|password|material[_-]?ref|materialref|raw[_-]?secret|raw[_-]?token)/i.test(normalized);
}

function redactSecretString(value: string): string {
  return value
    .replace(/\b(gh[pousr]_[A-Za-z0-9_]{20,}|xox[baprs]-[A-Za-z0-9-]{20,})\b/g, "[redacted]")
    .replace(/\bsk-(?:proj-)?[A-Za-z0-9_-]{16,}\b/g, "[redacted]")
    .replace(/\b((?:bearer|authorization)\s+)[A-Za-z0-9._:-]{6,}\b/gi, "$1[redacted]")
    .replace(/\b[A-Za-z0-9]+(?:[-_](?:secret|token|password|api[-_]?key))+[A-Za-z0-9_-]*\b(?!\s*=)/gi, "[redacted]");
}
