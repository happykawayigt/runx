import { mkdir, readFile, readdir, stat, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  type ActReceiptEnvelopeContract,
  type AgentActResolutionRequestContract,
  type ResolutionResponseContract,
  validateActReceiptEnvelopeContract,
} from "../packages/contracts/src/index.js";
import type {
  AdapterActInvocation,
  SkillAdapter,
} from "../packages/runtime-local/src/runner-local/adapter-types.js";
import { errorMessage } from "../packages/core/src/util/index.js";
import {
  buildManagedAgentActInvocation,
  buildManagedRuntimeInstructions,
  nativeAgentMetadata,
} from "../packages/adapters/src/agent/agent-act-invocation.js";
import {
  extractApiErrorMessage,
  isRecord,
  parseJsonObject,
} from "../packages/adapters/src/agent/helpers.js";
import { validateFinalPayload } from "../packages/adapters/src/agent/json-schema.js";
import {
  FINAL_RESULT_TOOL_NAME,
  type ManagedAgentExecutionTelemetry,
  type OpenAiResponseBody,
  type OpenAiToolCall,
} from "../packages/adapters/src/agent/types.js";

const workspaceRoot = path.resolve(fileURLToPath(new URL("..", import.meta.url)));
const fixtureRoot = path.join(workspaceRoot, "fixtures", "runtime", "adapters", "agent");
const oracleRoot = path.join(fixtureRoot, "oracles");
const check = process.argv.includes("--check");

process.chdir(workspaceRoot);

type ActReceiptEnvelope = ActReceiptEnvelopeContract;
type AgentActResolutionRequest = AgentActResolutionRequestContract;
type ResolutionResponse = ResolutionResponseContract;
const validateActReceiptEnvelope = validateActReceiptEnvelopeContract;

type JsonValue = null | boolean | number | string | JsonValue[] | { readonly [key: string]: JsonValue };

interface ManagedAgentConfig {
  readonly provider: "openai";
  readonly model: string;
  readonly apiKey: string;
}

interface AgentSource {
  readonly type: "agent" | "agent-step";
  readonly args: readonly string[];
  readonly agent?: string;
  readonly task?: string;
  readonly outputs?: Readonly<Record<string, JsonValue>>;
  readonly raw: Readonly<Record<string, JsonValue>>;
}

interface AgentRequest {
  readonly case: string;
  readonly mode: "agent-adapter";
  readonly skillName: string;
  readonly skillBody: string;
  readonly source: AgentSource;
  readonly inputs: Readonly<Record<string, JsonValue>>;
}

interface OracleCase {
  readonly name: string;
  readonly expectedStatus: "success" | "failure";
  readonly request: AgentRequest;
  readonly providerResponses: readonly ProviderResponse[];
}

interface ProviderResponse {
  readonly status: number;
  readonly body: JsonValue;
}

const config: ManagedAgentConfig = {
  provider: "openai",
  model: "gpt-fixture",
  apiKey: "sk-fixture-redacted",
};

