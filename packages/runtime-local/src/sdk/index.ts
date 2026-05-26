export const sdkJsPackage = "@runxhq/runtime-local/sdk";

export * from "./caller.js";
export * from "./act-assignment.js";
export * from "./host-protocol.js";
export * from "./trusted-host-outcome.js";

import { randomUUID } from "node:crypto";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";

import {
  loadLocalSkillPackage,
  resolvePathFromUserInput,
  resolveRunxGlobalHomeDir,
  resolveRunxProjectDir,
  resolveRunxRegistryTarget,
  resolveSkillInstallRoot,
} from "@runxhq/core/config";
import {
  errorMessage,
  fetchWithTimeout,
  hashString,
  isDefined,
  isNotFound,
  isRecord,
  readField,
  requireAnyString,
  safeReadDirNames,
  stringValue,
  unique,
} from "@runxhq/core/util";
import {
  createFixtureMarketplaceAdapter,
  searchMarketplaceAdapters,
  type MarketplaceAdapter,
  type SkillSearchResult,
} from "@runxhq/core/marketplaces";
import {
  resolveEnvToolCatalogAdapters,
  resolveCatalogTool,
  searchToolCatalogAdapters,
  createToolInspectResult,
  inspectCatalogResolvedTool,
  type ToolCatalogAdapter,
  type ToolInspectResult,
  type ToolCatalogSearchResult,
} from "@runxhq/runtime-local/tool-catalogs";
import {
  installLocalSkill,
  inspectLocalReceipt,
  listLocalHistory,
  resolveToolExecutionTarget,
  runLocalSkill,
  type AuthResolver,
  type Caller,
  type InstallLocalSkillResult,
  type ReceiptVerification,
  type RuntimeReceipt,
  type RunLocalSkillResult,
  type SkillAdapter,
} from "../runner-local/index.js";
import { validatePublishHarness, type PublishHarnessSummary } from "../harness/index.js";
import { createStructuredCaller, type StructuredCallerOptions } from "./caller.js";
import {
  createHostBridge,
  inspectLocalHostState,
  type HostBridge,
  type HostBoundaryResolver,
  type HostInspectOptions,
  type HostRunOptions,
  type HostRunResult,
  type HostRunState,
} from "./host-protocol.js";
import type { CatalogMetadata, SkillRunnerManifest, ValidatedSkill } from "../parser-types.js";
import {
  validateRunnerManifestYamlViaParser,
  validateSkillMarkdownViaParser,
  validateToolManifestJsonViaParser,
} from "../runner-local/parser-bridge.js";

export type RegistryTrustTier = "first_party" | "verified" | "community";
export type RegistryPublisherKind = "organization" | "user" | "team" | "service" | "publisher";
export type RegistryAttestationKind = "source" | "publisher" | "verification";
export type RegistryAttestationStatus = "verified" | "declared";

export interface RegistryPublisher {
  readonly kind: RegistryPublisherKind;
  readonly id: string;
  readonly handle?: string;
  readonly display_name?: string;
}

export interface RegistryAttestation {
  readonly kind: RegistryAttestationKind;
  readonly id: string;
  readonly status: RegistryAttestationStatus;
  readonly summary: string;
  readonly source?: string;
  readonly issued_at?: string;
  readonly metadata?: Readonly<Record<string, unknown>>;
}

export interface RegistrySourceMetadata {
  readonly provider: "github";
  readonly repo: string;
  readonly repo_url: string;
  readonly skill_path: string;
  readonly profile_path?: string;
  readonly ref: string;
  readonly sha: string;
  readonly default_branch: string;
  readonly event: "enrollment" | "push" | "tag" | "tombstone";
  readonly immutable: boolean;
  readonly live: boolean;
  readonly tombstoned?: boolean;
  readonly tag?: string;
  readonly publisher_handle?: string;
}

export interface RegistrySkillVersion {
  readonly skill_id: string;
  readonly owner: string;
  readonly name: string;
  readonly description?: string;
  readonly version: string;
  readonly digest: string;
  readonly markdown: string;
  readonly profile_document?: string;
  readonly profile_digest?: string;
  readonly runner_names: readonly string[];
  readonly source_type: string;
  readonly trust_tier: RegistryTrustTier;
  readonly catalog_kind?: "skill" | "graph";
  readonly catalog_audience?: "public" | "builder" | "operator";
  readonly catalog_visibility?: "public" | "private";
  readonly source_metadata?: RegistrySourceMetadata;
  readonly attestations?: readonly RegistryAttestation[];
  readonly required_scopes: readonly string[];
  readonly runtime?: unknown;
  readonly auth?: unknown;
  readonly risk?: unknown;
  readonly runx?: Readonly<Record<string, unknown>>;
  readonly tags: readonly string[];
  readonly publisher: RegistryPublisher;
  readonly created_at: string;
  readonly updated_at: string;
}

export interface RegistrySkill {
  readonly skill_id: string;
  readonly owner: string;
  readonly name: string;
  readonly description?: string;
  readonly latest_version: string;
  readonly latest_digest: string;
  readonly versions: readonly RegistrySkillVersion[];
}

export interface PutVersionOptions {
  readonly upsert?: boolean;
}

export interface RegistryStore {
  readonly putVersion: (
    version: RegistrySkillVersion,
    options?: PutVersionOptions,
  ) => Promise<RegistrySkillVersion>;
  readonly getVersion: (skillId: string, version?: string) => Promise<RegistrySkillVersion | undefined>;
  readonly listVersions: (skillId: string) => Promise<readonly RegistrySkillVersion[]>;
  readonly listSkills: () => Promise<readonly RegistrySkill[]>;
}

export interface IngestSkillOptions {
  readonly owner?: string;
  readonly version?: string;
  readonly createdAt?: string;
  readonly profileDocument?: string;
  readonly publisher?: RegistryPublisher;
  readonly trustTier?: RegistryTrustTier;
  readonly attestations?: readonly RegistryAttestation[];
  readonly sourceMetadata?: RegistrySourceMetadata;
  readonly upsert?: boolean;
  readonly parserEnv?: NodeJS.ProcessEnv;
}

export interface RunxLinkResolution {
  readonly link: string;
  readonly skill_id: string;
  readonly version: string;
  readonly digest: string;
  readonly registry_url?: string;
  readonly install_command: string;
  readonly run_command: string;
}

export interface PublishSkillMarkdownOptions extends IngestSkillOptions {
  readonly registryUrl?: string;
}

export interface PublishSkillMarkdownResult {
  readonly status: "published" | "unchanged";
  readonly skill_id: string;
  readonly name: string;
  readonly version: string;
  readonly digest: string;
  readonly profile_digest?: string;
  readonly runner_names: readonly string[];
  readonly source_type: string;
  readonly registry_url?: string;
  readonly link: RunxLinkResolution;
  readonly record: RegistrySkillVersion;
}

export interface RunxSdkOptions {
  readonly env?: NodeJS.ProcessEnv;
  readonly caller?: Caller;
  readonly callerOptions?: StructuredCallerOptions;
  readonly receiptDir?: string;
  readonly runxHome?: string;
  readonly registryDir?: string;
  readonly registryUrl?: string;
  readonly registryStore?: RegistryStore;
  readonly marketplaceAdapters?: readonly MarketplaceAdapter[];
  readonly toolCatalogAdapters?: readonly ToolCatalogAdapter[];
  readonly authResolver?: AuthResolver;
  readonly allowedSourceTypes?: readonly string[];
  readonly adapters?: readonly SkillAdapter[];
  readonly voiceProfilePath?: string;
}

