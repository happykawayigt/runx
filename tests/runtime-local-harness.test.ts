import { describe, expect, it } from "vitest";
import { createDefaultSkillAdapters } from "@runxhq/adapters";

import { parseHarnessFixture, runHarness, runHarnessTarget } from "@runxhq/runtime-local/harness";

describe("harness runner", () => {
  it("parses fixture shape and caller traces", () => {
    const fixture = parseHarnessFixture(`
name: echo-fixture
kind: skill
target: ../skills/echo
inputs:
  message: hello
caller:
  answers:
    fallback: value
  approvals:
    gate: true
expect:
  status: sealed
  receipt:
    schema: runx.harness_receipt.v1
    harness_id: hrn_echo-skill_echo
    state: sealed
    disposition: closed
    reason_code: process_closed
    act_ids:
      - act_echo
`);

    expect(fixture.name).toBe("echo-fixture");
    expect(fixture.kind).toBe("skill");
    expect(fixture.inputs).toEqual({ message: "hello" });
    expect(fixture.caller.answers).toEqual({ fallback: "value" });
    expect(fixture.caller.approvals).toEqual({ gate: true });
    expect(fixture.expect.receipt).toMatchObject({
      schema: "runx.harness_receipt.v1",
      harness_id: "hrn_echo-skill_echo",
      state: "sealed",
      disposition: "closed",
      reason_code: "process_closed",
      act_ids: ["act_echo"],
    });
  });

  it("runs an echo skill fixture and asserts receipt shape", async () => {
    const result = await runHarness("fixtures/harness/echo-skill.yaml", { adapters: createDefaultSkillAdapters() });

    expect(result.status).toBe("sealed");
    expect(result.assertionErrors).toEqual([]);
    expect(result.receipt).toBeDefined();
    expect(result.trace.events.map((event) => event.type)).toContain("completed");
  });

  it(
    "runs a sequential graph fixture and asserts migrated harness expectations",
    async () => {
      const result = await runHarness("fixtures/harness/sequential-graph.yaml", { adapters: createDefaultSkillAdapters() });

      expect(result.status).toBe("sealed");
      expect(result.assertionErrors).toEqual([]);
      expect(result.graphReceipt).toBeDefined();
    },
    15_000,
  );

  it(
    "runs inline harness cases from a skill directory",
    async () => {
      const result = await runHarnessTarget("skills/evolve", { adapters: createDefaultSkillAdapters() });

      expect(result.source).toBe("inline");
      if (!("cases" in result)) {
        throw new Error("expected inline harness suite");
      }
      expect(result.status).toBe("success");
      expect(result.assertionErrors).toEqual([]);
      expect(result.cases.map((entry) => entry.fixture.name)).toEqual(["evolve-introspect", "evolve-plan-spec"]);
      expect(result.cases[0]?.status).toBe("sealed");
      expect(result.cases[0]?.receipt).toBeDefined();
      expect(result.cases[1]?.receipt).toBeDefined();
    },
    15_000,
  );
});
