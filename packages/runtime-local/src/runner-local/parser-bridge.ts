import { spawn } from "node:child_process";
import process from "node:process";

import { errorMessage, isRecord } from "@runxhq/core/util";

export interface ParserBridgeOptions {
  readonly env?: NodeJS.ProcessEnv;
  readonly cwd?: string;
  readonly command?: string;
  readonly argsPrefix?: readonly string[];
  readonly timeoutMs?: number;
}

interface ParserSuccessEnvelope {
  readonly status: "success";
  readonly result: {
    readonly kind: "output";
    readonly value: unknown;
  };
}

export interface ValidateSkillMarkdownOptions {
  readonly mode?: "strict" | "lenient";
}

export interface ParserBridgeRawSkill {
  readonly frontmatter: Record<string, unknown>;
  readonly rawFrontmatter: string;
  readonly body: string;
}

export interface ParserBridgeRawRunnerManifest {
  readonly document: Record<string, unknown>;
  readonly raw: string;
}

export interface ParserBridgeRawToolManifest {
  readonly document: Record<string, unknown>;
  readonly raw: string;
}

export interface ParserBridgeRawGraph {
  readonly document: Record<string, unknown>;
}

export interface ParserBridgeSkillInput {
  readonly type: string;
  readonly required: boolean;
  readonly description?: string;
  readonly default?: unknown;
}

export interface ParserBridgeSkillSandbox {
  readonly profile: "readonly" | "workspace-write" | "network" | "unrestricted-local-dev";
  readonly cwdPolicy?: "skill-directory" | "workspace" | "custom";
  readonly envAllowlist?: readonly string[];
  readonly network?: boolean;
  readonly writablePaths: readonly string[];
  readonly requireEnforcement?: boolean;
  readonly approvedEscalation?: boolean;
  readonly raw: Record<string, unknown>;
}

export interface ParserBridgeSkillSource {
  readonly type: string;
  readonly command?: string;
  readonly args: readonly string[];
  readonly cwd?: string;
  readonly timeoutSeconds?: number;
  readonly inputMode?: "args" | "stdin" | "none";
  readonly sandbox?: ParserBridgeSkillSandbox;
  readonly server?: {
    readonly command: string;
    readonly args: readonly string[];
    readonly cwd?: string;
  };
  readonly catalogRef?: string;
  readonly tool?: string;
  readonly arguments?: Readonly<Record<string, unknown>>;
  readonly agentCardUrl?: string;
  readonly agentIdentity?: string;
  readonly agent?: string;
  readonly task?: string;
  readonly hook?: string;
  readonly outputs?: Readonly<Record<string, unknown>>;
  readonly graph?: ParserBridgeExecutionGraph;
  readonly raw: Record<string, unknown>;
}

export interface ParserBridgeSkillArtifactContract {
  readonly emits?: readonly string[];
  readonly namedEmits?: Readonly<Record<string, string>>;
  readonly wrapAs?: string;
}

export interface ParserBridgeSkillQualityProfile {
  readonly heading: "Quality Profile";
  readonly content: string;
}

export interface ParserBridgeValidatedSkill {
  readonly name: string;
  readonly description?: string;
  readonly body: string;
  readonly source: ParserBridgeSkillSource;
  readonly inputs: Readonly<Record<string, ParserBridgeSkillInput>>;
  readonly auth?: unknown;
  readonly risk?: unknown;
  readonly runtime?: unknown;
  readonly retry?: {
    readonly maxAttempts: number;
  };
  readonly idempotency?: {
    readonly key?: string;
  };
  readonly mutating?: boolean;
  readonly artifacts?: ParserBridgeSkillArtifactContract;
  readonly qualityProfile?: ParserBridgeSkillQualityProfile;
  readonly allowedTools?: readonly string[];
  readonly runx?: Record<string, unknown>;
  readonly raw: ParserBridgeRawSkill;
}

