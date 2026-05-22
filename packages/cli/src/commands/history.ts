import { resolvePathFromUserInput } from "@runxhq/core/config";

import { runNativeRunxJson } from "../native-runx.js";
import { relativeTime, shortId, statusIcon, theme } from "../ui.js";

export interface HistoryCommandArgs {
  readonly receiptDir?: string;
  readonly historyQuery?: string;
  readonly historySkill?: string;
  readonly historyStatus?: string;
  readonly historySource?: string;
  readonly historyActor?: string;
  readonly historyArtifactType?: string;
  readonly historySince?: string;
  readonly historyUntil?: string;
}

export interface LocalReceiptSummary {
  readonly id: string;
  readonly kind: string;
  readonly name: string;
  readonly status: string;
  readonly sourceType?: string;
  readonly disposition?: string;
  readonly outcomeState?: string;
  readonly startedAt?: string;
  readonly completedAt?: string;
  readonly actors?: readonly string[];
  readonly artifactTypes?: readonly string[];
  readonly runnerProvider?: string;
  readonly approval?: {
    readonly decision?: string;
    readonly gateType?: string;
  };
  readonly lineage?: {
    readonly kind: string;
    readonly sourceRunId: string;
  };
  readonly verification?: {
    readonly status?: string;
    readonly reason?: string;
  };
  readonly ledgerVerification?: {
    readonly status?: string;
    readonly reason?: string;
  };
  readonly harnessId?: string;
  readonly harnessState?: string;
  readonly harnessSealSummary?: string;
}

export interface PausedRunSummary {
  readonly id: string;
  readonly kind: string;
  readonly name: string;
  readonly status: string;
  readonly selectedRunner?: string;
  readonly stepIds: readonly string[];
  readonly stepLabels: readonly string[];
  readonly ledgerVerification?: {
    readonly status?: string;
    readonly reason?: string;
  };
}

export async function handleHistoryCommand(
  parsed: HistoryCommandArgs,
  env: NodeJS.ProcessEnv,
): Promise<{ readonly receipts: readonly LocalReceiptSummary[]; readonly pendingRuns: readonly PausedRunSummary[] }> {
  const args = ["history"];
  if (parsed.historyQuery) args.push(parsed.historyQuery);
  pushOptionalFlag(args, "--receipt-dir", parsed.receiptDir ? resolvePathFromUserInput(parsed.receiptDir, env) : undefined);
  pushOptionalFlag(args, "--skill", parsed.historySkill);
  pushOptionalFlag(args, "--status", parsed.historyStatus);
  pushOptionalFlag(args, "--source", parsed.historySource);
  pushOptionalFlag(args, "--actor", parsed.historyActor);
  pushOptionalFlag(args, "--artifact-type", parsed.historyArtifactType);
  pushOptionalFlag(args, "--since", parsed.historySince);
  pushOptionalFlag(args, "--until", parsed.historyUntil);
  args.push("--json");
  return normalizeHistoryProjection(await runNativeRunxJson(args, { env }));
}

function normalizeHistoryProjection(value: unknown): {
  readonly receipts: readonly LocalReceiptSummary[];
  readonly pendingRuns: readonly PausedRunSummary[];
} {
  const projection = asRecord(value);
  if (!projection) {
    throw new Error("native runx history returned a non-object payload.");
  }
  return {
    receipts: arrayValue(projection.receipts).map(normalizeHistoryReceipt),
    pendingRuns: arrayValue(projection.pendingRuns).map(normalizePausedRun),
  };
}

function normalizeHistoryReceipt(value: unknown): LocalReceiptSummary {
  const receipt = asRecord(value);
  if (!receipt || typeof receipt.id !== "string" || typeof receipt.name !== "string" || typeof receipt.status !== "string") {
    throw new Error("native runx history returned an invalid receipt entry.");
  }
  const verification = asRecord(receipt.verification);
  return {
    id: receipt.id,
    kind: stringValue(receipt.source_type) ?? "receipt",
    name: receipt.name,
    status: receipt.status,
    sourceType: stringValue(receipt.source_type),
    startedAt: stringValue(receipt.created_at),
    actors: stringArray(receipt.actors),
    artifactTypes: stringArray(receipt.artifact_types),
    verification: verification ? { status: stringValue(verification.status) } : undefined,
    harnessId: stringValue(receipt.harness_id),
    harnessState: stringValue(receipt.harness_state),
    harnessSealSummary: stringValue(receipt.summary),
  };
}

function normalizePausedRun(value: unknown): PausedRunSummary {
  const run = asRecord(value);
  if (!run || typeof run.id !== "string" || typeof run.name !== "string" || typeof run.kind !== "string" || typeof run.status !== "string") {
    throw new Error("native runx history returned an invalid pending run entry.");
  }
  const ledgerVerification = asRecord(run.ledgerVerification);
  return {
    id: run.id,
    name: run.name,
    kind: run.kind,
    status: run.status === "paused" ? "needs_agent" : run.status,
    selectedRunner: stringValue(run.selectedRunner),
    stepIds: stringArray(run.stepIds),
    stepLabels: stringArray(run.stepLabels),
    ledgerVerification: ledgerVerification
      ? {
          status: stringValue(ledgerVerification.status),
          reason: stringValue(ledgerVerification.reason),
        }
      : undefined,
  };
}

