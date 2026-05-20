import { spawnSync } from "node:child_process";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";

import { describe, expect, it } from "vitest";

import { createDefaultSkillAdapters } from "@runxhq/adapters";
import { runLocalGraph, type Caller, type SkillAdapter } from "@runxhq/runtime-local";

const nonInteractiveCaller: Caller = {
  resolve: async () => undefined,
  report: () => undefined,
};
const adapters = createDefaultSkillAdapters();
const workspaceRoot = process.cwd();
const cargo = process.platform === "win32" ? "cargo.exe" : "cargo";
const runxBinary = path.join(
  workspaceRoot,
  "crates",
  "target",
  "debug",
  process.platform === "win32" ? "runx.exe" : "runx",
);

describe("local fanout graph runner", () => {
  it("runs a fanout group with all-success sync policy", async () => {
    const tempDir = await mkdtemp(path.join(os.tmpdir(), "runx-fanout-all-"));
    const receiptDir = path.join(tempDir, "receipts");
    const runxHome = path.join(tempDir, "home");

    try {
      const result = await runLocalGraph({
        graphPath: path.resolve("fixtures/graphs/fanout/all.yaml"),
        caller: nonInteractiveCaller,
        receiptDir,
        runxHome,
        env: process.env,
        adapters,
      });

      expect(result.status).toBe("success");
      if (result.status !== "success") {
        return;
      }

      expect(result.steps.map((step) => [step.stepId, step.status, step.fanoutGroup])).toEqual([
        ["market", "success", "advisors"],
        ["risk", "success", "advisors"],
        ["finance", "success", "advisors"],
        ["synthesize", "success", undefined],
      ]);
      expect(result.steps[3].stdout).toBe("approved");
      expect(result.receipt.schema).toBe("runx.harness_receipt.v1");
      expect(runtimeSyncPoints(result.receipt)).toEqual([
        expect.objectContaining({
          group_id: "advisors",
          strategy: "all",
          decision: "proceed",
          rule_fired: "all.min_success",
          branch_count: 3,
          success_count: 3,
          failure_count: 0,
          required_successes: 3,
        }),
      ]);
    } finally {
      await rm(tempDir, { recursive: true, force: true });
    }
  });

  it("executes three one-second fanout branches concurrently", async () => {
    const tempDir = await mkdtemp(path.join(os.tmpdir(), "runx-fanout-parallel-"));
    const receiptDir = path.join(tempDir, "receipts");
    const runxHome = path.join(tempDir, "home");
    const graphPath = path.join(tempDir, "parallel.yaml");

    try {
      await Promise.all([
        writeSleepSkill(path.join(tempDir, "market"), "market"),
        writeSleepSkill(path.join(tempDir, "risk"), "risk"),
        writeSleepSkill(path.join(tempDir, "finance"), "finance"),
      ]);
      await writeFile(
        graphPath,
        `name: timed-fanout
owner: runx
fanout:
  groups:
    advisors:
      strategy: all
      on_branch_failure: halt
steps:
  - id: market
    mode: fanout
    fanout_group: advisors
    skill: ./market
  - id: risk
    mode: fanout
    fanout_group: advisors
    skill: ./risk
  - id: finance
    mode: fanout
    fanout_group: advisors
    skill: ./finance
`,
      );

      const started = performance.now();
      const result = await runLocalGraph({
        graphPath,
        caller: nonInteractiveCaller,
        receiptDir,
        runxHome,
        env: process.env,
        adapters,
      });
      const durationMs = performance.now() - started;

      expect(result.status).toBe("success");
      expect(durationMs).toBeLessThan(2000);
      if (result.status !== "success") {
        return;
      }
      expect(result.steps.map((step) => step.stepId)).toEqual(["market", "risk", "finance"]);
    } finally {
      await rm(tempDir, { recursive: true, force: true });
    }
  });

  it("runs a fanout group with quorum sync and linked branch receipts", async () => {
    const tempDir = await mkdtemp(path.join(os.tmpdir(), "runx-fanout-"));
    const receiptDir = path.join(tempDir, "receipts");
    const runxHome = path.join(tempDir, "home");

    try {
      const result = await runLocalGraph({
        graphPath: path.resolve("fixtures/graphs/fanout/graph.yaml"),
        caller: nonInteractiveCaller,
        receiptDir,
        runxHome,
        env: process.env,
        adapters,
      });

      expect(result.status).toBe("success");
      if (result.status !== "success") {
        return;
      }

      expect(result.steps.map((step) => [step.stepId, step.status, step.fanoutGroup])).toEqual([
        ["market", "success", "advisors"],
        ["risk", "success", "advisors"],
        ["finance", "failure", "advisors"],
        ["synthesize", "success", undefined],
      ]);
      expect(result.steps.slice(0, 3).map((step) => step.parentReceipt)).toEqual([undefined, undefined, undefined]);
      expect(result.steps[3].stdout).toBe("go");
      expect(result.receipt.schema).toBe("runx.harness_receipt.v1");
      expect(result.receipt.harness.child_harness_receipt_refs).toHaveLength(4);
      expect(runtimeSyncPoints(result.receipt)).toEqual([
        expect.objectContaining({
          group_id: "advisors",
          strategy: "quorum",
          decision: "proceed",
          rule_fired: "quorum.min_success",
          branch_count: 3,
          success_count: 2,
          failure_count: 1,
          required_successes: 2,
          branch_receipts: result.steps.slice(0, 3).map((step) => step.receiptId),
        }),
      ]);
    } finally {
      await rm(tempDir, { recursive: true, force: true });
    }
  });

  it("pauses and resumes deterministically when a structured threshold gate fires", async () => {
    const tempDir = await mkdtemp(path.join(os.tmpdir(), "runx-fanout-threshold-"));
    const receiptDir = path.join(tempDir, "receipts");
    const runxHome = path.join(tempDir, "home");
    const graphPath = path.resolve("fixtures/graphs/fanout/threshold.yaml");
    const approvingCaller: Caller = {
      resolve: async (request) => request.kind === "approval" ? { actor: "human", payload: true } : undefined,
      report: () => undefined,
    };

    try {
      const result = await runLocalGraph({
        graphPath,
        caller: nonInteractiveCaller,
        receiptDir,
        runxHome,
        env: process.env,
        adapters,
      });

      expect(result.status).toBe("needs_resolution");
      if (result.status !== "needs_resolution") {
        return;
      }
      expect(result.state.status).toBe("paused");

      expect(result.requests).toEqual([
        expect.objectContaining({
          id: "fanout.advisors.threshold.risk.risk_score.above",
          kind: "approval",
          gate: expect.objectContaining({
            type: "fanout-gate",
            reason: "risk.risk_score=0.91 exceeded 0.8",
          }),
        }),
      ]);

      const resumed = await runLocalGraph({
        graphPath,
        caller: approvingCaller,
        receiptDir,
        runxHome,
        env: process.env,
        adapters,
        resumeFromRunId: result.runId,
      });
      expect(resumed.status).toBe("success");
      if (resumed.status !== "success") {
        return;
      }
      expect(resumed.steps.map((step) => step.stepId)).toEqual(["market", "risk", "synthesize"]);
      expect(resumed.steps.slice(0, 2).map((step) => step.fanoutGroup)).toEqual(["advisors", "advisors"]);
      expect(resumed.output).toBe("go");
      expect(resumed.receipt.schema).toBe("runx.harness_receipt.v1");
      expect(runtimeSyncPoints(resumed.receipt)).toEqual([
        expect.objectContaining({
          group_id: "advisors",
          decision: "pause",
          rule_fired: "threshold.risk.risk_score.above",
          reason: "risk.risk_score=0.91 exceeded 0.8",
          branch_receipts: resumed.steps.slice(0, 2).map((step) => step.receiptId),
        }),
      ]);
    } finally {
      await rm(tempDir, { recursive: true, force: true });
    }
  });

  it("records structured fanout conflicts as explicit escalation outcomes", async () => {
    const tempDir = await mkdtemp(path.join(os.tmpdir(), "runx-fanout-escalate-"));
    const graphPath = path.join(tempDir, "graph.yaml");
    const receiptDir = path.join(tempDir, "receipts");
    const runxHome = path.join(tempDir, "home");
    const jsonSkillPath = path.resolve("fixtures/skills/json-output");

    try {
      await writeFile(
        graphPath,
        `name: fanout-escalate
fanout:
  groups:
    advisors:
      strategy: all
      on_branch_failure: halt
      conflict_gates:
        - field: recommendation
          action: escalate
          steps:
            - market
            - risk
steps:
  - id: market
    mode: fanout
    fanout_group: advisors
    skill: ${jsonSkillPath}
    inputs:
      recommendation: ship
  - id: risk
    mode: fanout
    fanout_group: advisors
    skill: ${jsonSkillPath}
    inputs:
      recommendation: hold
`,
      );

      const result = await runLocalGraph({
        graphPath,
        caller: nonInteractiveCaller,
        receiptDir,
        runxHome,
        env: process.env,
        adapters,
      });

      expect(result.status).toBe("escalated");
      if (result.status !== "escalated") {
        return;
      }
      expect(result.receipt.schema).toBe("runx.harness_receipt.v1");
      expect(result.receipt.seal.disposition).toBe("blocked");
      expect(result.state.status).toBe("escalated");
      expect(result.errorMessage).toBe("fanout escalation: fanout branches disagreed on structured field recommendation");
      expect(runtimeSyncPoints(result.receipt)).toEqual([
        expect.objectContaining({
          group_id: "advisors",
          decision: "escalate",
          rule_fired: "conflict.recommendation",
          reason: "fanout branches disagreed on structured field recommendation",
          branch_receipts: result.steps.slice(0, 2).map((step) => step.receiptId),
        }),
      ]);
      expect(result.receipt.metadata).toMatchObject({
        runx: {
          fanout_gate: {
            status: "escalated",
            group_id: "advisors",
            decision: "escalate",
            rule_fired: "conflict.recommendation",
          },
        },
      });
    } finally {
      await rm(tempDir, { recursive: true, force: true });
    }
  });

  it("applies graph transition policy before fanout branch execution", async () => {
    const tempDir = await mkdtemp(path.join(os.tmpdir(), "runx-fanout-transition-policy-"));
    const graphPath = path.join(tempDir, "graph.yaml");
    const receiptDir = path.join(tempDir, "receipts");
    const runxHome = path.join(tempDir, "home");
    const jsonSkillPath = path.resolve("fixtures/skills/json-output");

    try {
      await writeFile(
        graphPath,
        `name: fanout-transition-policy
policy:
  transitions:
    - to: market
      field: seed.allowed
      equals: true
fanout:
  groups:
    advisors:
      strategy: all
      on_branch_failure: halt
steps:
  - id: seed
    skill: ${jsonSkillPath}
    inputs:
      allowed: false
  - id: market
    mode: fanout
    fanout_group: advisors
    skill: ${jsonSkillPath}
    inputs:
      recommendation: go
  - id: risk
    mode: fanout
    fanout_group: advisors
    skill: ${jsonSkillPath}
    inputs:
      risk_score: 0.2
`,
      );

      const result = await runLocalGraph({
        graphPath,
        caller: nonInteractiveCaller,
        receiptDir,
        runxHome,
        env: process.env,
        adapters,
      });

      expect(result.status).toBe("policy_denied");
      if (result.status !== "policy_denied") {
        return;
      }
      expect(result.reasons).toEqual([
        "transition policy blocked step 'market': expected seed.allowed == true",
      ]);
      expect(result.receipt?.schema).toBe("runx.harness_receipt.v1");
      const graphSteps = runtimeGraphSteps(result.receipt);
      expect(graphSteps.map((step) => step.step_id)).toEqual(["seed", "market"]);
      expect(graphSteps[1]).toMatchObject({
        step_id: "market",
        status: "failure",
        disposition: "policy_denied",
        fanout_group: "advisors",
      });
      expect(result.receipt?.metadata).toMatchObject({
        authority_proof: {
          skill_name: "json-output",
          requested: {
            connected_auth: false,
            scopes: [],
            mutating: false,
          },
          credential_material: {
            status: "not_requested",
          },
        },
      });
    } finally {
      await rm(tempDir, { recursive: true, force: true });
    }
  });

  it("denies mutating retry fanout branches without idempotency before adapter execution", async () => {
    const tempDir = await mkdtemp(path.join(os.tmpdir(), "runx-fanout-retry-denied-"));
    const graphPath = path.join(tempDir, "graph.yaml");
    const receiptDir = path.join(tempDir, "receipts");
    const runxHome = path.join(tempDir, "home");
    const echoSkillPath = path.resolve("fixtures/skills/echo");
    const adapter = createCountingAdapter();

    try {
      await writeFile(
        graphPath,
        `name: fanout-retry-mutating-denied
fanout:
  groups:
    advisors:
      strategy: all
      on_branch_failure: halt
steps:
  - id: deploy
    mode: fanout
    fanout_group: advisors
    skill: ${echoSkillPath}
    mutation: true
    inputs:
      message: deploy
    retry:
      max_attempts: 2
`,
      );
      ensureRustRunxBinary();

      const result = await runLocalGraph({
        graphPath,
        caller: nonInteractiveCaller,
        receiptDir,
        runxHome,
        env: kernelEnv(),
        adapters: [adapter],
      });

      expect(result.status).toBe("policy_denied");
      if (result.status !== "policy_denied") {
        return;
      }
      expect(result.reasons).toEqual(["step 'deploy' declares mutating retry without an idempotency key"]);
      expect(adapter.callCount()).toBe(0);
      expect(result.receipt?.schema).toBe("runx.harness_receipt.v1");
      const graphSteps = runtimeGraphSteps(result.receipt);
      expect(graphSteps).toMatchObject([
        {
          step_id: "deploy",
          status: "failure",
          disposition: "policy_denied",
          fanout_group: "advisors",
          receipt_id: undefined,
        },
      ]);
    } finally {
      await rm(tempDir, { recursive: true, force: true });
    }
  });

  it("records sync policy decisions on the harness receipt", async () => {
    const tempDir = await mkdtemp(path.join(os.tmpdir(), "runx-fanout-inspect-"));
    const receiptDir = path.join(tempDir, "receipts");

    try {
      const result = await runLocalGraph({
        graphPath: path.resolve("fixtures/graphs/fanout/graph.yaml"),
        caller: nonInteractiveCaller,
        receiptDir,
        runxHome: path.join(tempDir, "home"),
        env: process.env,
        adapters,
      });
      expect(result.status).toBe("success");
      if (result.status !== "success") {
        return;
      }

      expect(runtimeSyncPoints(result.receipt)).toEqual([
        expect.objectContaining({
          group_id: "advisors",
          decision: "proceed",
          rule_fired: "quorum.min_success",
          reason: "2/3 branches succeeded; required 2",
        }),
      ]);
    } finally {
      await rm(tempDir, { recursive: true, force: true });
    }
  });
});

