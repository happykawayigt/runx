import { constants as fsConstants } from "node:fs";
import { access, mkdir, rename, rm, writeFile } from "node:fs/promises";
import path from "node:path";

import { isMarketplaceRef, resolveMarketplaceSkill, type MarketplaceAdapter } from "@runxhq/core/marketplaces";
import { asRecord, fetchWithTimeout, hashString, isNotFound, readOptionalFile } from "@runxhq/core/util";
import type { SkillInstallOrigin } from "../parser-types.js";
import {
  validateRunnerManifestYamlViaParser,
  validateSkillInstallViaParser,
} from "./parser-bridge.js";

type RegistryTrustTier = "first_party" | "verified" | "community";

export interface SkillInstallRegistrySkillVersion {
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

export interface SkillInstallRegistrySkill {
  readonly skill_id: string;
  readonly name: string;
}

export interface SkillInstallRegistryStore {
  readonly getVersion: (skillId: string, version?: string) => Promise<SkillInstallRegistrySkillVersion | undefined>;
  readonly listSkills?: () => Promise<readonly SkillInstallRegistrySkill[]>;
}

export interface InstallLocalSkillOptions {
  readonly ref: string;
  readonly registryStore?: SkillInstallRegistryStore;
  readonly marketplaceAdapters?: readonly MarketplaceAdapter[];
  readonly destinationRoot: string;
  readonly version?: string;
  readonly expectedDigest?: string;
  readonly registryUrl?: string;
  readonly installationId?: string;
  readonly env?: NodeJS.ProcessEnv;
}

export interface InstallLocalSkillResult {
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

interface FetchedInstallCandidate {
  readonly markdown: string;
  readonly profileDocument?: string;
  readonly origin: SkillInstallOrigin;
}

interface RegistrySkillResolution {
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

export async function installLocalSkill(options: InstallLocalSkillOptions): Promise<InstallLocalSkillResult> {
  const candidate = await fetchInstallCandidate(options);
  const actualDigest = hashString(candidate.markdown);
  const expectedDigest = options.expectedDigest ?? candidate.origin.digest;

  if (expectedDigest && expectedDigest !== actualDigest) {
    throw new Error(
      `Digest mismatch for ${options.ref}: expected sha256:${expectedDigest}, received sha256:${actualDigest}.`,
    );
  }

  const install = await validateSkillInstallViaParser(candidate.markdown, {
    ...candidate.origin,
    digest: actualDigest,
  }, { env: options.env });
  const profileDigest = candidate.profileDocument ? hashString(candidate.profileDocument) : undefined;
  if (candidate.origin.profile_digest && candidate.origin.profile_digest !== profileDigest) {
    throw new Error(
      `Binding digest mismatch for ${options.ref}: expected sha256:${candidate.origin.profile_digest}, received sha256:${profileDigest ?? "none"}.`,
    );
  }
  const runnerNames = await validateInstallBindingManifest(
    install.skill.name,
    candidate.profileDocument,
    candidate.origin.runner_names,
    options.env,
  );
  const packageRoot = path.join(options.destinationRoot, ...safeSkillPackageParts(options.ref, install.skill.name));
  const destination = path.join(packageRoot, "SKILL.md");
  const profileStatePath = candidate.profileDocument ? path.join(packageRoot, ".runx", "profile.json") : undefined;
  const existing = await readOptionalFile(destination);
  const existingProfileState = profileStatePath ? await readOptionalFile(profileStatePath) : undefined;
  const nextProfileState = candidate.profileDocument
    ? `${JSON.stringify(buildProfileState(install.skill.name, actualDigest, candidate.profileDocument, profileDigest, runnerNames, install.origin), null, 2)}\n`
    : undefined;
  const shouldWriteProfileState = profileStatePath !== undefined && existingProfileState === undefined;
  const result: InstallLocalSkillResult = {
    status: existing === undefined || shouldWriteProfileState ? "installed" : "unchanged",
    destination,
    skill_name: install.skill.name,
    source: install.origin.source,
    source_label: install.origin.source_label,
    skill_id: install.origin.skill_id,
    version: install.origin.version,
    digest: actualDigest,
    profileDigest,
    profileStatePath,
    runnerNames,
    trust_tier: install.origin.trust_tier,
  };

  if (existing !== undefined && hashString(existing) !== actualDigest) {
    throw new Error(`Skill install destination already exists with different content: ${destination}`);
  }
  if (profileStatePath && existingProfileState !== undefined && nextProfileState !== undefined && existingProfileState !== nextProfileState) {
    throw new Error(`Skill install profile state already exists with different content: ${profileStatePath}`);
  }

  await mkdir(packageRoot, { recursive: true });
  if (existing === undefined) {
    await writeAtomic(destination, install.markdown);
  }
  if (profileStatePath && nextProfileState && shouldWriteProfileState) {
    await mkdir(path.dirname(profileStatePath), { recursive: true });
    await writeAtomic(profileStatePath, nextProfileState);
  }

  return result;
}

async function fetchInstallCandidate(options: InstallLocalSkillOptions): Promise<FetchedInstallCandidate> {
  if (isMarketplaceRef(options.ref)) {
    const resolved = await resolveMarketplaceSkill(options.marketplaceAdapters ?? [], options.ref, {
      version: options.version,
    });
    if (!resolved) {
      throw new Error(`Marketplace skill not found: ${options.ref}`);
    }
    return {
      markdown: resolved.markdown,
      profileDocument: resolved.profileDocument,
      origin: {
        source: resolved.result.source,
        source_label: resolved.result.source_label,
        ref: options.ref,
        skill_id: resolved.result.skill_id,
        version: resolved.result.version,
        digest: resolved.result.digest,
        profile_digest: resolved.result.profile_digest,
        runner_names: resolved.result.runner_names,
        trust_tier: resolved.result.trust_tier,
      },
    };
  }

  if (isRemoteRegistryUrl(options.registryUrl)) {
    if (!options.installationId) {
      throw new Error("Remote registry installs require an installation id.");
    }
    const resolvedRef = await resolveRemoteRegistryRefForInstall(options.ref, {
      baseUrl: options.registryUrl,
      version: options.version,
    });
    if (!resolvedRef) {
      throw new Error(`Registry skill not found: ${options.ref}`);
    }
    const acquired = await acquireRegistrySkillForInstall(resolvedRef.skill_id, {
      baseUrl: options.registryUrl,
      installationId: options.installationId,
      version: resolvedRef.version,
      channel: "cli",
    });
    return {
      markdown: acquired.markdown,
      profileDocument: acquired.profile_document,
      origin: {
        source: "runx-registry",
        source_label: "runx registry",
        ref: options.ref,
        skill_id: acquired.skill_id,
        version: acquired.version,
        digest: acquired.digest,
        profile_digest: acquired.profile_digest,
        runner_names: acquired.runner_names,
        trust_tier: acquired.trust_tier,
      },
    };
  }

  if (!options.registryStore) {
    throw new Error("A local registry store is required when no remote registry URL is configured.");
  }

  const resolved = await resolveRegistrySkillForInstall(options.registryStore, options.ref, {
    version: options.version,
    registryUrl: options.registryUrl,
  });
  if (!resolved) {
    throw new Error(`Registry skill not found: ${options.ref}`);
  }
  return {
    markdown: resolved.markdown,
    profileDocument: resolved.profile_document,
    origin: {
      source: resolved.source,
      source_label: resolved.source_label,
      ref: options.ref,
      skill_id: resolved.skill_id,
      version: resolved.version,
      digest: resolved.digest,
      profile_digest: resolved.profile_digest,
      runner_names: resolved.runner_names,
      trust_tier: resolved.trust_tier,
    },
  };
}

function isRemoteRegistryUrl(value: string | undefined): value is string {
  return typeof value === "string" && /^https?:\/\//i.test(value);
}

interface ResolveRegistrySkillForInstallOptions {
  readonly version?: string;
  readonly registryUrl?: string;
}

interface ParsedRegistrySkillRef {
  readonly skillId: string;
  readonly version?: string;
}

async function resolveRegistrySkillForInstall(
  store: SkillInstallRegistryStore,
  ref: string,
  options: ResolveRegistrySkillForInstallOptions = {},
): Promise<RegistrySkillResolution | undefined> {
  const parsed = parseRegistrySkillRef(ref);
  const version = options.version ?? parsed.version;
  const record = parsed.skillId.includes("/")
    ? await store.getVersion(parsed.skillId, version)
    : await resolveRegistrySkillByNameForInstall(store, parsed.skillId, version);

  if (!record) {
    return undefined;
  }

  return registrySkillResolutionFromRecord(record, options.registryUrl);
}

async function resolveRegistrySkillByNameForInstall(
  store: SkillInstallRegistryStore,
  name: string,
  version?: string,
): Promise<SkillInstallRegistrySkillVersion | undefined> {
  if (!store.listSkills) {
    throw new Error(`Registry ref '${name}' requires a registry store that can list skills. Use '<owner>/<name>' instead.`);
  }
  const normalized = slugifyRegistryPart(name);
  const matches = (await store.listSkills()).filter(
    (skill) => skill.name === normalized || skill.skill_id.endsWith(`/${normalized}`),
  );

  if (matches.length === 0) {
    return undefined;
  }
  if (matches.length > 1) {
    throw new Error(`Registry ref '${name}' is ambiguous. Use '<owner>/<name>' instead.`);
  }

  return await store.getVersion(matches[0].skill_id, version);
}

function registrySkillResolutionFromRecord(
  record: SkillInstallRegistrySkillVersion,
  registryUrl?: string,
): RegistrySkillResolution {
  const ref = `${record.skill_id}@${record.version}`;
  const registryFlag = registryUrl ? ` --registry ${registryUrl}` : "";
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
    trust_tier: validateRegistryTrustTier(record.trust_tier, "registry_version.trust_tier"),
    registry_url: registryUrl,
    add_command: `runx skill add ${ref}${registryFlag}`,
    run_command: `runx skill ${record.name}`,
  };
}

function parseRegistrySkillRef(ref: string): ParsedRegistrySkillRef {
  const withoutProtocol = ref.startsWith("runx://skill/")
    ? decodeURIComponent(ref.slice("runx://skill/".length))
    : ref;
  const withoutPrefix = withoutProtocol.replace(/^(registry|runx-registry):/, "");
  const atIndex = withoutPrefix.lastIndexOf("@");

  if (atIndex <= 0) {
    return { skillId: withoutPrefix };
  }

  return {
    skillId: withoutPrefix.slice(0, atIndex),
    version: withoutPrefix.slice(atIndex + 1) || undefined,
  };
}

function slugifyRegistryPart(value: string): string {
  const slug = value
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9._-]+/g, "-")
    .replace(/^-+|-+$/g, "");
  if (!slug) {
    throw new Error("Registry slugs cannot be empty.");
  }
  return slug;
}