export interface ParserBridgeGraphStep {
  readonly id: string;
  readonly label?: string;
  readonly skill?: string;
  readonly tool?: string;
  readonly run?: Readonly<Record<string, unknown>>;
  readonly instructions?: string;
  readonly artifacts?: Readonly<Record<string, unknown>>;
  readonly runner?: string;
  readonly inputs: Readonly<Record<string, unknown>>;
  readonly context: Readonly<Record<string, string>>;
  readonly contextEdges: readonly {
    readonly input: string;
    readonly fromStep: string;
    readonly output: string;
  }[];
  readonly scopes: readonly string[];
  readonly allowedTools?: readonly string[];
  readonly retry?: {
    readonly maxAttempts: number;
    readonly backoffMs?: number;
  };
  readonly policy?: Readonly<Record<string, unknown>>;
  readonly fanoutGroup?: string;
  readonly mutating: boolean;
  readonly idempotencyKey?: string;
}

export interface ParserBridgeExecutionGraph {
  readonly name: string;
  readonly owner?: string;
  readonly steps: readonly ParserBridgeGraphStep[];
  readonly fanoutGroups: Readonly<Record<string, {
    readonly groupId: string;
    readonly strategy: "all" | "any" | "quorum";
    readonly minSuccess?: number;
    readonly onBranchFailure: "halt" | "continue";
    readonly thresholdGates: readonly {
      readonly step: string;
      readonly field: string;
      readonly above: number;
      readonly action: "pause" | "escalate";
    }[];
    readonly conflictGates: readonly {
      readonly field: string;
      readonly steps: readonly string[];
      readonly action: "pause" | "escalate";
    }[];
  }>>;
  readonly policy?: {
    readonly transitions: readonly {
      readonly to: string;
      readonly field: string;
      readonly equals?: unknown;
      readonly notEquals?: unknown;
    }[];
  };
  readonly raw: ParserBridgeRawGraph;
}

export interface ParserBridgeSkillRunnerDefinition {
  readonly name: string;
  readonly default: boolean;
  readonly source: ParserBridgeSkillSource;
  readonly inputs: Readonly<Record<string, ParserBridgeSkillInput>>;
  readonly auth?: unknown;
  readonly risk?: unknown;
  readonly runtime?: unknown;
  readonly retry?: {
    readonly maxAttempts: number;
  };
  readonly idempotency?: {
    readonly key?: string;
  };
  readonly mutating?: boolean;
  readonly artifacts?: ParserBridgeSkillArtifactContract;
  readonly allowedTools?: readonly string[];
  readonly runx?: Record<string, unknown>;
  readonly raw: Record<string, unknown>;
}

export interface ParserBridgeCatalogMetadata {
  readonly kind: "skill" | "graph";
  readonly audience: "public" | "builder" | "operator";
  readonly visibility: "public" | "private";
}

export interface ParserBridgeRunnerHarnessManifest {
  readonly cases: readonly {
    readonly name: string;
    readonly runner?: string;
    readonly inputs: Readonly<Record<string, unknown>>;
    readonly env: Readonly<Record<string, string>>;
    readonly caller: {
      readonly answers?: Readonly<Record<string, unknown>>;
      readonly approvals?: Readonly<Record<string, boolean>>;
    };
    readonly expect: {
      readonly status?: "sealed" | "failure" | "needs_agent" | "policy_denied" | "escalated";
      readonly receipt?: Readonly<Record<string, unknown>>;
      readonly steps?: readonly string[];
    };
  }[];
}

export interface ParserBridgeSkillRunnerManifest {
  readonly skill?: string;
  readonly catalog?: ParserBridgeCatalogMetadata;
  readonly runners: Readonly<Record<string, ParserBridgeSkillRunnerDefinition>>;
  readonly harness?: ParserBridgeRunnerHarnessManifest;
  readonly raw: ParserBridgeRawRunnerManifest;
}

export interface ParserBridgeValidatedTool {
  readonly name: string;
  readonly description?: string;
  readonly source: ParserBridgeSkillSource;
  readonly inputs: Readonly<Record<string, ParserBridgeSkillInput>>;
  readonly scopes: readonly string[];
  readonly risk?: unknown;
  readonly runtime?: unknown;
  readonly retry?: {
    readonly maxAttempts: number;
  };
  readonly idempotency?: {
    readonly key?: string;
  };
  readonly mutating?: boolean;
  readonly artifacts?: ParserBridgeSkillArtifactContract;
  readonly runx?: Record<string, unknown>;
  readonly raw: ParserBridgeRawToolManifest;
}

