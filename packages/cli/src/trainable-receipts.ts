import { readFile, readdir } from "node:fs/promises";
import path from "node:path";

import { validateReceiptContract, type ReceiptContract } from "@runxhq/contracts";
import { errorMessage, isNotFound } from "./cli-util.js";

export const TRAINING_SCHEMA_REFS = {
  trainable_receipt_row: "https://runx.ai/spec/training/trainable-receipt-row.schema.json",
} as const;

export interface StreamTrainableReceiptsOptions {
  readonly receiptDir: string;
  readonly runxHome?: string;
  /** Optional artifact directory used to hydrate `acts[].context_ref` + `artifact_refs`. */
  readonly artifactDir?: string;
  readonly since?: string;
  readonly until?: string;
  readonly status?: string;
  readonly source?: string;
}

type Reference = ReceiptContract["subject"]["ref"];
type ReceiptAct = ReceiptContract["acts"][number];
type ReceiptDecision = ReceiptContract["decisions"][number];

/**
 * Read-time verification of the receipt. The structural integrity properties the
 * projection can recompute without the Rust canonicalizer: every rolled-up seal
 * criterion is bound to an act criterion (binding or declared success criterion),
 * and every `decision.selected_act_id` refers to an inline act. Signature/digest
 * recompute is the Rust verifier's job; this is the trainable-side read check.
 */
export interface TrainableVerification {
  readonly criteria_bound: boolean;
  readonly selected_acts_resolved: boolean;
  readonly signature_present: boolean;
  readonly digest_present: boolean;
}

/** The hydrated agent-context envelope (instructions / inputs / output) for an act. */
export interface HydratedActContext {
  readonly act_id: string;
  readonly context_ref?: Reference;
  readonly artifact_refs: readonly Reference[];
  readonly envelope?: unknown;
  readonly artifacts: readonly unknown[];
}

/** A governance decision flattened into its trainable essentials. */
export interface TrainableDecision {
  readonly decision_id: string;
  readonly choice: ReceiptDecision["choice"];
  readonly proposed_purpose: string;
  readonly justification: string;
  readonly selected_act_id: ReceiptDecision["selected_act_id"];
}

/** An act flattened into its trainable essentials (intent + criteria outcomes, not ids). */
export interface TrainableAct {
  readonly act_id: string;
  readonly form: ReceiptAct["form"];
  readonly intent_purpose: string;
  readonly intent_legitimacy: string;
  readonly success_criteria: readonly { readonly criterion_id: string; readonly statement: string; readonly required: boolean }[];
  readonly criterion_outcomes: readonly { readonly criterion_id: string; readonly status: string; readonly summary?: string }[];
}

export interface TrainableReceiptRow {
  readonly kind: "runx.trainable-receipt-row.v1";
  readonly exported_at: string;
  readonly receipt_id: string;
  readonly subject_ref: Reference;
  readonly disposition: string;
  readonly reason_code: string;
  readonly actor_ref: ReceiptContract["authority"]["actor_ref"];
  // The training INPUT: where the run came from + a preview.
  readonly input: ReceiptContract["subject"]["input_context"];
  readonly signal_refs: readonly Reference[];
  // GOVERNANCE reasoning (why this run was admitted / escalated / deferred).
  readonly decisions: readonly TrainableDecision[];
  // ACTS with intent, success criteria, and criterion OUTCOMES (not just ids).
  readonly acts: readonly TrainableAct[];
  // Runner provenance per agent act.
  readonly runners: readonly ReceiptAct["by"][];
  // The OUTCOME: seal disposition + any review/verification verdict act.
  readonly outcome: {
    readonly disposition: string;
    readonly reason_code: string;
    readonly summary: string;
    readonly criteria: ReceiptContract["seal"]["criteria"];
    readonly verdict_acts: readonly TrainableAct[];
  };
  // The bulky agent I/O behind `context_ref` + `artifact_refs`, hydrated.
  readonly hydrated_context: readonly HydratedActContext[];
  readonly child_receipt_refs: readonly NonNullable<ReceiptContract["lineage"]>["children"][number][];
  // Verification computed ON READ.
  readonly verification: TrainableVerification;
  // The full rich receipt is embedded (proof + training signal are one artifact).
  readonly receipt: ReceiptContract;
}

export async function* streamTrainableReceipts(
  options: StreamTrainableReceiptsOptions,
): AsyncGenerator<TrainableReceiptRow> {
  const since = parseTimestamp(options.since, "since");
  const until = parseTimestamp(options.until, "until");
  const artifacts = await loadArtifacts(options.artifactDir);

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
      artifacts,
    });
  }
}