export interface RunSkillOptions {
  readonly skillPath: string;
  readonly inputs?: Readonly<Record<string, unknown>>;
  readonly answersPath?: string;
  readonly runner?: string;
  readonly receiptDir?: string;
  readonly runxHome?: string;
  readonly parentReceipt?: string;
  readonly contextFrom?: readonly string[];
  readonly caller?: Caller;
  readonly authResolver?: AuthResolver;
  readonly allowedSourceTypes?: readonly string[];
  readonly resumeFromRunId?: string;
  readonly adapters?: readonly SkillAdapter[];
  readonly voiceProfilePath?: string;
}

export interface SearchSkillsOptions {
  readonly query: string;
  readonly source?: string;
  readonly limit?: number;
}

export interface SearchToolsOptions {
  readonly query: string;
  readonly source?: string;
  readonly limit?: number;
}

export interface InspectToolOptions {
  readonly ref: string;
  readonly source?: string;
  readonly searchFromDirectory?: string;
}

export interface AddSkillOptions {
  readonly ref: string;
  readonly version?: string;
  readonly to?: string;
  readonly expectedDigest?: string;
  readonly registryUrl?: string;
}

export interface PublishSkillOptions extends PublishSkillMarkdownOptions {
  readonly skillPath: string;
}

export interface PublishSkillResult extends PublishSkillMarkdownResult {
  readonly harness: PublishHarnessSummary;
}

export interface InspectReceiptOptions {
  readonly receiptId: string;
  readonly receiptDir?: string;
}

export interface HistoryOptions {
  readonly receiptDir?: string;
  readonly limit?: number;
}

export interface HistoryEntry {
  readonly id: string;
  readonly kind: string;
  readonly status: string;
  readonly verification: ReceiptVerification;
  readonly path: string;
  readonly started_at?: string;
  readonly completed_at?: string;
}

export type InspectReceiptResult = RuntimeReceipt & {
  readonly verification: ReceiptVerification;
};

export class RunxSdk {
  constructor(private readonly options: RunxSdkOptions = {}) {}

  async runSkill(options: RunSkillOptions): Promise<RunLocalSkillResult> {
    return await runLocalSkill({
      skillPath: resolvePathFromUserInput(options.skillPath, this.env()),
      inputs: options.inputs,
      answersPath: options.answersPath ? resolvePathFromUserInput(options.answersPath, this.env()) : undefined,
      caller: this.caller(options.caller),
      env: this.env(),
      receiptDir: this.receiptDir(options.receiptDir),
      runner: options.runner,
      runxHome: options.runxHome ?? this.options.runxHome,
      parentReceipt: options.parentReceipt,
      contextFrom: options.contextFrom,
      allowedSourceTypes: options.allowedSourceTypes ?? this.options.allowedSourceTypes,
      authResolver: options.authResolver ?? this.options.authResolver,
      resumeFromRunId: options.resumeFromRunId,
      adapters: options.adapters ?? this.options.adapters,
      voiceProfilePath: options.voiceProfilePath ?? this.options.voiceProfilePath,
      toolCatalogAdapters: this.toolCatalogAdapters(),
    });
  }

  async inspectReceipt(options: InspectReceiptOptions): Promise<InspectReceiptResult> {
    const inspection = await inspectLocalReceipt({
      receiptId: options.receiptId,
      receiptDir: this.receiptDir(options.receiptDir),
      runxHome: this.options.runxHome ?? this.env().RUNX_HOME,
      env: this.env(),
    });
    return {
      ...inspection.receipt,
      verification: inspection.verification,
    };
  }

  async history(options: HistoryOptions = {}): Promise<readonly HistoryEntry[]> {
    const receiptDir = this.receiptDir(options.receiptDir);
    const history = await listLocalHistory({
      receiptDir,
      runxHome: this.options.runxHome ?? this.env().RUNX_HOME,
      env: this.env(),
      limit: options.limit,
    });
    return history.receipts.map((receipt) => ({
      id: receipt.id,
      kind: receipt.kind,
      status: receipt.status,
      verification: receipt.verification,
      path: path.join(receiptDir, `${receipt.id}.json`),
      started_at: receipt.startedAt,
      completed_at: receipt.completedAt,
    }));
  }

  async hostRun(options: HostRunOptions & {
    readonly resolver?: Parameters<HostBridge["run"]>[0]["resolver"];
  }): Promise<HostRunResult> {
    return await buildRunxHostBridge(this).run(options);
  }

  async hostResume(
    runId: string,
    options: Omit<HostRunOptions, "resumeFromRunId" | "skillPath"> & {
      readonly skillPath?: string;
      readonly resolver?: HostBoundaryResolver;
    } = {},
  ): Promise<HostRunResult> {
    return await buildRunxHostBridge(this).resume(runId, options);
  }

  async inspectHost(referenceId: string, options: HostInspectOptions = {}): Promise<HostRunState> {
    return await inspectLocalHostState(referenceId, {
      receiptDir: this.receiptDir(options.receiptDir),
      runxHome: options.runxHome ?? this.options.runxHome,
      env: this.env(),
    });
  }

  async searchSkills(options: SearchSkillsOptions): Promise<readonly SkillSearchResult[]> {
    const normalizedSource = options.source?.trim().toLowerCase();
    const results: SkillSearchResult[] = [];

    if (!normalizedSource || normalizedSource === "registry" || normalizedSource === "runx-registry") {
      const registryTarget = this.registryTarget();
      if (registryTarget.mode === "remote") {
        results.push(...(await searchRemoteRegistry(options.query, {
          baseUrl: registryTarget.registryUrl,
          limit: options.limit,
        })));
      } else {
        results.push(
          ...(await searchRegistry(this.registryStore(), options.query, {
            limit: options.limit,
            registryUrl: registryTarget.registryUrl,
          })),
        );
      }
    }

    const marketplaceAdapters = this.marketplaceAdapters(normalizedSource);
    results.push(...(await searchMarketplaceAdapters(marketplaceAdapters, options.query, { limit: options.limit })));

    return results.slice(0, options.limit ?? 20);
  }

  async searchTools(options: SearchToolsOptions): Promise<readonly ToolCatalogSearchResult[]> {
    const normalizedSource = options.source?.trim().toLowerCase();
    const results: ToolCatalogSearchResult[] = [];
    const registryTarget = this.registryTarget();

    if (registryTarget.mode === "remote") {
      results.push(...(await searchRemoteTools(options.query, {
        baseUrl: registryTarget.registryUrl,
        limit: options.limit,
        source: normalizedSource,
      })));
    }

    results.push(...(await searchToolCatalogAdapters(
      this.toolCatalogAdapters(options.source),
      options.query,
      { limit: options.limit },
    )));

    return dedupeToolSearchResults(results).slice(0, options.limit ?? 20);
  }

