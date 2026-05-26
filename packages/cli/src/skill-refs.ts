import { existsSync, readFileSync } from "node:fs";
import { mkdir, readFile, readdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  resolvePathFromUserInput,
  resolveRunxGlobalHomeDir,
  resolveRunxOfficialSkillsDir,
  resolveRunxProjectDir,
  resolveSkillInstallRoot,
} from "@runxhq/core/config";
import {
  createFixtureMarketplaceAdapter,
  searchMarketplaceAdapters,
} from "@runxhq/core/marketplaces";
import { acquireRegistrySkill, type AcquiredRegistrySkill, type SkillSearchResult } from "@runxhq/core/registry";

import { asRecord, errorMessage, hashString, readOptionalFile } from "@runxhq/core/util";

import { searchRegistryViaRustCli } from "./native-registry.js";
import { ensureRunxInstallState } from "./runx-state.js";

let cachedBundledSkillsDir: string | undefined | null = null;
let cachedOfficialSkillLock: readonly OfficialSkillLockEntry[] | undefined;

interface OfficialSkillLockEntry {
  readonly skill_id: string;
  readonly version: string;
  readonly digest: string;
}

interface ParsedRegistryRef {
  readonly kind: "registry";
  readonly skillId: string;
  readonly owner: string;
  readonly name: string;
  readonly version?: string;
  readonly raw: string;
}

interface OfficialSkillResolver {
  resolve(ref: ParsedRegistryRef): Promise<string | undefined>;
}

export function preferredRunCommand(skillName: string): string {
  return `runx skill ${skillName}`;
}

export async function runSkillSearch(
  query: string,
  sourceFilter: string | undefined,
  env: NodeJS.ProcessEnv,
  registryOverride?: string,
): Promise<readonly SkillSearchResult[]> {
  const results: SkillSearchResult[] = [];
  const normalizedSource = sourceFilter?.trim().toLowerCase();

  if (!normalizedSource || normalizedSource === "registry" || normalizedSource === "runx-registry") {
    results.push(...(await searchRegistryViaRustCli(query, { env, registryOverride })));
  }

  const marketplaceAdapters =
    env.RUNX_ENABLE_FIXTURE_MARKETPLACE === "1" &&
    (!normalizedSource || normalizedSource === "marketplace" || normalizedSource === "fixture-marketplace")
      ? [createFixtureMarketplaceAdapter()]
      : [];
  results.push(...(await searchMarketplaceAdapters(marketplaceAdapters, query)));

  if (!normalizedSource || normalizedSource === "bundled" || normalizedSource === "builtin") {
    results.push(...(await searchBundledSkills(query)));
  }

  return results;
}

export function resolveSkillReference(ref: string, env: NodeJS.ProcessEnv): string {
  const resolved = resolveLocalSkillReference(ref, env);
  if (resolved) {
    return resolved;
  }
  throw new Error(`Skill not found: ${ref}. Try \`runx skill search ${ref}\` to discover available skills.`);
}

export async function resolveRunnableSkillReference(ref: string, env: NodeJS.ProcessEnv): Promise<string> {
  const local = resolveLocalSkillReference(ref, env);
  if (local) {
    return local;
  }
  const official = officialSkillEntry(ref);
  if (!official) {
    throw new Error(`Skill not found: ${ref}. Try \`runx skill search ${ref}\` to discover available skills.`);
  }
  const globalHomeDir = resolveRunxGlobalHomeDir(env);
  const install = await ensureRunxInstallState(globalHomeDir);
  const registryBaseUrl = env.RUNX_REGISTRY_URL ?? "https://runx.ai";
  const cache = await ensureOfficialSkillCached({
    cacheRoot: resolveRunxOfficialSkillsDir(env),
    registryBaseUrl,
    installationId: install.state.installation_id,
    entry: official,
  });
  await rewriteOfficialSkillSiblingRefs(cache.skillPath, official.skill_id);
  return cache.skillPath;
}

