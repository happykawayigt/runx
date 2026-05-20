import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";

import { describe, expect, it } from "vitest";

import { runCli } from "../packages/cli/src/index.js";
import { createFileKnowledgeStore } from "@runxhq/core/knowledge";
import { inspectLocalReceipt, listLocalHistory, runLocalSkill, type Caller, type SkillAdapter } from "@runxhq/runtime-local";

const caller: Caller = {
  resolve: async () => undefined,
  report: () => undefined,
};

describe("history, inspect, and knowledge CLI", () => {
  it("uses receipt files for history/inspect and knowledge for project projections", async () => {
    const tempDir = await mkdtemp(path.join(os.tmpdir(), "runx-history-inspect-"));
    const receiptDir = path.join(tempDir, "receipts");
    const knowledgeDir = path.join(tempDir, "knowledge");
    const project = path.join(tempDir, "project");

    try {
      const runStdout = createMemoryStream();
      const runStderr = createMemoryStream();
      const runExit = await runCli(
        [
          "skill",
          "fixtures/skills/echo",
          "--message",
          "hi",
          "--receipt-dir",
          receiptDir,
          "--json",
        ],
        { stdin: process.stdin, stdout: runStdout, stderr: runStderr },
        {
          ...process.env,
          RUNX_CWD: process.cwd(),
        },
      );
      expect(runExit).toBe(0);
      expect(runStderr.contents()).toBe("");
      const runReport = JSON.parse(runStdout.contents()) as { receipt: { id: string } };

      const historyStdout = createMemoryStream();
      const historyExit = await runCli(
        ["history", "echo", "--receipt-dir", receiptDir, "--json"],
        { stdin: process.stdin, stdout: historyStdout, stderr: createMemoryStream() },
        {
          ...process.env,
          RUNX_CWD: process.cwd(),
        },
      );
      expect(historyExit).toBe(0);
      expect(JSON.parse(historyStdout.contents())).toMatchObject({
        status: "success",
        query: "echo",
        receipts: [
          {
            id: runReport.receipt.id,
            name: "echo",
            sourceType: "cli-tool",
          },
        ],
      });

      const inspectStdout = createMemoryStream();
      const inspectExit = await runCli(
        ["skill", "inspect", runReport.receipt.id, "--receipt-dir", receiptDir, "--json"],
        { stdin: process.stdin, stdout: inspectStdout, stderr: createMemoryStream() },
        {
          ...process.env,
          RUNX_CWD: process.cwd(),
        },
      );
      expect(inspectExit).toBe(0);
      expect(JSON.parse(inspectStdout.contents())).toMatchObject({
        summary: {
          id: runReport.receipt.id,
          name: "echo",
        },
      });

      await createFileKnowledgeStore(knowledgeDir).addProjection({
        project,
        scope: "project",
        key: "homepage_url",
        value: "https://example.test",
        source: "test",
        confidence: 0.95,
        freshness: "fresh",
        receiptId: runReport.receipt.id,
        createdAt: "2026-04-10T00:00:00Z",
      });

      const knowledgeStdout = createMemoryStream();
      const knowledgeExit = await runCli(
        ["knowledge", "show", "--project", project, "--json"],
        { stdin: process.stdin, stdout: knowledgeStdout, stderr: createMemoryStream() },
        {
          ...process.env,
          RUNX_KNOWLEDGE_DIR: knowledgeDir,
          RUNX_CWD: process.cwd(),
        },
      );
      expect(knowledgeExit).toBe(0);
      expect(JSON.parse(knowledgeStdout.contents())).toMatchObject({
        status: "success",
        project,
        projections: [
          {
            key: "homepage_url",
            value: "https://example.test",
            receipt_id: runReport.receipt.id,
          },
        ],
      });
    } finally {
      await rm(tempDir, { recursive: true, force: true });
    }
  });

  it("filters local history by actor and artifact type and exposes the same summary through inspect", async () => {
    const tempDir = await mkdtemp(path.join(os.tmpdir(), "runx-history-filters-"));
    const receiptDir = path.join(tempDir, "receipts");
    const runxHome = path.join(tempDir, "home");

    try {
      const builderReceiptId = await seedHistoryRun({
        tempDir,
        receiptDir,
        runxHome,
        name: "draft-content",
        artifactType: "draft_pull_request",
        inputs: { objective: "draft a pull request" },
        metadata: {
          agent_hook: {
            source_type: "agent-step",
            agent: "builder",
            task: "draft-pr",
            route: "provided",
            status: "success",
          },
          runner: {
            provider: "openai",
          },
        },
      });

      const reviewerReceiptId = await seedHistoryRun({
        tempDir,
        receiptDir,
        runxHome,
        name: "issue-intake",
        artifactType: "issue_intake_packet",
        inputs: { thread: "support request" },
        metadata: {
          runner: {
            provider: "anthropic",
          },
        },
      });

      await expect(listLocalHistory({ receiptDir, runxHome, actor: "builder" })).resolves.toMatchObject({
        receipts: [
          {
            id: builderReceiptId,
            actors: ["builder", "openai"],
            artifactTypes: ["draft_pull_request"],
          },
        ],
      });

      await expect(listLocalHistory({ receiptDir, runxHome, artifactType: "issue_intake_packet" })).resolves.toMatchObject({
        receipts: [
          {
            id: reviewerReceiptId,
            artifactTypes: ["issue_intake_packet"],
          },
        ],
      });

      await expect(inspectLocalReceipt({ receiptDir, runxHome, receiptId: builderReceiptId })).resolves.toMatchObject({
        summary: {
          id: builderReceiptId,
          actors: ["builder", "openai"],
          artifactTypes: ["draft_pull_request"],
        },
      });

      const historyStdout = createMemoryStream();
      const historyExit = await runCli(
        ["history", "--actor", "builder", "--artifact-type", "draft_pull_request", "--receipt-dir", receiptDir, "--json"],
        { stdin: process.stdin, stdout: historyStdout, stderr: createMemoryStream() },
        {
          ...process.env,
          RUNX_CWD: process.cwd(),
          RUNX_HOME: runxHome,
        },
      );
      expect(historyExit).toBe(0);
      expect(JSON.parse(historyStdout.contents())).toMatchObject({
        status: "success",
        filters: {
          actor: "builder",
          artifact_type: "draft_pull_request",
        },
        receipts: [
          {
            id: builderReceiptId,
            actors: ["builder", "openai"],
            artifactTypes: ["draft_pull_request"],
          },
        ],
      });
    } finally {
      await rm(tempDir, { recursive: true, force: true });
    }
  });
});

async function seedHistoryRun(options: {
  readonly tempDir: string;
  readonly receiptDir: string;
  readonly runxHome: string;
  readonly name: string;
  readonly artifactType: string;
  readonly inputs: Readonly<Record<string, unknown>>;
  readonly metadata: Readonly<Record<string, unknown>>;
}): Promise<string> {
  const skillDir = path.join(options.tempDir, "skills", options.name);
  await mkdir(skillDir, { recursive: true });
  await writeFile(
    path.join(skillDir, "SKILL.md"),
    `---
name: ${options.name}
description: Test history projections.
source:
  type: agent-step
  agent: codex
  task: ${options.name}
inputs: {}
runx:
  artifacts:
    wrap_as: ${options.artifactType}
---
Emit a history projection artifact.
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
      durationMs: 2,
      metadata: options.metadata,
    }),
  };
  const result = await runLocalSkill({
    skillPath: skillDir,
    inputs: options.inputs,
    caller,
    adapters: [adapter],
    receiptDir: options.receiptDir,
    runxHome: options.runxHome,
    env: {
      ...process.env,
      RUNX_CWD: options.tempDir,
    },
  });

  expect(result.status).toBe("success");
  if (result.status !== "success") {
    throw new Error("history seed run failed");
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