export interface ParserBridgeSkillInstallOrigin {
  readonly source: string;
  readonly source_label: string;
  readonly ref: string;
  readonly skill_id?: string;
  readonly version?: string;
  readonly digest?: string;
  readonly profile_digest?: string;
  readonly runner_names?: readonly string[];
  readonly trust_tier?: string;
}

export type ParserBridgePostRunReflectPolicy = "auto" | "always" | "never";

export interface ValidatedSkillInstall {
  readonly skill: ParserBridgeValidatedSkill;
  readonly origin: ParserBridgeSkillInstallOrigin;
  readonly markdown: string;
}

export async function validateSkillMarkdownViaParser(
  markdown: string,
  options: ValidateSkillMarkdownOptions = {},
  bridgeOptions: ParserBridgeOptions = {},
): Promise<ParserBridgeValidatedSkill> {
  const result = await evaluateParserDocument(
    {
      kind: "parser.validateSkillMarkdown",
      markdown,
      mode: options.mode ?? "strict",
    },
    bridgeOptions,
  );
  return parseValidatedSkill(result);
}

export async function validateRunnerManifestYamlViaParser(
  yaml: string,
  bridgeOptions: ParserBridgeOptions = {},
): Promise<ParserBridgeSkillRunnerManifest> {
  const result = await evaluateParserDocument(
    {
      kind: "parser.validateRunnerManifestYaml",
      yaml,
    },
    bridgeOptions,
  );
  return parseRunnerManifest(result);
}

export async function validateGraphYamlViaParser(
  yaml: string,
  bridgeOptions: ParserBridgeOptions = {},
): Promise<ParserBridgeExecutionGraph> {
  const result = await evaluateParserDocument(
    {
      kind: "parser.validateGraphYaml",
      yaml,
    },
    bridgeOptions,
  );
  return parseExecutionGraph(result);
}

export async function validateToolManifestJsonViaParser(
  json: string,
  bridgeOptions: ParserBridgeOptions = {},
): Promise<ParserBridgeValidatedTool> {
  const result = await evaluateParserDocument(
    {
      kind: "parser.validateToolManifestJson",
      json,
    },
    bridgeOptions,
  );
  return parseValidatedTool(result);
}

export async function validateSkillSourceViaParser(
  source: Readonly<Record<string, unknown>>,
  runx: Readonly<Record<string, unknown>> | undefined = undefined,
  bridgeOptions: ParserBridgeOptions = {},
): Promise<ParserBridgeSkillSource> {
  const result = await evaluateParserDocument(
    {
      kind: "parser.validateSkillSource",
      source,
      runx,
    },
    bridgeOptions,
  );
  return parseSkillSource(result);
}

export async function validateSkillArtifactContractViaParser(
  artifacts: unknown,
  field = "runx.artifacts",
  bridgeOptions: ParserBridgeOptions = {},
): Promise<ParserBridgeSkillArtifactContract | undefined> {
  const result = await evaluateParserDocument(
    {
      kind: "parser.validateSkillArtifactContract",
      artifacts,
      field,
    },
    bridgeOptions,
  );
  if (result === null || result === undefined) {
    return undefined;
  }
  return parseSkillArtifactContract(result);
}

export async function extractSkillQualityProfileViaParser(
  body: string,
  bridgeOptions: ParserBridgeOptions = {},
): Promise<ParserBridgeSkillQualityProfile | undefined> {
  const result = await evaluateParserDocument(
    {
      kind: "parser.extractSkillQualityProfile",
      body,
    },
    bridgeOptions,
  );
  if (result === null || result === undefined) {
    return undefined;
  }
  return parseSkillQualityProfile(result);
}

export async function resolvePostRunReflectPolicyViaParser(
  runx: Readonly<Record<string, unknown>> | undefined,
  bridgeOptions: ParserBridgeOptions = {},
): Promise<ParserBridgePostRunReflectPolicy> {
  const result = await evaluateParserDocument(
    {
      kind: "parser.resolvePostRunReflectPolicy",
      runx,
    },
    bridgeOptions,
  );
  if (result !== "auto" && result !== "always" && result !== "never") {
    throw new Error("Rust parser eval returned an invalid post-run reflect policy.");
  }
  return result;
}