export function createOfficialSkillResolver(env: NodeJS.ProcessEnv): OfficialSkillResolver {
  return {
    async resolve(parsed: ParsedRegistryRef): Promise<string | undefined> {
      const lock = loadOfficialSkillLock();
      const entry = lock.find((candidate) => candidate.skill_id === parsed.skillId);
      if (!entry) {
        return undefined;
      }
      if (parsed.version && entry.version !== parsed.version) {
        return undefined;
      }
      const globalHomeDir = resolveRunxGlobalHomeDir(env);
      const install = await ensureRunxInstallState(globalHomeDir);
      const registryBaseUrl = env.RUNX_REGISTRY_URL ?? "https://runx.ai";
      const cache = await ensureOfficialSkillCached({
        cacheRoot: resolveRunxOfficialSkillsDir(env),
        registryBaseUrl,
        installationId: install.state.installation_id,
        entry,
      });
      await rewriteOfficialSkillSiblingRefs(cache.skillPath, entry.skill_id);
      return cache.skillPath;
    },
  };
}

async function ensureOfficialSkillCached(options: {
  readonly cacheRoot: string;
  readonly registryBaseUrl: string;
  readonly installationId: string;
  readonly entry: OfficialSkillLockEntry;
}): Promise<{ readonly skillPath: string; readonly fromCache: boolean }> {
  const skillPath = officialSkillCachePath(options.cacheRoot, options.entry);
  const cachedMarkdown = await readOptionalFile(path.join(skillPath, "SKILL.md"));
  if (cachedMarkdown && hashString(cachedMarkdown) === options.entry.digest) {
    await syncPackagedOfficialSkillAssets(skillPath, options.entry.skill_id);
    await restoreOfficialRunnerManifestFromProfileState(skillPath);
    return { skillPath, fromCache: true };
  }

  const acquisition = await acquireRegistrySkill(options.entry.skill_id, {
    baseUrl: options.registryBaseUrl,
    installationId: options.installationId,
    version: options.entry.version,
    channel: "cli",
  });
  const computedDigest = hashString(acquisition.markdown);
  if (
    acquisition.version !== options.entry.version ||
    acquisition.digest !== options.entry.digest ||
    computedDigest !== options.entry.digest
  ) {
    throw new Error(
      `Official skill verification failed for ${options.entry.skill_id}: expected ${options.entry.version} sha256:${options.entry.digest}, received ${acquisition.version} sha256:${acquisition.digest} (computed sha256:${computedDigest}).`,
    );
  }

  await mkdir(skillPath, { recursive: true });
  await writeFile(path.join(skillPath, "SKILL.md"), acquisition.markdown, "utf8");
  await writeOfficialRunnerManifest(skillPath, acquisition);
  await writeOfficialProfileState(skillPath, acquisition);
  await syncPackagedOfficialSkillAssets(skillPath, acquisition.skill_id);
  return { skillPath, fromCache: false };
}

function officialSkillCachePath(cacheRoot: string, entry: OfficialSkillLockEntry): string {
  const [owner, name] = splitSkillId(entry.skill_id);
  return path.join(cacheRoot, owner, name, entry.version);
}

async function syncPackagedOfficialSkillAssets(targetSkillPath: string, skillId: string): Promise<void> {
  const packagedSkillDir = resolvePackagedOfficialSkillDir(skillId);
  if (!packagedSkillDir) {
    return;
  }
  const entries = await readdir(packagedSkillDir, { withFileTypes: true });
  for (const entry of entries) {
    if (!entry.isFile() || entry.name === "SKILL.md") {
      continue;
    }
    const sourcePath = path.join(packagedSkillDir, entry.name);
    const targetPath = path.join(targetSkillPath, entry.name);
    await mkdir(path.dirname(targetPath), { recursive: true });
    await writeFile(targetPath, await readFile(sourcePath));
  }
}

