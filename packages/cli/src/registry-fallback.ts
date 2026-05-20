import {
  resolveRunxGlobalHomeDir,
  resolveRunxRegistryPath,
  resolveRunxRegistryTarget,
} from "@runxhq/core/config";
import type { SkillSearchResult } from "@runxhq/core/marketplaces";
import {
  createRunxSdk,
  createFileRegistryStore,
  publishSkillMarkdown,
  searchRegistry,
  type PutVersionOptions,
  type RegistryAttestation,
  type RegistryPublisher,
  type RegistrySkill,
  type RegistrySkillVersion,
  type RegistryStore,
  type RegistryTrustTier,
} from "@runxhq/runtime-local/sdk";
import { asRecord, errorMessage, fetchWithTimeout } from "@runxhq/core/util";

import { ensureRunxInstallState } from "./runx-state.js";

export async function searchRegistryFallback(
  query: string,
  env: NodeJS.ProcessEnv,
  registryOverride?: string,
): Promise<readonly SkillSearchResult[]> {
  const registryTarget = resolveRunxRegistryTarget(env, { registry: registryOverride });
  if (registryTarget.mode === "remote") {
    return await createRunxSdk({ env, registryUrl: registryOverride }).searchSkills({
      query,
      source: "registry",
    });
  }
  return await searchRegistry(createFileRegistryStore(registryTarget.registryPath), query, {
    registryUrl: registryTarget.registryUrl,
  });
}

export function createCliFileRegistryStore(registryPath: string): RegistryStore {
  return createFileRegistryStore(registryPath);
}

export async function resolveCliRegistryStoreForGraphs(env: NodeJS.ProcessEnv): Promise<RegistryStore | undefined> {
  const target = resolveRunxRegistryTarget(env);
  if (target.mode === "local") {
    return createFileRegistryStore(target.registryPath);
  }
  if (!target.registryUrl) {
    return undefined;
  }
  const globalHomeDir = resolveRunxGlobalHomeDir(env);
  const install = await ensureRunxInstallState(globalHomeDir);
  return new CliHttpCachedRegistryStore({
    remoteBaseUrl: target.registryUrl,
    cache: createFileRegistryStore(resolveRunxRegistryPath(env)),
    installationId: install.state.installation_id,
    channel: "cli-graph",
  });
}

export async function publishRegistrySkillMarkdown(options: {
  readonly env: NodeJS.ProcessEnv;
  readonly registry?: string;
  readonly markdown: string;
  readonly owner?: string;
  readonly version?: string;
  readonly profileDocument?: string;
}) {
  return await publishSkillMarkdown(
    createFileRegistryStore(resolveRunxRegistryPath(options.env, { registry: options.registry })),
    options.markdown,
    {
      owner: options.owner,
      version: options.version,
      registryUrl: options.registry,
      profileDocument: options.profileDocument,
    },
  );
}

interface CliHttpCachedRegistryStoreOptions {
  readonly remoteBaseUrl: string;
  readonly installationId: string;
  readonly cache: RegistryStore;
  readonly channel: string;
}

class CliHttpCachedRegistryStore implements RegistryStore {
  constructor(private readonly options: CliHttpCachedRegistryStoreOptions) {}

  async getVersion(skillId: string, version?: string): Promise<RegistrySkillVersion | undefined> {
    const cached = await this.options.cache.getVersion(skillId, version);
    if (cached && version) {
      return cached;
    }

    const acquired = await safeAcquireRegistrySkill({
      skillId,
      baseUrl: this.options.remoteBaseUrl,
      installationId: this.options.installationId,
      version,
      channel: this.options.channel,
    });
    if (!acquired) {
      return cached;
    }
    return await this.options.cache.putVersion(acquiredRegistrySkillToVersion(acquired), { upsert: true });
  }

  async listVersions(skillId: string): Promise<readonly RegistrySkillVersion[]> {
    return await this.options.cache.listVersions(skillId);
  }

  async listSkills(): Promise<readonly RegistrySkill[]> {
    return await this.options.cache.listSkills();
  }

  async putVersion(version: RegistrySkillVersion, options?: PutVersionOptions): Promise<RegistrySkillVersion> {
    return await this.options.cache.putVersion(version, options);
  }
}

interface AcquiredRegistrySkill {
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
  readonly publisher: RegistryPublisher;
  readonly attestations: readonly RegistryAttestation[];
  readonly source_metadata?: Readonly<Record<string, unknown>>;
}

async function safeAcquireRegistrySkill(args: {
  readonly skillId: string;
  readonly baseUrl: string;
  readonly installationId: string;
  readonly version?: string;
  readonly channel: string;
}): Promise<AcquiredRegistrySkill | undefined> {
  try {
    return await acquireRegistrySkill(args);
  } catch (error) {
    const message = errorMessage(error);
    if (/HTTP 404/.test(message)) {
      return undefined;
    }
    throw error;
  }
}