const cases: readonly OracleCase[] = [
  {
    name: "agent-plain-success",
    expectedStatus: "success",
    request: {
      case: "agent-plain-success",
      mode: "agent-adapter",
      skillName: "fixture.agent",
      skillBody: "Summarize the input.",
      source: {
        type: "agent",
        args: [],
        agent: "assistant",
        task: "summarize",
        raw: { type: "agent", agent: "assistant", task: "summarize" },
      },
      inputs: { topic: "release notes" },
    },
    providerResponses: [
      {
        status: 200,
        body: {
          output: [
            {
              type: "message",
              role: "assistant",
              content: [{ type: "output_text", text: "plain final answer" }],
            },
          ],
        },
      },
    ],
  },
  {
    name: "agent-step-structured-success",
    expectedStatus: "success",
    request: {
      case: "agent-step-structured-success",
      mode: "agent-adapter",
      skillName: "fixture.structured",
      skillBody: "Return a structured release summary.",
      source: {
        type: "agent-step",
        args: [],
        agent: "assistant",
        task: "structured release",
        outputs: {
          title: "string",
          ready: "boolean",
        },
        raw: {
          type: "agent-step",
          agent: "assistant",
          task: "structured release",
          outputs: {
            title: "string",
            ready: "boolean",
          },
        },
      },
      inputs: { release: "2026.05" },
    },
    providerResponses: [
      {
        status: 200,
        body: {
          output: [
            {
              type: "function_call",
              call_id: "call_1",
              name: "submit_result",
              arguments: "{\"title\":\"Release\",\"ready\":true}",
            },
          ],
        },
      },
    ],
  },
  {
    name: "provider-error-sanitized",
    expectedStatus: "failure",
    request: {
      case: "provider-error-sanitized",
      mode: "agent-adapter",
      skillName: "fixture.fail",
      skillBody: "Fail without leaking credentials.",
      source: {
        type: "agent-step",
        args: [],
        agent: "assistant",
        task: "fail",
        raw: { type: "agent-step", agent: "assistant", task: "fail" },
      },
      inputs: { secret: "super-secret-value" },
    },
    providerResponses: [
      {
        status: 500,
        body: {
          error: {
            message: "managed provider failure",
          },
        },
      },
    ],
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

console.log(`${check ? "checked" : "generated"} ${cases.length} agent adapter oracle cases`);

async function materializeCaseFixture(oracleCase: OracleCase): Promise<void> {
  await writeOrCheck(
    path.join(casePath(oracleCase.name), "request.json"),
    `${JSON.stringify(oracleCase.request, null, 2)}\n`,
  );
}

async function runOracleCase(oracleCase: OracleCase): Promise<void> {
  const restoreFetch = installFetchFixture(oracleCase.providerResponses);
  try {
    const adapter = createFixtureManagedAgentAdapter(config, oracleCase.request.source.type);
    const receipt = validateActReceiptEnvelope(
      await adapter.invoke({
        skillName: oracleCase.request.skillName,
        skillBody: oracleCase.request.skillBody,
        source: oracleCase.request.source,
        inputs: oracleCase.request.inputs,
        skillDirectory: casePath(oracleCase.name),
        env: deterministicEnv(casePath(oracleCase.name)),
      }),
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
  } finally {
    restoreFetch();
  }
}

function createFixtureManagedAgentAdapter(
  config: ManagedAgentConfig,
  sourceType: "agent" | "agent-step",
): SkillAdapter {
  return {
    type: sourceType,
    invoke: async (request) => await invokeFixtureManagedAgentAdapter(config, request, sourceType),
  };
}

async function invokeFixtureManagedAgentAdapter(
  config: ManagedAgentConfig,
  request: AdapterActInvocation,
  sourceType: "agent" | "agent-step",
): Promise<ActReceiptEnvelope> {
  const started = performance.now();
  const invocation = buildManagedAgentActInvocation(request, sourceType);

  try {
    const execution = await resolveFixtureOpenAi(config, {
      id: invocation.id,
      kind: "agent_act",
      invocation,
    });
    return {
      status: "success",
      stdout: typeof execution.response.payload === "string"
        ? execution.response.payload
        : JSON.stringify(execution.response.payload),
      stderr: "",
      exitCode: 0,
      signal: null,
      durationMs: Math.round(performance.now() - started),
      metadata: nativeAgentMetadata(sourceType, request, config, execution, "success"),
    };
  } catch (error) {
    return {
      status: "failure",
      stdout: "",
      stderr: "",
      exitCode: null,
      signal: null,
      durationMs: Math.round(performance.now() - started),
      errorMessage: errorMessage(error),
      metadata: nativeAgentMetadata(sourceType, request, config, undefined, "failure"),
    };
  }
}

async function resolveFixtureOpenAi(
  config: ManagedAgentConfig,
  request: AgentActResolutionRequest,
): Promise<ManagedAgentExecutionTelemetry & { readonly response: ResolutionResponse }> {
  const response = await createFixtureOpenAiResponse(config, request);
  const functionCalls = collectOpenAiFunctionCalls(response);
  const toolCalls = functionCalls.length;

  if (functionCalls.length === 0) {
    const assistantText = extractOpenAiAssistantText(response);
    if (!assistantText.trim()) {
      throw new Error(`Managed agent resolution for ${request.id} returned no assistant text.`);
    }
    return {
      response: { actor: "agent", payload: assistantText },
      rounds: 1,
      toolCalls,
      tools: [],
      toolExecutions: [],
    };
  }

  for (const call of functionCalls) {
    if (call.name !== FINAL_RESULT_TOOL_NAME) {
      continue;
    }
    const submittedPayload = parseJsonObject(call.arguments, `${call.name}.arguments`);
    const validationError = validateFinalPayload(submittedPayload, request.invocation.envelope.output);
    if (validationError) {
      throw new Error(validationError);
    }
    return {
      response: { actor: "agent", payload: submittedPayload },
      rounds: 1,
      toolCalls,
      tools: [],
      toolExecutions: [],
    };
  }

  throw new Error(`Managed agent resolution for ${request.id} returned no final result.`);
}

async function createFixtureOpenAiResponse(
  config: ManagedAgentConfig,
  request: AgentActResolutionRequest,
): Promise<OpenAiResponseBody> {
  const response = await fetch("https://api.openai.com/v1/responses", {
    method: "POST",
    headers: {
      "Authorization": `Bearer ${config.apiKey}`,
      "Content-Type": "application/json",
    },
    body: JSON.stringify({
      model: config.model,
      store: false,
      parallel_tool_calls: false,
      instructions: buildManagedRuntimeInstructions(request),
      input: [
        {
          role: "user",
          content: [
            {
              type: "input_text",
              text: JSON.stringify({
                request_id: request.id,
                source_type: request.invocation.source_type,
                agent: request.invocation.agent,
                task: request.invocation.task,
                envelope: request.invocation.envelope,
              }, null, 2),
            },
          ],
        },
      ],
      tools: [],
    }),
  });

  if (!response.ok) {
    const bodyText = await response.text();
    throw new Error(`OpenAI Responses API ${response.status}: ${extractApiErrorMessage(bodyText)}`);
  }

  return await response.json() as OpenAiResponseBody;
}

function collectOpenAiFunctionCalls(response: OpenAiResponseBody): readonly OpenAiToolCall[] {
  return Array.isArray(response.output)
    ? response.output
      .filter((item): item is OpenAiToolCall =>
        isRecord(item)
        && item.type === "function_call"
        && typeof item.call_id === "string"
        && typeof item.name === "string"
        && typeof item.arguments === "string")
    : [];
}

function extractOpenAiAssistantText(response: OpenAiResponseBody): string {
  if (!Array.isArray(response.output)) {
    return "";
  }
  const parts: string[] = [];
  for (const item of response.output) {
    if (!isRecord(item) || item.type !== "message" || item.role !== "assistant" || !Array.isArray(item.content)) {
      continue;
    }
    for (const content of item.content) {
      if (isRecord(content) && content.type === "output_text" && typeof content.text === "string") {
        parts.push(content.text);
      }
    }
  }
  return parts.join("\n");
}

function installFetchFixture(responses: readonly ProviderResponse[]): () => void {
  const originalFetch = globalThis.fetch;
  const queue = [...responses];
  globalThis.fetch = async () => {
    const next = queue.shift();
    if (!next) {
      return new Response(JSON.stringify({ error: { message: "unexpected provider request" } }), {
        status: 500,
        headers: { "content-type": "application/json" },
      });
    }
    return new Response(JSON.stringify(next.body), {
      status: next.status,
      headers: { "content-type": "application/json" },
    });
  };
  return () => {
    globalThis.fetch = originalFetch;
  };
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
    RUNX_TOOL_ROOTS: `${path.join(cwd, "tools")}${path.delimiter}${path.join(cwd, "more-tools")}`,
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
      throw new Error(`stale agent adapter fixture: ${path.relative(workspaceRoot, filePath)}`);
    }
    return;
  }
  await mkdir(path.dirname(filePath), { recursive: true });
  await writeFile(filePath, contents);
}

async function checkNoStaleOracleFiles(): Promise<void> {
  for (const filePath of await collectFiles(oracleRoot)) {
    if (!expectedOracleFiles.has(filePath)) {
      throw new Error(`stale agent adapter oracle file: ${path.relative(workspaceRoot, filePath)}`);
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
    "RUNX_AGENT_API_KEY",
    "sk-fixture-redacted",
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
