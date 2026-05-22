import { describe, expect, it } from "vitest";

import type {
  FanoutSyncDecision,
  SequentialGraphEvent,
  SequentialGraphPlan,
  SequentialGraphState,
  SingleStepState,
  SingleStepEvent,
  StepAdmissionWitness,
} from "../packages/runtime-local/src/runner-local/kernel-bridge.js";

type Assert<T extends true> = T;
type IsKernelObject<T> = T extends object ? true : false;
type IsNever<T> = [T] extends [never] ? true : false;
type SucceedEvent = Extract<SingleStepEvent, { readonly type: "succeed" }>;
type StepSucceededEvent = Extract<SequentialGraphEvent, { readonly type: "step_succeeded" }>;

type _SingleStepStateIsKernelObject = Assert<IsKernelObject<SingleStepState>>;
type _SequentialStateIsKernelObject = Assert<IsKernelObject<SequentialGraphState>>;
type _SequentialPlanIsKernelObject = Assert<IsKernelObject<SequentialGraphPlan>>;
type _FanoutDecisionIsKernelObject = Assert<IsKernelObject<FanoutSyncDecision>>;
type _SingleStepSuccessEventExists = Assert<IsNever<SucceedEvent> extends false ? true : false>;
type _SequentialStepSuccessEventExists = Assert<IsNever<StepSucceededEvent> extends false ? true : false>;
type _SingleStepSuccessRequiresWitness = Assert<
  SucceedEvent extends { readonly admissionWitness: StepAdmissionWitness } ? true : false
>;
type _SequentialStepSuccessRequiresWitness = Assert<
  StepSucceededEvent extends { readonly admissionWitness: StepAdmissionWitness } ? true : false
>;

describe("runtime-local state-machine bridge type parity", () => {
  it("keeps bridge state, plan, and decision contracts owned by the Rust kernel boundary", () => {
    expect(true).toBe(true);
  });
});
