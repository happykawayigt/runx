import { createHash } from "node:crypto";
import type { Dirent } from "node:fs";
import { readdir, readFile, stat } from "node:fs/promises";

export function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

export function isPlainRecord(value: unknown): value is Readonly<Record<string, unknown>> {
  return isRecord(value);
}

export function asRecord(value: unknown): Record<string, unknown> | undefined {
  return isRecord(value) ? value : undefined;
}

export function readField(value: unknown, key: string): unknown {
  return asRecord(value)?.[key];
}

export function recordField(value: unknown, key: string): Readonly<Record<string, unknown>> | undefined {
  return asRecord(readField(value, key));
}

export function stringField(value: unknown, key: string): string | undefined {
  const field = readField(value, key);
  return typeof field === "string" ? field : undefined;
}

export function stringValue(value: unknown): string | undefined {
  return typeof value === "string" ? value : undefined;
}

export function parsePositiveInt(value: string | undefined): number | undefined {
  if (!value) return undefined;
  const parsed = Number.parseInt(value, 10);
  return Number.isFinite(parsed) && parsed > 0 ? parsed : undefined;
}

export function arrayValue(value: unknown): readonly unknown[] {
  return Array.isArray(value) ? value : [];
}

export function isNodeError(error: unknown): error is NodeJS.ErrnoException {
  return error instanceof Error && "code" in error;
}

export function isNotFound(error: unknown): boolean {
  return isNodeError(error) && error.code === "ENOENT";
}

export function isAlreadyExists(error: unknown): boolean {
  return isNodeError(error) && error.code === "EEXIST";
}

export function errorMessage(value: unknown): string {
  return value instanceof Error ? value.message : String(value);
}

export function firstNonEmptyOrUndefined(...values: readonly (string | undefined)[]): string | undefined {
  for (const value of values) {
    const trimmed = value?.trim();
    if (trimmed) {
      return trimmed;
    }
  }
  return undefined;
}

export function firstNonEmpty(...values: readonly (string | undefined)[]): string {
  return firstNonEmptyOrUndefined(...values) ?? "";
}

export function stableStringify(value: unknown): string {
  if (value === null || typeof value !== "object") {
    return JSON.stringify(value);
  }
  if (Array.isArray(value)) {
    return `[${value.map((item) => stableStringify(item)).join(",")}]`;
  }
  const entries = Object.entries(value as Record<string, unknown>)
    .filter(([, entryValue]) => entryValue !== undefined)
    .sort(([left], [right]) => left.localeCompare(right));
  return `{${entries.map(([key, entryValue]) => `${JSON.stringify(key)}:${stableStringify(entryValue)}`).join(",")}}`;
}

export function hashStable(value: unknown): string {
  return hashString(stableStringify(value));
}

export function hashString(value: string): string {
  return createHash("sha256").update(value).digest("hex");
}

export async function pathExists(candidate: string): Promise<boolean> {
  try {
    await stat(candidate);
    return true;
  } catch (error) {
    if (isNotFound(error)) {
      return false;
    }
    throw error;
  }
}

export async function readOptionalFile(filePath: string): Promise<string | undefined> {
  try {
    return await readFile(filePath, "utf8");
  } catch (error) {
    if (isNotFound(error)) {
      return undefined;
    }
    throw error;
  }
}

export async function safeReadDir(directory: string): Promise<readonly Dirent[]> {
  try {
    return await readdir(directory, { withFileTypes: true });
  } catch (error) {
    if (isNotFound(error)) {
      return [];
    }
    throw error;
  }
}

export interface FetchWithTimeoutOptions {
  readonly fetchImpl?: typeof fetch;
  readonly url: string;
  readonly init?: RequestInit;
  readonly signal?: AbortSignal;
  readonly timeoutMs?: number;
  readonly description: string;
}

export async function fetchWithTimeout(options: FetchWithTimeoutOptions): Promise<Response> {
  const fetchImpl = options.fetchImpl ?? globalThis.fetch;
  if (typeof fetchImpl !== "function") {
    throw new Error("Global fetch is not available. Use Node.js 20+ or inject fetchImpl.");
  }
  const timeoutMs = options.timeoutMs ?? 30_000;
  const controller = new AbortController();
  const onUpstreamAbort = (): void => controller.abort(options.signal?.reason);
  if (options.signal) {
    if (options.signal.aborted) {
      controller.abort(options.signal.reason);
    } else {
      options.signal.addEventListener("abort", onUpstreamAbort, { once: true });
    }
  }
  const timer = timeoutMs > 0
    ? setTimeout(() => controller.abort(new Error(`${options.description} timed out after ${timeoutMs}ms.`)), timeoutMs)
    : undefined;
  try {
    return await fetchImpl(options.url, { ...options.init, signal: controller.signal });
  } finally {
    if (timer) {
      clearTimeout(timer);
    }
    options.signal?.removeEventListener("abort", onUpstreamAbort);
  }
}