function resolvePackagedOfficialSkillDir(skillId: string): string | undefined {
  const bundledSkillsDir = resolveBundledSkillsDir();
  if (!bundledSkillsDir) {
    return undefined;
  }
  const [, name] = splitSkillId(skillId);
  const candidate = path.join(bundledSkillsDir, name);
  return existsSync(candidate) ? candidate : undefined;
}

async function writeOfficialProfileState(skillPath: string, acquisition: AcquiredRegistrySkill): Promise<void> {
  if (!acquisition.profile_document) {
    return;
  }
  const profileStatePath = path.join(skillPath, ".runx", "profile.json");
  await mkdir(path.dirname(profileStatePath), { recursive: true });
  await writeFile(
    profileStatePath,
    `${JSON.stringify(
      {
        schema_version: "runx.skill-profile.v1",
        skill: {
          name: acquisition.name,
          path: "SKILL.md",
          digest: acquisition.digest,
        },
        profile: {
          document: acquisition.profile_document,
          digest: acquisition.profile_digest,
          runner_names: acquisition.runner_names,
        },
        origin: {
          source: "runx-registry",
          skill_id: acquisition.skill_id,
          version: acquisition.version,
        },
      },
      null,
      2,
    )}\n`,
    "utf8",
  );
}

async function writeOfficialRunnerManifest(skillPath: string, acquisition: AcquiredRegistrySkill): Promise<void> {
  const document = acquisition.profile_document;
  if (!document) {
    return;
  }
  verifyProfileDigest(acquisition.skill_id, document, acquisition.profile_digest);
  await writeFile(path.join(skillPath, "X.yaml"), document, "utf8");
}

async function restoreOfficialRunnerManifestFromProfileState(skillPath: string): Promise<void> {
  const manifestPath = path.join(skillPath, "X.yaml");
  if (existsSync(manifestPath)) {
    return;
  }
  const stateRaw = await readOptionalFile(path.join(skillPath, ".runx", "profile.json"));
  if (!stateRaw) {
    return;
  }
  const state = asRecord(JSON.parse(stateRaw));
  const origin = asRecord(state?.origin);
  const profile = asRecord(state?.profile);
  const document = profile?.document;
  if (typeof document !== "string" || document.length === 0) {
    return;
  }
  verifyProfileDigest(
    typeof origin?.skill_id === "string" ? origin.skill_id : "official skill",
    document,
    typeof profile?.digest === "string" ? profile.digest : undefined,
  );
  await writeFile(manifestPath, document, "utf8");
}

function verifyProfileDigest(skillId: string, document: string, expectedDigest: string | undefined): void {
  if (!expectedDigest) {
    return;
  }
  const actualDigest = hashString(document);
  if (actualDigest !== expectedDigest) {
    throw new Error(
      `Official skill profile verification failed for ${skillId}: expected sha256:${expectedDigest}, computed sha256:${actualDigest}.`,
    );
  }
}

function splitSkillId(skillId: string): readonly [string, string] {
  const parts = skillId.split("/");
  if (parts.length !== 2 || !parts[0] || !parts[1]) {
    throw new Error(`Invalid registry skill id '${skillId}'. Expected '<owner>/<name>'.`);
  }
  return [parts[0], parts[1]];
}

const SIBLING_SKILL_REF_PATTERN = /(\bskill:\s*)\.\.\/([A-Za-z0-9][A-Za-z0-9_-]*)\b/g;

export function rewriteSiblingSkillRefs(
  text: string,
  owner: string,
  siblingVersions: ReadonlyMap<string, string>,
): { readonly text: string; readonly didRewrite: boolean } {
  let didRewrite = false;
  const out = text.replace(SIBLING_SKILL_REF_PATTERN, (match, prefix, siblingName) => {
    const siblingVersion = siblingVersions.get(siblingName);
    if (!siblingVersion) {
      return match;
    }
    didRewrite = true;
    return `${prefix}${owner}/${siblingName}@${siblingVersion}`;
  });
  return { text: out, didRewrite };
}

