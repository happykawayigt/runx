import { existsSync, readFileSync } from "node:fs";
import { mkdir, readdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  asRecord,
  fetchWithTimeout,
  hashString,
  readOptionalFile,
  optionalString,
  requireArray,
  requireEnum,
  requireRecord,
  requireString,
} from "@runxhq/core/util";

export type OfficialRegistryTrustTier = "first_party" | "verified" | "community";
export type OfficialRegistryPublisherKind = "organization" | "user" | "team" | "service" | "publisher";
export type OfficialRegistryAttestationKind = "source" | "publisher" | "verification";
export type OfficialRegistryAttestationStatus = "verified" | "declared";

export interface OfficialRegistryPublisher {
  readonly kind: OfficialRegistryPublisherKind;
  readonly id: string;
  readonly handle?: string;
  readonly display_name?: string;
  readonly url?: string;
}

export interface OfficialRegistryAttestation {
  readonly kind: OfficialRegistryAttestationKind;
  readonly id: string;
  readonly status: OfficialRegistryAttestationStatus;
  readonly summary: string;
  readonly source?: string;
  readonly issued_at?: string;
  readonly metadata?: Readonly<Record<string, unknown>>;
}

export interface AcquiredRegistrySkill {
  readonly skill_id: string;
  readonly owner: string;
  readonly name: string;
  readonly version: string;
  readonly digest: string;
  readonly markdown: string;
  readonly profile_document?: string;
  readonly profile_digest?: string;
  readonly runner_names: readonly string[];
  readonly trust_tier: OfficialRegistryTrustTier;
  readonly publisher: OfficialRegistryPublisher;
  readonly source_metadata?: Readonly<Record<string, unknown>>;
  readonly attestations: readonly OfficialRegistryAttestation[];
  readonly install_count: number;
}

export interface OfficialSkillLockEntry {
  readonly skill_id: string;
  readonly version: string;
  readonly digest: string;
}

export interface EnsureOfficialSkillCachedOptions {
  readonly cacheRoot: string;
  readonly registryBaseUrl: string;
  readonly installationId: string;
  readonly entry: OfficialSkillLockEntry;
  readonly fetchImpl?: typeof fetch;
}

export async function ensureOfficialSkillCached(
  options: EnsureOfficialSkillCachedOptions,
): Promise<{
  readonly skillPath: string;
  readonly fromCache: boolean;
  readonly acquisition: AcquiredRegistrySkill;
}> {
  const skillPath = officialSkillCachePath(options.cacheRoot, options.entry);
  const cachedMarkdown = await readOptionalFile(path.join(skillPath, "SKILL.md"));
  if (cachedMarkdown && hashString(cachedMarkdown) === options.entry.digest) {
    await syncPackagedRuntimeAssets(skillPath, options.entry.skill_id);
    const publisher = defaultOfficialRegistryPublisher(ownerFromSkillId(options.entry.skill_id));
    return {
      skillPath,
      fromCache: true,
      acquisition: {
        skill_id: options.entry.skill_id,
        owner: ownerFromSkillId(options.entry.skill_id),
        name: nameFromSkillId(options.entry.skill_id),
        version: options.entry.version,
        digest: options.entry.digest,
        markdown: cachedMarkdown,
        profile_document: await readProfileDocumentState(skillPath),
        trust_tier: "first_party",
        publisher,
        attestations: buildPublisherAttestations(publisher, "first_party", new Date().toISOString()),
        runner_names: [],
        install_count: 0,
      },
    };
  }

  const acquisition = await acquireOfficialRegistrySkill(options.entry.skill_id, {
    baseUrl: options.registryBaseUrl,
    installationId: options.installationId,
    version: options.entry.version,
    fetchImpl: options.fetchImpl,
  });
  const computedDigest = hashString(acquisition.markdown);
  if (
    acquisition.version !== options.entry.version
    || acquisition.digest !== options.entry.digest
    || computedDigest !== options.entry.digest
  ) {
    throw new Error(
      `Official skill verification failed for ${options.entry.skill_id}: expected ${options.entry.version} sha256:${options.entry.digest}, received ${acquisition.version} sha256:${acquisition.digest} (computed sha256:${computedDigest}).`,
    );
  }

  await mkdir(skillPath, { recursive: true });
  await writeFile(path.join(skillPath, "SKILL.md"), acquisition.markdown, "utf8");
  await writeProfileState(skillPath, acquisition);
  await syncPackagedRuntimeAssets(skillPath, acquisition.skill_id);
  return {
    skillPath,
    fromCache: false,
    acquisition,
  };
}