export async function validateSkillInstallViaParser(
  markdown: string,
  origin: ParserBridgeSkillInstallOrigin,
  bridgeOptions: ParserBridgeOptions = {},
): Promise<ValidatedSkillInstall> {
  const result = await evaluateParserDocument(
    {
      kind: "parser.validateSkillInstall",
      markdown,
      origin,
    },
    bridgeOptions,
  );
  return parseValidatedSkillInstall(result);
}

export async function evaluateParserDocument(
  input: unknown,
  options: ParserBridgeOptions = {},
): Promise<unknown> {
  const envelope = await runParserEval(input, options);
  return envelope.result.value;
}

async function runParserEval(
  input: unknown,
  options: ParserBridgeOptions,
): Promise<ParserSuccessEnvelope> {
  const command = resolveParserCommand(options);
  const args = [...(options.argsPrefix ?? []), "parser", "eval", "--input", "-", "--json"];
  const env = {
    ...process.env,
    ...(options.env ?? {}),
    NO_COLOR: "1",
    RUNX_RUST_CLI: "1",
  };

  const result = await spawnParserProcess({
    command,
    args,
    cwd: options.cwd ?? process.cwd(),
    env,
    stdin: JSON.stringify(input),
    timeoutMs: options.timeoutMs ?? 10_000,
  });

  if (result.status !== 0) {
    throw new Error(
      `Rust parser eval failed with exit ${result.status}: ${firstNonEmpty(result.stderr, result.stdout, "no output")}`,
    );
  }

  let parsed: unknown;
  try {
    parsed = JSON.parse(result.stdout);
  } catch (error) {
    throw new Error(`Rust parser eval returned invalid JSON: ${errorMessage(error)}`);
  }

  if (!isParserSuccessEnvelope(parsed)) {
    throw new Error("Rust parser eval returned an invalid success envelope.");
  }
  return parsed;
}

interface SpawnParserProcessOptions {
  readonly command: string;
  readonly args: readonly string[];
  readonly cwd: string;
  readonly env: NodeJS.ProcessEnv;
  readonly stdin: string;
  readonly timeoutMs: number;
}

interface SpawnParserProcessResult {
  readonly status: number | null;
  readonly stdout: string;
  readonly stderr: string;
}

function spawnParserProcess(options: SpawnParserProcessOptions): Promise<SpawnParserProcessResult> {
  return new Promise((resolve, reject) => {
    const child = spawn(options.command, options.args, {
      cwd: options.cwd,
      env: options.env,
      stdio: ["pipe", "pipe", "pipe"],
    });
    let settled = false;
    let timedOut = false;
    let stdout = "";
    let stderr = "";
    let killTimer: NodeJS.Timeout | undefined;

    const timer = setTimeout(() => {
      if (settled) {
        return;
      }
      timedOut = true;
      child.kill("SIGTERM");
      killTimer = setTimeout(() => {
        child.kill("SIGKILL");
        if (settled) {
          return;
        }
        settled = true;
        reject(new Error(`Rust parser eval timed out after ${options.timeoutMs}ms.`));
      }, 1_000);
    }, options.timeoutMs);

    const clearTimers = () => {
      clearTimeout(timer);
      if (killTimer) {
        clearTimeout(killTimer);
      }
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
      if (settled) {
        return;
      }
      settled = true;
      clearTimers();
      reject(new Error(`Failed to spawn Rust parser eval command '${options.command}': ${error.message}`));
    });
    child.on("close", (status) => {
      if (settled) {
        return;
      }
      settled = true;
      clearTimers();
      if (timedOut) {
        reject(new Error(`Rust parser eval timed out after ${options.timeoutMs}ms.`));
        return;
      }
      resolve({ status, stdout, stderr });
    });
    child.stdin.on("error", () => {
      // The child may exit before consuming stdin. The close handler reports
      // the parser process status with captured stdout/stderr.
    });
    child.stdin.end(options.stdin);
  });
}

function resolveParserCommand(options: ParserBridgeOptions): string {
  const env = options.env ?? process.env;
  const command =
    options.command
    ?? env.RUNX_PARSER_EVAL_BIN
    ?? env.RUNX_RUST_CLI_BIN
    ?? env.RUNX_KERNEL_EVAL_BIN;
  if (!command) {
    throw new Error(
      "Rust parser eval requires RUNX_PARSER_EVAL_BIN, RUNX_RUST_CLI_BIN, RUNX_KERNEL_EVAL_BIN, or an explicit command.",
    );
  }
  return command;
}

