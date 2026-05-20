import { readFile, readdir } from "node:fs/promises";
import path from "node:path";

import {
  validateHarnessReceiptContract,
  type HarnessReceiptContract,
} from "@runxhq/contracts";
import { errorMessage, isNotFound } from "@runxhq/core/util";

type HarnessReceiptWithVerificationSummary = HarnessReceiptContract & {
  readonly seal: HarnessReceiptContract["seal"] & {
    readonly verification_summary: NonNullable<HarnessReceiptContract["seal"]["verification_summary"]>;
  };
};

export const TRAINING_SCHEMA_REFS = {
  trainable_harness_receipt_row: "https://runx.ai/spec/training/trainable-harness-receipt-row.schema.json",
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
  readonly kind: "runx.trainable-harness-receipt-row.v1";
  readonly exported_at: string;
  readonly harness_receipt_id: string;
  readonly harness_id: string;
  readonly state: HarnessReceiptContract["harness"]["state"];
  readonly disposition: string;
  readonly reason_code: string;
  readonly host_ref: HarnessReceiptContract["harness"]["host_ref"];
  readonly harness_ref: HarnessReceiptContract["harness"]["harness_ref"];
  readonly act_ids: readonly string[];
  readonly decision_ids: readonly string[];
  readonly artifact_refs: HarnessReceiptContract["harness"]["artifact_refs"];
  readonly signal_refs: HarnessReceiptContract["harness"]["signal_refs"];
  readonly child_harness_receipt_refs: HarnessReceiptContract["harness"]["child_harness_receipt_refs"];
  readonly verification_summary: NonNullable<HarnessReceiptContract["seal"]["verification_summary"]>;
  readonly receipt: HarnessReceiptWithVerificationSummary;
}

export async function* streamTrainableReceipts(
  options: StreamTrainableReceiptsOptions,
): AsyncGenerator<TrainableReceiptRow> {
  const since = parseTimestamp(options.since, "since");
  const until = parseTimestamp(options.until, "until");

  for (const receipt of await listHarnessReceipts(options.receiptDir)) {
    const createdAt = Date.parse(receipt.created_at);
    if (since && createdAt < since) {
      continue;
    }
    if (until && createdAt > until) {
      continue;
    }
    if (options.status && receipt.harness.state !== options.status) {
      continue;
    }
    if (options.source && receipt.harness.host_ref.uri !== options.source && receipt.harness.host_ref.type !== options.source) {
      continue;
    }
    if (!receipt.seal.verification_summary) {
      process.stderr.write(`warning: skipping harness receipt ${receipt.id}: missing verification summary\n`);
      continue;
    }
    const receiptWithVerification = receipt as HarnessReceiptWithVerificationSummary;

    yield projectTrainableReceiptRow({
      receipt: receiptWithVerification,
      exportedAt: new Date().toISOString(),
    });
  }
}

export function projectTrainableReceiptRow(options: {
  readonly receipt: HarnessReceiptWithVerificationSummary;
  readonly exportedAt: string;
}): TrainableReceiptRow {
  const { receipt } = options;
  return {
    kind: "runx.trainable-harness-receipt-row.v1",
    exported_at: options.exportedAt,
    harness_receipt_id: receipt.id,
    harness_id: receipt.harness.harness_id,
    state: receipt.harness.state,
    disposition: receipt.seal.disposition,
    reason_code: receipt.seal.reason_code,
    host_ref: receipt.harness.host_ref,
    harness_ref: receipt.harness.harness_ref,
    act_ids: receipt.harness.acts.map((act) => act.act_id),
    decision_ids: receipt.harness.decisions.map((decision) => decision.decision_id),
    artifact_refs: receipt.harness.artifact_refs,
    signal_refs: receipt.harness.signal_refs,
    child_harness_receipt_refs: receipt.harness.child_harness_receipt_refs,
    verification_summary: receipt.seal.verification_summary ?? {
      signature_valid: false,
      hash_commitments_valid: false,
      authority_attenuation_valid: false,
      criteria_bound: false,
      redaction_valid: false,
      external_attestations_present: false,
    },
    receipt,
  };
}

async function listHarnessReceipts(directory: string): Promise<readonly HarnessReceiptContract[]> {
  let entries: readonly string[];
  try {
    entries = await readdir(directory);
  } catch (error) {
    if (isNotFound(error)) {
      return [];
    }
    throw error;
  }

  const receipts: HarnessReceiptContract[] = [];
  for (const entry of entries.filter((item) => item.endsWith(".json")).sort()) {
    const fullPath = path.join(directory, entry);
    let parsed: unknown;
    try {
      parsed = JSON.parse(await readFile(fullPath, "utf8"));
    } catch (error) {
      process.stderr.write(`warning: skipping harness receipt at ${fullPath}: ${errorMessage(error)}\n`);
      continue;
    }
    try {
      receipts.push(validateHarnessReceiptContract(parsed, fullPath));
    } catch (error) {
      process.stderr.write(`warning: skipping harness receipt at ${fullPath}: ${errorMessage(error)}\n`);
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