export function officialSkillCachePath(cacheRoot: string, entry: OfficialSkillLockEntry): string {
  const [owner, name] = splitSkillId(entry.skill_id);
  return path.join(cacheRoot, owner, name, entry.version);
}

let cachedCliSkillsRoot: string | undefined | null;

function ownerFromSkillId(skillId: string): string {
  return splitSkillId(skillId)[0];
}

function nameFromSkillId(skillId: string): string {
  return splitSkillId(skillId)[1];
}

async function syncPackagedRuntimeAssets(targetSkillPath: string, skillId: string): Promise<void> {
  const packagedSkillDir = resolvePackagedOfficialSkillDir(skillId);
  if (!packagedSkillDir) {
    return;
  }
  const entries = await readdir(packagedSkillDir, { withFileTypes: true });
  for (const entry of entries) {
    if (!entry.isFile()) {
      continue;
    }
    if (entry.name === "SKILL.md") {
      continue;
    }
    const sourcePath = path.join(packagedSkillDir, entry.name);
    const targetPath = path.join(targetSkillPath, entry.name);
    await mkdir(path.dirname(targetPath), { recursive: true });
    await writeFile(targetPath, await readFile(sourcePath));
  }
}

function resolvePackagedOfficialSkillDir(skillId: string): string | undefined {
  const skillsRoot = resolveCliSkillsRoot();
  if (!skillsRoot) {
    return undefined;
  }
  const candidate = path.join(skillsRoot, nameFromSkillId(skillId));
  return existsSync(candidate) ? candidate : undefined;
}

function resolveCliSkillsRoot(): string | undefined {
  if (cachedCliSkillsRoot !== undefined) {
    return cachedCliSkillsRoot ?? undefined;
  }
  let dir = path.dirname(fileURLToPath(import.meta.url));
  for (let index = 0; index < 10; index += 1) {
    const packageJsonPath = path.join(dir, "package.json");
    if (existsSync(packageJsonPath) && readPackageName(packageJsonPath) === "@runxhq/cli") {
      const skillsRoot = path.join(dir, "skills");
      cachedCliSkillsRoot = existsSync(skillsRoot) ? skillsRoot : null;
      return cachedCliSkillsRoot ?? undefined;
    }
    const parent = path.dirname(dir);
    if (parent === dir) {
      break;
    }
    dir = parent;
  }
  dir = path.dirname(fileURLToPath(import.meta.url));
  for (let index = 0; index < 10; index += 1) {
    const skillsRoot = path.join(dir, "skills");
    if (existsSync(skillsRoot)) {
      cachedCliSkillsRoot = skillsRoot;
      return skillsRoot;
    }
    const parent = path.dirname(dir);
    if (parent === dir) {
      break;
    }
    dir = parent;
  }
  cachedCliSkillsRoot = null;
  return undefined;
}

function readPackageName(packageJsonPath: string): string | undefined {
  const pkg = asRecord(JSON.parse(readFileSync(packageJsonPath, "utf8")));
  return typeof pkg?.name === "string" ? pkg.name : undefined;
}

interface AcquireOfficialRegistrySkillOptions {
  readonly baseUrl: string;
  readonly installationId: string;
  readonly version?: string;
  readonly fetchImpl?: typeof fetch;
}

async function acquireOfficialRegistrySkill(
  skillId: string,
  options: AcquireOfficialRegistrySkillOptions,
): Promise<AcquiredRegistrySkill> {
  const [owner, name] = splitSkillId(skillId);
  const response = await fetchWithTimeout({
    fetchImpl: options.fetchImpl,
    url: `${options.baseUrl.replace(/\/$/, "")}/v1/skills/${encodeURIComponent(owner)}/${encodeURIComponent(name)}/acquire`,
    init: {
      method: "POST",
      headers: {
        "content-type": "application/json",
      },
      body: JSON.stringify({
        installation_id: options.installationId,
        version: options.version,
        channel: "cli",
      }),
    },
    description: `Registry acquire for ${skillId}`,
  });

  if (!response.ok) {
    throw new Error(`Registry acquire failed for ${skillId}: HTTP ${response.status}`);
  }

  const payload = requireRecord(await response.json(), `Registry acquire payload for ${skillId}`);
  if (payload.status !== "success") {
    throw new Error(`Registry acquire returned an invalid payload for ${skillId}.`);
  }
  const acquisition = requireRecord(payload.acquisition, `Registry acquire acquisition for ${skillId}`);
  const runnerNames = requireArray(acquisition.runner_names, "remote_registry.acquisition.runner_names").map((entry) => {
    if (typeof entry !== "string") {
      throw new Error("remote_registry.acquisition.runner_names must be an array of strings.");
    }
    return entry;
  });

  return {
    skill_id: requireString(acquisition.skill_id, "remote_registry.acquisition.skill_id"),
    owner: requireString(acquisition.owner, "remote_registry.acquisition.owner"),
    name: requireString(acquisition.name, "remote_registry.acquisition.name"),
    version: requireString(acquisition.version, "remote_registry.acquisition.version"),
    digest: requireString(acquisition.digest, "remote_registry.acquisition.digest"),
    markdown: requireString(acquisition.markdown, "remote_registry.acquisition.markdown"),
    profile_document: optionalString(acquisition.profile_document, "remote_registry.acquisition.profile_document"),
    profile_digest: optionalString(acquisition.profile_digest, "remote_registry.acquisition.profile_digest"),
    runner_names: runnerNames,
    trust_tier: requireEnum(acquisition.trust_tier, ["first_party", "verified", "community"], "remote_registry.acquisition.trust_tier"),
    publisher: validateOfficialRegistryPublisher(acquisition.publisher, "remote_registry.acquisition.publisher"),
    source_metadata: optionalRecord(acquisition.source_metadata, "remote_registry.acquisition.source_metadata"),
    attestations: validateOfficialRegistryAttestations(acquisition.attestations, "remote_registry.acquisition.attestations"),
    install_count: typeof payload.install_count === "number" ? payload.install_count : 0,
  };
}