interface ResolveRemoteRegistryRefForInstallOptions {
  readonly baseUrl: string;
  readonly version?: string;
}

async function resolveRemoteRegistryRefForInstall(
  ref: string,
  options: ResolveRemoteRegistryRefForInstallOptions,
): Promise<{ readonly skill_id: string; readonly version?: string } | undefined> {
  const parsed = parseRegistrySkillRef(ref);
  if (parsed.skillId.includes("/")) {
    return {
      skill_id: parsed.skillId,
      version: options.version ?? parsed.version,
    };
  }

  const matches = (await searchRemoteRegistryForInstall(parsed.skillId, {
    baseUrl: options.baseUrl,
    limit: 100,
  })).filter((candidate) => candidate.name === parsed.skillId.trim().toLowerCase());
  if (matches.length === 0) {
    return undefined;
  }
  if (matches.length > 1) {
    throw new Error(`Registry ref '${parsed.skillId}' is ambiguous. Use '<owner>/<name>' instead.`);
  }
  return {
    skill_id: matches[0].skill_id,
    version: options.version ?? parsed.version ?? matches[0].version,
  };
}

interface SearchRemoteRegistryForInstallOptions {
  readonly baseUrl: string;
  readonly limit: number;
}

interface RemoteRegistrySearchResult {
  readonly skill_id: string;
  readonly name: string;
  readonly version?: string;
}