async function acquireRegistrySkill(args: {
  readonly skillId: string;
  readonly baseUrl: string;
  readonly installationId: string;
  readonly version?: string;
  readonly channel: string;
}): Promise<AcquiredRegistrySkill> {
  const [owner, name] = splitRegistrySkillId(args.skillId);
  const response = await fetchWithTimeout({
    url: `${args.baseUrl.replace(/\/$/, "")}/v1/skills/${encodeURIComponent(owner)}/${encodeURIComponent(name)}/acquire`,
    init: {
      method: "POST",
      headers: {
        "content-type": "application/json",
      },
      body: JSON.stringify({
        installation_id: args.installationId,
        version: args.version,
        channel: args.channel,
      }),
    },
    description: `Registry acquire for ${args.skillId}`,
  });
  if (!response.ok) {
    throw new Error(`Registry acquire failed for ${args.skillId}: HTTP ${response.status}`);
  }
  return validateAcquiredRegistrySkill(await response.json(), args.skillId);
}

function validateAcquiredRegistrySkill(value: unknown, skillId: string): AcquiredRegistrySkill {
  const payload = asRecord(value);
  const acquisition = asRecord(payload?.acquisition);
  if (
    payload?.status !== "success" ||
    !acquisition ||
    typeof acquisition.skill_id !== "string" ||
    typeof acquisition.owner !== "string" ||
    typeof acquisition.name !== "string" ||
    typeof acquisition.version !== "string" ||
    typeof acquisition.digest !== "string" ||
    typeof acquisition.markdown !== "string" ||
    !isStringArray(acquisition.runner_names) ||
    !isRegistryTrustTier(acquisition.trust_tier)
  ) {
    throw new Error(`Registry acquire returned an invalid payload for ${skillId}.`);
  }
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
    publisher: validateRegistryPublisher(acquisition.publisher, "remote_registry.acquisition.publisher"),
    attestations: validateRegistryAttestations(acquisition.attestations, "remote_registry.acquisition.attestations"),
    source_metadata: asRecord(acquisition.source_metadata),
  };
}

function acquiredRegistrySkillToVersion(acquired: AcquiredRegistrySkill): RegistrySkillVersion {
  const now = new Date().toISOString();
  return {
    skill_id: acquired.skill_id,
    owner: acquired.owner,
    name: acquired.name,
    version: acquired.version,
    digest: acquired.digest,
    markdown: acquired.markdown,
    profile_document: acquired.profile_document,
    profile_digest: acquired.profile_digest,
    runner_names: acquired.runner_names,
    source_type: "runx-registry",
    trust_tier: acquired.trust_tier,
    source_metadata: acquired.source_metadata as RegistrySkillVersion["source_metadata"],
    attestations: acquired.attestations,
    required_scopes: [],
    tags: [],
    publisher: acquired.publisher,
    created_at: now,
    updated_at: now,
  };
}

function validateRegistryPublisher(value: unknown, label: string): RegistryPublisher {
  const publisher = asRecord(value);
  if (
    !publisher ||
    (
      publisher.kind !== "organization" &&
      publisher.kind !== "user" &&
      publisher.kind !== "team" &&
      publisher.kind !== "service" &&
      publisher.kind !== "publisher"
    ) ||
    typeof publisher.id !== "string" ||
    publisher.id.length === 0
  ) {
    throw new Error(`${label} must be a valid registry publisher.`);
  }
  return {
    kind: publisher.kind,
    id: publisher.id,
    handle: optionalString(publisher.handle),
    display_name: optionalString(publisher.display_name),
  };
}

function validateRegistryAttestations(value: unknown, label: string): readonly RegistryAttestation[] {
  if (!Array.isArray(value)) {
    throw new Error(`${label} must be an array.`);
  }
  return value.map((entry, index) => validateRegistryAttestation(entry, `${label}[${index}]`));
}

function validateRegistryAttestation(value: unknown, label: string): RegistryAttestation {
  const attestation = asRecord(value);
  if (
    !attestation ||
    (attestation.kind !== "source" && attestation.kind !== "publisher" && attestation.kind !== "verification") ||
    (attestation.status !== "verified" && attestation.status !== "declared") ||
    typeof attestation.id !== "string" ||
    attestation.id.length === 0 ||
    typeof attestation.summary !== "string" ||
    attestation.summary.length === 0
  ) {
    throw new Error(`${label} must be a valid registry attestation.`);
  }
  return {
    kind: attestation.kind,
    id: attestation.id,
    status: attestation.status,
    summary: attestation.summary,
    source: optionalString(attestation.source),
    issued_at: optionalString(attestation.issued_at),
    metadata: asRecord(attestation.metadata),
  };
}

function splitRegistrySkillId(skillId: string): readonly [string, string] {
  const parts = skillId.split("/");
  if (parts.length !== 2 || !parts[0] || !parts[1]) {
    throw new Error(`Invalid registry skill id '${skillId}'. Expected '<owner>/<name>'.`);
  }
  return [parts[0], parts[1]];
}

function isStringArray(value: unknown): value is readonly string[] {
  return Array.isArray(value) && value.every((entry) => typeof entry === "string");
}

function isRegistryTrustTier(value: unknown): value is RegistryTrustTier {
  return value === "first_party" || value === "verified" || value === "community";
}

function optionalString(value: unknown): string | undefined {
  return typeof value === "string" && value.length > 0 ? value : undefined;
}