async function rewriteOfficialSkillSiblingRefs(skillDir: string, ownerSkillId: string): Promise<void> {
  const owner = ownerSkillId.split("/")[0];
  if (!owner) {
    return;
  }
  const lock = loadOfficialSkillLock();
  const lockBySiblingName = new Map<string, string>();
  for (const entry of lock) {
    const [entryOwner, entryName] = entry.skill_id.split("/");
    if (entryOwner === owner && entryName) {
      lockBySiblingName.set(entryName, entry.version);
    }
  }
  if (lockBySiblingName.size === 0) {
    return;
  }

  const profilePath = path.join(skillDir, "X.yaml");
  if (existsSync(profilePath)) {
    const original = await readFile(profilePath, "utf8");
    const { text: rewritten, didRewrite } = rewriteSiblingSkillRefs(original, owner, lockBySiblingName);
    if (didRewrite) {
      await writeFile(profilePath, rewritten);
    }
  }

  const profileStatePath = path.join(skillDir, ".runx", "profile.json");
  if (existsSync(profileStatePath)) {
    const stateText = await readFile(profileStatePath, "utf8");
    const state = asRecord(JSON.parse(stateText));
    const profile = asRecord(state?.profile);
    const document = profile?.document;
    if (state && typeof document === "string") {
      const { text: rewrittenDocument, didRewrite } = rewriteSiblingSkillRefs(document, owner, lockBySiblingName);
      if (didRewrite) {
        const nextState = {
          ...state,
          profile: { ...(profile ?? {}), document: rewrittenDocument },
        };
        await writeFile(profileStatePath, `${JSON.stringify(nextState, null, 2)}\n`);
      }
    }
  }
}

async function searchBundledSkills(query: string): Promise<readonly SkillSearchResult[]> {
  const bundledDir = resolveBundledSkillsDir();
  if (!bundledDir || !existsSync(bundledDir)) return [];
  const entries = await readdir(bundledDir, { withFileTypes: true });
  const needle = query.trim().toLowerCase();
  const out: SkillSearchResult[] = [];
  for (const entry of entries) {
    if (!entry.isDirectory()) continue;
    const skillMdPath = path.join(bundledDir, entry.name, "SKILL.md");
    if (!existsSync(skillMdPath)) continue;
    const raw = await readFile(skillMdPath, "utf8");
    const { name, description } = parseSkillFrontmatter(raw, entry.name);
    const hay = `${name}\n${description}`.toLowerCase();
    if (needle && !hay.includes(needle)) continue;
    const hasProfile = existsSync(path.join(path.dirname(bundledDir), "bindings", "runx", entry.name, "X.yaml"));
    out.push({
      skill_id: `runx/${name}`,
      name,
      summary: description,
      owner: "runx",
      source: "runx-registry",
      source_label: "runx (bundled)",
      source_type: "bundled",
      trust_tier: "first_party",
      required_scopes: [],
      tags: [],
      profile_mode: hasProfile ? "profiled" : "portable",
      runner_names: [],
      add_command: `runx skill add runx/${name}`,
      run_command: preferredRunCommand(name),
    });
  }
  return out;
}

function resolveBundledSkillsDir(): string | undefined {
  if (cachedBundledSkillsDir !== null) return cachedBundledSkillsDir ?? undefined;
  try {
    // Walk up from the compiled entry looking for the @runxhq/cli package root,
    // which owns a `skills/` sibling. Works across dev (src/), dist wrapper,
    // and nested-dist layouts without sentinel files.
    let dir = path.dirname(fileURLToPath(import.meta.url));
    for (let i = 0; i < 8; i += 1) {
      const pkgJsonPath = path.join(dir, "package.json");
      if (existsSync(pkgJsonPath)) {
        try {
          const pkg = JSON.parse(readFileSync(pkgJsonPath, "utf8"));
          if (pkg && pkg.name === "@runxhq/cli") {
            const skills = path.join(dir, "skills");
            cachedBundledSkillsDir = existsSync(skills) ? skills : undefined;
            return cachedBundledSkillsDir ?? undefined;
          }
        } catch {
          // ignore and keep walking
        }
      }
      const parent = path.dirname(dir);
      if (parent === dir) break;
      dir = parent;
    }
    cachedBundledSkillsDir = undefined;
    return undefined;
  } catch {
    cachedBundledSkillsDir = undefined;
    return undefined;
  }
}

