import { describe, expect, it } from "vitest";

import { buildReceiptViewModel } from "../plugins/ide-core/src/index.js";

describe("ide receipt view", () => {
  it("renders receipt metadata without raw output bodies", () => {
    const receipt = {
      id: "hrn_rcpt_fanout-docs",
      schema: "runx.receipt.v1",
      digest: "sha256:receipt",
      raw_output: "secret full output body",
      subject: {
        kind: "graph",
        ref: {
          type: "harness",
          uri: "runx:harness:fanout-docs_graph",
          label: "fanout-docs",
        },
        commitments: [
          {
            scope: "output",
            value: "hash-output",
          },
        ],
      },
      seal: {
        disposition: "closed",
        reason_code: "quorum_met",
        summary: "fanout completed",
      },
      acts: [],
      decisions: [],
      lineage: {
        children: [
          {
            type: "receipt",
            uri: "runx:receipt:hrn_rcpt_fanout-docs_research-a",
          },
          {
            type: "receipt",
            uri: "runx:receipt:hrn_rcpt_fanout-docs_synthesize",
          },
        ],
        sync: [
          {
            group_id: "research",
            strategy: "quorum",
            decision: "proceed",
            rule_fired: "quorum_met",
            reason: "2/2 branches succeeded",
            branch_count: 2,
            success_count: 2,
            failure_count: 0,
            required_successes: 2,
            branch_receipts: ["rx_1", "rx_2"],
          },
        ],
      },
    };

    const model = buildReceiptViewModel(receipt);
    expect(model.title).toBe("fanout-docs");
    expect(model.nodes.map((node) => node.kind)).toEqual(expect.arrayContaining(["receipt", "sync", "child-ref"]));
    expect(JSON.stringify(model)).toContain("quorum_met");
    expect(JSON.stringify(model)).toContain("hash-output");
    expect(JSON.stringify(model)).not.toContain("secret full output body");
  });
});
