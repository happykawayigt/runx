import { existsSync } from "node:fs";
import { mkdir, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import process from "node:process";
import { spawn } from "node:child_process";

import { asRecord, errorMessage, firstNonEmpty, isNotFound, readOptionalFile, stringValue } from "@runxhq/core/util";

export type RegistryTrustTier = "first_party" | "verified" | "community";

export interface RegistrySkillVersion {
  readonly markdown: string;
  readonly profile_document?: string;
  readonly profile_digest?: string;
  readonly runner_names: readonly string[];
  readonly skill_id: string;
  readonly name: string;
  readonly version: string;
  readonly digest: string;
  readonly source_type: string;
  readonly trust_tier: RegistryTrustTier;
}

export interface RegistrySkillResolution {
  readonly markdown: string;
  readonly profile_document?: string;
  readonly profile_digest?: string;
  readonly runner_names: readonly string[];
  readonly skill_id: string;
  readonly name: string;
  readonly version: string;
  readonly digest: string;
  readonly source: "runx-registry";
  readonly source_label: "runx registry";
  readonly source_type: string;
  readonly trust_tier: RegistryTrustTier;
  readonly registry_url?: string;
  readonly add_command: string;
  readonly run_command: string;
}

export interface RegistryStore {
  readonly getVersion: (skillId: string, version?: string) => Promise<RegistrySkillVersion | undefined>;
  readonly listVersions: (skillId: string) => Promise<readonly RegistrySkillVersion[]>;
}

export interface ParsedRegistryRef {
  readonly kind: "registry";
  readonly skillId: string;
  readonly owner: string;
  readonly name: string;
  readonly version?: string;
  readonly raw: string;
}

const REGISTRY_REF_PATTERN = /^[a-z0-9][a-z0-9_-]*\/[a-z0-9][a-z0-9_-]*(?:@[a-z0-9._-]+)?$/i;

export function isRegistryRef(value: string): boolean {
  if (!value || value.length === 0) {
    return false;
  }
  if (value.startsWith("./") || value.startsWith("../") || value.startsWith("/")) {
    return false;
  }
  return REGISTRY_REF_PATTERN.test(value);
}

export function parseRegistryRef(value: string): ParsedRegistryRef {
  if (!isRegistryRef(value)) {
    throw new Error(
      `Invalid registry ref '${value}'. Expected '<owner>/<name>' or '<owner>/<name>@<version>'.`,
    );
  }
  const atIndex = value.lastIndexOf("@");
  const hasVersion = atIndex > 0;
  const skillId = hasVersion ? value.slice(0, atIndex) : value;
  const version = hasVersion ? value.slice(atIndex + 1) : undefined;
  const slashIndex = skillId.indexOf("/");
  const owner = skillId.slice(0, slashIndex);
  const name = skillId.slice(slashIndex + 1);
  return {
    kind: "registry",
    skillId,
    owner,
    name,
    version,
    raw: value,
  };
}

export interface MaterializeRegistrySkillOptions {
  readonly ref: string;
  readonly store?: RegistryStore;
  readonly cacheDir: string;
  readonly env?: NodeJS.ProcessEnv;
}

export interface MaterializedRegistrySkill {
  readonly skillDirectory: string;
  readonly skillPath: string;
  readonly resolution: RegistrySkillResolution;
}

export async function materializeRegistrySkill(
  options: MaterializeRegistrySkillOptions,
): Promise<MaterializedRegistrySkill> {
  const parsed = parseRegistryRef(options.ref);
  const resolution = await lookupRegistrySkill(options, parsed);

  if (!resolution) {
    const available = options.store ? await safeListVersions(options.store, parsed.skillId) : [];
    if (parsed.version && available.length > 0) {
      throw new Error(
        `Registry skill '${parsed.skillId}@${parsed.version}' not found (available: ${available.join(", ")}).`,
      );
    }
    throw new Error(`Registry skill '${parsed.skillId}' not found in registry.`);
  }

  const skillDirectory = cachePathFor(options.cacheDir, resolution);
  const skillPath = path.join(skillDirectory, "SKILL.md");
  const markerPath = path.join(skillDirectory, ".runx-registry-digest");
  const profilePath = path.join(skillDirectory, "X.yaml");
  const expectedMarker = `${JSON.stringify({
    digest: resolution.digest,
    profile_digest: resolution.profile_digest ?? null,
  })}\n`;
  const existingMarker = await readOptionalFile(markerPath);

  if (existingMarker !== expectedMarker || !existsSync(skillPath)) {
    await mkdir(skillDirectory, { recursive: true });
    await writeFile(skillPath, resolution.markdown, "utf8");
    if (resolution.profile_document) {
      await writeFile(profilePath, resolution.profile_document, "utf8");
    } else {
      await rm(profilePath, { force: true });
    }
    await writeFile(markerPath, expectedMarker, "utf8");
  }

  return { skillDirectory, skillPath, resolution };
}

export function defaultRegistrySkillCacheDir(env: NodeJS.ProcessEnv = process.env): string {
  const fromEnv = env.RUNX_SKILL_CACHE?.trim();
  if (fromEnv && fromEnv.length > 0) {
    return path.resolve(fromEnv);
  }
  return path.join(os.homedir(), ".runx", "cache", "skills");
}

export function nativeRegistryResolveRequested(env: NodeJS.ProcessEnv = process.env): boolean {
  return truthyEnv(env.RUNX_RUST_REGISTRY_RESOLVE);
}

async function lookupRegistrySkill(
  options: MaterializeRegistrySkillOptions,
  parsed: ParsedRegistryRef,
): Promise<RegistrySkillResolution | undefined> {
  try {
    const env = options.env ?? process.env;
    if (nativeRegistryResolveRequested(env)) {
      return await resolveRegistrySkillViaRustCli(parsed, env);
    }
    if (!options.store) {
      throw new Error("no registry store is configured and RUNX_RUST_REGISTRY_RESOLVE is not enabled");
    }
    return await resolveRegistrySkillFromStore(options.store, parsed);
  } catch (error) {
    const message = errorMessage(error);
    throw new Error(`Registry lookup failed for '${parsed.raw}': ${message}`);
  }
}

async function resolveRegistrySkillFromStore(
  store: RegistryStore,
  parsed: ParsedRegistryRef,
): Promise<RegistrySkillResolution | undefined> {
  const record = await store.getVersion(parsed.skillId, parsed.version);
  if (!record) {
    return undefined;
  }
  return {
    markdown: record.markdown,
    profile_document: record.profile_document,
    profile_digest: record.profile_digest,
    runner_names: record.runner_names,
    skill_id: record.skill_id,
    name: record.name,
    version: record.version,
    digest: record.digest,
    source: "runx-registry",
    source_label: "runx registry",
    source_type: record.source_type,
    trust_tier: record.trust_tier,
    add_command: `runx skill add ${record.skill_id}@${record.version}`,
    run_command: `runx skill ${record.name}`,
  };
}

async function safeListVersions(store: RegistryStore, skillId: string): Promise<string[]> {
  try {
    const versions = await store.listVersions(skillId);
    return versions.map((version) => version.version);
  } catch (error) {
    if (isNotFound(error)) {
      return [];
    }
    throw error;
  }
}

function cachePathFor(cacheDir: string, resolution: RegistrySkillResolution): string {
  const slashIndex = resolution.skill_id.indexOf("/");
  const owner = resolution.skill_id.slice(0, slashIndex);
  const name = resolution.skill_id.slice(slashIndex + 1);
  const digestSlug = resolution.digest.slice(0, 16);
  return path.join(cacheDir, owner, name, resolution.version, digestSlug);
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

async function resolveRegistrySkillViaRustCli(
  parsed: ParsedRegistryRef,
  env: NodeJS.ProcessEnv,
): Promise<RegistrySkillResolution | undefined> {
  const command = env.RUNX_RUST_REGISTRY_BIN;
  if (!command) {
    throw new Error("Rust registry resolve requires RUNX_RUST_REGISTRY_BIN when the native registry boundary is enabled.");
  }

  const args = ["registry", "resolve", parsed.skillId, "--json"];
  if (parsed.version) {
    args.push("--version", parsed.version);
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
    const output = firstNonEmpty(result.stderr, result.stdout, "no output");
    if (/registry skill not found/i.test(output)) {
      return undefined;
    }
    throw new Error(`Rust registry resolve failed with exit ${result.status}: ${output}`);
  }
  return parseRustRegistryResolveEnvelope(parseJson(result.stdout));
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
        reject(new Error(`Rust registry resolve timed out after ${options.timeoutMs}ms.`));
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
        reject(new Error(`Rust registry resolve timed out after ${options.timeoutMs}ms.`));
        return;
      }
      resolve({ status, stdout, stderr });
    });
  });
}