function splitSkillId(skillId: string): readonly [string, string] {
  const parts = skillId.split("/");
  if (parts.length !== 2 || !parts[0] || !parts[1]) {
    throw new Error(`Invalid registry skill id '${skillId}'. Expected '<owner>/<name>'.`);
  }
  return [parts[0], parts[1]];
}

function defaultOfficialRegistryPublisher(owner: string): OfficialRegistryPublisher {
  return owner === "runx"
    ? { kind: "organization", id: owner, handle: owner }
    : { kind: "publisher", id: owner, handle: owner };
}

function buildPublisherAttestations(
  publisher: OfficialRegistryPublisher,
  trustTier: OfficialRegistryTrustTier,
  issuedAt: string,
): readonly OfficialRegistryAttestation[] {
  const label = publisher.display_name ?? publisher.handle ?? publisher.id;
  return [
    {
      kind: "publisher",
      id: `publisher:${publisher.id}`,
      status: trustTier === "community" ? "declared" : "verified",
      summary: label,
      issued_at: issuedAt,
      metadata: {
        publisher_id: publisher.id,
        publisher_kind: publisher.kind,
        publisher_handle: publisher.handle,
        publisher_display_name: publisher.display_name,
        trust_tier: trustTier,
      },
    },
  ];
}

function validateOfficialRegistryPublisher(value: unknown, label: string): OfficialRegistryPublisher {
  const record = requireRecord(value, label);
  return {
    kind: requireEnum(record.kind, ["organization", "user", "team", "service", "publisher"], `${label}.kind`),
    id: requireString(record.id, `${label}.id`),
    handle: optionalString(record.handle, `${label}.handle`),
    display_name: optionalString(record.display_name, `${label}.display_name`),
    url: optionalString(record.url, `${label}.url`),
  };
}

function validateOfficialRegistryAttestations(value: unknown, label: string): readonly OfficialRegistryAttestation[] {
  return requireArray(value, label).map((entry, index) => {
    const record = requireRecord(entry, `${label}[${index}]`);
    return {
      kind: requireEnum(record.kind, ["source", "publisher", "verification"], `${label}[${index}].kind`),
      id: requireString(record.id, `${label}[${index}].id`),
      status: requireEnum(record.status, ["verified", "declared"], `${label}[${index}].status`),
      summary: requireString(record.summary, `${label}[${index}].summary`),
      source: optionalString(record.source, `${label}[${index}].source`),
      issued_at: optionalString(record.issued_at, `${label}[${index}].issued_at`),
      metadata: optionalRecord(record.metadata, `${label}[${index}].metadata`),
    };
  });
}

function optionalRecord(value: unknown, label: string): Readonly<Record<string, unknown>> | undefined {
  if (value === undefined) {
    return undefined;
  }
  return requireRecord(value, label);
}

async function readProfileDocumentState(skillPath: string): Promise<string | undefined> {
  const state = await readOptionalFile(path.join(skillPath, ".runx", "profile.json"));
  if (!state) {
    return undefined;
  }
  const parsed = asRecord(JSON.parse(state));
  const profile = asRecord(parsed?.profile);
  return typeof profile?.document === "string" ? profile.document : undefined;
}

async function writeProfileState(skillPath: string, acquisition: AcquiredRegistrySkill): Promise<void> {
  const profileStatePath = path.join(skillPath, ".runx", "profile.json");
  if (!acquisition.profile_document) {
    return;
  }
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
