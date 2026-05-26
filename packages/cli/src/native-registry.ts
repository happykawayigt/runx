import type { SkillSearchResult } from "@runxhq/core/registry";
import { asRecord, errorMessage, firstNonEmpty } from "@runxhq/core/util";

import { runNativeRunx } from "./native-runx.js";

export interface NativeRegistryOptions {
  readonly env: NodeJS.ProcessEnv;
  readonly registryOverride?: string;
}

export async function searchRegistryViaRustCli(
  query: string,
  options: NativeRegistryOptions,
): Promise<readonly SkillSearchResult[]> {
  const args = ["registry", "search", query, "--json"];
  if (options.registryOverride) {
    args.push("--registry", options.registryOverride);
  }
  const result = await runNativeRegistryCommand("search", args, options.env);
  return parseRustRegistrySearchResults(parseJson(result.stdout, "search"));
}

interface NativeRegistryProcessResult {
  readonly status: number | null;
  readonly stdout: string;
  readonly stderr: string;
}

async function runNativeRegistryCommand(
  action: "search",
  args: readonly string[],
  env: NodeJS.ProcessEnv,
): Promise<NativeRegistryProcessResult> {
  const result = await runNativeRunx(args, {
    env,
    timeoutMs: parsePositiveInt(env.RUNX_RUST_REGISTRY_TIMEOUT_MS) ?? 10_000,
  });
  if (result.status !== 0) {
    throw new Error(
      `Rust registry ${action} failed with exit ${result.status}: ${firstNonEmpty(result.stderr, result.stdout, "no output")}`,
    );
  }
  return result;
}

function parseJson(stdout: string, action: string): unknown {
  try {
    return JSON.parse(stdout);
  } catch (error) {
    throw new Error(`Rust registry ${action} returned invalid JSON: ${errorMessage(error)}`);
  }
}

function parseRustRegistrySearchResults(value: unknown): readonly SkillSearchResult[] {
  const envelope = asRecord(value);
  const registry = asRecord(envelope?.registry);
  if (envelope?.status !== "success" || registry?.action !== "search" || !Array.isArray(registry.results)) {
    throw new Error("Rust registry search returned an invalid search envelope.");
  }
  return registry.results.map((result) => normalizeRustRegistrySearchResult(result));
}

function normalizeRustRegistrySearchResult(value: unknown): SkillSearchResult {
  const result = asRecord(value);
  const addCommand = stringField(result, "add_command") ?? stringField(result, "install_command");
  if (
    !result ||
    typeof result.skill_id !== "string" ||
    typeof result.name !== "string" ||
    typeof result.owner !== "string" ||
    result.source !== "runx-registry" ||
    typeof result.source_label !== "string" ||
    typeof result.source_type !== "string" ||
    typeof result.trust_tier !== "string" ||
    !Array.isArray(result.required_scopes) ||
    !Array.isArray(result.tags) ||
    typeof result.profile_mode !== "string" ||
    !Array.isArray(result.runner_names) ||
    typeof addCommand !== "string" ||
    typeof result.run_command !== "string"
  ) {
    throw new Error("Rust registry search returned an invalid result.");
  }
  return {
    ...result,
    add_command: addCommand,
  } as unknown as SkillSearchResult;
}

function parsePositiveInt(value: string | undefined): number | undefined {
  if (!value) return undefined;
  const parsed = Number.parseInt(value, 10);
  return Number.isFinite(parsed) && parsed > 0 ? parsed : undefined;
}

function stringField(value: Readonly<Record<string, unknown>> | undefined, key: string): string | undefined {
  const field = value?.[key];
  return typeof field === "string" ? field : undefined;
}