function parseJson(stdout: string): unknown {
  try {
    return JSON.parse(stdout);
  } catch (error) {
    throw new Error(`Rust registry resolve returned invalid JSON: ${errorMessage(error)}`);
  }
}

function parseRustRegistryResolveEnvelope(value: unknown): RegistrySkillResolution {
  const envelope = asRecord(value);
  const registry = asRecord(envelope?.registry);
  const resolution = asRecord(registry?.resolution);
  if (envelope?.status !== "success" || registry?.action !== "resolve" || !resolution) {
    throw new Error("Rust registry resolve returned an invalid resolve envelope.");
  }
  if (resolution.kind !== "local") {
    throw new Error("Rust registry resolve returned a non-local resolution without materialized skill content.");
  }
  return parseRustRegistrySkillResolution(resolution);
}

function parseRustRegistrySkillResolution(value: Record<string, unknown>): RegistrySkillResolution {
  const source = requiredString(value, "source");
  const sourceLabel = requiredString(value, "source_label");
  if (source !== "runx-registry" || sourceLabel !== "runx registry") {
    throw new Error("Rust registry resolve returned an invalid registry source.");
  }
  return {
    markdown: requiredString(value, "markdown"),
    profile_document: stringValue(value.profile_document),
    profile_digest: stringValue(value.profile_digest),
    runner_names: requiredStringArray(value, "runner_names"),
    skill_id: requiredString(value, "skill_id"),
    name: requiredString(value, "name"),
    version: requiredString(value, "version"),
    digest: requiredString(value, "digest"),
    source,
    source_label: sourceLabel,
    source_type: requiredString(value, "source_type"),
    trust_tier: requiredTrustTier(value, "trust_tier"),
    registry_url: stringValue(value.registry_url),
    add_command: requiredString(value, "add_command"),
    run_command: requiredString(value, "run_command"),
  };
}

function requiredString(record: Record<string, unknown>, field: string): string {
  const value = record[field];
  if (typeof value !== "string") {
    throw new Error(`Rust registry resolve returned invalid ${field}.`);
  }
  return value;
}

function requiredStringArray(record: Record<string, unknown>, field: string): readonly string[] {
  const value = record[field];
  if (!Array.isArray(value) || value.some((entry) => typeof entry !== "string")) {
    throw new Error(`Rust registry resolve returned invalid ${field}.`);
  }
  return value as readonly string[];
}

function requiredTrustTier(record: Record<string, unknown>, field: string): RegistryTrustTier {
  const value = requiredString(record, field);
  if (value !== "first_party" && value !== "verified" && value !== "community") {
    throw new Error(`Rust registry resolve returned invalid ${field}.`);
  }
  return value;
}

function parsePositiveInt(value: string | undefined): number | undefined {
  if (!value) return undefined;
  const parsed = Number.parseInt(value, 10);
  return Number.isFinite(parsed) && parsed > 0 ? parsed : undefined;
}

function truthyEnv(value: string | undefined): boolean {
  return value !== undefined && value !== "" && value !== "0";
}
