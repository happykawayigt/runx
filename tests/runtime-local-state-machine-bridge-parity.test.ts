import { describe, expect, it } from "vitest";
import type {
  FanoutSyncDecision as CoreFanoutSyncDecision,
  SequentialGraphPlan as CoreSequentialGraphPlan,
  SequentialGraphState as CoreSequentialGraphState,
  SingleStepState as CoreSingleStepState,
} from "@runxhq/core/state-machine";

import type {
  FanoutSyncDecision,
  SequentialGraphPlan,
  SequentialGraphState,
  SingleStepState,
} from "../packages/runtime-local/src/runner-local/kernel-bridge.js";

type Assert<T extends true> = T;
type Extends<Left, Right> = [Left] extends [Right] ? true : false;

type _BridgeSingleStepMatchesCore = Assert<Extends<SingleStepState, CoreSingleStepState>>;
type _CoreSingleStepMatchesBridge = Assert<Extends<CoreSingleStepState, SingleStepState>>;
type _BridgeSequentialStateMatchesCore = Assert<Extends<SequentialGraphState, CoreSequentialGraphState>>;
type _CoreSequentialStateMatchesBridge = Assert<Extends<CoreSequentialGraphState, SequentialGraphState>>;
type _BridgeSequentialPlanMatchesCore = Assert<Extends<SequentialGraphPlan, CoreSequentialGraphPlan>>;
type _CoreSequentialPlanMatchesBridge = Assert<Extends<CoreSequentialGraphPlan, SequentialGraphPlan>>;
type _BridgeFanoutDecisionMatchesCore = Assert<Extends<FanoutSyncDecision, CoreFanoutSyncDecision>>;
type _CoreFanoutDecisionMatchesBridge = Assert<Extends<CoreFanoutSyncDecision, FanoutSyncDecision>>;

describe("runtime-local state-machine bridge type parity", () => {
  it("keeps bridge state, plan, and decision result types assignable to the legacy TS surface", () => {
    expect(true).toBe(true);
  });
});