function kernelEnv(): NodeJS.ProcessEnv {
  return {
    ...process.env,
    RUNX_KERNEL_EVAL_BIN: runxBinary,
  };
}

function ensureRustRunxBinary(): void {
  const result = spawnSync(
    cargo,
    ["build", "--quiet", "--manifest-path", "crates/Cargo.toml", "-p", "runx-cli", "--bin", "runx"],
    {
      cwd: workspaceRoot,
      encoding: "utf8",
      env: process.env,
      maxBuffer: 8 * 1024 * 1024,
    },
  );

  expect(result.status, result.stderr || result.stdout).toBe(0);
}

function runtimeSyncPoints(receipt: ({ readonly metadata?: Readonly<Record<string, unknown>> } & { readonly sync_points?: unknown }) | undefined): readonly unknown[] {
  const syncPoints = receipt?.sync_points;
  expect(Array.isArray(syncPoints)).toBe(true);
  return syncPoints as readonly unknown[];
}

interface RuntimeGraphStep {
  readonly step_id: string;
  readonly status?: string;
  readonly disposition?: string;
  readonly receipt_id?: string;
  readonly fanout_group?: string;
}

function runtimeGraphSteps(receipt: { readonly metadata?: Readonly<Record<string, unknown>> } | undefined): readonly RuntimeGraphStep[] {
  const runx = receipt?.metadata?.runx;
  expect(runx).toEqual(expect.any(Object));
  const steps = (runx as { readonly steps?: unknown } | undefined)?.steps;
  expect(Array.isArray(steps)).toBe(true);
  return steps as readonly RuntimeGraphStep[];
}

function createCountingAdapter(): SkillAdapter & { callCount: () => number } {
  let calls = 0;
  return {
    type: "cli-tool",
    callCount: () => calls,
    invoke: async (request) => {
      calls += 1;
      return {
        status: "success",
        stdout: String(request.inputs.message ?? "ok"),
        stderr: "",
        exitCode: 0,
        signal: null,
        durationMs: 1,
      };
    },
  };
}

async function writeSleepSkill(directory: string, label: string): Promise<void> {
  await mkdir(directory, { recursive: true });
  await writeFile(
    path.join(directory, "SKILL.md"),
    `---
name: ${label}
description: Sleep for one second and then emit the skill label.
source:
  type: cli-tool
  command: node
  args:
    - -e
    - "setTimeout(() => process.stdout.write('${label}'), 1000)"
  timeout_seconds: 5
inputs: {}
---

Emit ${label} after a one-second delay.
`,
  );
}
