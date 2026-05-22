import { mkdtemp, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";

import { describe, expect, it } from "vitest";

import { hashString } from "@runxhq/core/util";

import type { AdapterActInvocation, SkillAdapter } from "./adapter-types.js";
import { runLocalSkill } from "./index.js";

describe("voice profile injection", () => {
  it("injects voice_profile separately from project context and pins receipt metadata", async () => {
    const tempDir = await mkdtemp(path.join(os.tmpdir(), "runx-voice-profile-"));
    const receiptDir = path.join(tempDir, "receipts");
    const runxHome = path.join(tempDir, "home");
    const voiceProfilePath = path.join(tempDir, "VOICE.md");
    const voiceProfileContent = "# Test Voice\n\nWrite like the repo means it.\n";
    let capturedRequest: AdapterActInvocation | undefined;

    const adapter: SkillAdapter = {
      type: "agent-step",
      invoke: async (request) => {
        capturedRequest = request;
        return {
          status: "sealed",
          stdout: JSON.stringify({ verdict: "pass" }),
          stderr: "",
          exitCode: 0,
          signal: null,
          durationMs: 1,
        };
      },
    };

    await writeFile(voiceProfilePath, voiceProfileContent, "utf8");

    try {
      const result = await runLocalSkill({
        skillPath: path.resolve("fixtures/skills/agent-step"),
        inputs: { prompt: "Check the boundary." },
        caller: {
          resolve: async () => undefined,
          report: async () => undefined,
        },
        env: {
          ...process.env,
          RUNX_CWD: tempDir,
          INIT_CWD: tempDir,
          RUNX_HOME: runxHome,
        },
        receiptDir,
        runxHome,
        adapters: [adapter],
        voiceProfilePath,
      });

      expect(result.status).toBe("sealed");
      expect(capturedRequest?.context).toBeUndefined();
      expect(capturedRequest?.voiceProfile).toMatchObject({
        path: voiceProfilePath,
        sha256: hashString(voiceProfileContent),
        content: voiceProfileContent,
      });

      if (result.status !== "sealed" || result.receipt.schema !== "runx.receipt.v1") {
        return;
      }

      expect(result.receipt.metadata).toMatchObject({
        voice_profile: {
          path: voiceProfilePath,
          sha256: hashString(voiceProfileContent),
        },
      });
      expect(result.receipt.metadata?.context).toBeUndefined();
    } finally {
      await rm(tempDir, { recursive: true, force: true });
    }
  });
});
