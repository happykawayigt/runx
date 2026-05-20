import { spawnSync } from "node:child_process";
import { mkdtemp, mkdir, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";

import { describe, expect, it } from "vitest";

import { createA2aAdapter } from "../packages/adapters/src/a2a/index.js";
import { createDefaultLocalSkillRuntime } from "@runxhq/adapters/runtime";
import { createA2aFixtureTransport } from "@runxhq/runtime-local/harness";
import { runLocalGraph, type Caller, type SkillAdapter } from "@runxhq/runtime-local";

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

describe("governed graph runner governance", () => {
  it("selects a named cli-tool binding runner from a graph step", async () => {
    const tempDir = await mkdtemp(path.join(os.tmpdir(), "runx-composite-runner-cli-"));

    try {
      const skillDir = path.join(tempDir, "skills", "package-echo");
      await writePackageEchoSkill(skillDir);
      const graphPath = path.join(tempDir, "graph.yaml");
      const runtime = await createDefaultLocalSkillRuntime({
        root: tempDir,
        receiptDir: path.join(tempDir, "receipts"),
        runxHome: path.join(tempDir, "home"),
        env: process.env,
      });
      await writeFile(
        graphPath,
        `name: graph-runner-cli
steps:
  - id: echo
    skill: ./skills/package-echo
    runner: package-echo-cli
    inputs:
      message: selected runner
`,
      );

      const result = await runLocalGraph({
        graphPath,
        caller,
        adapters: runtime.adapters,
        receiptDir: runtime.paths.receiptDir,
        runxHome: runtime.paths.runxHome,
        env: runtime.env,
      });

      expect(result.status).toBe("success");
      if (result.status !== "success") {
        return;
      }
      expect(result.steps[0]).toMatchObject({
        runner: "package-echo-cli",
        stdout: "selected runner",
      });
      expect(result.receipt.schema).toBe("runx.harness_receipt.v1");
      expect(result.receipt.harness.child_harness_receipt_refs).toHaveLength(1);
      expect(result.steps[0]).toMatchObject({
        runner: "package-echo-cli",
        governance: {
          scopeAdmission: {
            status: "allow",
            requestedScopes: [],
            grantedScopes: ["*"],
            grantId: "local-default",
          },
        },
      });
    } finally {
      await rm(tempDir, { recursive: true, force: true });
    }
  });

  it("selects an A2A binding runner from a graph step", async () => {
    const tempDir = await mkdtemp(path.join(os.tmpdir(), "runx-composite-runner-a2a-"));
    const graphPath = path.join(tempDir, "graph.yaml");

    try {
      const runtime = await createDefaultLocalSkillRuntime({
        root: tempDir,
        receiptDir: path.join(tempDir, "receipts"),
        runxHome: path.join(tempDir, "home"),
        env: process.env,
        adapters: [createA2aAdapter({ transport: createA2aFixtureTransport() })],
      });
      await writeFile(
        graphPath,
        `name: graph-runner-a2a
steps:
  - id: echo
    skill: ${path.resolve("fixtures/skills/a2a-echo")}
    runner: fixture-a2a
    inputs:
      message: hi from graph
`,
      );

      const result = await runLocalGraph({
        graphPath,
        caller,
        adapters: runtime.adapters,
        receiptDir: runtime.paths.receiptDir,
        runxHome: runtime.paths.runxHome,
        env: runtime.env,
      });

      expect(result.status).toBe("success");
      if (result.status !== "success") {
        return;
      }
      expect(result.steps[0]).toMatchObject({
        runner: "fixture-a2a",
        stdout: "hi from graph",
      });
      expect(result.receipt.harness.child_harness_receipt_refs).toHaveLength(1);
      expect(result.steps[0]?.runner).toBe("fixture-a2a");
    } finally {
      await rm(tempDir, { recursive: true, force: true });
    }
  });

  it("denies step scopes that exceed the parent graph grant before execution", async () => {
    const tempDir = await mkdtemp(path.join(os.tmpdir(), "runx-graph-scope-deny-"));
    const adapter = createCountingAdapter();

    try {
      const skillDir = path.join(tempDir, "skills", "package-echo");
      await writePackageEchoSkill(skillDir);
      const graphPath = path.join(tempDir, "graph.yaml");
      await writeFile(
        graphPath,
        `name: graph-scope-deny
steps:
  - id: deploy
    skill: ./skills/package-echo
    runner: package-echo-cli
    scopes:
      - deployments:write
    inputs:
      message: should not run
`,
      );
      ensureRustRunxBinary();

      const result = await runLocalGraph({
        graphPath,
        caller,
        receiptDir: path.join(tempDir, "receipts"),
        runxHome: path.join(tempDir, "home"),
        env: kernelEnv(),
        adapters: [adapter],
        graphGrant: {
          grant_id: "grant_checks",
          scopes: ["checks:read"],
        },
      });

      expect(result.status).toBe("policy_denied");
      if (result.status !== "policy_denied") {
        return;
      }
      expect(result.reasons).toEqual(["step 'deploy' requested scope(s) outside graph grant: deployments:write"]);
      expect(adapter.callCount()).toBe(0);
      expect(result.receipt?.schema).toBe("runx.harness_receipt.v1");
      expect(result.receipt?.seal.disposition).toBe("declined");
      expect(runtimeMetadata(result.receipt)).toMatchObject({
        disposition: "policy_denied",
        outcome_state: "complete",
      });
      expect(runtimeGraphSteps(result.receipt)[0]).toMatchObject({
        step_id: "deploy",
        runner: "package-echo-cli",
        status: "failure",
        disposition: "policy_denied",
        outcome_state: "complete",
        governance: {
          scope_admission: {
            status: "deny",
            requested_scopes: ["deployments:write"],
            granted_scopes: ["checks:read"],
            grant_id: "grant_checks",
          },
        },
      });
      expect(runtimeGraphSteps(result.receipt)[0]?.receipt_id).toBeUndefined();
    } finally {
      await rm(tempDir, { recursive: true, force: true });
    }
  });

  it("fails closed with a signed receipt when scoped admission cannot reach the Rust kernel", async () => {
    const tempDir = await mkdtemp(path.join(os.tmpdir(), "runx-graph-scope-bridge-missing-"));
    const adapter = createCountingAdapter();

    try {
      const skillDir = path.join(tempDir, "skills", "package-echo");
      await writePackageEchoSkill(skillDir);
      const graphPath = path.join(tempDir, "graph.yaml");
      await writeFile(
        graphPath,
        `name: graph-scope-bridge-missing
steps:
  - id: read
    skill: ./skills/package-echo
    runner: package-echo-cli
    scopes:
      - repo:read
    inputs:
      message: should not run
`,
      );

      const result = await runLocalGraph({
        graphPath,
        caller,
        receiptDir: path.join(tempDir, "receipts"),
        runxHome: path.join(tempDir, "home"),
        env: { ...process.env, RUNX_KERNEL_EVAL_BIN: "" },
        adapters: [adapter],
        graphGrant: {
          grant_id: "grant_repo",
          scopes: ["repo:*"],
        },
      });

      expect(result.status).toBe("policy_denied");
      if (result.status !== "policy_denied") {
        return;
      }
      expect(result.reasons).toEqual([
        "graph step scope admission failed closed: Rust kernel eval requires RUNX_KERNEL_EVAL_BIN or an explicit command.",
      ]);
      expect(adapter.callCount()).toBe(0);
      expect(result.receipt?.schema).toBe("runx.harness_receipt.v1");
      expect(result.receipt?.seal.disposition).toBe("declined");
    } finally {
      await rm(tempDir, { recursive: true, force: true });
    }
  });
});

async function writePackageEchoSkill(skillDir: string): Promise<void> {
  await mkdir(skillDir, { recursive: true });
  await mkdir(path.join(skillDir, ".runx"), { recursive: true });
  await writeFile(
    path.join(skillDir, "SKILL.md"),
    `---
name: package-echo
description: Portable package echo.
---

Echo a message.
`,
  );
  const profileDocument = `skill: package-echo
runners:
  package-echo-cli:
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
          name: "package-echo",
          path: "SKILL.md",
          digest: "fixture-skill-digest",
        },
        profile: {
          document: profileDocument,
          digest: "fixture-profile-digest",
          runner_names: ["package-echo-cli"],
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
  readonly receipt_id?: string;
  readonly governance?: unknown;
}

function runtimeMetadata(receipt: { readonly metadata?: Readonly<Record<string, unknown>> } | undefined): Record<string, unknown> {
  const runx = receipt?.metadata?.runx;
  expect(runx).toEqual(expect.any(Object));
  return runx as Record<string, unknown>;
}

function runtimeGraphSteps(receipt: { readonly metadata?: Readonly<Record<string, unknown>> } | undefined): readonly RuntimeGraphStep[] {
  const steps = runtimeMetadata(receipt).steps;
  expect(Array.isArray(steps)).toBe(true);
  return steps as readonly RuntimeGraphStep[];
}

function createCountingAdapter(): SkillAdapter & { callCount: () => number } {
  let calls = 0;
  return {
    type: "cli-tool",
    callCount: () => calls,
    invoke: async () => {
      calls += 1;
      return {
        status: "success",
        stdout: "called",
        stderr: "",
        exitCode: 0,
        signal: null,
        durationMs: 1,
      };
    },
  };
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
