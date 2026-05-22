import { readFile, readdir } from "node:fs/promises";
import path from "node:path";

import { validateReceiptContract, type ReceiptContract } from "@runxhq/contracts";
import { errorMessage, isNotFound } from "@runxhq/core/util";

export const TRAINING_SCHEMA_REFS = {
  trainable_receipt_row: "https://runx.ai/spec/training/trainable-receipt-row.schema.json",
} as const;

export interface StreamTrainableReceiptsOptions {
  readonly receiptDir: string;
  readonly runxHome?: string;
  readonly since?: string;
  readonly until?: string;
  readonly status?: string;
  readonly source?: string;
}

export interface TrainableReceiptRow {
  readonly kind: "runx.trainable-receipt-row.v1";
  readonly exported_at: string;
  readonly receipt_id: string;
  readonly subject_ref: ReceiptContract["subject"]["ref"];
  readonly disposition: string;
  readonly reason_code: string;
  readonly actor_ref: ReceiptContract["authority"]["actor_ref"];
  readonly act_ids: readonly string[];
  // Runner provenance per agent act drives the trainable-export projection.
  readonly runners: readonly ReceiptContract["acts"][number]["by"][];
  readonly child_receipt_refs: readonly NonNullable<ReceiptContract["lineage"]>["children"][number][];
  readonly signal_refs: readonly NonNullable<ReceiptContract["lineage"]>["signal_refs"][number][];
  readonly journal_ref?: NonNullable<ReceiptContract["lineage"]>["journal_ref"];
  readonly receipt: ReceiptContract;
}

export async function* streamTrainableReceipts(
  options: StreamTrainableReceiptsOptions,
): AsyncGenerator<TrainableReceiptRow> {
  const since = parseTimestamp(options.since, "since");
  const until = parseTimestamp(options.until, "until");

  for (const receipt of await listReceipts(options.receiptDir)) {
    const createdAt = Date.parse(receipt.created_at);
    if (since && createdAt < since) {
      continue;
    }
    if (until && createdAt > until) {
      continue;
    }
    if (options.status && receipt.seal.disposition !== options.status) {
      continue;
    }
    if (
      options.source &&
      receipt.authority.actor_ref.uri !== options.source &&
      receipt.authority.actor_ref.type !== options.source
    ) {
      continue;
    }

    yield projectTrainableReceiptRow({
      receipt,
      exportedAt: new Date().toISOString(),
    });
  }
}

export function projectTrainableReceiptRow(options: {
  readonly receipt: ReceiptContract;
  readonly exportedAt: string;
}): TrainableReceiptRow {
  const { receipt } = options;
  return {
    kind: "runx.trainable-receipt-row.v1",
    exported_at: options.exportedAt,
    receipt_id: receipt.id,
    subject_ref: receipt.subject.ref,
    disposition: receipt.seal.disposition,
    reason_code: receipt.seal.reason_code,
    actor_ref: receipt.authority.actor_ref,
    act_ids: receipt.acts.map((act) => act.id),
    runners: receipt.acts.map((act) => act.by),
    child_receipt_refs: receipt.lineage?.children ?? [],
    signal_refs: receipt.lineage?.signal_refs ?? [],
    journal_ref: receipt.lineage?.journal_ref,
    receipt,
  };
}

async function listReceipts(directory: string): Promise<readonly ReceiptContract[]> {
  let entries: readonly string[];
  try {
    entries = await readdir(directory);
  } catch (error) {
    if (isNotFound(error)) {
      return [];
    }
    throw error;
  }

  const receipts: ReceiptContract[] = [];
  for (const entry of entries.filter((item) => item.endsWith(".json")).sort()) {
    const fullPath = path.join(directory, entry);
    let parsed: unknown;
    try {
      parsed = JSON.parse(await readFile(fullPath, "utf8"));
    } catch (error) {
      process.stderr.write(`warning: skipping receipt at ${fullPath}: ${errorMessage(error)}\n`);
      continue;
    }
    try {
      receipts.push(validateReceiptContract(parsed, fullPath));
    } catch (error) {
      process.stderr.write(`warning: skipping receipt at ${fullPath}: ${errorMessage(error)}\n`);
    }
  }
  return receipts.sort((left, right) => right.created_at.localeCompare(left.created_at));
}

function parseTimestamp(value: string | undefined, label: string): number | undefined {
  if (!value) {
    return undefined;
  }
  const timestamp = Date.parse(value);
  if (Number.isNaN(timestamp)) {
    throw new Error(`Invalid ${label} timestamp '${value}'. Expected ISO-8601.`);
  }
  return timestamp;
}
