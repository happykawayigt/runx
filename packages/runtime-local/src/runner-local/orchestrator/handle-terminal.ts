import {
  fanoutGateReceiptMetadata,
} from "../graph-fanout-gates.js";
import {
  latestFanoutReceiptIds,
  toGraphReceiptSyncPoint,
} from "../graph-governance.js";
import { resolveSequentialGraphFailureReason } from "../graph-hydration.js";
import {
  transitionSequentialGraphViaKernel,
  type SequentialGraphPlan,
} from "../kernel-bridge.js";
import type { RunLocalGraphOptions } from "../index.js";

import type { HandlerContinuation, RunContext } from "./run-context.js";

type CompletePlan = Extract<SequentialGraphPlan, { type: "complete" }>;
type FailedPlan = Extract<SequentialGraphPlan, { type: "failed" }>;
type BlockedPlan = Extract<SequentialGraphPlan, { type: "blocked" }>;
type EscalatedPlan = Extract<SequentialGraphPlan, { type: "escalated" }>;

export async function handleCompletePlan(
  ctx: RunContext,
  _plan: CompletePlan,
  options: RunLocalGraphOptions,
): Promise<HandlerContinuation> {
  ctx.state = await transitionSequentialGraphViaKernel(ctx.state, { type: "complete" }, { env: options.env });
  return { kind: "break" };
}

export async function handleFailedPlan(
  ctx: RunContext,
  plan: FailedPlan,
  options: RunLocalGraphOptions,
): Promise<HandlerContinuation> {
  ctx.finalError = resolveSequentialGraphFailureReason(plan, ctx.state, ctx.stepRuns);
  if (plan.syncDecision) {
    ctx.syncPoints.push(toGraphReceiptSyncPoint(plan.syncDecision, latestFanoutReceiptIds(ctx.stepRuns, plan.syncDecision.groupId)));
  }
  ctx.state = await transitionSequentialGraphViaKernel(
    ctx.state,
    { type: "fail_graph", error: ctx.finalError },
    { env: options.env },
  );
  return { kind: "break" };
}

export async function handleBlockedPlan(
  ctx: RunContext,
  plan: BlockedPlan,
  options: RunLocalGraphOptions,
): Promise<HandlerContinuation> {
  ctx.finalError = plan.reason;
  if (plan.syncDecision) {
    ctx.syncPoints.push(toGraphReceiptSyncPoint(plan.syncDecision, latestFanoutReceiptIds(ctx.stepRuns, plan.syncDecision.groupId)));
  }
  ctx.state = await transitionSequentialGraphViaKernel(
    ctx.state,
    { type: "fail_graph", error: plan.reason },
    { env: options.env },
  );
  return { kind: "break" };
}

export async function handleEscalatedPlan(
  ctx: RunContext,
  plan: EscalatedPlan,
  options: RunLocalGraphOptions,
): Promise<HandlerContinuation> {
  const syncPoint = toGraphReceiptSyncPoint(
    plan.syncDecision,
    latestFanoutReceiptIds(ctx.stepRuns, plan.syncDecision.groupId),
  );
  ctx.syncPoints.push(syncPoint);
  ctx.finalError = `fanout escalation: ${plan.reason}`;
  ctx.terminalReceiptMetadata = fanoutGateReceiptMetadata(plan.syncDecision, "escalated");
  await options.caller.report({
    type: "warning",
    message: ctx.finalError,
    data: {
      kind: "fanout_escalated",
      syncPoint,
    },
  });
  ctx.state = await transitionSequentialGraphViaKernel(
    ctx.state,
    { type: "escalate_graph", reason: ctx.finalError },
    { env: options.env },
  );
  return { kind: "break" };
}
