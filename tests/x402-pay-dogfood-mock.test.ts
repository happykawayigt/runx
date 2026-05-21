import { existsSync } from "node:fs";
import { mkdtemp, readFile, readdir, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";

import { describe, expect, it } from "vitest";

import { createDefaultSkillAdapters } from "@runxhq/adapters";
import { runHarness } from "@runxhq/runtime-local/harness";

import { runCli } from "../packages/cli/src/index.js";

const mockScenarioPunchlist = ".scafld/specs/drafts/x402-pay-phase1-mock-scenario-punchlist.md";
const paymentGraphFixture = "fixtures/harness/payment-approval-graph.yaml";
const paymentGraphPath = path.resolve("fixtures/graphs/payment/approval-spend.yaml");
const mockRailSessionMaterialRef = "rail-session-material:mock:payment-execution-001";
const rustKernelBin = path.resolve(
  "crates",
  "target",
  "debug",
  process.platform === "win32" ? "runx.exe" : "runx",
);

const coveredScenarios = new Set(["P1.1", "P1.5", "P1.6", "P1.15", "P1.16"]);
const punchlistedScenarios = [
  "P1.2",
  "P1.3",
  "P1.4",
  "P1.7",
  "P1.8",
  "P1.9",
  "P1.10",
  "P1.11",
  "P1.12",
  "P1.13",
  "P1.14",
  "P1.17",
] as const;

describe("x402-pay Phase 1 mock dogfood fixtures", () => {
  it("runs the mock approval graph through the CLI harness surface", async () => {
    const tempDir = await mkdtemp(path.join(os.tmpdir(), "runx-x402-pay-cli-"));
    const stdout = createMemoryStream();
    const stderr = createMemoryStream();

    try {
      const exitCode = await runCli(
        ["harness", paymentGraphFixture, "--json"],
        { stdin: process.stdin, stdout, stderr },
        {
          ...paymentDogfoodEnv(),
          RUNX_CWD: process.cwd(),
          RUNX_HOME: path.join(tempDir, "home"),
        },
      );

      expect(exitCode).toBe(0);
      expect(stderr.contents()).toBe("");
      const report = JSON.parse(stdout.contents()) as {
        readonly status: string;
        readonly assertionErrors: readonly string[];
        readonly graphReceipt?: {
          readonly harness: {
            readonly child_harness_receipt_refs?: readonly { readonly uri?: string }[];
          };
        };
      };
      expect(report.status).toBe("sealed");
      expect(report.assertionErrors).toEqual([]);
      expect(report.graphReceipt?.harness.child_harness_receipt_refs?.map((ref) => ref.uri)).toEqual([
        "runx:harness_receipt:hrn_rcpt_payment-approval_approve-spend",
        "runx:harness_receipt:hrn_rcpt_payment-approval_fulfill",
      ]);
    } finally {
      await rm(tempDir, { recursive: true, force: true });
    }
  }, 30_000);

  it("seals the happy path only after the mock rail proof is present and history can observe it", async () => {
    let receiptRoot: string | undefined;

    try {
      const result = await runHarness(paymentGraphFixture, {
        adapters: createDefaultSkillAdapters(),
        env: paymentDogfoodEnv(),
        keepFiles: true,
      });
      receiptRoot = path.dirname(result.receiptDir);

      expect(result.status).toBe("sealed");
      expect(result.assertionErrors).toEqual([]);
      const graphReceipt = requireRecord(result.graphReceipt, "graphReceipt");
      expect(childReceiptUris(graphReceipt)).toEqual([
        "runx:harness_receipt:hrn_rcpt_payment-approval_approve-spend",
        "runx:harness_receipt:hrn_rcpt_payment-approval_fulfill",
      ]);
      expect(result.trace.resolutions).toEqual([
        expect.objectContaining({
          request: expect.objectContaining({ id: "spend-approval", kind: "approval" }),
          response: expect.objectContaining({ actor: "human", payload: true }),
        }),
      ]);

      const receipts = await readReceiptObjects(result.receiptDir);
      const fulfillReceipt = requireRecord(
        receipts.find((receipt) => receipt.id === "hrn_rcpt_payment-approval_fulfill"),
        "fulfillReceipt",
      );
      expect(actRefs(fulfillReceipt, "verification_refs")).toEqual(
        expect.arrayContaining([
          expect.objectContaining({
            type: "verification",
            uri: "receipt-proof:mock:payment-execution-001",
            locator: "payment:payment-execution-001",
            proof_kind: "payment_rail",
          }),
        ]),
      );
      const fulfillReceiptText = JSON.stringify(fulfillReceipt);
      expect(fulfillReceiptText).not.toContain("rail_session_material_ref");
      expect(fulfillReceiptText).not.toContain(mockRailSessionMaterialRef);
      expect(fulfillReceiptText).not.toContain("credential_envelope");

      const ledger = await readFile(path.join(result.receiptDir, "ledgers", `${graphReceipt.id}.jsonl`), "utf8");
      expect(ledger).toContain('"type":"run_event"');
      expect(ledger).toContain('"step_id":"fulfill"');
      expect(ledger).toContain('"type":"receipt_link"');
      const allLedgers = await readLedgerContents(result.receiptDir);
      expect(allLedgers).toContain("rail_session_material_ref");
      expect(allLedgers).toContain(mockRailSessionMaterialRef);

      const history = await runHistory(result.receiptDir, result.runxHome);
      expect(history.receipts).toEqual(
        expect.arrayContaining([
          expect.objectContaining({
            id: graphReceipt.id,
            status: "sealed",
            disposition: "closed",
          }),
        ]),
      );
    } finally {
      if (receiptRoot) {
        await rm(receiptRoot, { recursive: true, force: true });
      }
    }
  }, 30_000);

  it("halts cleanly when the payment approval gate is denied", async () => {
    const tempDir = await mkdtemp(path.join(os.tmpdir(), "runx-x402-pay-denied-"));
    let receiptRoot: string | undefined;

    try {
      const fixturePath = path.join(tempDir, "payment-approval-denied.yaml");
      await writeFile(
        fixturePath,
        [
          "name: payment-approval-denied",
          "kind: graph",
          `target: ${paymentGraphPath}`,
          "caller:",
          "  approvals:",
          "    spend-approval: false",
          "expect:",
          "  status: policy_denied",
          "",
        ].join("\n"),
        "utf8",
      );

      const result = await runHarness(fixturePath, {
        adapters: createDefaultSkillAdapters(),
        env: paymentDogfoodEnv(),
        keepFiles: true,
      });
      receiptRoot = path.dirname(result.receiptDir);

      expect(result.status).toBe("policy_denied");
      expect(result.assertionErrors).toEqual([]);
      expect(childReceiptUris(requireRecord(result.graphReceipt, "graphReceipt"))).toEqual([
        "runx:harness_receipt:hrn_rcpt_payment-approval_approve-spend",
      ]);
      expect(result.trace.resolutions).toEqual([
        expect.objectContaining({
          request: expect.objectContaining({ id: "spend-approval", kind: "approval" }),
          response: expect.objectContaining({ actor: "human", payload: false }),
        }),
      ]);
      expect(result.trace.events).not.toEqual(
        expect.arrayContaining([
          expect.objectContaining({ type: "step_started", data: expect.objectContaining({ stepId: "fulfill" }) }),
        ]),
      );

      const receipts = await readReceiptObjects(result.receiptDir);
      expect(receipts.map((receipt) => receipt.id).sort()).toEqual([
        "hrn_rcpt_payment-approval_approve-spend",
        "hrn_rcpt_payment-approval_graph",
      ]);
    } finally {
      if (receiptRoot) {
        await rm(receiptRoot, { recursive: true, force: true });
      }
      await rm(tempDir, { recursive: true, force: true });
    }
  }, 30_000);

  it("keeps every Phase 1 mock eventuality either asserted or explicitly punch-listed", async () => {
    const punchlist = await readFile(mockScenarioPunchlist, "utf8");
    const allScenarioIds = Array.from({ length: 17 }, (_, index) => `P1.${index + 1}`);
    const missing = allScenarioIds.filter(
      (scenarioId) => !coveredScenarios.has(scenarioId) && !punchlist.includes(`| ${scenarioId} |`),
    );

    expect(missing).toEqual([]);
    for (const scenarioId of punchlistedScenarios) {
      const row = punchlist.split("\n").find((line) => line.startsWith(`| ${scenarioId} |`));
      expect(row, scenarioId).toBeDefined();
      expect(row, scenarioId).toContain("Open");
      expect(row, scenarioId).toContain("Missing");
    }
  });
});

function paymentDogfoodEnv(): NodeJS.ProcessEnv {
  const configured = process.env.RUNX_KERNEL_EVAL_BIN;
  const kernelBin = configured && configured.length > 0
    ? configured
    : existsSync(rustKernelBin)
      ? rustKernelBin
      : undefined;
  if (!kernelBin) {
    throw new Error(
      "x402 mock dogfood fixtures require RUNX_KERNEL_EVAL_BIN or a built crates/target/debug/runx binary.",
    );
  }
  return {
    ...process.env,
    RUNX_KERNEL_EVAL_BIN: kernelBin,
  };
}

async function runHistory(receiptDir: string, runxHome: string): Promise<{ readonly receipts: readonly Record<string, unknown>[] }> {
  const stdout = createMemoryStream();
  const stderr = createMemoryStream();
  const exitCode = await runCli(
    ["history", "--receipt-dir", receiptDir, "--json"],
    { stdin: process.stdin, stdout, stderr },
    {
      ...paymentDogfoodEnv(),
      RUNX_CWD: process.cwd(),
      RUNX_HOME: runxHome,
    },
  );
  expect(exitCode).toBe(0);
  expect(stderr.contents()).toBe("");
  return JSON.parse(stdout.contents()) as { readonly receipts: readonly Record<string, unknown>[] };
}

async function readReceiptObjects(receiptDir: string): Promise<readonly Record<string, unknown>[]> {
  const entries = await readdir(receiptDir);
  const receipts: Record<string, unknown>[] = [];
  for (const entry of entries.filter((candidate) => candidate.endsWith(".json")).sort()) {
    receipts.push(JSON.parse(await readFile(path.join(receiptDir, entry), "utf8")) as Record<string, unknown>);
  }
  return receipts;
}

async function readLedgerContents(receiptDir: string): Promise<string> {
  const ledgerDir = path.join(receiptDir, "ledgers");
  const entries = await readdir(ledgerDir);
  const ledgers = await Promise.all(
    entries.filter((candidate) => candidate.endsWith(".jsonl")).sort()
      .map((entry) => readFile(path.join(ledgerDir, entry), "utf8")),
  );
  return ledgers.join("\n");
}

function childReceiptUris(receipt: Record<string, unknown>): readonly string[] {
  const harness = requireRecord(receipt.harness, "receipt.harness");
  const refs = Array.isArray(harness.child_harness_receipt_refs) ? harness.child_harness_receipt_refs : [];
  return refs.map((ref) => requireRecord(ref, "child_harness_receipt_ref").uri).filter(isString);
}

function actRefs(receipt: Record<string, unknown>, field: "verification_refs" | "source_refs"): readonly Record<string, unknown>[] {
  const harness = requireRecord(receipt.harness, "receipt.harness");
  const acts = Array.isArray(harness.acts) ? harness.acts : [];
  return acts.flatMap((act) => {
    const refs = requireRecord(act, "receipt.harness.acts[]")[field];
    return Array.isArray(refs) ? refs.filter(isRecord) : [];
  });
}

function requireRecord(value: unknown, label: string): Record<string, unknown> {
  if (!isRecord(value)) {
    throw new Error(`${label} must be an object.`);
  }
  return value;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isString(value: unknown): value is string {
  return typeof value === "string";
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