async function searchRemoteRegistryForInstall(
  query: string,
  options: SearchRemoteRegistryForInstallOptions,
): Promise<readonly RemoteRegistrySearchResult[]> {
  const params = new URLSearchParams();
  if (query.trim().length > 0) {
    params.set("q", query.trim());
  }
  params.set("limit", String(options.limit));
  const response = await fetchWithTimeout({
    url: `${options.baseUrl.replace(/\/$/, "")}/v1/skills?${params.toString()}`,
    description: `Registry search for '${query}'`,
  });
  if (!response.ok) {
    throw new Error(`Registry search failed for '${query}': HTTP ${response.status}`);
  }
  const payload = asRecord(await response.json());
  const skills = Array.isArray(payload?.skills) ? payload.skills : undefined;
  if (payload?.status !== "success" || !skills) {
    throw new Error(`Registry search returned an invalid payload for '${query}'.`);
  }
  return skills.map((skill) => validateRemoteRegistrySearchResult(skill, query));
}

function validateRemoteRegistrySearchResult(value: unknown, query: string): RemoteRegistrySearchResult {
  const skill = asRecord(value);
  if (
    !skill
    || typeof skill.skill_id !== "string"
    || typeof skill.name !== "string"
    || typeof skill.owner !== "string"
    || typeof skill.source_type !== "string"
    || (skill.profile_mode !== "portable" && skill.profile_mode !== "profiled")
    || !isStringArray(skill.runner_names)
    || !isStringArray(skill.required_scopes)
    || !isStringArray(skill.tags)
    || !isRegistryTrustTier(skill.trust_tier)
    || typeof skill.install_command !== "string"
    || typeof skill.run_command !== "string"
  ) {
    throw new Error(`Registry search returned an invalid skill entry for '${query}'.`);
  }
  return {
    skill_id: skill.skill_id,
    name: skill.name,
    version: optionalString(skill.version),
  };
}

