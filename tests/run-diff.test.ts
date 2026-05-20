import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";

import { describe, expect, it } from "vitest";

import { diffLocalRuns, runLocalSkill, type Caller, type RunLineageMetadata, type SkillAdapter } from "@runxhq/runtime-local";
import { runCli } from "../packages/cli/src/index.js";

const caller: Caller = {
  resolve: async () => undefined,
  report: () => undefined,
};

describe("run diff", () => {
  it("diffs receipt and ledger summaries without a second state store", async () => {
    const tempDir = await mkdtemp(path.join(os.tmpdir(), "runx-run-diff-"));
    const receiptDir = path.join(tempDir, "receipts");
    const runxHome = path.join(tempDir, "home");

    try {
      const left = await seedReceipt({
        tempDir,
        receiptDir,
        runxHome,
        skillName: "sourcey",
        artifactType: "docs_site",
        runnerProvider: "openai",
      });
      const right = await seedReceipt({
        tempDir,
        receiptDir,
        runxHome,
        skillName: "sourcey",
        artifactType: "review_note",
        runnerProvider: "anthropic",
        approvalDecision: "approved",
        lineage: {
          kind: "rerun",
          sourceRunId: left,
          sourceReceiptId: left,
        },
      });

      await expect(diffLocalRuns({
        left,
        right,
        receiptDir,
        runxHome,
      })).resolves.toMatchObject({
        changed: true,
        fields: {
          runner_provider: {
            left: "openai",
            right: "anthropic",
          },
          approval: {
            right: {
              decision: "approved",
            },
          },
          lineage: {
            right: {
              kind: "rerun",
              sourceRunId: left,
            },
          },
        },
        artifactTypes: {
          added: ["review_note"],
          removed: ["docs_site"],
        },
      });

      const stdout = createMemoryStream();
      const exit = await runCli(
        ["diff", left, right, "--receipt-dir", receiptDir, "--json"],
        { stdin: process.stdin, stdout, stderr: createMemoryStream() },
        {
          ...process.env,
          RUNX_CWD: process.cwd(),
          RUNX_HOME: runxHome,
        },
      );
      expect(exit).toBe(0);
      expect(JSON.parse(stdout.contents())).toMatchObject({
        status: "success",
        diff: {
          changed: true,
        },
      });
    } finally {
      await rm(tempDir, { recursive: true, force: true });
    }
  });
});

async function seedReceipt(options: {
  readonly tempDir: string;
  readonly receiptDir: string;
  readonly runxHome: string;
  readonly skillName: string;
  readonly artifactType: string;
  readonly runnerProvider: string;
  readonly approvalDecision?: "approved" | "denied";
  readonly lineage?: RunLineageMetadata;
}): Promise<string> {
  const skillDir = path.join(options.tempDir, "skills", `${options.skillName}-${options.artifactType}`);
  await mkdir(skillDir, { recursive: true });
  await writeFile(
    path.join(skillDir, "SKILL.md"),
    `---
name: ${options.skillName}
description: Test run diff projections.
source:
  type: agent-step
  agent: codex
  task: ${options.artifactType}
inputs: {}
runx:
  artifacts:
    wrap_as: ${options.artifactType}
---
Emit a diff projection artifact.
`,
  );
  const adapter: SkillAdapter = {
    type: "agent-step",
    invoke: async () => ({
      status: "success",
      stdout: JSON.stringify({ ok: true }),
      stderr: "",
      exitCode: 0,
      signal: null,
      durationMs: 3,
      metadata: {
        runner: {
          provider: options.runnerProvider,
        },
        approval: options.approvalDecision
          ? {
              gate_id: `${options.skillName}.approval`,
              gate_type: "human",
              decision: options.approvalDecision,
              reason: "reviewed",
            }
          : undefined,
        runx: options.lineage
          ? {
              lineage: options.lineage,
            }
          : undefined,
      },
    }),
  };
  const result = await runLocalSkill({
    skillPath: skillDir,
    inputs: { project: "." },
    caller,
    adapters: [adapter],
    receiptDir: options.receiptDir,
    runxHome: options.runxHome,
    lineage: options.lineage,
    env: {
      ...process.env,
      RUNX_CWD: options.tempDir,
    },
  });
  expect(result.status).toBe("success");
  if (result.status !== "success") {
    throw new Error("diff seed run failed");
  }
  return result.receipt.id;
}

function createMemoryStream(): NodeJS.WriteStream & { contents: () => string } {
  let contents = "";
  return {
    write(chunk: unknown) {
      contents += String(chunk);
      return true;
    },
    contents: () => contents,
  } as NodeJS.WriteStream & { contents: () => string };
}
