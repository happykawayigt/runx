import { mkdir, readFile, readdir, stat, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  type ActReceiptEnvelopeContract,
  validateActReceiptEnvelopeContract,
} from "../packages/contracts/src/index.js";
import { invokeA2a } from "../packages/adapters/src/a2a/index.js";
import { createA2aFixtureTransport } from "../packages/runtime-local/src/harness/a2a-fixture.js";

type ActReceiptEnvelope = ActReceiptEnvelopeContract;
const validateActReceiptEnvelope = validateActReceiptEnvelopeContract;

const workspaceRoot = path.resolve(fileURLToPath(new URL("..", import.meta.url)));
const fixtureRoot = path.join(workspaceRoot, "fixtures", "runtime", "adapters", "a2a");
const oracleRoot = path.join(fixtureRoot, "oracles");
const check = process.argv.includes("--check");

process.chdir(workspaceRoot);

type JsonValue = null | boolean | number | string | JsonValue[] | { readonly [key: string]: JsonValue };

interface A2aSource {
  readonly type: "a2a";
  readonly args: readonly string[];
  readonly agentCardUrl?: string;
  readonly agentIdentity?: string;
  readonly task?: string;
  readonly arguments?: Readonly<Record<string, JsonValue>>;
  readonly timeoutSeconds?: number;
  readonly raw: Readonly<Record<string, JsonValue>>;
}

interface A2aRequest {
  readonly case: string;
  readonly mode: "a2a-adapter";
  readonly skillName: string;
  readonly source: A2aSource;
  readonly inputs: Readonly<Record<string, JsonValue>>;
  readonly resolvedInputs?: Readonly<Record<string, string>>;
}

interface OracleCase {
  readonly name: string;
  readonly expectedStatus: "success" | "failure";
  readonly request: A2aRequest;
}

const baseSource: A2aSource = {
  type: "a2a",
  args: [],
  agentCardUrl: "fixture://echo-agent",
  agentIdentity: "echo-agent",
  task: "echo",
  arguments: { message: "{{message}}" },
  timeoutSeconds: 1,
  raw: {
    type: "a2a",
    agent_card_url: "fixture://echo-agent",
    agent_identity: "echo-agent",
    task: "echo",
    arguments: { message: "{{message}}" },
  },
};

const cases: readonly OracleCase[] = [
  {
    name: "fixture-success",
    expectedStatus: "success",
    request: {
      case: "fixture-success",
      mode: "a2a-adapter",
      skillName: "fixture-success",
      source: baseSource,
      inputs: { message: "hi" },
    },
  },
  {
    name: "fixture-failure-sanitized",
    expectedStatus: "failure",
    request: {
      case: "fixture-failure-sanitized",
      mode: "a2a-adapter",
      skillName: "fixture-failure-sanitized",
      source: {
        ...baseSource,
        task: "fail",
        raw: { ...baseSource.raw, task: "fail" },
      },
      inputs: { message: "super-secret-value" },
    },
  },
  {
    name: "missing-metadata",
    expectedStatus: "failure",
    request: {
      case: "missing-metadata",
      mode: "a2a-adapter",
      skillName: "missing-metadata",
      source: {
        type: "a2a",
        args: [],
        raw: { type: "a2a" },
      },
      inputs: {},
    },
  },
  {
    name: "embedded-template",
    expectedStatus: "success",
    request: {
      case: "embedded-template",
      mode: "a2a-adapter",
      skillName: "embedded-template",
      source: {
        ...baseSource,
        arguments: { message: "count={{count}} payload={{payload}}" },
        raw: {
          ...baseSource.raw,
          arguments: { message: "count={{count}} payload={{payload}}" },
        },
      },
      inputs: {
        count: 3,
        payload: { ok: true },
      },
    },
  },
  {
    name: "exact-template",
    expectedStatus: "success",
    request: {
      case: "exact-template",
      mode: "a2a-adapter",
      skillName: "exact-template",
      source: {
        ...baseSource,
        arguments: { message: "{{payload}}" },
        raw: {
          ...baseSource.raw,
          arguments: { message: "{{payload}}" },
        },
      },
      inputs: {
        payload: { ok: true },
      },
    },
  },
  {
    name: "resolved-inputs",
    expectedStatus: "success",
    request: {
      case: "resolved-inputs",
      mode: "a2a-adapter",
      skillName: "resolved-inputs",
      source: {
        ...baseSource,
        arguments: {
          exact: "{{payload}}",
          embedded: "message={{message}}",
        },
        raw: {
          ...baseSource.raw,
          arguments: {
            exact: "{{payload}}",
            embedded: "message={{message}}",
          },
        },
      },
      inputs: {
        payload: "raw",
        message: "raw",
      },
      resolvedInputs: {
        payload: "resolved",
        message: "resolved",
      },
    },
  },
  {
    name: "unsupported-agent-card",
    expectedStatus: "failure",
    request: {
      case: "unsupported-agent-card",
      mode: "a2a-adapter",
      skillName: "unsupported-agent-card",
      source: {
        ...baseSource,
        agentCardUrl: "https://agent.example/card.json",
        raw: {
          ...baseSource.raw,
          agent_card_url: "https://agent.example/card.json",
        },
      },
      inputs: { message: "super-secret-value" },
    },
  },
];

