import { describe, expect, it } from "vitest";

import { renderHistory } from "./history.js";

type LocalReceiptSummary = Parameters<typeof renderHistory>[0][number];

describe("renderHistory", () => {
  it("surfaces compact harness status summaries", () => {
    const output = renderHistory([
      {
        id: "rx_history_harness",
        name: "issue-intake",
        kind: "runx.receipt.v1",
        status: "success",
        sourceType: "agent-step",
        verification: {
          status: "verified",
          reason: "ok",
        },
        ledgerVerification: {
          status: "valid",
          reason: "ok",
          runId: "run_history_harness",
          ledgerPath: "/tmp/runx-history-fixture/ledger.jsonl",
          entryCount: 1,
          headHash: "sha256:fixture",
        },
        harnessId: "harness_fixture_123",
        harnessState: "sealed",
        harnessSealSummary: "PR is ready for human merge gate.",
      } as LocalReceiptSummary,
    ], { NO_COLOR: "1" } as NodeJS.ProcessEnv);

    expect(output).toContain("harness harness_fixture_123");
    expect(output).toContain("sealed");
    expect(output).toContain("PR is ready for human merge gate.");
  });
});