interface AcquireRegistrySkillForInstallOptions {
  readonly baseUrl: string;
  readonly installationId: string;
  readonly version?: string;
  readonly channel?: string;
}

interface AcquiredRegistrySkillForInstall {
  readonly skill_id: string;
  readonly owner: string;
  readonly name: string;
  readonly version: string;
  readonly digest: string;
  readonly markdown: string;
  readonly profile_document?: string;
  readonly profile_digest?: string;
  readonly runner_names: readonly string[];
  readonly trust_tier: RegistryTrustTier;
}

async function acquireRegistrySkillForInstall(
  skillId: string,
  options: AcquireRegistrySkillForInstallOptions,
): Promise<AcquiredRegistrySkillForInstall> {
  const [owner, name] = splitRegistrySkillId(skillId);
  const response = await fetchWithTimeout({
    url: `${options.baseUrl.replace(/\/$/, "")}/v1/skills/${encodeURIComponent(owner)}/${encodeURIComponent(name)}/acquire`,
    init: {
      method: "POST",
      headers: {
        "content-type": "application/json",
      },
      body: JSON.stringify({
        installation_id: options.installationId,
        version: options.version,
        channel: options.channel ?? "cli",
      }),
    },
    description: `Registry acquire for ${skillId}`,
  });

  if (!response.ok) {
    throw new Error(`Registry acquire failed for ${skillId}: HTTP ${response.status}`);
  }

  return validateAcquiredRegistrySkillForInstall(await response.json(), skillId);
}

