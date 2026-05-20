import { spawn } from "node:child_process";
import process from "node:process";

import type { SkillSearchResult } from "@runxhq/core/marketplaces";
import { asRecord, errorMessage } from "@runxhq/core/util";

export interface NativeRegistryOptions {
  readonly env: NodeJS.ProcessEnv;
  readonly registryOverride?: string;
}

export interface NativeRegistryInstallOptions extends NativeRegistryOptions {
  readonly ref: string;
  readonly destinationRoot: string;
  readonly version?: string;
  readonly expectedDigest?: string;
  readonly installationId?: string;
}

export interface NativeRegistryInstallResult {
  readonly status: "installed" | "unchanged";
  readonly destination: string;
  readonly skill_name: string;
  readonly source: string;
  readonly source_label: string;
  readonly skill_id?: string;
  readonly version?: string;
  readonly digest: string;
  readonly profileDigest?: string;
  readonly profileStatePath?: string;
  readonly runnerNames: readonly string[];
  readonly trust_tier?: string;
}

export function nativeRegistrySearchRequested(env: NodeJS.ProcessEnv): boolean {
  return truthyEnv(env.RUNX_RUST_REGISTRY_SEARCH);
}

export function nativeRegistryInstallRequested(env: NodeJS.ProcessEnv): boolean {
  return truthyEnv(env.RUNX_RUST_REGISTRY_INSTALL);
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

export async function installRegistryViaRustCli(
  options: NativeRegistryInstallOptions,
): Promise<NativeRegistryInstallResult> {
  const args = ["registry", "install", options.ref, "--json", "--to", options.destinationRoot];
  if (options.registryOverride) {
    args.push("--registry", options.registryOverride);
  }
  if (options.version) {
    args.push("--version", options.version);
  }
  if (options.expectedDigest) {
    args.push("--digest", options.expectedDigest);
  }
  if (options.installationId) {
    args.push("--installation-id", options.installationId);
  }

  const result = await runNativeRegistryCommand("install", args, options.env);
  return parseRustRegistryInstallResult(parseJson(result.stdout, "install"));
}

interface SpawnRegistryProcessOptions {
  readonly command: string;
  readonly args: readonly string[];
  readonly cwd: string;
  readonly env: NodeJS.ProcessEnv;
  readonly timeoutMs: number;
}

interface SpawnRegistryProcessResult {
  readonly status: number | null;
  readonly stdout: string;
  readonly stderr: string;
}

async function runNativeRegistryCommand(
  action: "search" | "install",
  args: readonly string[],
  env: NodeJS.ProcessEnv,
): Promise<SpawnRegistryProcessResult> {
  const command = env.RUNX_RUST_REGISTRY_BIN;
  if (!command) {
    throw new Error(`Rust registry ${action} requires RUNX_RUST_REGISTRY_BIN when the native registry boundary is enabled.`);
  }
  const result = await spawnRegistryProcess({
    command,
    args,
    env: {
      ...process.env,
      ...env,
      NO_COLOR: "1",
      RUNX_RUST_CLI: "1",
    },
    cwd: env.RUNX_CWD || process.cwd(),
    timeoutMs: parsePositiveInt(env.RUNX_RUST_REGISTRY_TIMEOUT_MS) ?? 10_000,
  });
  if (result.status !== 0) {
    throw new Error(
      `Rust registry ${action} failed with exit ${result.status}: ${firstNonEmpty(result.stderr, result.stdout, "no output")}`,
    );
  }
  return result;
}

function spawnRegistryProcess(options: SpawnRegistryProcessOptions): Promise<SpawnRegistryProcessResult> {
  return new Promise((resolve, reject) => {
    const child = spawn(options.command, options.args, {
      cwd: options.cwd,
      env: options.env,
      stdio: ["ignore", "pipe", "pipe"],
    });
    let settled = false;
    let timedOut = false;
    let stdout = "";
    let stderr = "";
    let killTimer: NodeJS.Timeout | undefined;

    const timer = setTimeout(() => {
      if (settled) return;
      timedOut = true;
      child.kill("SIGTERM");
      killTimer = setTimeout(() => {
        child.kill("SIGKILL");
        if (settled) return;
        settled = true;
        reject(new Error(`Rust registry ${options.args[1] ?? "command"} timed out after ${options.timeoutMs}ms.`));
      }, 1_000);
    }, options.timeoutMs);

    const clearTimers = () => {
      clearTimeout(timer);
      if (killTimer) clearTimeout(killTimer);
    };

    child.stdout.setEncoding("utf8");
    child.stderr.setEncoding("utf8");
    child.stdout.on("data", (chunk: string) => {
      stdout += chunk;
    });
    child.stderr.on("data", (chunk: string) => {
      stderr += chunk;
    });
    child.on("error", (error) => {
      if (settled) return;
      settled = true;
      clearTimers();
      reject(new Error(`Failed to spawn Rust registry command '${options.command}': ${error.message}`));
    });
    child.on("close", (status) => {
      if (settled) return;
      settled = true;
      clearTimers();
      if (timedOut) {
        reject(new Error(`Rust registry ${options.args[1] ?? "command"} timed out after ${options.timeoutMs}ms.`));
        return;
      }
      resolve({ status, stdout, stderr });
    });
  });
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
  for (const result of registry.results) {
    assertSkillSearchResult(result);
  }
  return registry.results as readonly SkillSearchResult[];
}

function parseRustRegistryInstallResult(value: unknown): NativeRegistryInstallResult {
  const envelope = asRecord(value);
  const registry = asRecord(envelope?.registry);
  const install = asRecord(registry?.install);
  if (envelope?.status !== "success" || registry?.action !== "install" || !install) {
    throw new Error("Rust registry install returned an invalid install envelope.");
  }
  if (
    (install.status !== "installed" && install.status !== "unchanged") ||
    typeof install.destination !== "string" ||
    typeof install.skill_name !== "string" ||
    typeof install.source !== "string" ||
    typeof install.source_label !== "string" ||
    typeof install.digest !== "string" ||
    !Array.isArray(install.runner_names)
  ) {
    throw new Error("Rust registry install returned an invalid install result.");
  }
  return {
    status: install.status,
    destination: install.destination,
    skill_name: install.skill_name,
    source: install.source,
    source_label: install.source_label,
    skill_id: optionalString(install.skill_id),
    version: optionalString(install.version),
    digest: install.digest,
    profileDigest: optionalString(install.profile_digest),
    profileStatePath: optionalString(install.profile_state_path),
    runnerNames: install.runner_names.filter((runnerName): runnerName is string => typeof runnerName === "string"),
    trust_tier: optionalString(install.trust_tier),
  };
}

function assertSkillSearchResult(value: unknown): void {
  const result = asRecord(value);
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
    typeof result.add_command !== "string" ||
    typeof result.run_command !== "string"
  ) {
    throw new Error("Rust registry search returned an invalid result.");
  }
}

function parsePositiveInt(value: string | undefined): number | undefined {
  if (!value) return undefined;
  const parsed = Number.parseInt(value, 10);
  return Number.isFinite(parsed) && parsed > 0 ? parsed : undefined;
}

function firstNonEmpty(...values: readonly string[]): string {
  for (const value of values) {
    const trimmed = value.trim();
    if (trimmed) return trimmed;
  }
  return "";
}

function optionalString(value: unknown): string | undefined {
  return typeof value === "string" ? value : undefined;
}

function truthyEnv(value: string | undefined): boolean {
  return value !== undefined && value !== "" && value !== "0";
}
