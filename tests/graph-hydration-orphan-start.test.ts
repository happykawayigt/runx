import { spawnSync } from "node:child_process";
import path from "node:path";

import { beforeAll, describe, expect, it } from "vitest";

import type { ArtifactEnvelope } from "@runxhq/core/artifacts";

import { hydrateGraphFromLedger } from "../packages/runtime-local/src/runner-local/graph-hydration.js";
import {
  type KernelBridgeOptions,
  type SequentialGraphState,
} from "../packages/runtime-local/src/runner-local/kernel-bridge.js";

const workspaceRoot = process.cwd();
const runxBinary = path.join(
  workspaceRoot,
  "crates",
  "target",
  "debug",
  process.platform === "win32" ? "runx.exe" : "runx",
);
const kernel: KernelBridgeOptions = { command: runxBinary, cwd: workspaceRoot, timeoutMs: 30_000 };

const STEP_A = {
  id: "step-a",
  label: "step A",
  run: { type: "agent-step" as const, agent: "builder", task: "step-a" },
};

const STEPS = [STEP_A];

const STEP_SKILL = {
  name: "step-a",
  description: "fixture",
  body: "fixture",
  source: { type: "agent-step" as const, agent: "builder", task: "step-a", outputs: {} },
  inputs: {},
  artifacts: undefined,
  qualityProfile: undefined,
  runx: undefined,
  raw: { frontmatter: {}, rawFrontmatter: "", body: "fixture" },
} as never;

function runEvent(kind: string, stepId: string, ts: string, runId: string, receiptId?: string): ArtifactEnvelope {
  return {
    type: "run_event",
    version: "1",
    data: {
      kind,
      status: kind === "step_started" ? "started" : "completed",
      step_id: stepId,
      detail: receiptId ? { receipt_id: receiptId } : {},
    },
    meta: {
      artifact_id: `ax_${kind}_${stepId}`,
      run_id: runId,
      step_id: stepId,
      producer: { skill: "fixture", runner: "graph" },
      created_at: ts,
      hash: "0".repeat(64),
      size_bytes: 64,
      parent_artifact_id: null,
      receipt_id: null,
      redacted: false,
    },
  } as unknown as ArtifactEnvelope;
}

function initialState(runId: string): SequentialGraphState {
  return {
    graphId: runId,
    status: "pending",
    steps: [{ stepId: "step-a", status: "pending", attempts: 0 }],
  };
}

function newRefs(state: SequentialGraphState) {
  let stateValue = state;
  let lastReceiptValue: string | undefined;
  return {
    stateRef: {
      get value() {
        return stateValue;
      },
      set value(next: SequentialGraphState) {
        stateValue = next;
      },
    },
    lastReceiptRef: {
      get value() {
        return lastReceiptValue;
      },
      set value(next: string | undefined) {
        lastReceiptValue = next;
      },
    },
    getState: () => stateValue,
  };
}

