import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";

import { describe, expect, it } from "vitest";

import { createIdeActionCore } from "../plugins/ide-core/src/index.js";
import { createFileRegistryStore, seedRegistrySkill } from "./registry-fixtures.js";

describe("ide plugin actions", () => {
  it("runs skills and surfaces input-resolution requests as structured output", async () => {
    const tempDir = await mkdtemp(path.join(os.tmpdir(), "runx-ide-actions-"));
    try {
      const core = createIdeActionCore({
        env: { ...process.env, RUNX_CWD: process.cwd(), RUNX_HOME: path.join(tempDir, "home") },
        receiptDir: path.join(tempDir, "receipts"),
      });

      const missing = await core.runSkill({ skillPath: "fixtures/skills/echo" });
      expect(missing.status).toBe("needs_agent");
      expect(missing.data).toMatchObject({
        status: "needs_agent",
        requests: [
          {
            kind: "input",
            questions: [
              expect.objectContaining({
                id: "message",
                type: "string",
              }),
            ],
          },
        ],
      });

      const success = await core.runSkill({ skillPath: "fixtures/skills/echo", inputs: { message: "from-ide" } });
      expect(success.status).toBe("success");
      expect(JSON.stringify(success.data)).toContain("from-ide");
    } finally {
      await rm(tempDir, { recursive: true, force: true });
    }
  });

  it("wraps receipt inspection, registry, and harness actions", async () => {
    const tempDir = await mkdtemp(path.join(os.tmpdir(), "runx-ide-actions-registry-"));
    try {
      const registryStore = createFileRegistryStore(path.join(tempDir, "registry"));
      await seedRegistrySkill(registryStore, await readFile(path.resolve("skills/sourcey/SKILL.md"), "utf8"), {
        owner: "acme",
        version: "1.0.0",
        createdAt: "2026-04-10T00:00:00.000Z",
      });
      const core = createIdeActionCore({
        env: { ...process.env, RUNX_CWD: process.cwd(), RUNX_HOME: path.join(tempDir, "home") },
        receiptDir: path.join(tempDir, "receipts"),
        registryStore,
      });

      const skillRun = await core.runSkill({ skillPath: "fixtures/skills/echo", inputs: { message: "from-ide" } });
      expect(skillRun.status).toBe("success");
      const receiptId = receiptIdFrom(skillRun.data);
      expect(receiptId).toBeDefined();

      const inspect = await core.inspectReceipt(receiptId ?? "");
      expect(inspect.status).toBe("success");
      const history = await core.history();
      expect(JSON.stringify(history.data)).toContain(receiptId ?? "");

      const search = await core.searchSkills({ query: "sourcey" });
      expect(JSON.stringify(search.data)).toContain("acme/sourcey");
      const add = await core.addSkill({ ref: "acme/sourcey@1.0.0", to: path.join(tempDir, "installed") });
      expect(add.status).toBe("success");

      const harnessFixturePath = path.join(tempDir, "echo-skill-current.yaml");
      await writeFile(
        harnessFixturePath,
        `name: echo-skill
kind: skill
target: ${JSON.stringify(path.resolve("fixtures/skills/echo"))}
inputs:
  message: hello from harness
expect:
  status: sealed
  receipt:
    schema: runx.receipt.v1
    state: sealed
    disposition: closed
    reason_code: process_closed
`,
        "utf8",
      );
      const harness = await core.harnessRun(harnessFixturePath);
      expect(harness.status).toBe("success");
      expect(harness.data?.assertionErrors).toEqual([]);
      const receipt = expectRecord(harness.data?.receipt);
      expect(receipt).toMatchObject({
        schema: "runx.receipt.v1",
        seal: {
          disposition: "closed",
          reason_code: "process_closed",
        },
      });
      const subject = expectRecord(receipt.subject);
      const subjectRef = expectRecord(subject.ref);
      const acts = expectArray(receipt.acts).map(expectRecord);
      expect(receipt.digest).toMatch(/^sha256:[a-f0-9]{64}$/);
      expect(subjectRef.uri).toMatch(/^hrn_/);
      expect(acts).toHaveLength(1);
      expect(acts[0]?.id).toMatch(/^act_/);

    } finally {
      await rm(tempDir, { recursive: true, force: true });
    }
  });
});

function receiptIdFrom(data: unknown): string | undefined {
  return isRecord(data) && isRecord(data.receipt) && typeof data.receipt.id === "string" ? data.receipt.id : undefined;
}

function isRecord(value: unknown): value is Readonly<Record<string, unknown>> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function expectRecord(value: unknown): Readonly<Record<string, unknown>> {
  expect(isRecord(value)).toBe(true);
  return value as Readonly<Record<string, unknown>>;
}

function expectArray(value: unknown): readonly unknown[] {
  expect(Array.isArray(value)).toBe(true);
  return value as readonly unknown[];
}
