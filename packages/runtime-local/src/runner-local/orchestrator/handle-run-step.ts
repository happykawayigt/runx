import {
  materializeArtifacts,
} from "@runxhq/core/artifacts";
import { errorMessage } from "@runxhq/core/util";
import { findGraphStep, materializeContext, materializeStepInputs } from "../graph-context.js";
import {
  appendGraphLedgerEntries,
  appendGraphStepFailureLedgerEntry,
  appendGraphStepStartedLedgerEntry,
  appendPendingGraphLedgerEntry,
} from "../graph-ledger.js";
import {
  graphProducerSkillName,
  graphStepExecutionDirectory,
  graphStepRunner,
  reportGraphStepCompleted,
  reportGraphStepStarted,
  reportGraphStepWaitingResolution,
} from "../graph-reporting.js";
import {
  buildDeniedGraphStepRun,
  buildGraphStepGovernance,
  graphStepAuthorityProofMetadata,
  graphStepScopeAdmission,
  governanceReceiptMetadata,
  runnerReceiptDisposition,
  runnerReceiptEvidenceRefs,
  runnerReceiptInputContext,
  runnerReceiptOutcome,
  runnerReceiptOutcomeState,
  runnerReceiptSurfaceRefs,
  writePolicyDeniedGraphReceipt,
} from "../graph-governance.js";
import { admitGraphTransition } from "../graph-hydration.js";
import {
  admitRetryPolicyViaKernel,
  transitionSequentialGraphViaKernel,
  type SequentialGraphPlan,
} from "../kernel-bridge.js";
import { resolveGraphStepExecution } from "../execution-targets.js";
import { materializeDeclaredInputs } from "../inputs.js";
import {
  buildRetryReceiptContext,
  isAgentMediatedSource,
  mergeMetadata,
} from "../runner-helpers.js";
import { runValidatedSkill, type GraphStepRun, type RunLocalGraphOptions } from "../index.js";

import type { HandlerContinuation, RunContext } from "./run-context.js";

type RunStepPlan = Extract<SequentialGraphPlan, { type: "run_step" }>;