describe("hydrateGraphFromLedger orphan step_started", () => {
  beforeAll(() => {
    const result = spawnSync(
      process.platform === "win32" ? "cargo.exe" : "cargo",
      ["build", "--quiet", "--manifest-path", "crates/Cargo.toml", "-p", "runx-cli", "--bin", "runx"],
      {
        cwd: workspaceRoot,
        encoding: "utf8",
        env: process.env,
        maxBuffer: 8 * 1024 * 1024,
      },
    );
    expect(result.status, result.stderr || result.stdout).toBe(0);
  }, 120_000);

  it("orphan step_started (no terminal event) leaves the step in pending state", async () => {
    const runId = "gx_orphan_test";
    const entries: ArtifactEnvelope[] = [
      runEvent("run_started", "", "2026-04-28T00:00:00.000Z", runId),
      runEvent("step_started", "step-a", "2026-04-28T00:00:01.000Z", runId),
      // No terminal event — the prior process was interrupted.
    ];
    const graph = { name: "test", owner: "fixture", steps: STEPS as never } as never;
    const refs = newRefs(initialState(runId));
    await hydrateGraphFromLedger({
      entries,
      graph,
      graphStepCache: new Map([["step-a", STEP_SKILL]]),
      graphSteps: [{ id: "step-a", contextFrom: [] }],
      stepRuns: [],
      outputs: new Map(),
      syncPoints: [],
      stateRef: refs.stateRef,
      lastReceiptRef: refs.lastReceiptRef,
      kernel,
    });
    const stepA = refs.getState().steps.find((s) => s.stepId === "step-a");
    expect(stepA?.status).toBe("pending");
  }, 20_000);

  it("step_started followed by step_succeeded hydrates to succeeded", async () => {
    const runId = "gx_succeeded_test";
    const entries: ArtifactEnvelope[] = [
      runEvent("run_started", "", "2026-04-28T00:00:00.000Z", runId),
      runEvent("step_started", "step-a", "2026-04-28T00:00:01.000Z", runId),
      runEvent("step_succeeded", "step-a", "2026-04-28T00:00:02.000Z", runId, "rx_step_a"),
    ];
    const graph = { name: "test", owner: "fixture", steps: STEPS as never } as never;
    const refs = newRefs(initialState(runId));
    await hydrateGraphFromLedger({
      entries,
      graph,
      graphStepCache: new Map([["step-a", STEP_SKILL]]),
      graphSteps: [{ id: "step-a", contextFrom: [] }],
      stepRuns: [],
      outputs: new Map(),
      syncPoints: [],
      stateRef: refs.stateRef,
      lastReceiptRef: refs.lastReceiptRef,
      kernel,
    });
    const stepA = refs.getState().steps.find((s) => s.stepId === "step-a");
    expect(stepA?.status).toBe("succeeded");
    expect(stepA?.receiptId).toBe("rx_step_a");
  }, 20_000);

  it("step_succeeded without a receipt id does not hydrate as success", async () => {
    const runId = "gx_missing_receipt_test";
    const entries: ArtifactEnvelope[] = [
      runEvent("run_started", "", "2026-04-28T00:00:00.000Z", runId),
      runEvent("step_started", "step-a", "2026-04-28T00:00:01.000Z", runId),
      runEvent("step_succeeded", "step-a", "2026-04-28T00:00:02.000Z", runId),
    ];
    const graph = { name: "test", owner: "fixture", steps: STEPS as never } as never;
    const refs = newRefs(initialState(runId));
    await hydrateGraphFromLedger({
      entries,
      graph,
      graphStepCache: new Map([["step-a", STEP_SKILL]]),
      graphSteps: [{ id: "step-a", contextFrom: [] }],
      stepRuns: [],
      outputs: new Map(),
      syncPoints: [],
      stateRef: refs.stateRef,
      lastReceiptRef: refs.lastReceiptRef,
      kernel,
    });
    const stepA = refs.getState().steps.find((s) => s.stepId === "step-a");
    expect(stepA?.status).toBe("pending");
  });

  it("step_started followed by step_failed hydrates to failed", async () => {
    const runId = "gx_failed_test";
    const entries: ArtifactEnvelope[] = [
      runEvent("run_started", "", "2026-04-28T00:00:00.000Z", runId),
      runEvent("step_started", "step-a", "2026-04-28T00:00:01.000Z", runId),
      runEvent("step_failed", "step-a", "2026-04-28T00:00:02.000Z", runId),
    ];
    const graph = { name: "test", owner: "fixture", steps: STEPS as never } as never;
    const refs = newRefs(initialState(runId));
    await hydrateGraphFromLedger({
      entries,
      graph,
      graphStepCache: new Map([["step-a", STEP_SKILL]]),
      graphSteps: [{ id: "step-a", contextFrom: [] }],
      stepRuns: [],
      outputs: new Map(),
      syncPoints: [],
      stateRef: refs.stateRef,
      lastReceiptRef: refs.lastReceiptRef,
      kernel,
    });
    const stepA = refs.getState().steps.find((s) => s.stepId === "step-a");
    expect(stepA?.status).toBe("failed");
  }, 20_000);
});
