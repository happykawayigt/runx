import { mkdir, mkdtemp, readFile, readdir, rm, stat, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  type ActReceiptEnvelopeContract,
  validateActReceiptEnvelopeContract,
} from "../packages/contracts/src/index.js";
import { invokeMcp } from "../packages/adapters/src/mcp/index.js";

type ActReceiptEnvelope = ActReceiptEnvelopeContract;
const validateActReceiptEnvelope = validateActReceiptEnvelopeContract;

const workspaceRoot = path.resolve(fileURLToPath(new URL("..", import.meta.url)));
const fixtureRoot = path.join(workspaceRoot, "fixtures", "runtime", "adapters", "mcp");
const oracleRoot = path.join(fixtureRoot, "oracles");
const check = process.argv.includes("--check");

process.chdir(workspaceRoot);

type JsonValue = null | boolean | number | string | JsonValue[] | { readonly [key: string]: JsonValue };

interface RuntimeMcpAdapterRequest {
  readonly case: string;
  readonly mode: "mcp-adapter";
  readonly skillName: string;
  readonly source: {
    readonly type: "mcp";
    readonly args: readonly string[];
    readonly server?: {
      readonly command: string;
      readonly args: readonly string[];
      readonly cwd?: string;
    };
    readonly tool?: string;
    readonly arguments?: Readonly<Record<string, unknown>>;
    readonly timeoutSeconds?: number;
    readonly sandbox?: {
      readonly profile: string;
      readonly cwdPolicy?: string;
      readonly envAllowlist?: readonly string[];
      readonly network?: boolean;
      readonly writablePaths: readonly string[];
      readonly requireEnforcement?: boolean;
      readonly raw: Readonly<Record<string, unknown>>;
    };
    readonly raw: Readonly<Record<string, unknown>>;
  };
  readonly inputs: Readonly<Record<string, unknown>>;
  readonly resolvedInputs?: Readonly<Record<string, string>>;
}

interface OracleCase {
  readonly name: string;
  readonly request: RuntimeMcpAdapterRequest;
  readonly expectedStatus: "success" | "failure";
}

const fixtureServer = {
  command: "node",
  args: ["fixtures/runtime/adapters/mcp/stdio-server.mjs"],
  cwd: ".",
} as const;

const cases: readonly OracleCase[] = [
  {
    name: "fixture-success",
    expectedStatus: "success",
    request: {
      case: "fixture-success",
      mode: "mcp-adapter",
      skillName: "fixture-success",
      source: {
        type: "mcp",
        args: [],
        server: fixtureServer,
        tool: "echo",
        arguments: {
          message: "{{message}}",
        },
        timeoutSeconds: 5,
        raw: {
          type: "mcp",
        },
      },
      inputs: {
        message: "hi",
      },
    },
  },
  {
    name: "fixture-failure-sanitized",
    expectedStatus: "failure",
    request: {
      case: "fixture-failure-sanitized",
      mode: "mcp-adapter",
      skillName: "fixture-failure-sanitized",
      source: {
        type: "mcp",
        args: [],
        server: fixtureServer,
        tool: "fail",
        arguments: {
          message: "{{message}}",
        },
        timeoutSeconds: 5,
        raw: {
          type: "mcp",
        },
      },
      inputs: {
        message: "super-secret-value",
      },
    },
  },
  {
    name: "sandbox-env-allowed",
    expectedStatus: "success",
    request: {
      case: "sandbox-env-allowed",
      mode: "mcp-adapter",
      skillName: "sandbox-env-allowed",
      source: {
        type: "mcp",
        args: [],
        server: fixtureServer,
        tool: "env",
        arguments: {
          name: "ALLOWED_VALUE",
        },
        timeoutSeconds: 5,
        sandbox: readonlySandbox(),
        raw: {
          type: "mcp",
        },
      },
      inputs: {},
    },
  },
  {
    name: "sandbox-env-blocked",
    expectedStatus: "success",
    request: {
      case: "sandbox-env-blocked",
      mode: "mcp-adapter",
      skillName: "sandbox-env-blocked",
      source: {
        type: "mcp",
        args: [],
        server: fixtureServer,
        tool: "env",
        arguments: {
          name: "RUNX_SECRET_VALUE",
        },
        timeoutSeconds: 5,
        sandbox: readonlySandbox(),
        raw: {
          type: "mcp",
        },
      },
      inputs: {},
    },
  },
  {
    name: "missing-metadata",
    expectedStatus: "failure",
    request: {
      case: "missing-metadata",
      mode: "mcp-adapter",
      skillName: "missing-metadata",
      source: {
        type: "mcp",
        args: [],
        raw: {
          type: "mcp",
        },
      },
      inputs: {},
    },
  },
];

const tempRoot = await mkdtemp(path.join(os.tmpdir(), "runx-runtime-mcp-oracles-"));
const expectedOracleFiles = new Set<string>();

try {
  for (const oracleCase of cases) {
    await materializeCaseFixture(oracleCase);
    await runOracleCase(oracleCase);
  }

  if (check) {
    await checkNoStaleOracleFiles();
  }

  console.log(`${check ? "checked" : "generated"} ${cases.length} runtime MCP oracle cases`);
} finally {
  await rm(tempRoot, { recursive: true, force: true });
}

function readonlySandbox(): NonNullable<RuntimeMcpAdapterRequest["source"]["sandbox"]> {
  return {
    profile: "readonly",
    cwdPolicy: "workspace",
    envAllowlist: ["PATH", "ALLOWED_VALUE"],
    writablePaths: [],
    raw: {},
  };
}

async function materializeCaseFixture(oracleCase: OracleCase): Promise<void> {
  await writeOrCheck(
    path.join(casePath(oracleCase.name), "request.json"),
    `${JSON.stringify(oracleCase.request, null, 2)}\n`,
  );
}

async function runOracleCase(oracleCase: OracleCase): Promise<void> {
  const receipt = validateActReceiptEnvelope(await invokeMcp({
    source: oracleCase.request.source,
    inputs: oracleCase.request.inputs,
    resolvedInputs: oracleCase.request.resolvedInputs,
    skillDirectory: workspaceRoot,
    env: deterministicEnv(path.join(tempRoot, oracleCase.name)),
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

function deterministicEnv(caseTempRoot: string): NodeJS.ProcessEnv {
  return stripUndefined({
    CI: "1",
    FORCE_COLOR: "0",
    HOME: path.join(caseTempRoot, "home"),
    LANG: "C",
    LC_ALL: "C",
    NO_COLOR: "1",
    PATH: process.env.PATH,
    RUNX_CWD: workspaceRoot,
    RUNX_HOME: path.join(caseTempRoot, "runx-home"),
    RUNX_SECRET_VALUE: "secret",
    TEMP: path.join(caseTempRoot, "tmp"),
    TMP: path.join(caseTempRoot, "tmp"),
    TMPDIR: path.join(caseTempRoot, "tmp"),
    TZ: "UTC",
    ALLOWED_VALUE: "allowed",
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
      throw new Error(`stale runtime MCP fixture: ${path.relative(workspaceRoot, filePath)}`);
    }
    return;
  }
  await mkdir(path.dirname(filePath), { recursive: true });
  await writeFile(filePath, contents);
}

async function checkNoStaleOracleFiles(): Promise<void> {
  for (const filePath of await collectFiles(oracleRoot)) {
    if (!expectedOracleFiles.has(filePath)) {
      throw new Error(`stale runtime MCP oracle file: ${path.relative(workspaceRoot, filePath)}`);
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
    "super-secret-value",
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
