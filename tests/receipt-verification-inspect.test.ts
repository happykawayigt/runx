import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";

import { describe, expect, it } from "vitest";

import { inspectLocalReceipt, listLocalHistory } from "@runxhq/runtime-local";
import { runCli } from "../packages/cli/src/index.js";

describe("receipt verification for inspect/history", () => {
  it("marks locally signed receipts as verified", async () => {
    const tempDir = await mkdtemp(path.join(os.tmpdir(), "runx-receipt-verify-"));
    const receiptDir = path.join(tempDir, "receipts");
    const runxHome = path.join(tempDir, "home");

    try {
      const receipt = await writeFixtureReceipt(receiptDir, runxHome);

      await expect(inspectLocalReceipt({ receiptDir, runxHome, receiptId: receipt.id })).resolves.toMatchObject({
        verification: { status: "verified" },
        summary: {
          id: receipt.id,
          verification: { status: "verified" },
        },
      });
      await expect(listLocalHistory({ receiptDir, runxHome })).resolves.toMatchObject({
        receipts: [
          {
            id: receipt.id,
            verification: { status: "verified" },
          },
        ],
      });
    } finally {
      await rm(tempDir, { recursive: true, force: true });
    }
  });

  it("marks tampered receipts as invalid", async () => {
    const tempDir = await mkdtemp(path.join(os.tmpdir(), "runx-receipt-tamper-"));
    const receiptDir = path.join(tempDir, "receipts");
    const runxHome = path.join(tempDir, "home");

    try {
      const receipt = await writeFixtureReceipt(receiptDir, runxHome);
      const persisted = path.join(receiptDir, `${receipt.id}.json`);
      const contents = await readFile(persisted, "utf8");
      const tamperedReceipt = JSON.parse(contents) as {
        seal: { disposition: string };
        harness: { seal: { disposition: string } };
      };
      tamperedReceipt.seal.disposition = "failed";
      tamperedReceipt.harness.seal.disposition = "failed";
      await writeFile(persisted, `${JSON.stringify(tamperedReceipt, null, 2)}\n`);

      await expect(inspectLocalReceipt({ receiptDir, runxHome, receiptId: receipt.id })).resolves.toMatchObject({
        receipt: {
          id: receipt.id,
          seal: {
            disposition: "failed",
          },
        },
        verification: { status: "invalid", reason: "signature_mismatch" },
        summary: {
          verification: { status: "invalid", reason: "signature_mismatch" },
        },
      });
    } finally {
      await rm(tempDir, { recursive: true, force: true });
    }
  });

  it("marks receipts as unverified when local key material is unavailable", async () => {
    const tempDir = await mkdtemp(path.join(os.tmpdir(), "runx-receipt-unverified-"));
    const receiptDir = path.join(tempDir, "receipts");

    try {
      const receipt = await writeFixtureReceipt(receiptDir, path.join(tempDir, "signing-home"));

      await expect(
        inspectLocalReceipt({
          receiptDir,
          runxHome: path.join(tempDir, "empty-home"),
          receiptId: receipt.id,
        }),
      ).resolves.toMatchObject({
        receipt: { id: receipt.id },
        verification: { status: "unverified", reason: "local_public_key_missing" },
      });
    } finally {
      await rm(tempDir, { recursive: true, force: true });
    }
  });
});

async function writeFixtureReceipt(receiptDir: string, runxHome: string) {
  const stdout = createMemoryStream();
  const stderr = createMemoryStream();
  const exit = await runCli(
    ["skill", "fixtures/skills/echo", "--message", "hi", "--receipt-dir", receiptDir, "--json"],
    { stdin: process.stdin, stdout, stderr },
    { ...process.env, RUNX_HOME: runxHome, RUNX_CWD: process.cwd() },
  );
  expect(exit).toBe(0);
  expect(stderr.contents()).toBe("");
  return (JSON.parse(stdout.contents()) as { receipt: { id: string } }).receipt;
}

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
