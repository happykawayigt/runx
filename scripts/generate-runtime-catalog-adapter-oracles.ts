import { statSync } from "node:fs";
import { mkdir, mkdtemp, readFile, readdir, rm, stat, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  type ActReceiptEnvelopeContract,
  validateActReceiptEnvelopeContract,
} from "../packages/contracts/src/index.js";
import type {
  ToolCatalogAdapter,
  ToolCatalogResolvedTool,
} from "../packages/runtime-local/src/tool-catalogs/index.js";
import { parseToolManifestJson, validateToolManifest } from "../packages/core/src/parser/index.js";
import { invokeCatalog } from "../packages/adapters/src/catalog/index.js";
import { invokeCliTool } from "../packages/adapters/src/cli-tool/index.js";
import { createFixtureMcpToolCatalogAdapter } from "../packages/runtime-local/src/tool-catalogs/index.js";

const workspaceRoot = path.resolve(fileURLToPath(new URL("..", import.meta.url)));
const fixtureRoot = path.join(workspaceRoot, "fixtures", "runtime", "adapters", "catalog");
const oracleRoot = path.join(fixtureRoot, "oracles");
const check = process.argv.includes("--check");

process.chdir(workspaceRoot);

type ActReceiptEnvelope = ActReceiptEnvelopeContract;
const validateActReceiptEnvelope = validateActReceiptEnvelopeContract;

type JsonValue = null | boolean | number | string | JsonValue[] | { readonly [key: string]: JsonValue };

interface RuntimeCatalogAdapterRequest {
  readonly case: string;
  readonly mode: "catalog-adapter";
  readonly catalogAdapters: readonly ("fixture-mcp" | "local-manifest")[];
  readonly skillName: string;
  readonly source: {
    readonly type: "catalog";
    readonly args: readonly string[];
    readonly catalogRef?: string;
    readonly raw: {
      readonly type: "catalog";
      readonly catalog_ref?: string;
    };
  };
  readonly inputs: Readonly<Record<string, unknown>>;
}

interface OracleCase {
  readonly name: string;
  readonly request: RuntimeCatalogAdapterRequest;
  readonly expectedStatus: "success" | "failure";
  readonly files?: Readonly<Record<string, string>>;
}

const cases: readonly OracleCase[] = [
  {
    name: "missing-catalog-ref",
    expectedStatus: "failure",
    request: {
      case: "missing-catalog-ref",
      mode: "catalog-adapter",
      catalogAdapters: [],
      skillName: "missing-catalog-ref",
      source: {
        type: "catalog",
        args: [],
        raw: {
          type: "catalog",
        },
      },
      inputs: {},
    },
  },
  {
    name: "missing-imported-tool",
    expectedStatus: "failure",
    request: {
      case: "missing-imported-tool",
      mode: "catalog-adapter",
      catalogAdapters: ["fixture-mcp"],
      skillName: "missing-imported-tool",
      source: catalogSource("fixture-mcp:fixture.missing"),
      inputs: {
        message: "not-found",
      },
    },
  },
  {
    name: "fixture-success",
    expectedStatus: "success",
    request: {
      case: "fixture-success",
      mode: "catalog-adapter",
      catalogAdapters: ["fixture-mcp"],
      skillName: "fixture-success",
      source: catalogSource("fixture-mcp:fixture.echo"),
      inputs: {
        message: "catalog fixture success",
      },
    },
  },
  {
    name: "fixture-failure",
    expectedStatus: "failure",
    request: {
      case: "fixture-failure",
      mode: "catalog-adapter",
      catalogAdapters: ["fixture-mcp"],
      skillName: "fixture-failure",
      source: catalogSource("fixture-mcp:fixture.fail"),
      inputs: {
        message: "catalog fixture failure",
      },
    },
  },
  {
    name: "local-precedence",
    expectedStatus: "success",
    request: {
      case: "local-precedence",
      mode: "catalog-adapter",
      catalogAdapters: ["local-manifest", "fixture-mcp"],
      skillName: "local-precedence",
      source: catalogSource("fixture.echo"),
      inputs: {
        message: "catalog fixture collision",
      },
    },
    files: {
      "tools/fixture/echo/manifest.json": `${JSON.stringify({
        schema: "runx.tool.manifest.v1",
        name: "fixture.echo",
        description: "Local tool that wins over the fixture MCP catalog collision.",
        source: {
          type: "cli-tool",
          command: "node",
          args: ["./run.mjs"],
          sandbox: {
            profile: "workspace-write",
          },
        },
        inputs: {
          message: {
            type: "string",
            required: true,
            description: "Message to echo.",
          },
        },
        output: {},
        scopes: ["fixture.local"],
        runtime: {
          command: "node",
          args: ["./run.mjs"],
        },
        source_hash: "sha256:local-precedence-source",
        schema_hash: "sha256:local-precedence-schema",
        toolkit_version: "0.1.5",
      }, null, 2)}\n`,
      "tools/fixture/echo/run.mjs": [
        "#!/usr/bin/env node",
        "const message = process.env.RUNX_INPUT_MESSAGE || \"\";",
        "process.stdout.write(`local:${message}`);",
        "",
      ].join("\n"),
    },
  },
];

