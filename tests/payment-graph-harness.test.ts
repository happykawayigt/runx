import { existsSync } from "node:fs";
import { mkdtemp, rm } from "node:fs/promises";
import os from "node:os";
import path from "node:path";

import { describe, expect, it } from "vitest";

import { runCli } from "../packages/cli/src/index.js";

const rustKernelBin = path.resolve(
  "crates",
  "target",
  "debug",
  process.platform === "win32" ? "runx.exe" : "runx",
);
const graphSkills = [
  { skill: "mock-pay", caseName: "mock-pay-mock-path" },
  { skill: "mpp-pay", caseName: "mpp-pay-mpp-path" },
  { skill: "stripe-pay", caseName: "stripe-pay-stripe-spt-path" },
  { skill: "x402-pay", caseName: "x402-pay-x402-path" },
];

describe("canonical payment graph harnesses", () => {
  it.each(graphSkills)("$skill inline harness seals", async ({ skill, caseName }) => {
    const tempDir = await mkdtemp(path.join(os.tmpdir(), `runx-${skill}-harness-`));
    const stdout = createMemoryStream();
    const stderr = createMemoryStream();

    try {
      const exitCode = await runCli(
        ["harness", `skills/${skill}`, "--json"],
        { stdin: process.stdin, stdout, stderr },
        {
          ...paymentHarnessEnv(),
          RUNX_CWD: process.cwd(),
          RUNX_HOME: path.join(tempDir, "home"),
        },
      );

      expect(exitCode, stderr.contents()).toBe(0);
      const report = JSON.parse(stdout.contents()) as {
        source: string;
        status: string;
        cases: Array<{
          fixture: { name: string };
          status: string;
          assertionErrors: string[];
        }>;
        assertionErrors: string[];
      };
      expect(report.source).toBe("inline");
      expect(report.status).toBe("success");
      expect(report.assertionErrors).toEqual([]);
      expect(report.cases).toMatchObject([
        { fixture: { name: caseName }, status: "sealed", assertionErrors: [] },
      ]);
      expect(stderr.contents()).toBe("");
    } finally {
      await rm(tempDir, { recursive: true, force: true });
    }
  }, 20_000);
});

function paymentHarnessEnv(): NodeJS.ProcessEnv {
  const configured = process.env.RUNX_KERNEL_EVAL_BIN;
  const kernelBin = configured && configured.length > 0
    ? configured
    : existsSync(rustKernelBin)
      ? rustKernelBin
      : undefined;
  if (!kernelBin) {
    throw new Error(
      "payment graph harnesses require RUNX_KERNEL_EVAL_BIN or a built crates/target/debug/runx binary.",
    );
  }
  return {
    ...process.env,
    RUNX_KERNEL_EVAL_BIN: kernelBin,
  };
}

function createMemoryStream(): NodeJS.WriteStream & { contents: () => string } {
  let buffer = "";
  return {
    write: (chunk: string | Uint8Array) => {
      buffer += chunk.toString();
      return true;
    },
    contents: () => buffer,
  } as NodeJS.WriteStream & { contents: () => string };
}