const expectedOracleFiles = new Set<string>();

for (const oracleCase of cases) {
  await materializeCaseFixture(oracleCase);
  await runOracleCase(oracleCase);
}

if (check) {
  await checkNoStaleOracleFiles();
}

console.log(`${check ? "checked" : "generated"} ${cases.length} A2A adapter oracle cases`);

async function materializeCaseFixture(oracleCase: OracleCase): Promise<void> {
  await writeOrCheck(
    path.join(casePath(oracleCase.name), "request.json"),
    `${JSON.stringify(oracleCase.request, null, 2)}\n`,
  );
}

async function runOracleCase(oracleCase: OracleCase): Promise<void> {
  const receipt = validateActReceiptEnvelope(
    await invokeA2a(
      {
        source: oracleCase.request.source,
        inputs: oracleCase.request.inputs,
        resolvedInputs: oracleCase.request.resolvedInputs,
        skillDirectory: casePath(oracleCase.name),
        env: deterministicEnv(casePath(oracleCase.name)),
      },
      { transport: createA2aFixtureTransport() },
    ),
    `${oracleCase.name}.receipt`,
  );

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

function deterministicEnv(cwd: string): NodeJS.ProcessEnv {
  return stripUndefined({
    CI: "1",
    FORCE_COLOR: "0",
    HOME: path.join(cwd, ".home"),
    INIT_CWD: cwd,
    LANG: "C",
    LC_ALL: "C",
    NO_COLOR: "1",
    PATH: process.env.PATH,
    RUNX_CWD: cwd,
    RUNX_HOME: path.join(cwd, ".runx"),
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

function normalizeReceipt(receipt: ActReceiptEnvelope): Record<string, JsonValue> {
  return normalizeValue({ ...receipt, durationMs: 0 }) as Record<string, JsonValue>;
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
      throw new Error(`stale A2A adapter fixture: ${path.relative(workspaceRoot, filePath)}`);
    }
    return;
  }
  await mkdir(path.dirname(filePath), { recursive: true });
  await writeFile(filePath, contents);
}

async function checkNoStaleOracleFiles(): Promise<void> {
  for (const filePath of await collectFiles(oracleRoot)) {
    if (!expectedOracleFiles.has(filePath)) {
      throw new Error(`stale A2A adapter oracle file: ${path.relative(workspaceRoot, filePath)}`);
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
    "OPENAI_API_KEY",
    "ANTHROPIC_API_KEY",
    "GITHUB_TOKEN",
    "super-secret-value",
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