export async function handleRunStepPlan(
  ctx: RunContext,
  plan: RunStepPlan,
  options: RunLocalGraphOptions,
): Promise<HandlerContinuation> {
  const step = findGraphStep(ctx.graph, plan.stepId);
  const context = materializeContext(step, ctx.outputs);
  const contextFromReceiptIds = context
    .map((edge) => edge.receiptId)
    .filter((receiptId): receiptId is string => typeof receiptId === "string");
  const resolvedStep = await resolveGraphStepExecution({
    step,
    graphDirectory: ctx.graphDirectory,
    graphStepCache: ctx.graphStepCache,
    skillEnvironment: options.skillEnvironment,
    registryStore: options.registryStore,
    skillCacheDir: options.skillCacheDir,
    toolCatalogAdapters: options.toolCatalogAdapters,
    officialSkillResolver: options.officialSkillResolver,
    env: options.env,
  });
  const stepSkillPath = resolvedStep.skillPath;
  const stepSkill = resolvedStep.skill;
  ctx.involvedAgentMediatedWork ||= isAgentMediatedSource(stepSkill.source.type);
  const stepInputs = materializeDeclaredInputs(stepSkill.inputs, {
    ...(options.inputs ?? {}),
    ...materializeStepInputs(step.inputs, options.inputs ?? {}),
    ...Object.fromEntries(context.map((edge) => [edge.input, edge.value])),
  });
  const governance = await buildGraphStepGovernance(step, ctx.graphGrant, { env: options.env });
  const transitionGate = admitGraphTransition(ctx.graph.policy, step.id, ctx.outputs);
  if (transitionGate.status === "deny") {
    const deniedRun = buildDeniedGraphStepRun({
      step,
      stepSkillPath,
      attempt: plan.attempt,
      parentReceipt: ctx.lastReceiptId,
      governance,
      context,
      stderr: transitionGate.reason,
    });
    const receipt = await writePolicyDeniedGraphReceipt({
      receiptDir: ctx.receiptDir,
      runxHome: options.runxHome ?? options.env?.RUNX_HOME,
      graph: ctx.graph,
      graphId: ctx.graphId,
      startedAt: ctx.startedAt,
      startedAtMs: ctx.startedAtMs,
      inputs: options.inputs ?? {},
      stepRuns: [...ctx.stepRuns, deniedRun],
      errorMessage: transitionGate.reason,
      executionSemantics: ctx.executionSemantics,
      receiptMetadata: mergeMetadata(
        ctx.inheritedReceiptMetadata,
        await graphStepAuthorityProofMetadata({ graphId: ctx.graphId, step, stepSkill, governance, env: options.env }),
      ),
    });
    return {
      kind: "return",
      result: {
        status: "policy_denied",
        graph: ctx.graph,
        stepId: step.id,
        skill: stepSkill,
        reasons: [transitionGate.reason],
        state: ctx.state,
        receipt,
      },
    };
  }
  if (governance.scopeAdmission.status === "deny") {
    const deniedRun = buildDeniedGraphStepRun({
      step,
      stepSkillPath,
      attempt: plan.attempt,
      parentReceipt: ctx.lastReceiptId,
      governance,
      context,
    });
    const receipt = await writePolicyDeniedGraphReceipt({
      receiptDir: ctx.receiptDir,
      runxHome: options.runxHome ?? options.env?.RUNX_HOME,
      graph: ctx.graph,
      graphId: ctx.graphId,
      startedAt: ctx.startedAt,
      startedAtMs: ctx.startedAtMs,
      inputs: options.inputs ?? {},
      stepRuns: [...ctx.stepRuns, deniedRun],
      errorMessage: governance.scopeAdmission.reasons?.join("; ") ?? "graph step scope denied",
      executionSemantics: ctx.executionSemantics,
      receiptMetadata: mergeMetadata(
        ctx.inheritedReceiptMetadata,
        await graphStepAuthorityProofMetadata({ graphId: ctx.graphId, step, stepSkill, governance, env: options.env }),
      ),
    });
    return {
      kind: "return",
      result: {
        status: "policy_denied",
        graph: ctx.graph,
        stepId: step.id,
        skill: stepSkill,
        reasons: governance.scopeAdmission.reasons ?? [],
        state: ctx.state,
        receipt,
      },
    };
  }
  const effectiveRetry = step.retry ?? stepSkill.retry;
  const retryContext = buildRetryReceiptContext(step, stepInputs, plan.attempt, stepSkill, effectiveRetry);
  let retryAdmission;
  try {
    retryAdmission = await admitRetryPolicyViaKernel({
      stepId: step.id,
      retry: effectiveRetry,
      mutating: step.mutating || stepSkill.mutating === true,
      idempotencyKey: retryContext.idempotencyKey,
    }, { env: options.env });
  } catch (error) {
    retryAdmission = {
      status: "deny" as const,
      reasons: [`retry admission failed closed: ${errorMessage(error)}`],
    };
  }
  if (retryAdmission.status === "deny") {
    const deniedRun = buildDeniedGraphStepRun({
      step,
      stepSkillPath,
      attempt: plan.attempt,
      parentReceipt: ctx.lastReceiptId,
      governance,
      context,
      stderr: retryAdmission.reasons.join("; "),
    });
    const receipt = await writePolicyDeniedGraphReceipt({
      receiptDir: ctx.receiptDir,
      runxHome: options.runxHome ?? options.env?.RUNX_HOME,
      graph: ctx.graph,
      graphId: ctx.graphId,
      startedAt: ctx.startedAt,
      startedAtMs: ctx.startedAtMs,
      inputs: options.inputs ?? {},
      stepRuns: [...ctx.stepRuns, deniedRun],
      errorMessage: retryAdmission.reasons.join("; "),
      executionSemantics: ctx.executionSemantics,
      receiptMetadata: mergeMetadata(
        ctx.inheritedReceiptMetadata,
        await graphStepAuthorityProofMetadata({ graphId: ctx.graphId, step, stepSkill, governance, env: options.env }),
      ),
    });
    return {
      kind: "return",
      result: {
        status: "policy_denied",
        graph: ctx.graph,
        stepId: step.id,
        skill: stepSkill,
        reasons: retryAdmission.reasons,
        state: ctx.state,
        receipt,
      },
    };
  }

  const stepStartedAt = new Date().toISOString();
  ctx.state = await transitionSequentialGraphViaKernel(
    ctx.state,
    {
      type: "start_step",
      stepId: step.id,
      at: stepStartedAt,
    },
    { env: options.env },
  );
  await reportGraphStepStarted(options.caller, step, resolvedStep.reference);
  await appendGraphStepStartedLedgerEntry({
    receiptDir: ctx.receiptDir,
    runId: ctx.graphId,
    topLevelSkillName: graphProducerSkillName(options.skillEnvironment?.name, ctx.graph.name),
    step,
    reference: resolvedStep.reference,
    createdAt: stepStartedAt,
  });

  const stepResult = await runValidatedSkill({
    skill: stepSkill,
    skillDirectory: graphStepExecutionDirectory(step, stepSkillPath, ctx.graphDirectory),
    requestedSkillPath: resolvedStep.reference,
    runId: options.stepRunIds?.[step.id],
    inputs: stepInputs,
    caller: options.caller,
    env: options.env,
    receiptDir: ctx.receiptDir,
    runxHome: options.runxHome,
    knowledgeDir: options.knowledgeDir,
    parentReceipt: ctx.lastReceiptId,
    contextFrom: contextFromReceiptIds,
    adapters: options.adapters,
    allowedSourceTypes: options.allowedSourceTypes,
    authResolver: options.authResolver,
    receiptMetadata: mergeMetadata(
      ctx.inheritedReceiptMetadata,
      retryContext.receiptMetadata,
      governanceReceiptMetadata(step, governance),
    ),
    orchestrationRunId: ctx.graphId,
    orchestrationStepId: step.id,
    currentContext: context,
    registryStore: options.registryStore,
    skillCacheDir: options.skillCacheDir,
    toolCatalogAdapters: options.toolCatalogAdapters,
    context: ctx.contextSnapshot,
    voiceProfile: ctx.voiceProfile,
    voiceProfilePath: options.voiceProfilePath,
    workspacePolicy: ctx.workspacePolicy,
    authorityMutating: step.mutating || stepSkill.mutating === true,
    authorityScopeAdmission: graphStepScopeAdmission(governance),
  });

  if (stepResult.status === "needs_agent") {
    await reportGraphStepWaitingResolution(
      options.caller,
      step,
      resolvedStep.reference,
      stepResult.requests,
    );
    await appendPendingGraphLedgerEntry({
      receiptDir: ctx.receiptDir,
      runId: ctx.graphId,
      topLevelSkillName: graphProducerSkillName(options.skillEnvironment?.name, ctx.graph.name),
      stepId: step.id,
      kind: "step_waiting_resolution",
      detail: {
        request_ids: stepResult.requests.map((request) => request.id),
        resolution_kinds: Array.from(new Set(stepResult.requests.map((request) => request.kind))),
        requests: stepResult.requests,
        runner: graphStepRunner(step) ?? "default",
        step_label: step.label,
      },
      createdAt: new Date().toISOString(),
    });
    return {
      kind: "return",
      result: {
        status: "needs_agent",
        graph: ctx.graph,
        stepIds: [step.id],
        stepLabels: [step.label ?? step.id],
        skillPath: stepSkillPath,
        skill: stepSkill,
        requests: stepResult.requests,
        state: ctx.state,
        runId: ctx.graphId,
      },
    };
  }

  if (stepResult.status === "policy_denied") {
    await reportGraphStepCompleted(options.caller, step, resolvedStep.reference, "failure", {
      reason: `policy denied: ${stepResult.reasons.join("; ")}`,
    });
    await appendGraphStepFailureLedgerEntry({
      receiptDir: ctx.receiptDir,
      runId: ctx.graphId,
      topLevelSkillName: graphProducerSkillName(options.skillEnvironment?.name, ctx.graph.name),
      stepId: step.id,
      reason: `policy denied: ${stepResult.reasons.join("; ")}`,
    });
    return {
      kind: "return",
      result: {
        status: "policy_denied",
        graph: ctx.graph,
        stepId: step.id,
        skill: stepResult.skill,
        reasons: stepResult.reasons,
        state: await transitionSequentialGraphViaKernel(ctx.state, {
          type: "step_failed",
          stepId: step.id,
          at: new Date().toISOString(),
          error: `policy denied: ${stepResult.reasons.join("; ")}`,
        }, { env: options.env }),
      },
    };
  }

  const stepCompletedAt = new Date().toISOString();
  const artifactResult = materializeArtifacts({
    stdout: stepResult.execution.stdout,
    contract: stepResult.skill.artifacts,
    runId: ctx.graphId,
    stepId: step.id,
    producer: {
      skill: stepResult.skill.name,
      runner: stepResult.skill.source.type,
    },
    createdAt: stepCompletedAt,
  });
  const stepRun: GraphStepRun = {
    stepId: step.id,
    skill: resolvedStep.reference,
    skillPath: stepSkillPath,
    runner: graphStepRunner(step),
    attempt: plan.attempt,
    status: stepResult.status,
    receiptId: stepResult.receipt.id,
    stdout: stepResult.execution.stdout,
    stderr: stepResult.execution.stderr,
    parentReceipt: ctx.lastReceiptId,
    retry: retryContext.receipt,
    governance,
    artifactIds: artifactResult.envelopes.map((envelope) => envelope.meta.artifact_id),
    disposition: runtimeDisposition(runnerReceiptDisposition(stepResult.receipt)),
    inputContext: runnerReceiptInputContext(stepResult.receipt),
    outcomeState: runtimeOutcomeState(runnerReceiptOutcomeState(stepResult.receipt)),
    outcome: runnerReceiptOutcome(stepResult.receipt),
    surfaceRefs: runnerReceiptSurfaceRefs(stepResult.receipt),
    evidenceRefs: runnerReceiptEvidenceRefs(stepResult.receipt),
    contextFrom: context.map((edge) => ({
      input: edge.input,
      fromStep: edge.fromStep,
      output: edge.output,
      receiptId: edge.receiptId,
    })),
  };
  ctx.stepRuns.push(stepRun);
  ctx.outputs.set(step.id, {
    status: stepResult.status,
    stdout: stepResult.execution.stdout,
    stderr: stepResult.execution.stderr,
    receiptId: stepResult.receipt.id,
    fields: artifactResult.fields,
    artifactIds: artifactResult.envelopes.map((envelope) => envelope.meta.artifact_id),
    artifacts: artifactResult.envelopes,
  });
  ctx.lastReceiptId = stepResult.receipt.id;
  ctx.finalOutput = stepResult.execution.stdout;
  await appendGraphLedgerEntries({
    receiptDir: ctx.receiptDir,
    runId: ctx.graphId,
    topLevelSkillName: graphProducerSkillName(options.skillEnvironment?.name, ctx.graph.name),
    stepId: step.id,
    skill: stepResult.skill,
    artifactEnvelopes: artifactResult.envelopes,
    receiptId: stepResult.receipt.id,
    status: stepResult.status,
    detail: {
      runner: graphStepRunner(step) ?? "default",
    },
    createdAt: stepCompletedAt,
  });

  ctx.state =
    stepResult.status === "sealed"
      ? await transitionSequentialGraphViaKernel(ctx.state, {
          type: "step_succeeded",
          stepId: step.id,
          at: stepCompletedAt,
          receiptId: stepResult.receipt.id,
          admissionWitness: { stepId: step.id, receiptId: stepResult.receipt.id },
          outputs: artifactResult.fields,
        }, { env: options.env })
      : await transitionSequentialGraphViaKernel(ctx.state, {
          type: "step_failed",
          stepId: step.id,
          at: stepCompletedAt,
          error: stepResult.execution.errorMessage ?? stepResult.execution.stderr,
        }, { env: options.env });
  await reportGraphStepCompleted(options.caller, step, resolvedStep.reference, stepResult.status, {
    receiptId: stepResult.receipt.id,
  });

  return { kind: "continue" };
}

function runtimeDisposition(disposition: ReturnType<typeof runnerReceiptDisposition>): GraphStepRun["disposition"] {
  if (disposition === "declined") return "policy_denied";
  if (disposition === "deferred") return "needs_agent";
  if (disposition === "blocked") return "escalated";
  return disposition === "closed" ? "completed" : "completed";
}

function runtimeOutcomeState(value: string | undefined): GraphStepRun["outcomeState"] {
  return value === "pending" || value === "expired" || value === "complete" ? value : "complete";
}