  async inspectTool(options: InspectToolOptions): Promise<ToolInspectResult> {
    const searchFromDirectory = resolvePathFromUserInput(
      options.searchFromDirectory ?? this.env().RUNX_CWD ?? process.cwd(),
      this.env(),
    );
    const adapters = this.toolCatalogAdapters(options.source);
    let localError: Error | undefined;
    try {
      const resolvedExecutionTarget = await resolveToolExecutionTarget(options.ref, searchFromDirectory, {
        env: this.env(),
        toolCatalogAdapters: adapters,
      });

      if (resolvedExecutionTarget.referencePath.startsWith("catalog:")) {
        const resolvedCatalogTool = await resolveCatalogTool(adapters, options.ref, {
          env: this.env(),
          searchFromDirectory,
        });
        if (!resolvedCatalogTool) {
          throw new Error(`Imported tool '${options.ref}' was resolved for execution but could not be inspected.`);
        }
        return inspectCatalogResolvedTool(options.ref, resolvedCatalogTool);
      }

      const tool = await validateToolManifestJsonViaParser(
        await readFile(resolvedExecutionTarget.referencePath, "utf8"),
        { env: this.env() },
      );
      return createToolInspectResult({
        ref: options.ref,
        tool,
        referencePath: resolvedExecutionTarget.referencePath,
        skillDirectory: resolvedExecutionTarget.skillDirectory,
        provenance: {
          origin: "local",
        },
      });
    } catch (error) {
      localError = error instanceof Error ? error : new Error(errorMessage(error));
    }

    const registryTarget = this.registryTarget();
    if (registryTarget.mode === "remote") {
      const remoteTool = await readRemoteTool(options.ref, {
        baseUrl: registryTarget.registryUrl,
        source: options.source,
      });
      if (remoteTool) {
        return remoteTool;
      }
    }

    throw localError ?? new Error(`Tool '${options.ref}' was not found.`);
  }

  async addSkill(options: AddSkillOptions): Promise<InstallLocalSkillResult> {
    const registryTarget = this.registryTarget(options.registryUrl);
    const installState = registryTarget.mode === "remote"
      ? await ensureRunxInstallState(resolveRunxGlobalHomeDir(this.env()))
      : undefined;
    return await installLocalSkill({
      ref: options.ref,
      registryStore: registryTarget.mode === "local" ? this.registryStore(options.registryUrl) : undefined,
      marketplaceAdapters: this.marketplaceAdapters(),
      destinationRoot: resolveSkillInstallRoot(this.env(), options.to),
      version: options.version,
      expectedDigest: options.expectedDigest,
      registryUrl: registryTarget.mode === "remote" ? registryTarget.registryUrl : options.registryUrl ?? this.options.registryUrl,
      installationId: installState?.state.installation_id,
      env: this.env(),
    });
  }

  async publishSkill(options: PublishSkillOptions): Promise<PublishSkillResult> {
    const resolvedSkillPath = resolvePathFromUserInput(options.skillPath, this.env());
    const harness = await validatePublishHarness(resolvedSkillPath, {
      env: this.env(),
      adapters: this.options.adapters,
    });
    if (harness.status === "failed") {
      throw new Error(`Harness failed for ${resolvedSkillPath}: ${harness.assertion_errors.join("; ")}`);
    }
    const skillPackage = await loadLocalSkillPackage(resolvedSkillPath);
    const publish = await publishSkillMarkdown(this.registryStore(options.registryUrl), skillPackage.markdown, {
      owner: options.owner,
      version: options.version,
      createdAt: options.createdAt,
      registryUrl: options.registryUrl ?? this.options.registryUrl,
      profileDocument: skillPackage.profileDocument,
      parserEnv: this.env(),
    });
    return {
      ...publish,
      harness,
    };
  }

  private caller(override?: Caller): Caller {
    return override ?? this.options.caller ?? createStructuredCaller(this.options.callerOptions);
  }

  private env(): NodeJS.ProcessEnv {
    return this.options.env ?? process.env;
  }

  private receiptDir(override?: string): string {
    return resolvePathFromUserInput(
      override ?? this.options.receiptDir ?? this.env().RUNX_RECEIPT_DIR ?? path.join(resolveRunxProjectDir(this.env()), "receipts"),
      this.env(),
    );
  }

  private registryStore(registryUrl = this.options.registryUrl): RegistryStore {
    if (this.options.registryStore) {
      return this.options.registryStore;
    }
    const target = this.registryTarget(registryUrl);
    return createFileRegistryStore(
      target.mode === "local" ? target.registryPath : path.join(resolveRunxGlobalHomeDir(this.env()), "registry"),
    );
  }

  private registryTarget(registryUrl = this.options.registryUrl) {
    return resolveRunxRegistryTarget(this.env(), { registry: registryUrl, registryDir: this.options.registryDir });
  }

  private marketplaceAdapters(source?: string): readonly MarketplaceAdapter[] {
    if (this.options.marketplaceAdapters) {
      return this.options.marketplaceAdapters;
    }
    if (
      this.env().RUNX_ENABLE_FIXTURE_MARKETPLACE === "1" &&
      (!source || source === "marketplace" || source === "fixture-marketplace")
    ) {
      return [createFixtureMarketplaceAdapter()];
    }
    return [];
  }

  private toolCatalogAdapters(source?: string): readonly ToolCatalogAdapter[] {
    if (this.options.toolCatalogAdapters) {
      return this.options.toolCatalogAdapters;
    }
    return resolveEnvToolCatalogAdapters(this.env(), source);
  }

}

export function createRunxSdk(options: RunxSdkOptions = {}): RunxSdk {
  return new RunxSdk(options);
}

export function createRunxHostBridge(options: RunxSdkOptions = {}): HostBridge {
  return buildRunxHostBridge(createRunxSdk(options));
}

export async function runSkill(options: RunSkillOptions & RunxSdkOptions): Promise<RunLocalSkillResult> {
  return await createRunxSdk(options).runSkill(options);
}

export async function hostRun(
  options: HostRunOptions & RunxSdkOptions & {
    readonly resolver?: Parameters<HostBridge["run"]>[0]["resolver"];
  },
): Promise<HostRunResult> {
  return await createRunxSdk(options).hostRun(options);
}

export async function hostResume(
  runId: string,
  options: Omit<HostRunOptions, "resumeFromRunId" | "skillPath"> & RunxSdkOptions & {
    readonly skillPath?: string;
    readonly resolver?: HostBoundaryResolver;
  } = {},
): Promise<HostRunResult> {
  return await createRunxSdk(options).hostResume(runId, options);
}

export async function inspectHost(
  referenceId: string,
  options: HostInspectOptions & RunxSdkOptions = {},
): Promise<HostRunState> {
  return await createRunxSdk(options).inspectHost(referenceId, options);
}

function buildRunxHostBridge(sdk: RunxSdk): HostBridge {
  return createHostBridge({
    execute: sdk.runSkill.bind(sdk),
    inspect: sdk.inspectHost.bind(sdk),
  });
}

export async function inspect(options: InspectReceiptOptions & RunxSdkOptions): Promise<InspectReceiptResult> {
  return await createRunxSdk(options).inspectReceipt(options);
}

export async function history(options: HistoryOptions & RunxSdkOptions = {}): Promise<readonly HistoryEntry[]> {
  return await createRunxSdk(options).history(options);
}

export async function search(options: SearchSkillsOptions & RunxSdkOptions): Promise<readonly SkillSearchResult[]> {
  return await createRunxSdk(options).searchSkills(options);
}

export async function searchTools(options: SearchToolsOptions & RunxSdkOptions): Promise<readonly ToolCatalogSearchResult[]> {
  return await createRunxSdk(options).searchTools(options);
}

export async function inspectTool(options: InspectToolOptions & RunxSdkOptions): Promise<ToolInspectResult> {
  return await createRunxSdk(options).inspectTool(options);
}

function dedupeToolSearchResults(results: readonly ToolCatalogSearchResult[]): readonly ToolCatalogSearchResult[] {
  const deduped = new Map<string, ToolCatalogSearchResult>();
  for (const result of results) {
    deduped.set(result.catalog_ref, result);
  }
  return Array.from(deduped.values());
}

type RegistrySearchResult = SkillSearchResult & {
  readonly source: "runx-registry";
  readonly trust_tier: RegistryTrustTier;
};

class FileRegistryStore implements RegistryStore {
  constructor(private readonly root: string) {}

