import path from "node:path";

import { describe, expect, it } from "vitest";

import {
  analyzeCleanKernelPrs,
  normalizeGitHubPullRequests,
  runCountCleanKernelPrsCli,
} from "../scripts/count-clean-kernel-prs.js";

const fixturePath = path.join(process.cwd(), "tests", "fixtures", "clean-kernel-prs.json");

describe("clean kernel PR counter", () => {
  it("counts only TS kernel PRs and deliberate kernel fixture refreshes with passing evidence", async () => {
    const result = await runCountCleanKernelPrsCli(["--fixture", fixturePath, "--min", "3"]);
    const report = JSON.parse(result.stdout) as ReturnType<typeof analyzeCleanKernelPrs>;

    expect(result).toMatchObject({ exitCode: 0, stderr: "" });
    expect(report.count).toBe(4);
    expect(report.counting.map((entry) => entry.number)).toEqual([101, 102, 103, 108]);
    expect(report.counting.map((entry) => entry.reason)).toEqual([
      "ts_kernel",
      "ts_kernel",
      "kernel_fixture_refresh",
      "kernel_fixture_refresh",
    ]);
  });

  it("requires explicit classification for mixed TS kernel and kernel fixture refresh PRs", async () => {
    const result = await runCountCleanKernelPrsCli(["--fixture", fixturePath, "--min", "3"]);
    const report = JSON.parse(result.stdout) as ReturnType<typeof analyzeCleanKernelPrs>;

    expect(report.counting).toEqual(expect.arrayContaining([
      expect.objectContaining({
        number: 108,
        reason: "kernel_fixture_refresh",
        passing_evidence: true,
      }),
    ]));
    expect(report.non_counting).toEqual(expect.arrayContaining([
      expect.objectContaining({
        number: 109,
        reason: "outside_kernel_promotion_scope",
        passing_evidence: true,
      }),
    ]));
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
      expect.objectContaining({
        number: 110,
        reason: "missing_passing_evidence",
        passing_evidence: false,
      }),
    ]));
  });

  it("does not let evidence-object pass tokens override skipped or ambiguous required checks", () => {
    const report = analyzeCleanKernelPrs({
      advisory_start: "local advisory baseline",
      prs: [
        {
          number: 201,
          title: "policy: ambiguous check state",
          files: ["packages/core/src/policy/public-work.ts"],
          evidence: {
            status: "passed",
            checks: [
              { name: "test:fast", conclusion: "success" },
              { name: "rust:kernel-parity", conclusion: "skipped" },
            ],
          },
        },
        {
          number: 202,
          title: "policy: explicit passing checks",
          files: ["packages/core/src/policy/public-work.ts"],
          evidence: {
            status: "passed",
            checks: [
              { name: "test:fast", conclusion: "success" },
              { name: "rust:kernel-parity", conclusion: "success" },
            ],
          },
        },
        {
          number: 203,
          title: "policy: audited fixture operator evidence",
          files: ["packages/core/src/policy/public-work.ts"],
          passing_evidence: true,
          evidence: {
            status: "renamed",
          },
        },
      ],
    });

    expect(report.counting.map((entry) => entry.number)).toEqual([202, 203]);
    expect(report.non_counting).toEqual(expect.arrayContaining([
      expect.objectContaining({
        number: 201,
        reason: "missing_passing_evidence",
        passing_evidence: false,
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
    const result = await runCountCleanKernelPrsCli(["--fixture", fixturePath, "--min", "5"]);
    const report = JSON.parse(result.stdout) as ReturnType<typeof analyzeCleanKernelPrs>;

    expect(result.exitCode).toBe(1);
    expect(result.stderr).toBe("clean kernel PR count 4 is below required minimum 5\n");
    expect(report).toMatchObject({
      count: 4,
      minimum: 5,
      meets_minimum: false,
    });
  });

  it("normalizes GitHub PR list output into the audited fixture shape", () => {
    const prs = normalizeGitHubPullRequests([
      {
        number: 301,
        title: "policy: enforce local admission",
        mergedAt: "2026-05-20T00:00:00Z",
        files: [
          { path: "packages/core/src/policy/index.ts" },
          { path: "packages/core/src/policy/index.test.ts" },
        ],
        statusCheckRollup: [
          { name: "test:fast", conclusion: "SUCCESS" },
          { name: "rust:kernel-parity", conclusion: "SUCCESS" },
        ],
      },
    ]);

    expect(prs).toEqual([
      {
        number: 301,
        title: "policy: enforce local admission",
        merged_at: "2026-05-20T00:00:00Z",
        metadata_source: "github",
        files: [
          "packages/core/src/policy/index.ts",
          "packages/core/src/policy/index.test.ts",
        ],
        evidence: {
          require_rust_kernel_parity: true,
          checks: [
            { name: "test:fast", conclusion: "success" },
            { name: "rust:kernel-parity", conclusion: "success" },
          ],
        },
      },
    ]);

    const report = analyzeCleanKernelPrs({
      advisory_start: "2026-05-19T00:00:00Z",
      prs,
    });
    expect(report.counting).toEqual([
      expect.objectContaining({
        number: 301,
        reason: "ts_kernel",
        passing_evidence: true,
      }),
    ]);
  });

  it("requires a parseable advisory timestamp for PRs with merge times", () => {
    expect(() => analyzeCleanKernelPrs({
      advisory_start: "manual live metadata probe",
      prs: [
        {
          number: 401,
          title: "policy: merged after unknown start",
          merged_at: "2026-05-20T01:00:00Z",
          files: ["packages/core/src/policy/index.ts"],
          evidence: { status: "passed" },
        },
      ],
    })).toThrow(/not parseable/);
  });

  it("fails closed for PRs outside or missing the live advisory window", () => {
    const report = analyzeCleanKernelPrs({
      advisory_start: {
        timestamp: "2026-05-20T01:00:00Z",
        source: "manual audited advisory start",
      },
      prs: [
        {
          number: 501,
          title: "policy: before advisory start",
          merged_at: "2026-05-20T00:59:59Z",
          files: ["packages/core/src/policy/index.ts"],
          evidence: { status: "passed" },
        },
        {
          number: 502,
          title: "policy: live metadata without merge time",
          metadata_source: "github",
          files: ["packages/core/src/policy/index.ts"],
          evidence: { status: "passed" },
        },
        {
          number: 503,
          title: "policy: after advisory start",
          merged_at: "2026-05-20T01:00:01Z",
          files: ["packages/core/src/policy/index.ts"],
          evidence: { status: "passed" },
        },
      ],
    });

    expect(report.counting.map((entry) => entry.number)).toEqual([503]);
    expect(report.non_counting).toEqual(expect.arrayContaining([
      expect.objectContaining({ number: 501, reason: "outside_advisory_window" }),
      expect.objectContaining({ number: 502, reason: "outside_advisory_window" }),
    ]));
  });

  it("requires passing Rust kernel parity checks without letting unrelated checks block live evidence", () => {
    const report = analyzeCleanKernelPrs({
      advisory_start: "2026-05-20T01:00:00Z",
      prs: normalizeGitHubPullRequests([
        {
          number: 601,
          title: "policy: unrelated check failure after parity pass",
          mergedAt: "2026-05-20T01:01:00Z",
          files: ["packages/core/src/policy/index.ts"],
          statusCheckRollup: [
            { name: "lint", conclusion: "FAILURE" },
            { name: "Advisory Rust kernel parity", conclusion: "SUCCESS" },
          ],
        },
        {
          number: 602,
          title: "policy: missing parity check",
          mergedAt: "2026-05-20T01:02:00Z",
          files: ["packages/core/src/policy/index.ts"],
          statusCheckRollup: [
            { name: "lint", conclusion: "SUCCESS" },
          ],
        },
      ]),
    });

    expect(report.counting.map((entry) => entry.number)).toEqual([601]);
    expect(report.non_counting).toEqual(expect.arrayContaining([
      expect.objectContaining({
        number: 602,
        reason: "missing_passing_evidence",
      }),
    ]));
  });
});