const tempRoot = await mkdtemp(path.join(os.tmpdir(), "runx-runtime-catalog-adapter-oracles-"));
const expectedOracleFiles = new Set<string>();

try {
  for (const oracleCase of cases) {
    await materializeCaseFixture(oracleCase);
    await runOracleCase(oracleCase);
  }

  if (check) {
    await checkNoStaleOracleFiles();
  }

  console.log(`${check ? "checked" : "generated"} ${cases.length} runtime catalog adapter oracle cases`);
} finally {
  await rm(tempRoot, { recursive: true, force: true });
}

function catalogSource(ref: string): RuntimeCatalogAdapterRequest["source"] {
  return {
    type: "catalog",
    args: [],
    catalogRef: ref,
    raw: {
      type: "catalog",
      catalog_ref: ref,
    },
  };
}

async function materializeCaseFixture(oracleCase: OracleCase): Promise<void> {
  const caseDir = casePath(oracleCase.name);
  await writeOrCheck(
    path.join(caseDir, "request.json"),
    `${JSON.stringify(oracleCase.request, null, 2)}\n`,
  );
  for (const [relativePath, contents] of Object.entries(oracleCase.files ?? {})) {
    await writeOrCheck(path.join(caseDir, relativePath), contents);
  }
}

async function runOracleCase(oracleCase: OracleCase): Promise<void> {
  const caseDir = casePath(oracleCase.name);
  const env = deterministicEnv(caseDir, path.join(tempRoot, oracleCase.name), oracleCase.request.catalogAdapters);
  const adapters = await toolCatalogAdapters(oracleCase, env);
  const receipt = validateActReceiptEnvelope(await invokeCatalog({
    skillName: oracleCase.request.skillName,
    source: oracleCase.request.source,
    inputs: oracleCase.request.inputs,
    skillDirectory: caseDir,
    env,
    runId: "run_catalog_adapter_oracle",
    stepId: oracleCase.name,
    toolCatalogAdapters: adapters,
  }), `${oracleCase.name}.receipt`);

  if (receipt.status !== oracleCase.expectedStatus) {
    throw new Error(`${oracleCase.name}: expected status ${oracleCase.expectedStatus}, got ${receipt.status}`);
  }

  const normalized = normalizeReceipt(receipt);
  const stdout = String(normalized.stdout ?? "");
  const stderr = String(normalized.stderr ?? "");
  const status = String(normalized.status);
  const json = `${JSON.stringify(normalized, null, 2)}\n`;

  assertCleanOracle(oracleCase.name, stdout);
  assertCleanOracle(oracleCase.name, stderr);
  assertCleanOracle(oracleCase.name, status);
  assertCleanOracle(oracleCase.name, json);

  await writeOracle(oracleCase.name, "stdout", stdout);
  await writeOracle(oracleCase.name, "stderr", stderr);
  await writeOracle(oracleCase.name, "status", `${status}\n`);
  await writeOracle(oracleCase.name, "json", json);
}