  async putVersion(version: RegistrySkillVersion, options?: PutVersionOptions): Promise<RegistrySkillVersion> {
    const versionPath = this.versionPath(version.skill_id, version.version);
    await mkdir(path.dirname(versionPath), { recursive: true });

    const existing = await this.getVersion(version.skill_id, version.version);
    if (existing) {
      if (existing.digest !== version.digest || existing.profile_digest !== version.profile_digest) {
        if (!options?.upsert) {
          throw new Error(`Registry version ${version.skill_id}@${version.version} already exists with a different digest.`);
        }
        const upserted = { ...version, updated_at: new Date().toISOString() };
        await writeFile(versionPath, `${JSON.stringify(upserted, null, 2)}\n`, { flag: "w", mode: 0o600 });
        return upserted;
      }
      const refreshed = {
        ...version,
        created_at: existing.created_at,
        updated_at: new Date().toISOString(),
      };
      if (JSON.stringify(existing) !== JSON.stringify(refreshed)) {
        await writeFile(versionPath, `${JSON.stringify(refreshed, null, 2)}\n`, { flag: "w", mode: 0o600 });
      }
      return refreshed;
    }

    await writeFile(versionPath, `${JSON.stringify(version, null, 2)}\n`, { flag: "wx", mode: 0o600 });
    return version;
  }

  async getVersion(skillId: string, version?: string): Promise<RegistrySkillVersion | undefined> {
    const versions = await this.listVersions(skillId);
    if (versions.length === 0) {
      return undefined;
    }
    if (!version) {
      return versions[versions.length - 1];
    }
    return versions.find((candidate) => candidate.version === version);
  }

  async listVersions(skillId: string): Promise<readonly RegistrySkillVersion[]> {
    const skillDir = this.skillDir(skillId);
    const files = await safeReadDirNames(skillDir);
    const versions = await Promise.all(
      files
        .filter((file) => file.endsWith(".json"))
        .slice()
        .sort()
        .map(async (file) => normalizeRegistrySkillVersion(JSON.parse(await readFile(path.join(skillDir, file), "utf8")))),
    );
    return versions.sort((left, right) => left.created_at.localeCompare(right.created_at) || left.version.localeCompare(right.version));
  }

  async listSkills(): Promise<readonly RegistrySkill[]> {
    const owners = await safeReadDirNames(this.root);
    const skills: RegistrySkill[] = [];
    for (const owner of owners) {
      const ownerDir = path.join(this.root, owner);
      for (const name of await safeReadDirNames(ownerDir)) {
        const skillId = `${decodePart(owner)}/${decodePart(name)}`;
        const versions = await this.listVersions(skillId);
        const latest = versions[versions.length - 1];
        if (!latest) {
          continue;
        }
        skills.push({
          skill_id: skillId,
          owner: latest.owner,
          name: latest.name,
          description: latest.description,
          latest_version: latest.version,
          latest_digest: latest.digest,
          versions,
        });
      }
    }
    return skills.sort((left, right) => left.skill_id.localeCompare(right.skill_id));
  }

  private versionPath(skillId: string, version: string): string {
    return path.join(this.skillDir(skillId), `${encodePart(version)}.json`);
  }

  private skillDir(skillId: string): string {
    const [owner, name] = splitSkillId(skillId);
    return path.join(this.root, encodePart(owner), encodePart(name));
  }
}

export function createFileRegistryStore(root: string): RegistryStore {
  return new FileRegistryStore(root);
}

export async function searchRegistry(
  store: RegistryStore,
  query: string,
  options: { readonly limit?: number; readonly registryUrl?: string } = {},
): Promise<readonly RegistrySearchResult[]> {
  const normalizedQuery = normalizeSearchText(query);
  const skills = await store.listSkills();
  const latestVersions = skills.map((skill) => skill.versions[skill.versions.length - 1]).filter(isDefined);
  const matches = latestVersions
    .filter((version) => normalizedQuery.length === 0 || registrySearchableText(version).includes(normalizedQuery))
    .sort((left, right) => left.skill_id.localeCompare(right.skill_id))
    .slice(0, options.limit ?? 20);

  return matches.map((version) => {
    const link = runxLinkForVersion(version, options.registryUrl);
    return normalizeRegistrySearchResult({
      skill_id: version.skill_id,
      name: version.name,
      summary: version.description,
      owner: version.owner,
      version: version.version,
      digest: version.digest,
      source_type: version.source_type,
      trust_tier: version.trust_tier,
      required_scopes: version.required_scopes,
      tags: version.tags,
      profile_mode: version.profile_document ? "profiled" : "portable",
      runner_names: version.runner_names,
      profile_digest: version.profile_digest,
      profile_trust_tier: version.profile_document ? version.trust_tier : undefined,
      trust_signals: deriveTrustSignals(version),
      add_command: link.install_command,
      run_command: link.run_command,
    });
  });
}

export async function publishSkillMarkdown(
  store: RegistryStore,
  markdown: string,
  options: PublishSkillMarkdownOptions = {},
): Promise<PublishSkillMarkdownResult> {
  const { registryUrl, ...createOptions } = options;
  const result = await createRegistrySkillVersion(store, markdown, createOptions);
  const link = runxLinkForVersion(result.record, registryUrl);
  return {
    status: result.created ? "published" : "unchanged",
    skill_id: result.record.skill_id,
    name: result.record.name,
    version: result.record.version,
    digest: result.record.digest,
    profile_digest: result.record.profile_digest,
    runner_names: result.record.runner_names,
    source_type: result.record.source_type,
    registry_url: registryUrl,
    link,
    record: result.record,
  };
}

async function searchRemoteRegistry(
  query: string,
  options: {
    readonly baseUrl: string;
    readonly limit?: number;
    readonly fetchImpl?: typeof fetch;
    readonly signal?: AbortSignal;
    readonly timeoutMs?: number;
  },
): Promise<readonly RegistrySearchResult[]> {
  const params = new URLSearchParams();
  if (query.trim().length > 0) {
    params.set("q", query.trim());
  }
  params.set("limit", String(options.limit ?? 20));
  const response = await fetchWithTimeout({
    fetchImpl: options.fetchImpl,
    url: `${options.baseUrl.replace(/\/$/, "")}/v1/skills?${params.toString()}`,
    signal: options.signal,
    timeoutMs: options.timeoutMs,
    description: `Registry search for '${query}'`,
  });
  if (!response.ok) {
    throw new Error(`Registry search failed for '${query}': HTTP ${response.status}`);
  }
  const payload = await response.json() as {
    readonly status?: string;
    readonly skills?: ReadonlyArray<{
      readonly skill_id?: string;
      readonly name?: string;
      readonly description?: string;
      readonly owner?: string;
      readonly version?: string;
      readonly source_type?: string;
      readonly profile_mode?: "portable" | "profiled";
      readonly runner_names?: readonly string[];
      readonly required_scopes?: readonly string[];
      readonly tags?: readonly string[];
      readonly trust_tier?: RegistryTrustTier;
      readonly trust_signals?: RegistrySearchResult["trust_signals"];
      readonly install_command?: string;
      readonly run_command?: string;
    }>;
  };
  if (payload.status !== "success" || !Array.isArray(payload.skills)) {
    throw new Error(`Registry search returned an invalid payload for '${query}'.`);
  }
  return payload.skills.map((skill) => {
    if (
      typeof skill.skill_id !== "string"
      || typeof skill.name !== "string"
      || typeof skill.owner !== "string"
      || typeof skill.source_type !== "string"
      || (skill.profile_mode !== "portable" && skill.profile_mode !== "profiled")
      || !Array.isArray(skill.runner_names)
      || !Array.isArray(skill.required_scopes)
      || !Array.isArray(skill.tags)
      || (skill.trust_tier !== "first_party" && skill.trust_tier !== "verified" && skill.trust_tier !== "community")
      || typeof skill.install_command !== "string"
      || typeof skill.run_command !== "string"
    ) {
      throw new Error(`Registry search returned an invalid skill entry for '${query}'.`);
    }
    return normalizeRegistrySearchResult({
      skill_id: skill.skill_id,
      name: skill.name,
      summary: skill.description,
      owner: skill.owner,
      version: typeof skill.version === "string" ? skill.version : undefined,
      source_type: skill.source_type,
      trust_tier: validateRegistryTrustTier(skill.trust_tier, "remote_registry.skills[].trust_tier"),
      required_scopes: skill.required_scopes,
      tags: skill.tags,
      profile_mode: skill.profile_mode,
      runner_names: skill.runner_names,
      trust_signals: Array.isArray(skill.trust_signals) ? skill.trust_signals : undefined,
      add_command: skill.install_command,
      run_command: skill.run_command,
    });
  });
}

