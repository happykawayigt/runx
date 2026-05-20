import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";

import { describe, expect, it } from "vitest";

import { createDefaultSkillAdapters } from "@runxhq/adapters";
import { runLocalSkill, type Caller } from "@runxhq/runtime-local";

const caller: Caller = {
  resolve: async () => undefined,
  report: () => undefined,
};

describe("manifest-agnostic runtime semantics", () => {
  it("supports direct caller semantics", async () => {
    const tempDir = await mkdtemp(path.join(os.tmpdir(), "runx-direct-semantics-"));
    const receiptDir = path.join(tempDir, "receipts");

    try {
      const result = await runLocalSkill({
        skillPath: path.resolve("fixtures/skills/echo"),
        inputs: {
          message: "x".repeat(512),
        },
        caller,
        adapters: createDefaultSkillAdapters(),
        receiptDir,
        runxHome: path.join(tempDir, "home"),
        env: process.env,
        executionSemantics: {
          disposition: "observing",
          outcome_state: "pending",
          input_context: {
            capture: true,
            max_bytes: 64,
          },
          surface_refs: [{ type: "issue", uri: "github://owner/repo/issues/99" }],
        },
      });

      expect(result.status).toBe("success");
      if (result.status !== "success") {
        return;
      }

      expect(result.receipt.schema).toBe("runx.harness_receipt.v1");
      expect(result.receipt.seal.disposition).toBe("deferred");
      expect(result.receipt.harness.acts[0]?.surface_refs).toMatchObject([
        { type: "github_issue", uri: "github://owner/repo/issues/99" },
      ]);
    } finally {
      await rm(tempDir, { recursive: true, force: true });
    }
  });

  it("lets a manifest project optional execution hints into the same runtime contract", async () => {
    const tempDir = await mkdtemp(path.join(os.tmpdir(), "runprofiled-semantics-"));

    try {
      const skillDir = path.join(tempDir, "manifest-skill");
      const fixtureMarkdown = await readFile(path.resolve("fixtures/runtime-semantics/manifest-skill.md"), "utf8");
      await mkdir(skillDir, { recursive: true });
      await writeFile(path.join(skillDir, "SKILL.md"), fixtureMarkdown);
      const result = await runLocalSkill({
        skillPath: skillDir,
        inputs: {
          message: "manifest-driven",
        },
        caller,
        adapters: createDefaultSkillAdapters(),
        receiptDir: path.join(tempDir, "receipts"),
        runxHome: path.join(tempDir, "home"),
        env: process.env,
      });

      expect(result.status).toBe("success");
      if (result.status !== "success") {
        return;
      }

      expect(result.receipt.schema).toBe("runx.harness_receipt.v1");
      expect(result.receipt.seal.disposition).toBe("deferred");
      expect(result.receipt.harness.acts[0]?.surface_refs).toMatchObject([
        { type: "github_issue", uri: "github://owner/repo/issues/77" },
      ]);
    } finally {
      await rm(tempDir, { recursive: true, force: true });
    }
  });

  it("converges manifest-driven and direct-caller semantics on the same receipt model", async () => {
    const tempDir = await mkdtemp(path.join(os.tmpdir(), "runx-semantics-converge-"));

    try {
      const skillDir = path.join(tempDir, "manifest-skill");
      const fixtureMarkdown = await readFile(path.resolve("fixtures/runtime-semantics/manifest-skill.md"), "utf8");
      await mkdir(skillDir, { recursive: true });
      await writeFile(path.join(skillDir, "SKILL.md"), fixtureMarkdown);

      const [manifestResult, directResult] = await Promise.all([
        runLocalSkill({
          skillPath: skillDir,
          inputs: { message: "same-shape" },
          caller,
          adapters: createDefaultSkillAdapters(),
          receiptDir: path.join(tempDir, "manifest-receipts"),
          runxHome: path.join(tempDir, "manifest-home"),
          env: process.env,
        }),
        runLocalSkill({
          skillPath: path.resolve("fixtures/skills/echo"),
          inputs: { message: "same-shape" },
          caller,
          adapters: createDefaultSkillAdapters(),
          receiptDir: path.join(tempDir, "direct-receipts"),
          runxHome: path.join(tempDir, "direct-home"),
          env: process.env,
          executionSemantics: {
            disposition: "observing",
            outcome_state: "pending",
            input_context: {
              capture: true,
              max_bytes: 128,
            },
            surface_refs: [{ type: "issue", uri: "github://owner/repo/issues/77" }],
          },
        }),
      ]);

      expect(manifestResult.status).toBe("success");
      expect(directResult.status).toBe("success");
      if (
        manifestResult.status !== "success" ||
        directResult.status !== "success" ||
        !("receipt" in manifestResult) ||
        !("receipt" in directResult)
      ) {
        return;
      }

      const summarize = (receipt: typeof manifestResult.receipt) => ({
        schema: receipt.schema,
        seal_disposition: receipt.seal.disposition,
        surface_refs: receipt.harness.acts[0]?.surface_refs,
      });

      expect(summarize(manifestResult.receipt)).toEqual(summarize(directResult.receipt));
    } finally {
      await rm(tempDir, { recursive: true, force: true });
    }
  });
});