function parseValidatedSkill(value: unknown): ParserBridgeValidatedSkill {
  const record = requireRecord(value, "validated skill");
  requireString(record.name, "validated skill name");
  requireString(record.body, "validated skill body");
  requireRecord(record.source, "validated skill source");
  requireRecord(record.inputs, "validated skill inputs");
  requireRecord(record.raw, "validated skill raw");
  return record as unknown as ParserBridgeValidatedSkill;
}

function parseRunnerManifest(value: unknown): ParserBridgeSkillRunnerManifest {
  const record = requireRecord(value, "runner manifest");
  requireRecord(record.runners, "runner manifest runners");
  requireRecord(record.raw, "runner manifest raw");
  return record as unknown as ParserBridgeSkillRunnerManifest;
}

function parseExecutionGraph(value: unknown): ParserBridgeExecutionGraph {
  const record = requireRecord(value, "execution graph");
  requireString(record.name, "execution graph name");
  requireArray(record.steps, "execution graph steps");
  requireRecord(record.fanoutGroups, "execution graph fanout groups");
  requireRecord(record.raw, "execution graph raw");
  return record as unknown as ParserBridgeExecutionGraph;
}

function parseValidatedTool(value: unknown): ParserBridgeValidatedTool {
  const record = requireRecord(value, "validated tool");
  requireString(record.name, "validated tool name");
  requireRecord(record.source, "validated tool source");
  requireRecord(record.inputs, "validated tool inputs");
  requireArray(record.scopes, "validated tool scopes");
  requireRecord(record.raw, "validated tool raw");
  return record as unknown as ParserBridgeValidatedTool;
}

function parseSkillSource(value: unknown): ParserBridgeSkillSource {
  const record = requireRecord(value, "skill source");
  requireString(record.type, "skill source type");
  requireArray(record.args, "skill source args");
  requireRecord(record.raw, "skill source raw");
  return record as unknown as ParserBridgeSkillSource;
}

function parseSkillArtifactContract(value: unknown): ParserBridgeSkillArtifactContract {
  return requireRecord(value, "skill artifact contract") as unknown as ParserBridgeSkillArtifactContract;
}

function parseSkillQualityProfile(value: unknown): ParserBridgeSkillQualityProfile {
  const record = requireRecord(value, "skill quality profile");
  if (record.heading !== "Quality Profile") {
    throw new Error("Rust parser eval returned an invalid skill quality profile heading.");
  }
  requireString(record.content, "skill quality profile content");
  return record as unknown as ParserBridgeSkillQualityProfile;
}

function parseValidatedSkillInstall(value: unknown): ValidatedSkillInstall {
  const record = requireRecord(value, "validated skill install");
  const skill = parseValidatedSkill(record.skill);
  requireRecord(record.origin, "validated skill install origin");
  const markdown = requireString(record.markdown, "validated skill install markdown");
  return {
    skill,
    origin: record.origin as unknown as ParserBridgeSkillInstallOrigin,
    markdown,
  };
}

function isParserSuccessEnvelope(value: unknown): value is ParserSuccessEnvelope {
  return isRecord(value)
    && value.status === "success"
    && isRecord(value.result)
    && value.result.kind === "output"
    && "value" in value.result;
}

function requireRecord(value: unknown, label: string): Record<string, unknown> {
  if (!isRecord(value)) {
    throw new Error(`Rust parser eval returned a non-object ${label}.`);
  }
  return value;
}

function requireString(value: unknown, label: string): string {
  if (typeof value !== "string") {
    throw new Error(`Rust parser eval returned invalid ${label}.`);
  }
  return value;
}

function requireArray(value: unknown, label: string): readonly unknown[] {
  if (!Array.isArray(value)) {
    throw new Error(`Rust parser eval returned invalid ${label}.`);
  }
  return value;
}

function firstNonEmpty(...values: readonly (string | undefined)[]): string {
  return values.find((value) => value !== undefined && value.trim().length > 0) ?? "";
}