async function searchRemoteTools(
  query: string,
  options: {
    readonly baseUrl: string;
    readonly limit?: number;
    readonly source?: string;
    readonly fetchImpl?: typeof fetch;
    readonly signal?: AbortSignal;
    readonly timeoutMs?: number;
  },
): Promise<readonly ToolCatalogSearchResult[]> {
  const params = new URLSearchParams();
  if (query.trim().length > 0) {
    params.set("q", query.trim());
  }
  if (options.source?.trim()) {
    params.set("source", options.source.trim());
  }
  params.set("limit", String(options.limit ?? 20));
  const response = await fetchWithTimeout({
    fetchImpl: options.fetchImpl,
    url: `${options.baseUrl.replace(/\/$/, "")}/v1/tools?${params.toString()}`,
    signal: options.signal,
    timeoutMs: options.timeoutMs,
    description: `Remote tool search for '${query}'`,
  });
  if (!response.ok) {
    throw new Error(`Remote tool search failed for '${query}': HTTP ${response.status}`);
  }
  const payload = await response.json() as {
    readonly status?: string;
    readonly tools?: readonly ToolCatalogSearchResult[];
  };
  if (payload.status !== "success" || !Array.isArray(payload.tools)) {
    throw new Error(`Remote tool search returned an invalid payload for '${query}'.`);
  }
  return payload.tools.map(validateRemoteToolSearchResult);
}

async function readRemoteTool(
  ref: string,
  options: {
    readonly baseUrl: string;
    readonly source?: string;
    readonly fetchImpl?: typeof fetch;
    readonly signal?: AbortSignal;
    readonly timeoutMs?: number;
  },
): Promise<ToolInspectResult | undefined> {
  const params = new URLSearchParams();
  if (options.source?.trim()) {
    params.set("source", options.source.trim());
  }
  const query = params.toString();
  const response = await fetchWithTimeout({
    fetchImpl: options.fetchImpl,
    url: `${options.baseUrl.replace(/\/$/, "")}/v1/tools/${encodeURIComponent(ref)}${query ? `?${query}` : ""}`,
    signal: options.signal,
    timeoutMs: options.timeoutMs,
    description: `Remote tool read for ${ref}`,
  });
  if (response.status === 404) {
    return undefined;
  }
  if (!response.ok) {
    throw new Error(`Remote tool read failed for ${ref}: HTTP ${response.status}`);
  }
  const payload = await response.json() as {
    readonly status?: string;
    readonly tool?: ToolInspectResult;
  };
  if (payload.status !== "success" || payload.tool === undefined) {
    throw new Error(`Remote tool read returned an invalid payload for ${ref}.`);
  }
  return validateRemoteToolInspectResult(payload.tool);
}

interface CreateRegistrySkillVersionResult {
  readonly record: RegistrySkillVersion;
  readonly created: boolean;
}

async function createRegistrySkillVersion(
  store: RegistryStore,
  markdown: string,
  options: IngestSkillOptions = {},
): Promise<CreateRegistrySkillVersionResult> {
  const record = await buildRegistrySkillVersion(markdown, options);
  const existing = await store.getVersion(record.skill_id, record.version);
  if (existing) {
    if (existing.digest !== record.digest || existing.profile_digest !== record.profile_digest) {
      if (!options.upsert) {
        throw new Error(`Registry version ${record.skill_id}@${record.version} already exists with a different digest.`);
      }
      return {
        record: await store.putVersion(record, { upsert: true }),
        created: false,
      };
    }
    return {
      record: await store.putVersion({
        ...record,
        created_at: existing.created_at,
      }),
      created: false,
    };
  }
  return {
    record: await store.putVersion(record),
    created: true,
  };
}

async function buildRegistrySkillVersion(markdown: string, options: IngestSkillOptions = {}): Promise<RegistrySkillVersion> {
  const skill = await validateSkillMarkdownViaParser(markdown, { mode: "strict" }, { env: options.parserEnv });
  const digest = hashString(markdown);
  const bindingArtifact = await buildBindingArtifact(skill, options.profileDocument, options.parserEnv);
  const catalog = resolveCatalogMetadata(bindingArtifact.manifest);
  const owner = options.owner ?? "local";
  const createdAt = options.createdAt ?? new Date().toISOString();
  const publisher = options.publisher ?? defaultRegistryPublisher(owner);
  const trustTier = options.trustTier ?? deriveRegistryTrustTier({ owner });
  const version = options.version ?? `sha-${defaultRegistryVersionSeed(digest, bindingArtifact.digest).slice(0, 12)}`;
  return {
    skill_id: buildSkillId(owner, skill.name),
    owner,
    name: skill.name,
    description: skill.description,
    version,
    digest,
    markdown,
    profile_document: options.profileDocument,
    profile_digest: bindingArtifact.digest,
    runner_names: bindingArtifact.runnerNames,
    source_type: skill.source.type,
    trust_tier: trustTier,
    catalog_kind: catalog.kind,
    catalog_audience: catalog.audience,
    catalog_visibility: catalog.visibility,
    source_metadata: options.sourceMetadata,
    attestations: mergeRegistryAttestations(
      buildPublisherAttestations(publisher, trustTier, createdAt),
      buildSourceAttestations(options.sourceMetadata, createdAt),
      options.attestations,
    ),
    required_scopes: unique([...extractScopes(skill), ...extractRunnerScopes(bindingArtifact.manifest)]),
    runtime: skill.runtime ?? readField(skill.runx, "runtime") ?? extractRunnerRuntime(bindingArtifact.manifest),
    auth: skill.auth,
    risk: skill.risk ?? readField(skill.runx, "risk"),
    runx: skill.runx,
    tags: unique([...extractTags(skill), ...extractRunnerTags(bindingArtifact.manifest)]),
    publisher,
    created_at: createdAt,
    updated_at: new Date().toISOString(),
  };
}

interface BindingArtifact {
  readonly digest?: string;
  readonly runnerNames: readonly string[];
  readonly manifest?: SkillRunnerManifest;
}

async function buildBindingArtifact(
  skill: ValidatedSkill,
  profileDocument: string | undefined,
  parserEnv?: NodeJS.ProcessEnv,
): Promise<BindingArtifact> {
  if (!profileDocument) {
    return {
      runnerNames: [],
    };
  }
  const manifest = await validateRunnerManifestYamlViaParser(profileDocument, { env: parserEnv });
  if (manifest.skill && manifest.skill !== skill.name) {
    throw new Error(`Runner manifest skill '${manifest.skill}' does not match skill '${skill.name}'.`);
  }
  return {
    digest: hashString(profileDocument),
    runnerNames: Object.keys(manifest.runners),
    manifest,
  };
}