function officialSkillEntry(ref: string): OfficialSkillLockEntry | undefined {
  if (!/^[A-Za-z0-9_.-]+$/.test(ref)) {
    return undefined;
  }
  return loadOfficialSkillLock().find((entry) => entry.skill_id === `runx/${ref}`);
}

function loadOfficialSkillLock(): readonly OfficialSkillLockEntry[] {
  if (cachedOfficialSkillLock) {
    return cachedOfficialSkillLock;
  }
  const lockUrl = new URL("./official-skills.lock.json", import.meta.url);
  let raw: string;
  try {
    raw = readFileSync(lockUrl, "utf8");
  } catch (error) {
    throw new Error(
      `Official skills lock file is missing at ${lockUrl.href}. The CLI install may be incomplete; reinstall to restore it. (${errorMessage(error)})`,
      { cause: error },
    );
  }
  let parsed: unknown;
  try {
    parsed = JSON.parse(raw);
  } catch (error) {
    throw new Error(
      `Official skills lock file at ${lockUrl.href} is not valid JSON: ${errorMessage(error)}`,
      { cause: error },
    );
  }
  if (!Array.isArray(parsed)) {
    throw new Error(`Official skills lock file at ${lockUrl.href} must contain a JSON array.`);
  }
  cachedOfficialSkillLock = parsed as readonly OfficialSkillLockEntry[];
  return cachedOfficialSkillLock;
}

function resolveLocalSkillReference(ref: string, env: NodeJS.ProcessEnv): string | undefined {
  if (!ref) {
    throw new Error("Missing skill reference.");
  }
  const looksLikePath = ref.includes("/") || ref.includes(path.sep) || ref.startsWith(".") || ref.startsWith("~");
  if (looksLikePath) {
    const resolved = resolvePathFromUserInput(ref, env);
    assertSkillReferencePath(resolved);
    return resolved;
  }
  const directCandidate = resolvePathFromUserInput(ref, env);
  if (existsSync(directCandidate)) {
    assertSkillReferencePath(directCandidate);
    return directCandidate;
  }

  const projectSkillDir = path.join(resolveRunxProjectDir(env), "skills", ref);
  if (existsSync(path.join(projectSkillDir, "SKILL.md"))) {
    return projectSkillDir;
  }

  const installedSkillDir = path.join(resolveSkillInstallRoot(env), ref);
  if (existsSync(path.join(installedSkillDir, "SKILL.md"))) {
    return installedSkillDir;
  }

  return undefined;
}

function assertSkillReferencePath(resolved: string): void {
  if (path.extname(resolved).toLowerCase() === ".md" && path.basename(resolved).toLowerCase() !== "skill.md") {
    throw new Error(
      `Skill references must point to a skill package directory or SKILL.md. Flat markdown files are not supported: ${resolved}`,
    );
  }
}

function parseSkillFrontmatter(raw: string, fallbackName: string): { name: string; description: string } {
  const match = raw.match(/^---\n([\s\S]*?)\n---/);
  let name = fallbackName;
  let description = "";
  if (match) {
    for (const line of match[1].split("\n")) {
      const kv = line.match(/^(name|description):\s*(.*)$/);
      if (!kv) continue;
      const value = kv[2].trim().replace(/^["']|["']$/g, "");
      if (kv[1] === "name") name = value || fallbackName;
      else if (kv[1] === "description") description = value;
    }
  }
  return { name, description };
}
