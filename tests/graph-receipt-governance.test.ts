import { spawnSync } from "node:child_process";
import { mkdtemp, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";

import { describe, expect, it } from "vitest";

import { createDefaultSkillAdapters } from "@runxhq/adapters";
import { runLocalGraph, type Caller } from "@runxhq/runtime-local";

const caller: Caller = {
  resolve: async () => undefined,
  report: () => undefined,
};
const workspaceRoot = process.cwd();
const cargo = process.platform === "win32" ? "cargo.exe" : "cargo";
const runxBinary = path.join(
  workspaceRoot,
  "crates",
  "target",
  "debug",
  process.platform === "win32" ? "runx.exe" : "runx",
);
let rustRunxBuilt = false;

describe("graph receipt governance metadata", () => {
  it("records runner and allowed scope admission in graph and step receipts", async () => {
    const tempDir = await mkdtemp(path.join(os.tmpdir(), "runx-graph-receipt-governance-"));
    const receiptDir = path.join(tempDir, "receipts");

    try {
      await writeGovernedSkill(path.join(tempDir, "skills", "governed-echo"));
      const graphPath = path.join(tempDir, "graph.yaml");
      await writeFile(
        graphPath,
        `name: graph-receipt-governance
steps:
  - id: echo
    skill: ./skills/governed-echo
    runner: governed-echo-cli
    mutation: true
    scopes:
      - repo:read
    inputs:
      message: scoped ok
`,
      );
      ensureRustRunxBinary();

      const result = await runLocalGraph({
        graphPath,
        caller,
        receiptDir,
        runxHome: path.join(tempDir, "home"),
        env: kernelEnv(),
        graphGrant: {
          grant_id: "grant_repo",
          scopes: ["repo:*"],
        },
        adapters: createDefaultSkillAdapters(),
      });

      expect(result.status).toBe("success");
      if (result.status !== "success") {
        return;
      }

      expect(result.receipt.schema).toBe("runx.harness_receipt.v1");
      expect(result.receipt.harness.child_harness_receipt_refs).toHaveLength(1);
      expect(result.steps[0]).toMatchObject({
        runner: "governed-echo-cli",
        governance: {
          scopeAdmission: {
            status: "allow",
            requestedScopes: ["repo:read"],
            grantedScopes: ["repo:*"],
            grantId: "grant_repo",
          },
        },
      });

      const stepReceipt = JSON.parse(await readFile(path.join(receiptDir, `${result.steps[0].receiptId}.json`), "utf8")) as {
        metadata?: Record<string, unknown>;
      };
      expect(stepReceipt.metadata).toMatchObject({
        authority_proof: {
          requested: {
            mutating: true,
          },
          scope_admission: {
            status: "allow",
            requested_scopes: ["repo:read"],
            granted_scopes: ["repo:*"],
            grant_id: "grant_repo",
          },
        },
        graph_governance: {
          step_id: "echo",
          selected_runner: "governed-echo-cli",
          scope_admission: {
            status: "allow",
            requested_scopes: ["repo:read"],
            granted_scopes: ["repo:*"],
            grant_id: "grant_repo",
          },
        },
      });
    } finally {
      await rm(tempDir, { recursive: true, force: true });
    }
  });

  it("records denied scope admission in the graph receipt without a step receipt", async () => {
    const tempDir = await mkdtemp(path.join(os.tmpdir(), "runx-graph-receipt-denied-"));
    const receiptDir = path.join(tempDir, "receipts");

    try {
      await writeGovernedSkill(path.join(tempDir, "skills", "governed-echo"));
      const graphPath = path.join(tempDir, "graph.yaml");
      await writeFile(
        graphPath,
        `name: graph-receipt-denied
steps:
  - id: deploy
    skill: ./skills/governed-echo
    runner: governed-echo-cli
    scopes:
      - deployments:write
    inputs:
      message: denied
`,
      );
      ensureRustRunxBinary();

      const result = await runLocalGraph({
        graphPath,
        caller,
        receiptDir,
        runxHome: path.join(tempDir, "home"),
        env: kernelEnv(),
        graphGrant: {
          grant_id: "grant_repo",
          scopes: ["repo:read"],
        },
        adapters: createDefaultSkillAdapters(),
      });

      expect(result.status).toBe("policy_denied");
      if (result.status !== "policy_denied") {
        return;
      }

      expect(result.receipt?.schema).toBe("runx.harness_receipt.v1");
      expect(result.receipt?.seal.disposition).toBe("declined");
      expect(runtimeGraphSteps(result.receipt)).toMatchObject([
          {
            step_id: "deploy",
            runner: "governed-echo-cli",
            status: "failure",
            receipt_id: undefined,
            governance: {
              scope_admission: {
                status: "deny",
                requested_scopes: ["deployments:write"],
                granted_scopes: ["repo:read"],
                grant_id: "grant_repo",
                reasons: ["step 'deploy' requested scope(s) outside graph grant: deployments:write"],
              },
            },
          },
      ]);
    } finally {
      await rm(tempDir, { recursive: true, force: true });
    }
  });

  it("carries fanout step mutating authority into branch receipt proofs", async () => {
    const tempDir = await mkdtemp(path.join(os.tmpdir(), "runx-graph-fanout-authority-"));
    const receiptDir = path.join(tempDir, "receipts");

    try {
      await Promise.all([
        writeGovernedSkill(path.join(tempDir, "skills", "left")),
        writeGovernedSkill(path.join(tempDir, "skills", "right")),
      ]);
      const graphPath = path.join(tempDir, "graph.yaml");
      await writeFile(
        graphPath,
        `name: graph-fanout-authority
fanout:
  groups:
    branches:
      strategy: all
steps:
  - id: left
    mode: fanout
    fanout_group: branches
    skill: ./skills/left
    runner: governed-echo-cli
    mutation: true
    inputs:
      message: left
  - id: right
    mode: fanout
    fanout_group: branches
    skill: ./skills/right
    runner: governed-echo-cli
    inputs:
      message: right
`,
      );

      const result = await runLocalGraph({
        graphPath,
        caller,
        receiptDir,
        runxHome: path.join(tempDir, "home"),
        env: process.env,
        adapters: createDefaultSkillAdapters(),
      });

      expect(result.status).toBe("success");
      if (result.status !== "success") {
        return;
      }

      const left = result.steps.find((step) => step.stepId === "left");
      expect(left?.receiptId).toEqual(expect.any(String));
      const stepReceipt = JSON.parse(await readFile(path.join(receiptDir, `${left?.receiptId}.json`), "utf8")) as {
        metadata?: Record<string, unknown>;
      };
      expect(stepReceipt.metadata).toMatchObject({
        authority_proof: {
          requested: {
            mutating: true,
          },
        },
      });
    } finally {
      await rm(tempDir, { recursive: true, force: true });
    }
  });
});

async function writeGovernedSkill(skillDir: string): Promise<void> {
  await mkdir(skillDir, { recursive: true });
  await mkdir(path.join(skillDir, ".runx"), { recursive: true });
  await writeFile(
    path.join(skillDir, "SKILL.md"),
    `---
name: governed-echo
description: Portable governed echo.
---

Echo a message.
`,
  );
  const profileDocument = `skill: governed-echo
runners:
  governed-echo-cli:
    type: cli-tool
    command: node
    args:
      - -e
      - "process.stdout.write(process.env.RUNX_INPUT_MESSAGE || '')"
    inputs:
      message:
        type: string
        required: true
`;
  await writeFile(
    path.join(skillDir, ".runx/profile.json"),
    `${JSON.stringify(
      {
        schema_version: "runx.skill-profile.v1",
        skill: {
          name: "governed-echo",
          path: "SKILL.md",
          digest: "fixture-skill-digest",
        },
        profile: {
          document: profileDocument,
          digest: "fixture-profile-digest",
          runner_names: ["governed-echo-cli"],
        },
        origin: {
          source: "fixture",
        },
      },
      null,
      2,
    )}\n`,
  );
}

interface RuntimeGraphStep {
  readonly runner?: string;
  readonly governance?: unknown;
}

function runtimeGraphSteps(receipt: { readonly metadata?: Readonly<Record<string, unknown>> } | undefined): readonly RuntimeGraphStep[] {
  const runx = receipt?.metadata?.runx;
  expect(runx).toEqual(expect.any(Object));
  const steps = (runx as { readonly steps?: unknown } | undefined)?.steps;
  expect(Array.isArray(steps)).toBe(true);
  return steps as readonly RuntimeGraphStep[];
}

function kernelEnv(): NodeJS.ProcessEnv {
  return {
    ...process.env,
    RUNX_KERNEL_EVAL_BIN: runxBinary,
  };
}

function ensureRustRunxBinary(): void {
  if (rustRunxBuilt) {
    return;
  }
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
  rustRunxBuilt = true;
}