function normalizeRegistrySkillVersion(value: unknown): RegistrySkillVersion {
  const record = requireRecord(value, "registry_version");
  const owner = requireNonEmptyString(record.owner, "registry_version.owner");
  const createdAt = requireNonEmptyString(record.created_at, "registry_version.created_at");
  const publisher = validateRegistryPublisher(record.publisher, "registry_version.publisher");
  const trustTier = normalizeRegistryVersionTrustTier(record.trust_tier);
  const sourceMetadata = validateRegistrySourceMetadata(record.source_metadata, "registry_version.source_metadata");
  const attestations = validateRegistryAttestations(record.attestations, "registry_version.attestations");
  return {
    ...(record as unknown as RegistrySkillVersion),
    skill_id: requireNonEmptyString(record.skill_id, "registry_version.skill_id"),
    owner,
    name: requireNonEmptyString(record.name, "registry_version.name"),
    description: optionalNonEmptyString(record.description, "registry_version.description"),
    version: requireNonEmptyString(record.version, "registry_version.version"),
    digest: requireNonEmptyString(record.digest, "registry_version.digest"),
    markdown: requireAnyString(record.markdown, "registry_version.markdown"),
    profile_document: stringValue(record.profile_document),
    profile_digest: optionalNonEmptyString(record.profile_digest, "registry_version.profile_digest"),
    runner_names: normalizeStringArray(record.runner_names, "registry_version.runner_names"),
    source_type: requireNonEmptyString(record.source_type, "registry_version.source_type"),
    trust_tier: trustTier,
    catalog_kind: record.catalog_kind === "graph" ? "graph" : "skill",
    catalog_audience: record.catalog_audience === "builder" || record.catalog_audience === "operator" ? record.catalog_audience : "public",
    catalog_visibility: record.catalog_visibility === "private" ? "private" : "public",
    source_metadata: sourceMetadata,
    attestations: normalizeRegistryAttestations(attestations, sourceMetadata, publisher, trustTier, createdAt),
    required_scopes: normalizeStringArray(record.required_scopes, "registry_version.required_scopes"),
    tags: normalizeStringArray(record.tags, "registry_version.tags"),
    publisher,
    created_at: createdAt,
    updated_at: optionalNonEmptyString(record.updated_at, "registry_version.updated_at") ?? createdAt,
  };
}

function normalizeRegistrySearchResult(
  input: Omit<RegistrySearchResult, "source" | "source_label">,
): RegistrySearchResult {
  return {
    ...input,
    source: "runx-registry",
    source_label: "runx registry",
  };
}

function runxLinkForVersion(record: RegistrySkillVersion, registryUrl?: string): RunxLinkResolution {
  const ref = `${record.skill_id}@${record.version}`;
  const registryFlag = registryUrl ? ` --registry ${registryUrl}` : "";
  return {
    link: `runx://skill/${encodeURIComponent(record.skill_id)}@${encodeURIComponent(record.version)}`,
    skill_id: record.skill_id,
    version: record.version,
    digest: record.digest,
    registry_url: registryUrl,
    install_command: `runx skill add ${ref}${registryFlag}`,
    run_command: `runx skill ${record.name}`,
  };
}

function buildSkillId(owner: string, name: string): string {
  return `${slugify(owner)}/${slugify(name)}`;
}

function splitSkillId(skillId: string): readonly [string, string] {
  const parts = skillId.split("/");
  if (parts.length !== 2 || !parts[0] || !parts[1]) {
    throw new Error(`Invalid registry skill id '${skillId}'. Expected '<owner>/<name>'.`);
  }
  return [parts[0], parts[1]];
}

