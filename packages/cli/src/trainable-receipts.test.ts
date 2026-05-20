import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";

import { describe, expect, it } from "vitest";

import { runCli } from "./index.js";
import { TRAINING_SCHEMA_REFS } from "./trainable-receipts.js";

describe("trainable receipts export", () => {
  it("publishes the canonical trainable harness row schema ref", () => {
    expect(TRAINING_SCHEMA_REFS.trainable_harness_receipt_row).toBe(
      "https://runx.ai/spec/training/trainable-harness-receipt-row.schema.json",
    );
  });

  it("streams filtered JSONL records from harness receipt fixtures", async () => {
    const tempDir = await mkdtemp(path.join(os.tmpdir(), "runx-trainable-harness-"));
    const directory = path.join(tempDir, "receipts");

    try {
      await mkdir(directory, { recursive: true });
      const fixtureDocument = JSON.parse(await readFile(path.resolve("fixtures/contracts/harness-spine/harness-receipt-success.json"), "utf8")) as {
        expected: { id: string };
      };
      const fixture = fixtureDocument.expected;
      await writeFile(path.join(directory, `${fixture.id}.json`), `${JSON.stringify(fixture, null, 2)}\n`);

      const stdout = createMemoryStream();
      const stderr = createMemoryStream();
      const exitCode = await runCli(
        [
          "export-receipts",
          "--trainable",
          "--receipt-dir",
          directory,
          "--since",
          "2026-05-17T00:00:00Z",
          "--status",
          "sealed",
          "--source",
          "host",
        ],
        { stdin: process.stdin, stdout, stderr },
        {
          ...process.env,
          RUNX_CWD: process.cwd(),
        },
      );

      expect(exitCode).toBe(0);
      expect(stderr.contents()).toBe("");

      const rows = stdout.contents().trim().split("\n").map((line) => JSON.parse(line));
      expect(rows).toHaveLength(1);
      expect(rows[0]).toMatchObject({
        kind: "runx.trainable-harness-receipt-row.v1",
        harness_receipt_id: fixture.id,
        harness_id: "hrn_123",
        state: "sealed",
        disposition: "closed",
        reason_code: "completed",
        act_ids: ["act_revision_1"],
        decision_ids: ["dec_open_1"],
        host_ref: {
          type: "host",
          uri: "runx:host:cli",
        },
        verification_summary: {
          signature_valid: true,
        },
      });
      expect(typeof rows[0].exported_at).toBe("string");
      expect(rows[0].receipt.id).toBe(fixture.id);
    } finally {
      await rm(tempDir, { recursive: true, force: true });
    }
  });
});

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
