import path from "node:path";

import { describe, expect, it } from "vitest";

import {
  analyzeCleanKernelPrs,
  runCountCleanKernelPrsCli,
} from "../scripts/count-clean-kernel-prs.js";

const fixturePath = path.join(process.cwd(), "tests", "fixtures", "clean-kernel-prs.json");

describe("clean kernel PR counter", () => {
  it("counts only TS kernel PRs and deliberate kernel fixture refreshes with passing evidence", async () => {
    const result = await runCountCleanKernelPrsCli(["--fixture", fixturePath, "--min", "3"]);
    const report = JSON.parse(result.stdout) as ReturnType<typeof analyzeCleanKernelPrs>;

    expect(result).toMatchObject({ exitCode: 0, stderr: "" });
    expect(report.count).toBe(3);
    expect(report.counting.map((entry) => entry.number)).toEqual([101, 102, 103]);
    expect(report.counting.map((entry) => entry.reason)).toEqual([
      "ts_kernel",
      "ts_kernel",
      "kernel_fixture_refresh",
    ]);
  });

  it("reports Rust-only and parser-only PRs as non-counting", async () => {
    const result = await runCountCleanKernelPrsCli(["--fixture", fixturePath, "--min", "3"]);
    const report = JSON.parse(result.stdout) as ReturnType<typeof analyzeCleanKernelPrs>;

    expect(report.non_counting).toEqual(expect.arrayContaining([
      expect.objectContaining({
        number: 104,
        reason: "rust_only",
        passing_evidence: true,
      }),
      expect.objectContaining({
        number: 105,
        reason: "parser_only",
        passing_evidence: true,
      }),
      expect.objectContaining({
        number: 106,
        reason: "missing_passing_evidence",
        passing_evidence: false,
      }),
      expect.objectContaining({
        number: 107,
        reason: "outside_kernel_promotion_scope",
        passing_evidence: true,
      }),
    ]));
  });

  it("requires explicit advisory start evidence from the fixture or CLI", () => {
    expect(() => analyzeCleanKernelPrs({ prs: [] })).toThrow(
      /missing advisory start evidence/,
    );

    expect(analyzeCleanKernelPrs({
      advisoryStart: "local advisory baseline",
      pullRequests: [],
    })).toMatchObject({
      advisory_start: "local advisory baseline",
      advisory_start_source: "fixture",
    });

    expect(analyzeCleanKernelPrs(
      { prs: [] },
      { advisoryStart: "cli advisory baseline" },
    )).toMatchObject({
      advisory_start: "cli advisory baseline",
      advisory_start_source: "cli",
    });
  });

  it("fails closed when the requested minimum is not met", async () => {
    const result = await runCountCleanKernelPrsCli(["--fixture", fixturePath, "--min", "4"]);
    const report = JSON.parse(result.stdout) as ReturnType<typeof analyzeCleanKernelPrs>;

    expect(result.exitCode).toBe(1);
    expect(result.stderr).toBe("clean kernel PR count 3 is below required minimum 4\n");
    expect(report).toMatchObject({
      count: 3,
      minimum: 4,
      meets_minimum: false,
    });
  });
});