function slugify(value: string): string {
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

function encodePart(value: string): string {
  return encodeURIComponent(value);
}

function decodePart(value: string): string {
  return decodeURIComponent(value);
}

function defaultRegistryVersionSeed(markdownDigest: string, profileDigest: string | undefined): string {
  if (!profileDigest) {
    return markdownDigest;
  }
  return hashString(JSON.stringify({
    markdown_digest: markdownDigest,
    profile_digest: profileDigest,
  }));
}

function resolveCatalogMetadata(manifest: SkillRunnerManifest | undefined): CatalogMetadata {
  return manifest?.catalog ?? {
    kind: "skill",
    audience: "public",
    visibility: "public",
  };
}

function defaultRegistryPublisher(owner: string): RegistryPublisher {
  return owner === "runx"
    ? { kind: "organization", id: owner, handle: owner }
    : { kind: "publisher", id: owner, handle: owner };
}

function deriveRegistryTrustTier(options: {
  readonly owner: string;
  readonly trust_tier?: RegistryTrustTier;
}): RegistryTrustTier {
  if (options.trust_tier === "first_party" || options.trust_tier === "verified" || options.trust_tier === "community") {
    return options.trust_tier;
  }
  if (options.owner === "runx") {
    return "first_party";
  }
  return "community";
}

function buildSourceAttestations(
  sourceMetadata: RegistrySourceMetadata | undefined,
  issuedAt: string,
): readonly RegistryAttestation[] {
  if (!sourceMetadata) {
    return [];
  }
  return [
    {
      kind: "source",
      id: `${sourceMetadata.provider}_source`,
      status: "verified",
      summary: `${sourceMetadata.provider}:${sourceMetadata.repo}@${sourceMetadata.sha}`,
      source: sourceMetadata.repo_url,
      issued_at: issuedAt,
      metadata: {
        repo: sourceMetadata.repo,
        ref: sourceMetadata.ref,
        sha: sourceMetadata.sha,
        event: sourceMetadata.event,
        skill_path: sourceMetadata.skill_path,
        profile_path: sourceMetadata.profile_path,
      },
    },
  ];
}

function buildPublisherAttestations(
  publisher: RegistryPublisher,
  trustTier: RegistryTrustTier,
  issuedAt: string,
): readonly RegistryAttestation[] {
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

function mergeRegistryAttestations(
  ...groups: readonly (readonly RegistryAttestation[] | undefined)[]
): readonly RegistryAttestation[] | undefined {
  const merged = new Map<string, RegistryAttestation>();
  for (const group of groups) {
    if (!group) {
      continue;
    }
    for (const attestation of group) {
      merged.set(`${attestation.kind}:${attestation.id}`, attestation);
    }
  }
  return merged.size > 0 ? Array.from(merged.values()) : undefined;
}

function normalizeRegistryAttestations(
  attestations: readonly RegistryAttestation[] | undefined,
  sourceMetadata: RegistrySourceMetadata | undefined,
  publisher: RegistryPublisher,
  trustTier: RegistryTrustTier,
  createdAt: string,
): readonly RegistryAttestation[] | undefined {
  return mergeRegistryAttestations(
    buildPublisherAttestations(publisher, trustTier, createdAt),
    buildSourceAttestations(sourceMetadata, createdAt),
    attestations,
  );
}

function deriveTrustSignals(version: RegistrySkillVersion): SkillSearchResult["trust_signals"] {
  const trustTier = deriveRegistryTrustTier(version);
  const provenance = sourceProvenance(version.source_metadata, version.attestations);
  const publisherAttestation = version.attestations?.find((attestation) => attestation.kind === "publisher");
  const publisherLabel = version.publisher.display_name ?? version.publisher.handle ?? version.publisher.id;
  return [
    {
      id: "digest",
      label: "Immutable digest",
      status: "verified",
      value: `sha256:${version.digest}`,
    },
    {
      id: "trust_tier",
      label: "Trust tier",
      status: trustTier === "community" ? "declared" : "verified",
      value: trustTier,
    },
    {
      id: "publisher",
      label: "Publisher identity",
      status: publisherAttestation?.status ?? "not_declared",
      value: publisherLabel,
    },
    {
      id: "provenance",
      label: "Source provenance",
      status: provenance ? "verified" : "not_declared",
      value: provenance ?? "no source attestation",
    },
    {
      id: "source_type",
      label: "Execution source",
      status: "declared",
      value: version.source_type,
    },
    {
      id: "scopes",
      label: "Required scopes",
      status: version.required_scopes.length > 0 ? "declared" : "not_declared",
      value: version.required_scopes.length > 0 ? version.required_scopes.join(", ") : "none declared",
    },
    {
      id: "runtime",
      label: "Runtime requirements",
      status: version.runtime ? "declared" : "not_declared",
      value: version.runtime ? "declared in skill metadata" : "none declared",
    },
    {
      id: "runner_metadata",
      label: "Materialized binding",
      status: version.profile_digest ? "verified" : "not_declared",
      value: version.profile_digest
        ? `${version.runner_names.length} runner(s), binding sha256:${version.profile_digest}`
        : "portable agent runner",
    },
  ];
}

function sourceProvenance(
  sourceMetadata: RegistrySourceMetadata | undefined,
  attestations: readonly RegistryAttestation[] | undefined,
): string | undefined {
  if (sourceMetadata) {
    return `${sourceMetadata.provider}:${sourceMetadata.repo}@${sourceMetadata.sha}`;
  }
  const sourceAttestation = attestations?.find((attestation) => attestation.kind === "source");
  return sourceAttestation?.summary;
}

function extractScopes(skill: ValidatedSkill): readonly string[] {
  const authScopes = recordArrayField(skill.auth, "scopes");
  const runxScopes = recordArrayField(skill.runx, "scopes");
  return unique([...authScopes, ...runxScopes]);
}

function extractRunnerScopes(manifest: SkillRunnerManifest | undefined): readonly string[] {
  if (!manifest) {
    return [];
  }
  return unique(
    Object.values(manifest.runners).flatMap((runner) => [
      ...recordArrayField(runner.auth, "scopes"),
      ...recordArrayField(runner.raw.runx, "scopes"),
    ]),
  );
}

function extractRunnerRuntime(manifest: SkillRunnerManifest | undefined): unknown {
  if (!manifest) {
    return undefined;
  }
  const runnersWithRuntime = Object.values(manifest.runners)
    .filter((runner) => runner.runtime !== undefined)
    .map((runner) => runner.name);
  return runnersWithRuntime.length > 0 ? { runners: runnersWithRuntime } : undefined;
}

function extractRunnerTags(manifest: SkillRunnerManifest | undefined): readonly string[] {
  if (!manifest) {
    return [];
  }
  return unique(Object.values(manifest.runners).flatMap((runner) => recordArrayField(runner.raw.runx, "tags")));
}

function extractTags(skill: ValidatedSkill): readonly string[] {
  return unique(recordArrayField(skill.runx, "tags"));
}

function registrySearchableText(version: RegistrySkillVersion): string {
  return normalizeSearchText(
    [
      version.skill_id,
      version.name,
      version.description,
      version.owner,
      version.source_type,
      ...version.runner_names,
      ...version.tags,
    ].filter(isDefined).join(" "),
  );
}

function normalizeSearchText(value: string): string {
  return value.trim().toLowerCase();
}

function validateRegistryPublisher(value: unknown, label = "publisher"): RegistryPublisher {
  const record = requireRecord(value, label);
  const kind = record.kind;
  if (
    kind !== "organization"
    && kind !== "user"
    && kind !== "team"
    && kind !== "service"
    && kind !== "publisher"
  ) {
    throw new Error(`${label}.kind must be one of organization, user, team, service, or publisher.`);
  }
  return {
    kind,
    id: requireNonEmptyString(record.id, `${label}.id`),
    handle: optionalNonEmptyString(record.handle, `${label}.handle`),
    display_name: optionalNonEmptyString(record.display_name, `${label}.display_name`),
  };
}

function normalizeRegistryVersionTrustTier(value: unknown): RegistryTrustTier {
  if (value === undefined || value === null) {
    return "community";
  }
  return validateRegistryTrustTier(value, "registry_version.trust_tier");
}

function validateRegistryTrustTier(value: unknown, label = "trust_tier"): RegistryTrustTier {
  if (value === "first_party" || value === "verified" || value === "community") {
    return value;
  }
  throw new Error(`${label} must be one of first_party, verified, or community.`);
}

function validateRegistryAttestations(
  value: unknown,
  label = "attestations",
): readonly RegistryAttestation[] | undefined {
  if (value === undefined) {
    return undefined;
  }
  if (!Array.isArray(value)) {
    throw new Error(`${label} must be an array when provided.`);
  }
  return value.map((entry, index) => validateRegistryAttestation(entry, `${label}[${index}]`));
}

function validateRegistryAttestation(value: unknown, label: string): RegistryAttestation {
  const record = requireRecord(value, label);
  const kind = record.kind;
  if (kind !== "source" && kind !== "publisher" && kind !== "verification") {
    throw new Error(`${label}.kind must be one of source, publisher, or verification.`);
  }
  const status = record.status;
  if (status !== "verified" && status !== "declared") {
    throw new Error(`${label}.status must be one of verified or declared.`);
  }
  const metadata = record.metadata;
  if (metadata !== undefined && !isRecord(metadata)) {
    throw new Error(`${label}.metadata must be an object when provided.`);
  }
  return {
    kind,
    id: requireNonEmptyString(record.id, `${label}.id`),
    status,
    summary: requireNonEmptyString(record.summary, `${label}.summary`),
    source: optionalNonEmptyString(record.source, `${label}.source`),
    issued_at: optionalNonEmptyString(record.issued_at, `${label}.issued_at`),
    metadata: metadata as Readonly<Record<string, unknown>> | undefined,
  };
}

function validateRegistrySourceMetadata(
  value: unknown,
  label = "source_metadata",
): RegistrySourceMetadata | undefined {
  if (value === undefined) {
    return undefined;
  }
  const record = requireRecord(value, label);
  const provider = record.provider;
  if (provider !== "github") {
    throw new Error(`${label}.provider must be github.`);
  }
  const event = record.event;
  if (event !== "enrollment" && event !== "push" && event !== "tag" && event !== "tombstone") {
    throw new Error(`${label}.event must be one of enrollment, push, tag, or tombstone.`);
  }
  return {
    provider,
    repo: requireNonEmptyString(record.repo, `${label}.repo`),
    repo_url: requireNonEmptyString(record.repo_url, `${label}.repo_url`),
    skill_path: requireNonEmptyString(record.skill_path, `${label}.skill_path`),
    profile_path: optionalNonEmptyString(record.profile_path, `${label}.profile_path`),
    ref: requireNonEmptyString(record.ref, `${label}.ref`),
    sha: requireNonEmptyString(record.sha, `${label}.sha`),
    default_branch: requireNonEmptyString(record.default_branch, `${label}.default_branch`),
    event,
    immutable: requireBoolean(record.immutable, `${label}.immutable`),
    live: requireBoolean(record.live, `${label}.live`),
    tombstoned: optionalBoolean(record.tombstoned, `${label}.tombstoned`),
    tag: optionalNonEmptyString(record.tag, `${label}.tag`),
    publisher_handle: optionalNonEmptyString(record.publisher_handle, `${label}.publisher_handle`),
  };
}

function validateRemoteToolSearchResult(value: unknown): ToolCatalogSearchResult {
  const record = requireRecord(value, "remote_tools.tools[]");
  return {
    tool_id: requireAnyString(record.tool_id, "remote_tools.tools[].tool_id"),
    name: requireAnyString(record.name, "remote_tools.tools[].name"),
    summary: stringValue(record.summary),
    source: requireAnyString(record.source, "remote_tools.tools[].source"),
    source_label: requireAnyString(record.source_label, "remote_tools.tools[].source_label"),
    source_type: requireAnyString(record.source_type, "remote_tools.tools[].source_type"),
    namespace: requireAnyString(record.namespace, "remote_tools.tools[].namespace"),
    external_name: requireAnyString(record.external_name, "remote_tools.tools[].external_name"),
    required_scopes: requireStringArray(record.required_scopes, "remote_tools.tools[].required_scopes"),
    tags: requireStringArray(record.tags, "remote_tools.tools[].tags"),
    catalog_ref: requireAnyString(record.catalog_ref, "remote_tools.tools[].catalog_ref"),
  };
}

function validateRemoteToolInspectResult(value: unknown): ToolInspectResult {
  const record = requireRecord(value, "remote_tools.tool");
  const inputsRecord = requireRecord(record.inputs, "remote_tools.tool.inputs");
  const inputs: Record<string, { readonly type: string; readonly required: boolean; readonly description?: string }> = {};
  for (const [name, entry] of Object.entries(inputsRecord)) {
    const input = requireRecord(entry, `remote_tools.tool.inputs.${name}`);
    inputs[name] = {
      type: requireAnyString(input.type, `remote_tools.tool.inputs.${name}.type`),
      required: requireBoolean(input.required, `remote_tools.tool.inputs.${name}.required`),
      description: stringValue(input.description),
    };
  }
  const provenanceRecord = requireRecord(record.provenance, "remote_tools.tool.provenance");
  return {
    ref: requireAnyString(record.ref, "remote_tools.tool.ref"),
    name: requireAnyString(record.name, "remote_tools.tool.name"),
    description: stringValue(record.description),
    execution_source_type: requireAnyString(record.execution_source_type, "remote_tools.tool.execution_source_type"),
    inputs,
    scopes: requireStringArray(record.scopes, "remote_tools.tool.scopes"),
    mutating: optionalBoolean(record.mutating, "remote_tools.tool.mutating"),
    runtime: optionalRecord(record.runtime, "remote_tools.tool.runtime"),
    risk: optionalRecord(record.risk, "remote_tools.tool.risk"),
    runx: optionalRecord(record.runx, "remote_tools.tool.runx"),
    reference_path: requireAnyString(record.reference_path, "remote_tools.tool.reference_path"),
    skill_directory: requireAnyString(record.skill_directory, "remote_tools.tool.skill_directory"),
    provenance: {
      origin: requireEnum(provenanceRecord.origin, "remote_tools.tool.provenance.origin", ["local", "imported"]) as "local" | "imported",
      source: stringValue(provenanceRecord.source),
      source_label: stringValue(provenanceRecord.source_label),
      source_type: stringValue(provenanceRecord.source_type),
      namespace: stringValue(provenanceRecord.namespace),
      external_name: stringValue(provenanceRecord.external_name),
      catalog_ref: stringValue(provenanceRecord.catalog_ref),
      tool_id: stringValue(provenanceRecord.tool_id),
      tags: optionalStringArray(provenanceRecord.tags, "remote_tools.tool.provenance.tags"),
    },
  };
}

function normalizeStringArray(value: unknown, label: string): readonly string[] {
  if (value === undefined) {
    return [];
  }
  if (!Array.isArray(value)) {
    throw new Error(`${label} must be an array.`);
  }
  return value.map((entry, index) => requireNonEmptyString(entry, `${label}[${index}]`));
}

function recordArrayField(value: unknown, field: string): readonly string[] {
  if (!isRecord(value)) {
    return [];
  }
  const arrayValue = value[field];
  if (!Array.isArray(arrayValue)) {
    return [];
  }
  return arrayValue.filter((item): item is string => typeof item === "string" && item.length > 0);
}

function requireRecord(value: unknown, label: string): Record<string, unknown> {
  if (!isRecord(value)) {
    throw new Error(`${label} must be an object.`);
  }
  return value;
}

function requireNonEmptyString(value: unknown, label: string): string {
  if (typeof value !== "string" || value.length === 0) {
    throw new Error(`${label} must be a non-empty string.`);
  }
  return value;
}

function optionalNonEmptyString(value: unknown, label: string): string | undefined {
  if (value === undefined || value === null) {
    return undefined;
  }
  return requireNonEmptyString(value, label);
}

function requireStringArray(value: unknown, label: string): readonly string[] {
  if (!Array.isArray(value) || value.some((entry) => typeof entry !== "string")) {
    throw new Error(`${label} must be an array of strings.`);
  }
  return value;
}

function optionalStringArray(value: unknown, label: string): readonly string[] | undefined {
  if (value === undefined) {
    return undefined;
  }
  return requireStringArray(value, label);
}

function requireBoolean(value: unknown, label: string): boolean {
  if (typeof value !== "boolean") {
    throw new Error(`${label} must be a boolean.`);
  }
  return value;
}

function optionalBoolean(value: unknown, label: string): boolean | undefined {
  if (value === undefined) {
    return undefined;
  }
  return requireBoolean(value, label);
}

function optionalRecord(value: unknown, label: string): Readonly<Record<string, unknown>> | undefined {
  if (value === undefined) {
    return undefined;
  }
  return requireRecord(value, label);
}

function requireEnum(value: unknown, label: string, allowed: readonly string[]): string {
  if (typeof value !== "string" || !allowed.includes(value)) {
    throw new Error(`${label} must be one of ${allowed.join(", ")}.`);
  }
  return value;
}

export interface RunxInstallState {
  readonly version: 1;
  readonly installation_id: string;
  readonly created_at: string;
}

export async function ensureRunxInstallState(
  globalHomeDir: string,
  now: () => string = () => new Date().toISOString(),
): Promise<{ readonly state: RunxInstallState; readonly created: boolean }> {
  const existing = await readRunxInstallState(globalHomeDir);
  if (existing) {
    return {
      state: existing,
      created: false,
    };
  }
  const state: RunxInstallState = {
    version: 1,
    installation_id: `inst_${randomUUID()}`,
    created_at: now(),
  };
  await mkdir(globalHomeDir, { recursive: true });
  await writeFile(path.join(globalHomeDir, "install.json"), `${JSON.stringify(state, null, 2)}\n`, { mode: 0o600 });
  return {
    state,
    created: true,
  };
}

export async function readRunxInstallState(globalHomeDir: string): Promise<RunxInstallState | undefined> {
  const installPath = path.join(globalHomeDir, "install.json");
  let contents: string;
  try {
    contents = await readFile(installPath, "utf8");
  } catch (error) {
    if (isNotFound(error)) {
      return undefined;
    }
    throw error;
  }
  const parsed: unknown = JSON.parse(contents);
  if (
    !isRecord(parsed)
    || parsed.version !== 1
    || typeof parsed.installation_id !== "string"
    || typeof parsed.created_at !== "string"
  ) {
    throw new Error(`${installPath} is not a valid Runx install state.`);
  }
  return {
    version: 1,
    installation_id: parsed.installation_id,
    created_at: parsed.created_at,
  };
}

export async function add(options: AddSkillOptions & RunxSdkOptions): Promise<InstallLocalSkillResult> {
  return await createRunxSdk(options).addSkill(options);
}

export async function publish(options: PublishSkillOptions & RunxSdkOptions): Promise<PublishSkillMarkdownResult> {
  return await createRunxSdk(options).publishSkill(options);
}