function validateAcquiredRegistrySkillForInstall(value: unknown, skillId: string): AcquiredRegistrySkillForInstall {
  const payload = asRecord(value);
  const acquisition = asRecord(payload?.acquisition);
  if (
    payload?.status !== "success"
    || !acquisition
    || typeof acquisition.skill_id !== "string"
    || typeof acquisition.owner !== "string"
    || typeof acquisition.name !== "string"
    || typeof acquisition.version !== "string"
    || typeof acquisition.digest !== "string"
    || typeof acquisition.markdown !== "string"
    || !isStringArray(acquisition.runner_names)
    || !isRegistryTrustTier(acquisition.trust_tier)
    || acquisition.publisher === undefined
    || acquisition.attestations === undefined
  ) {
    throw new Error(`Registry acquire returned an invalid payload for ${skillId}.`);
  }
  validateRegistryPublisher(acquisition.publisher, "remote_registry.acquisition.publisher");
  validateRegistryAttestations(acquisition.attestations, "remote_registry.acquisition.attestations");

  return {
    skill_id: acquisition.skill_id,
    owner: acquisition.owner,
    name: acquisition.name,
    version: acquisition.version,
    digest: acquisition.digest,
    markdown: acquisition.markdown,
    profile_document: optionalString(acquisition.profile_document),
    profile_digest: optionalString(acquisition.profile_digest),
    runner_names: acquisition.runner_names,
    trust_tier: acquisition.trust_tier,
  };
}

function validateRegistryPublisher(value: unknown, label: string): void {
  const publisher = asRecord(value);
  if (!publisher) {
    throw new Error(`${label} must be an object.`);
  }
  if (
    publisher.kind !== "organization"
    && publisher.kind !== "user"
    && publisher.kind !== "team"
    && publisher.kind !== "service"
    && publisher.kind !== "publisher"
  ) {
    throw new Error(`${label}.kind must be one of organization, user, team, service, or publisher.`);
  }
  requiredString(publisher.id, `${label}.id`);
  optionalNonEmptyString(publisher.handle, `${label}.handle`);
  optionalNonEmptyString(publisher.display_name, `${label}.display_name`);
}

function validateRegistryAttestations(value: unknown, label: string): void {
  if (!Array.isArray(value)) {
    throw new Error(`${label} must be an array when provided.`);
  }
  for (const [index, entry] of value.entries()) {
    validateRegistryAttestation(entry, `${label}[${index}]`);
  }
}

function validateRegistryAttestation(value: unknown, label: string): void {
  const attestation = asRecord(value);
  if (!attestation) {
    throw new Error(`${label} must be an object.`);
  }
  if (attestation.kind !== "source" && attestation.kind !== "publisher" && attestation.kind !== "verification") {
    throw new Error(`${label}.kind must be one of source, publisher, or verification.`);
  }
  if (attestation.status !== "verified" && attestation.status !== "declared") {
    throw new Error(`${label}.status must be one of verified or declared.`);
  }
  requiredString(attestation.id, `${label}.id`);
  requiredString(attestation.summary, `${label}.summary`);
  optionalNonEmptyString(attestation.source, `${label}.source`);
  optionalNonEmptyString(attestation.issued_at, `${label}.issued_at`);
}

function validateRegistryTrustTier(value: unknown, label: string): RegistryTrustTier {
  if (isRegistryTrustTier(value)) {
    return value;
  }
  throw new Error(`${label} must be one of first_party, verified, or community.`);
}

function isRegistryTrustTier(value: unknown): value is RegistryTrustTier {
  return value === "first_party" || value === "verified" || value === "community";
}

function isStringArray(value: unknown): value is readonly string[] {
  return Array.isArray(value) && value.every((entry) => typeof entry === "string");
}

function requiredString(value: unknown, label: string): string {
  if (typeof value !== "string" || value.length === 0) {
    throw new Error(`${label} must be a non-empty string.`);
  }
  return value;
}