function pushOptionalFlag(args: string[], flag: string, value: string | undefined): void {
  if (value !== undefined && value.length > 0) {
    args.push(flag, value);
  }
}

function asRecord(value: unknown): Readonly<Record<string, unknown>> | undefined {
  return typeof value === "object" && value !== null && !Array.isArray(value)
    ? value as Readonly<Record<string, unknown>>
    : undefined;
}

function arrayValue(value: unknown): readonly unknown[] {
  return Array.isArray(value) ? value : [];
}

function stringValue(value: unknown): string | undefined {
  return typeof value === "string" ? value : undefined;
}

function stringArray(value: unknown): readonly string[] {
  return Array.isArray(value) ? value.filter((entry): entry is string => typeof entry === "string") : [];
}

export function renderHistory(
  receipts: readonly LocalReceiptSummary[],
  env: NodeJS.ProcessEnv = process.env,
  query?: string,
  pendingRuns: readonly PausedRunSummary[] = [],
): string {
  const t = theme(undefined, env);
  const totalCount = receipts.length + pendingRuns.length;
  if (totalCount === 0) {
    return query
      ? `\n  ${t.dim}No receipts matched ${t.cyan}${query}${t.reset}${t.dim}.${t.reset}\n  ${t.dim}Try ${t.cyan}runx history${t.reset}${t.dim} to see every local run.${t.reset}\n\n`
      : `\n  ${t.dim}No receipts yet. Try a run first:${t.reset}\n  ${t.cyan}runx skill <skill-dir> --json${t.reset}\n  ${t.cyan}runx list skills${t.reset}\n\n`;
  }
  const now = Date.now();
  const allNames = [...receipts.map((r) => r.name), ...pendingRuns.map((r) => r.name)];
  const nameWidth = Math.min(32, Math.max(...allNames.map((name) => name.length)));
  const lines: string[] = [""];
  const summary = pendingRuns.length > 0
    ? `${receipts.length} receipt(s), ${pendingRuns.length} needs_agent`
    : `${totalCount} receipt(s)`;
  lines.push(`  ${t.bold}history${t.reset}${query ? `  ${t.dim}· ${query}${t.reset}` : ""}  ${t.dim}${summary}${t.reset}`);
  lines.push("");
  for (const pending of pendingRuns) {
    const name = pending.name.padEnd(nameWidth);
    const id = shortId(pending.id);
    const stepLabel = pending.stepLabels[0] ?? pending.stepIds[0] ?? "—";
    lines.push(
      `  ${t.cyan}◇${t.reset}  ${t.bold}${name}${t.reset}  ${t.dim}${pending.status.padEnd(16)}${t.reset}  ${t.dim}${stepLabel.padEnd(10)}${t.reset}  ${t.dim}${"".padEnd(10)}${t.reset}  ${t.dim}${id}${t.reset}`,
    );
  }
  for (const receipt of receipts) {
    const icon = statusIcon(receipt.status, t);
    const name = receipt.name.padEnd(nameWidth);
    const when = receipt.startedAt ? relativeTime(receipt.startedAt, now) : "";
    const source = receipt.sourceType ?? receipt.kind;
    const id = shortId(receipt.id);
    const verification = formatHistoryVerification(receipt);
    lines.push(
      `  ${icon}  ${t.bold}${name}${t.reset}  ${t.dim}${source.padEnd(16)}${t.reset}  ${t.dim}${verification.padEnd(16)}${t.reset}  ${t.dim}${when.padEnd(10)}${t.reset}  ${t.dim}${id}${t.reset}`,
    );
    const harnessStatus = formatHarnessHistoryStatus(receipt);
    if (harnessStatus) {
      lines.push(`     ${t.dim}${harnessStatus}${t.reset}`);
    }
  }
  lines.push("");
  if (pendingRuns.length > 0) {
    lines.push(`  ${t.dim}next${t.reset}  runx skill <same-skill-ref> --run-id <run-id> --answers answers.json  ${t.dim}or${t.reset}  runx history --json`);
  } else {
    lines.push(`  ${t.dim}next${t.reset}  runx history --json`);
  }
  lines.push("");
  return lines.join("\n");
}

function formatHarnessHistoryStatus(receipt: LocalReceiptSummary): string | undefined {
  if (!receipt.harnessState && !receipt.harnessSealSummary && !receipt.harnessId) {
    return undefined;
  }
  const parts = [
    receipt.harnessId ? `harness ${receipt.harnessId}` : "harness",
    receipt.harnessState,
    receipt.harnessSealSummary,
  ].filter((value): value is string => Boolean(value));
  return parts.join(" · ");
}

function formatHistoryVerification(receipt: LocalReceiptSummary): string {
  const signature = receipt.verification?.status ?? "unknown";
  const ledger = receipt.ledgerVerification?.status ?? "unknown";
  return `${signature}/${ledger}`;
}
