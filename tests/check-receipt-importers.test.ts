import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { describe, expect, it } from "vitest";

import {
  hasDeletionBlockers,
  scanReceiptImporters,
  scanFile,
  summarizeReceiptAudit,
  type ReceiptAuditReport,
} from "../scripts/check-receipt-importers.js";

describe("receipt importer audit classifier", () => {
  it("classifies live @runxhq/core/receipts imports as deletion blockers", () => {
    const findings = scanFile(
      "packages/runtime-local/src/runner-local/approval.ts",
      `import { writeLocalReceipt, type LocalSkillReceipt } from "@runxhq/core/receipts";\n`,
    );

    expect(findings).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          kind: "retired_receipt_import",
          classification: "active_blocker",
          token: "@runxhq/core/receipts",
        }),
        expect.objectContaining({
          kind: "retired_receipt_type",
          classification: "active_blocker",
          token: "LocalSkillReceipt",
        }),
      ]),
    );
  });

  it("keeps package-internal receipt imports and live TS receipt shapes as deletion blockers", () => {
    expect(scanFile("packages/core/src/knowledge/local-store.ts", `import type { LocalReceipt } from "../receipts/index.js";\n`)).toEqual([
      expect.objectContaining({
        kind: "retired_receipt_import",
        classification: "active_blocker",
        token: "../receipts/index.js",
      }),
    ]);

    expect(scanFile("packages/runtime-local/src/runner-local/history.ts", `kind: "graph_execution",\n`)).toEqual([
      expect.objectContaining({
        kind: "retired_receipt_shape",
        classification: "active_blocker",
        token: "graph_execution",
      }),
    ]);
  });

  it("separates generated stale artifacts and archived fixtures from live blockers", () => {
    expect(scanFile("scripts/generate-rust-skill-fixtures.ts", `kind: "skill_execution",\n`)).toEqual([
      expect.objectContaining({
        kind: "retired_receipt_shape",
        classification: "generated_stale_artifact",
      }),
    ]);

    expect(scanFile("fixtures/runtime/skills/issue-intake/metadata.json", `"skill_name": "issue-intake"\n`)).toEqual([
      expect.objectContaining({
        kind: "retired_receipt_shape",
        classification: "fixture_archive",
      }),
    ]);
  });

  it("does not scan local generated .runx runtime receipts", async () => {
    const tempDir = await mkdtemp(path.join(os.tmpdir(), "runx-receipt-importer-audit-"));
    try {
      await mkdir(path.join(tempDir, "packages/cli/.runx/receipts"), { recursive: true });
      await mkdir(path.join(tempDir, "packages/runtime-local/src"), { recursive: true });
      await writeFile(
        path.join(tempDir, "packages/cli/.runx/receipts/rx_local.json"),
        `{"kind":"skill_execution","skill_name":"local"}\n`,
        "utf8",
      );
      await writeFile(
        path.join(tempDir, "packages/runtime-local/src/live.ts"),
        `export const receipt = { kind: "skill_execution" };\n`,
        "utf8",
      );

      const report = await scanReceiptImporters({
        workspaceRoot: tempDir,
        roots: ["packages"],
        includeCloudSibling: false,
      });

      expect(report.findings).toEqual([
        expect.objectContaining({
          file: "packages/runtime-local/src/live.ts",
          classification: "active_blocker",
          token: "skill_execution",
        }),
      ]);
      expect(report.scannedFiles).toBe(1);
    } finally {
      await rm(tempDir, { recursive: true, force: true });
    }
  });

  it("marks canonical harness receipt references as migrated evidence", () => {
    expect(scanFile("packages/contracts/src/schemas/spine.ts", `schema: "runx.harness_receipt.v1",\n`)).toEqual([
      expect.objectContaining({
        kind: "harness_receipt_shape",
        classification: "migrated",
      }),
    ]);
  });

  it("marks explicit runtime pseudo-signature policy code as dev-only", () => {
    expect(scanFile("crates/runx-runtime/src/receipts.rs", `        value: "sig:pending".to_owned(),\n`)).toEqual([
      expect.objectContaining({
        kind: "runtime_pseudo_signature",
        classification: "false_positive",
        token: "sig:pending",
      }),
    ]);

    expect(scanFile("crates/runx-receipts/src/tree.rs", `        receipt.signature.value = format!("sig:{digest}");\n`)).toEqual([
      expect.objectContaining({
        kind: "runtime_pseudo_signature",
        classification: "false_positive",
        token: "sig:{digest}",
      }),
    ]);
  });

  it("marks Rust retired-field rejection guards as non-blocking", () => {
    expect(scanFile("crates/runx-runtime/src/harness/fixtures.rs", `    "skill_execution",\n`)).toEqual([
      expect.objectContaining({
        kind: "retired_receipt_shape",
        classification: "false_positive",
        token: "skill_execution",
      }),
    ]);

    expect(scanFile("crates/runx-runtime/tests/harness_fixtures.rs", `        "graph_name",\n`)).toEqual([
      expect.objectContaining({
        kind: "retired_receipt_shape",
        classification: "false_positive",
        token: "graph_name",
      }),
    ]);
  });

  it("does not treat Rust skill_name and graph_name identifiers as retired receipt contracts", () => {
    expect(scanFile("crates/runx-runtime/src/receipts.rs", `fn step_receipt_id(graph_name: &str, step_id: &str) -> String {\n`)).toEqual([
      expect.objectContaining({
        kind: "retired_receipt_shape",
        classification: "false_positive",
        token: "graph_name",
      }),
    ]);

    expect(scanFile("crates/runx-runtime/src/journal.rs", `name: metadata_string(receipt.metadata.as_ref(), &["skill_name", "name"])\n`)).toEqual([
      expect.objectContaining({
        kind: "retired_receipt_shape",
        classification: "false_positive",
        token: "skill_name",
      }),
    ]);
  });

  it("summarizes blockers for deletion-gate mode", () => {
    const report: ReceiptAuditReport = {
      workspaceRoot: "/workspace",
      scannedFiles: 2,
      cloudSibling: "not_found",
      findings: [
        ...scanFile("packages/core/package.json", `"./receipts": {\n`),
        ...scanFile("scripts/generate-rust-contract-fixtures.ts", `kind: "skill_execution",\n`),
      ],
    };

    expect(summarizeReceiptAudit(report)).toMatchObject({
      active_blocker: 1,
      generated_stale_artifact: 1,
    });
    expect(hasDeletionBlockers(report)).toBe(true);
  });
});