async function toolCatalogAdapters(
  oracleCase: OracleCase,
  env: NodeJS.ProcessEnv,
): Promise<readonly ToolCatalogAdapter[]> {
  return oracleCase.request.catalogAdapters.map((adapter) => {
    if (adapter === "fixture-mcp") {
      return createFixtureMcpToolCatalogAdapter();
    }
    if (adapter === "local-manifest") {
      return createLocalManifestToolCatalogAdapter(casePath(oracleCase.name), env);
    }
    throw new Error(`${oracleCase.name}: unsupported catalog adapter '${adapter}'`);
  });
}

function createLocalManifestToolCatalogAdapter(caseDir: string, env: NodeJS.ProcessEnv): ToolCatalogAdapter {
  return {
    source: "local-manifest",
    label: "Local Manifest Fixture Catalog",
    search: async () => [],
    resolve: async (ref, options = {}) => {
      const manifestPath = localManifestPath(options.searchFromDirectory ?? caseDir, ref);
      if (!manifestPath) {
        return undefined;
      }
      const tool = validateToolManifest(parseToolManifestJson(await readFile(manifestPath, "utf8")));
      const skillDirectory = path.dirname(manifestPath);
      return {
        tool,
        result: {
          tool_id: `local/${tool.name}`,
          name: tool.name,
          summary: tool.description,
          source: "local-manifest",
          source_label: "Local Manifest Fixture Catalog",
          source_type: tool.source.type,
          namespace: tool.name.split(".")[0] ?? "local",
          external_name: tool.name.split(".").slice(1).join(".") || tool.name,
          required_scopes: tool.scopes,
          tags: ["local"],
          catalog_ref: ref,
        },
        skillDirectory,
        referencePath: manifestPath,
        invoke: async (request) => {
          const result = await invokeCliTool({
            source: tool.source,
            inputs: request.inputs,
            resolvedInputs: request.resolvedInputs,
            skillDirectory,
            env: request.env ?? env,
            signal: request.signal,
          });
          return {
            status: result.status,
            stdout: result.stdout,
            stderr: result.stderr,
            errorMessage: result.errorMessage,
            metadata: result.metadata,
          };
        },
      } satisfies ToolCatalogResolvedTool;
    },
  };
}

function localManifestPath(searchFromDirectory: string, ref: string): string | undefined {
  const segments = ref.split(".").filter(Boolean);
  if (segments.length < 2) {
    return undefined;
  }
  let current = path.resolve(searchFromDirectory);
  while (true) {
    const candidate = path.join(current, "tools", ...segments, "manifest.json");
    if (fileExists(candidate)) {
      return candidate;
    }
    const parent = path.dirname(current);
    if (parent === current) {
      return undefined;
    }
    current = parent;
  }
}

function fileExists(filePath: string): boolean {
  try {
    return statSyncFile(filePath);
  } catch {
    return false;
  }
}

function statSyncFile(filePath: string): boolean {
  return statSync(filePath).isFile();
}

function deterministicEnv(
  cwd: string,
  caseTempRoot: string,
  catalogAdapters: readonly string[],
): NodeJS.ProcessEnv {
  return stripUndefined({
    CI: "1",
    FORCE_COLOR: "0",
    HOME: path.join(caseTempRoot, "home"),
    INIT_CWD: cwd,
    LANG: "C",
    LC_ALL: "C",
    NO_COLOR: "1",
    PATH: process.env.PATH,
    RUNX_CWD: cwd,
    RUNX_ENABLE_FIXTURE_TOOL_CATALOG: catalogAdapters.includes("fixture-mcp") ? "1" : undefined,
    RUNX_HOME: path.join(caseTempRoot, "runx-home"),
    RUNX_KNOWLEDGE_DIR: path.join(caseTempRoot, "knowledge"),
    RUNX_OFFICIAL_SKILLS_DIR: path.join(caseTempRoot, "official-skills"),
    RUNX_PROJECT_DIR: path.join(caseTempRoot, "project"),
    RUNX_REGISTRY_DIR: path.join(caseTempRoot, "registry"),
    RUNX_REGISTRY_URL: "",
    RUNX_SANDBOX_REQUIRE_ENFORCEMENT: "0",
    TEMP: path.join(caseTempRoot, "tmp"),
    TMP: path.join(caseTempRoot, "tmp"),
    TMPDIR: path.join(caseTempRoot, "tmp"),
    TZ: "UTC",
    SystemRoot: process.env.SystemRoot,
    WINDIR: process.env.WINDIR,
  });
}