export function projectTrainableReceiptRow(options: {
  readonly receipt: ReceiptContract;
  readonly exportedAt: string;
  readonly artifacts?: ReadonlyMap<string, unknown>;
}): TrainableReceiptRow {
  const { receipt } = options;
  const artifacts = options.artifacts ?? new Map<string, unknown>();
  const acts = receipt.acts.map(projectAct);
  const verdictForms = new Set<ReceiptAct["form"]>(["review", "verification"]);
  return {
    kind: "runx.trainable-receipt-row.v1",
    exported_at: options.exportedAt,
    receipt_id: receipt.id,
    subject_ref: receipt.subject.ref,
    disposition: receipt.seal.disposition,
    reason_code: receipt.seal.reason_code,
    actor_ref: receipt.authority.actor_ref,
    input: receipt.subject.input_context,
    signal_refs: receipt.signals,
    decisions: receipt.decisions.map(projectDecision),
    acts,
    runners: receipt.acts.map((act) => act.by),
    outcome: {
      disposition: receipt.seal.disposition,
      reason_code: receipt.seal.reason_code,
      summary: receipt.seal.summary,
      criteria: receipt.seal.criteria,
      verdict_acts: acts.filter((_act, index) => verdictForms.has(receipt.acts[index].form)),
    },
    hydrated_context: receipt.acts.map((act) => hydrateActContext(act, artifacts)),
    child_receipt_refs: receipt.lineage?.children ?? [],
    verification: computeVerification(receipt),
    receipt,
  };
}

function projectDecision(decision: ReceiptDecision): TrainableDecision {
  return {
    decision_id: decision.decision_id,
    choice: decision.choice,
    proposed_purpose: decision.proposed_intent.purpose,
    justification: decision.justification.summary,
    selected_act_id: decision.selected_act_id,
  };
}

function projectAct(act: ReceiptAct): TrainableAct {
  return {
    act_id: act.id,
    form: act.form,
    intent_purpose: act.intent.purpose,
    intent_legitimacy: act.intent.legitimacy,
    success_criteria: act.intent.success_criteria.map((criterion) => ({
      criterion_id: criterion.criterion_id,
      statement: criterion.statement,
      required: criterion.required,
    })),
    criterion_outcomes: act.criterion_bindings.map((binding) => ({
      criterion_id: binding.criterion_id,
      status: binding.status,
      summary: binding.summary,
    })),
  };
}

function hydrateActContext(
  act: ReceiptAct,
  artifacts: ReadonlyMap<string, unknown>,
): HydratedActContext {
  const envelope = act.context_ref ? artifacts.get(act.context_ref.uri) : undefined;
  return {
    act_id: act.id,
    context_ref: act.context_ref,
    artifact_refs: act.artifact_refs,
    envelope,
    artifacts: act.artifact_refs
      .map((reference) => artifacts.get(reference.uri))
      .filter((value): value is unknown => value !== undefined),
  };
}

function computeVerification(receipt: ReceiptContract): TrainableVerification {
  const actCriterionIds = new Set<string>();
  for (const act of receipt.acts) {
    for (const binding of act.criterion_bindings) {
      actCriterionIds.add(binding.criterion_id);
    }
    for (const criterion of act.intent.success_criteria) {
      actCriterionIds.add(criterion.criterion_id);
    }
  }
  const criteriaBound =
    receipt.acts.length === 0 ||
    receipt.seal.criteria.every((criterion) => actCriterionIds.has(criterion.criterion_id));

  const actIds = new Set(receipt.acts.map((act) => act.id));
  const selectedActsResolved = receipt.decisions.every(
    (decision) => decision.selected_act_id === null || actIds.has(decision.selected_act_id),
  );

  return {
    criteria_bound: criteriaBound,
    selected_acts_resolved: selectedActsResolved,
    signature_present: receipt.signature.value.length > 0,
    digest_present: receipt.digest.length > 0,
  };
}

async function loadArtifacts(
  artifactDir: string | undefined,
): Promise<ReadonlyMap<string, unknown>> {
  const artifacts = new Map<string, unknown>();
  if (!artifactDir) {
    return artifacts;
  }
  let entries: readonly string[];
  try {
    entries = await readdir(artifactDir);
  } catch (error) {
    if (isNotFound(error)) {
      return artifacts;
    }
    throw error;
  }
  for (const entry of entries.filter((item) => item.endsWith(".json")).sort()) {
    const fullPath = path.join(artifactDir, entry);
    let parsed: unknown;
    try {
      parsed = JSON.parse(await readFile(fullPath, "utf8"));
    } catch (error) {
      process.stderr.write(`warning: skipping artifact at ${fullPath}: ${errorMessage(error)}\n`);
      continue;
    }
    const uri = artifactUri(parsed);
    if (uri) {
      artifacts.set(uri, parsed);
    }
  }
  return artifacts;
}

function artifactUri(value: unknown): string | undefined {
  if (typeof value !== "object" || value === null) {
    return undefined;
  }
  const record = value as Record<string, unknown>;
  if (typeof record.uri === "string") {
    return record.uri;
  }
  const reference = record.ref;
  if (typeof reference === "object" && reference !== null) {
    const referenceUri = (reference as Record<string, unknown>).uri;
    if (typeof referenceUri === "string") {
      return referenceUri;
    }
  }
  return undefined;
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