function optionalNonEmptyString(value: unknown, label: string): string | undefined {
  if (value === undefined) {
    return undefined;
  }
  return requiredString(value, label);
}

function optionalString(value: unknown): string | undefined {
  return typeof value === "string" ? value : undefined;
}

function splitRegistrySkillId(skillId: string): readonly [string, string] {
  const parts = skillId.split("/");
  if (parts.length !== 2 || !parts[0] || !parts[1]) {
    throw new Error(`Invalid registry skill id '${skillId}'. Expected '<owner>/<name>'.`);
  }
  return [parts[0], parts[1]];
}

function buildProfileState(
  skillName: string,
  digest: string,
  profileDocument: string,
  profileDigest: string | undefined,
  runnerNames: readonly string[],
  origin: SkillInstallOrigin,
): Readonly<Record<string, unknown>> {
  return {
    schema_version: "runx.skill-profile.v1",
    skill: {
      name: skillName,
      path: "SKILL.md",
      digest,
    },
    profile: {
      document: profileDocument,
      digest: profileDigest,
      runner_names: runnerNames,
    },
    origin,
  };
}

async function validateInstallBindingManifest(
  skillName: string,
  profileDocument: string | undefined,
  advertisedRunnerNames: readonly string[] | undefined,
  env?: NodeJS.ProcessEnv,
): Promise<readonly string[]> {
  if (!profileDocument) {
    return advertisedRunnerNames ?? [];
  }

  const manifest = await validateRunnerManifestYamlViaParser(profileDocument, { env });
  if (manifest.skill && manifest.skill !== skillName) {
    throw new Error(`Runner manifest skill '${manifest.skill}' does not match skill '${skillName}'.`);
  }

  const runnerNames = Object.keys(manifest.runners);
  if (
    advertisedRunnerNames &&
    (advertisedRunnerNames.length !== runnerNames.length ||
      advertisedRunnerNames.some((runnerName, index) => runnerName !== runnerNames[index]))
  ) {
    throw new Error(`Runner manifest runners do not match advertised runner metadata for skill '${skillName}'.`);
  }
  return runnerNames;
}


async function writeAtomic(destination: string, contents: string, replace = false): Promise<void> {
  const tempPath = `${destination}.tmp-${process.pid}-${Date.now()}`;
  await writeFile(tempPath, contents, { flag: "wx", mode: 0o600 });
  try {
    if (!replace) {
      await assertMissing(destination);
    }
    await rename(tempPath, destination);
  } catch (error) {
    await rm(tempPath, { force: true });
    throw error;
  }
}

async function assertMissing(destination: string): Promise<void> {
  try {
    await access(destination, fsConstants.F_OK);
  } catch (error) {
    if (isNotFound(error)) {
      return;
    }
    throw error;
  }
  throw new Error(`Skill install destination already exists: ${destination}`);
}

function safeSkillPackageParts(ref: string, skillName: string): readonly string[] {
  const normalizedRef = normalizeInstallRef(ref);
  const rawParts = normalizedRef.includes("/") ? normalizedRef.split("/") : [skillName];
  const parts = rawParts.map(safeSkillPathPart).filter((part) => part.length > 0);
  if (parts.length === 0) {
    return [safeSkillPathPart(skillName)];
  }
  return parts;
}

function normalizeInstallRef(ref: string): string {
  const withoutProtocol = ref.startsWith("runx://skill/")
    ? decodeURIComponent(ref.slice("runx://skill/".length))
    : ref;
  const withoutPrefix = withoutProtocol.replace(/^[a-z0-9._-]+:/i, "");
  const atIndex = withoutPrefix.lastIndexOf("@");
  return atIndex > 0 ? withoutPrefix.slice(0, atIndex) : withoutPrefix;
}

function safeSkillPathPart(name: string): string {
  const part = name
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9._-]+/g, "-")
    .replace(/^-+|-+$/g, "");
  if (!part || part === "." || part === "..") {
    throw new Error("Skill name cannot produce an empty install path part.");
  }
  return part;
}