function stripUndefined(value: Record<string, string | undefined>): NodeJS.ProcessEnv {
  return Object.fromEntries(
    Object.entries(value).filter((entry): entry is [string, string] => entry[1] !== undefined),
  );
}

function normalizeReceipt(receipt: ActReceiptEnvelope): JsonValue {
  return normalizeValue({
    ...receipt,
    durationMs: 0,
  });
}

function normalizeValue(value: unknown): JsonValue {
  if (value === undefined) {
    return null;
  }
  if (value === null || typeof value === "boolean" || typeof value === "number") {
    return value;
  }
  if (typeof value === "string") {
    return normalizeString(value);
  }
  if (Array.isArray(value)) {
    return value.map((entry) => normalizeValue(entry));
  }
  if (typeof value === "object") {
    return Object.fromEntries(
      Object.entries(value as Record<string, unknown>)
        .filter(([, entry]) => entry !== undefined)
        .map(([key, entry]) => [key, normalizeValue(entry)]),
    );
  }
  return String(value);
}

function normalizeString(value: string): string {
  return value
    .split(workspaceRoot).join("<repo>")
    .split(tempRoot).join("<temp>")
    .replaceAll("\\", "/");
}

async function writeOracle(name: string, extension: string, contents: string): Promise<void> {
  const filePath = path.join(oracleRoot, `${name}.${extension}`);
  expectedOracleFiles.add(filePath);
  await writeOrCheck(filePath, contents);
}

async function writeOrCheck(filePath: string, contents: string): Promise<void> {
  if (check) {
    const existing = await readFile(filePath, "utf8");
    if (existing !== contents) {
      throw new Error(`stale runtime catalog adapter fixture: ${path.relative(workspaceRoot, filePath)}`);
    }
    return;
  }
  await mkdir(path.dirname(filePath), { recursive: true });
  await writeFile(filePath, contents);
}

async function checkNoStaleOracleFiles(): Promise<void> {
  for (const filePath of await collectFiles(oracleRoot)) {
    if (!expectedOracleFiles.has(filePath)) {
      throw new Error(`stale runtime catalog adapter oracle file: ${path.relative(workspaceRoot, filePath)}`);
    }
  }
}

async function collectFiles(directory: string): Promise<readonly string[]> {
  try {
    const directoryStat = await stat(directory);
    if (!directoryStat.isDirectory()) {
      return [];
    }
  } catch {
    return [];
  }

  const files: string[] = [];
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const entryPath = path.join(directory, entry.name);
    if (entry.isDirectory()) {
      files.push(...await collectFiles(entryPath));
    } else if (entry.isFile()) {
      files.push(entryPath);
    }
  }
  return files.sort();
}

function assertCleanOracle(name: string, contents: string): void {
  const forbidden = [
    workspaceRoot,
    tempRoot,
    "OPENAI_API_KEY",
    "ANTHROPIC_API_KEY",
    "GITHUB_TOKEN",
  ];
  for (const value of forbidden) {
    if (value && contents.includes(value)) {
      throw new Error(`${name}: oracle contains forbidden value '${value}'`);
    }
  }
  if (/\b(?:sk-[A-Za-z0-9_-]+|ghp_[A-Za-z0-9_]+)\b/.test(contents)) {
    throw new Error(`${name}: oracle appears to contain a secret token`);
  }
  if (/\b20\d{2}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?Z\b/.test(contents)) {
    throw new Error(`${name}: oracle contains a wall-clock timestamp`);
  }
}

function casePath(name: string): string {
  return path.join(fixtureRoot, name);
}
