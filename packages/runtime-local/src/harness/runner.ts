import { mkdtemp, readFile, rm, stat } from "node:fs/promises";
import os from "node:os";
import path from "node:path";

import { parseDocument } from "yaml";

import { resolveLocalSkillProfile } from "@runxhq/core/config";
import { isRecord } from "@runxhq/core/util";
import {
  type ResolutionRequestContract as ResolutionRequest,
  type ResolutionResponseContract as ResolutionResponse,
} from "@runxhq/contracts";
import {
  runLocalGraph,
  runLocalSkill,
  runnerReceiptStatus,
  type Caller,
  type ExecutionEvent,
  type RunLocalGraphResult,
  type RunLocalSkillResult,
} from "../runner-local/index.js";
import type { SkillAdapter } from "../runner-local/adapter-types.js";
import type { RegistryStore } from "../runner-local/registry-resolver.js";
import type { ToolCatalogAdapter } from "@runxhq/runtime-local/tool-catalogs";
import type {
  HarnessCallerFixture,
  HarnessExpectation,
  HarnessReceiptExpectation,
  RunnerHarnessCase,
} from "../parser-types.js";
import { parseSkillFrontmatter } from "./skill-frontmatter.js";
import {
  validateGraphYamlViaParser,
  validateRunnerManifestYamlViaParser,
} from "../runner-local/parser-bridge.js";

const harnessReceiptSchema = "runx.harness_receipt.v1";

type HarnessKind = "skill" | "graph";

interface HarnessReceiptShapeExpectation extends HarnessReceiptExpectation {
  readonly schema?: typeof harnessReceiptSchema;
  readonly body_digest?: string;
  readonly receipt_digest?: string;
  readonly harness_id?: string;
  readonly state?: string;
  readonly disposition?: string;
  readonly reason_code?: string;
  readonly act_ids?: readonly string[];
  readonly child_receipt_refs?: readonly string[];
}

interface HarnessResultExpectation extends Omit<HarnessExpectation, "receipt"> {
  readonly receipt?: HarnessReceiptShapeExpectation;
}

interface HarnessReceiptShape {
  readonly schema: typeof harnessReceiptSchema;
  readonly id: string;
  readonly signature?: {
    readonly value?: string;
  };
  readonly harness: {
    readonly harness_id: string;
    readonly state: string;
    readonly acts?: readonly {
      readonly act_id?: string;
    }[];
    readonly child_harness_receipt_refs?: readonly {
      readonly uri?: string;
    }[];
  };
  readonly seal: {
    readonly digest?: string;
    readonly disposition: string;
    readonly reason_code: string;
  };
}

interface HarnessReceiptIds {
  readonly runId: string;
  readonly stepRunIds?: Readonly<Record<string, string>>;
}

export interface HarnessFixture {
  readonly name: string;
  readonly kind: HarnessKind;
  readonly target: string;
  readonly runner?: string;
  readonly inputs: Readonly<Record<string, unknown>>;
  readonly env: Readonly<Record<string, string>>;
  readonly caller: HarnessCallerFixture;
  readonly expect: HarnessResultExpectation;
}

export interface HarnessRunOptions {
  readonly env?: NodeJS.ProcessEnv;
  readonly keepFiles?: boolean;
  readonly registryStore?: RegistryStore;
  readonly skillCacheDir?: string;
  readonly toolCatalogAdapters?: readonly ToolCatalogAdapter[];
  readonly adapters?: readonly SkillAdapter[];
  readonly voiceProfilePath?: string;
}

export interface CallerTrace {
  readonly resolutions: readonly {
    readonly request: ResolutionRequest;
    readonly response?: ResolutionResponse;
  }[];
  readonly events: readonly ExecutionEvent[];
}

export interface HarnessRunResult {
  readonly source: "fixture" | "inline";
  readonly fixture: HarnessFixture;
  readonly fixturePath: string;
  readonly targetPath: string;
  readonly receiptDir: string;
  readonly runxHome: string;
  readonly status: RunLocalSkillResult["status"] | RunLocalGraphResult["status"];
  readonly receipt?: RunLocalSkillResult extends infer SkillResult
    ? SkillResult extends { readonly receipt: infer Receipt }
      ? Receipt
      : never
    : never;
  readonly graphReceipt?: RunLocalGraphResult extends infer GraphResult
    ? GraphResult extends { readonly receipt: infer Receipt }
      ? Receipt
      : never
    : never;
  readonly trace: CallerTrace;
  readonly assertionErrors: readonly string[];
}

export interface HarnessSuiteResult {
  readonly source: "inline";
  readonly targetPath: string;
  readonly skillPath: string;
  readonly profileSourcePath: string;
  readonly status: "success" | "failure";
  readonly cases: readonly HarnessRunResult[];
  readonly assertionErrors: readonly string[];
}

export type HarnessTargetResult = HarnessRunResult | HarnessSuiteResult;

interface ResolvedInlineHarnessTarget {
  readonly skillPath: string;
  readonly profileDocument: string;
  readonly profileSourcePath: string;
}

export async function parseHarnessFixtureFile(fixturePath: string): Promise<HarnessFixture> {
  return parseHarnessFixture(await readFile(fixturePath, "utf8"));
}

export function parseHarnessFixture(contents: string): HarnessFixture {
  const document = parseDocument(contents, { prettyErrors: false });
  if (document.errors.length > 0) {
    throw new Error(document.errors.map((error: { readonly message: string }) => error.message).join("; "));
  }

  const parsed = document.toJS() as unknown;
  if (!isRecord(parsed)) {
    throw new Error("Harness fixture must be a YAML object.");
  }

  const kind = requiredString(parsed.kind, "kind");
  if (kind !== "skill" && kind !== "graph") {
    throw new Error("Harness fixture kind must be skill or graph.");
  }

  return {
    name: requiredString(parsed.name, "name"),
    kind,
    target: requiredString(parsed.target, "target"),
    runner: optionalString(parsed.runner, "runner"),
    inputs: optionalRecord(parsed.inputs, "inputs") ?? {},
    env: validateEnv(optionalRecord(parsed.env, "env") ?? {}),
    caller: validateCaller(optionalRecord(parsed.caller, "caller") ?? {}),
    expect: validateExpectation(optionalRecord(parsed.expect, "expect") ?? {}),
  };
}

export async function runHarnessTarget(targetPath: string, options: HarnessRunOptions = {}): Promise<HarnessTargetResult> {
  const resolvedTargetPath = path.resolve(targetPath);
  const targetStat = await stat(resolvedTargetPath);

  if (isInlineHarnessTarget(resolvedTargetPath, targetStat)) {
    return await runInlineHarnessSuite(resolvedTargetPath, options);
  }

  return await runHarness(resolvedTargetPath, options);
}

export async function runHarness(fixturePath: string, options: HarnessRunOptions = {}): Promise<HarnessRunResult> {
  const resolvedFixturePath = path.resolve(fixturePath);
  const fixture = await parseHarnessFixtureFile(resolvedFixturePath);
  const fixtureDir = path.dirname(resolvedFixturePath);
  const targetPath = path.resolve(fixtureDir, fixture.target);
  return await executeHarnessFixture({
    fixture,
    fixturePath: resolvedFixturePath,
    targetPath,
    source: "fixture",
    options,
  });
}

async function runInlineHarnessSuite(targetPath: string, options: HarnessRunOptions): Promise<HarnessSuiteResult> {
  const resolved = await resolveInlineHarnessTarget(targetPath);
  const manifest = await validateRunnerManifestYamlViaParser(resolved.profileDocument, { env: options.env });
  if (!manifest.harness || manifest.harness.cases.length === 0) {
    throw new Error(`Inline harness target does not declare harness.cases: ${resolved.profileSourcePath}`);
  }

  const cases: HarnessRunResult[] = [];
  for (const entry of manifest.harness.cases) {
    const fixture = createInlineHarnessFixture(entry, resolved.skillPath);
    cases.push(
      await executeHarnessFixture({
        fixture,
        fixturePath: resolved.profileSourcePath,
        targetPath: resolved.skillPath,
        source: "inline",
        options,
      }),
    );
  }

  const assertionErrors = cases.flatMap((result) => result.assertionErrors.map((error) => `${result.fixture.name}: ${error}`));
  return {
    source: "inline",
    targetPath: resolved.skillPath,
    skillPath: resolved.skillPath,
    profileSourcePath: resolved.profileSourcePath,
    status: assertionErrors.length === 0 ? "success" : "failure",
    cases,
    assertionErrors,
  };
}

async function executeHarnessFixture(args: {
  readonly fixture: HarnessFixture;
  readonly fixturePath: string;
  readonly targetPath: string;
  readonly source: "fixture" | "inline";
  readonly options: HarnessRunOptions;
}): Promise<HarnessRunResult> {
  const tempDir = await mkdtemp(path.join(os.tmpdir(), "runx-harness-"));
  const receiptDir = path.join(tempDir, "receipts");
  const runxHome = path.join(tempDir, "home");
  const trace = createTrace();
  const caller = createReplayCaller(args.fixture.caller, trace);
  const receiptIds = await deterministicHarnessReceiptIds(args.fixture, args.targetPath, args.options.env);
  const env = {
    ...(args.options.env ?? process.env),
    ...args.fixture.env,
    RUNX_RECEIPT_DIR: receiptDir,
    RUNX_HOME: runxHome,
    // Sandbox cli-tool skills to the harness tempdir so tools like
    // scafld that persist state to cwd do not leak files into the
    // runx repo when harness cases run against cli-tool skills.
    RUNX_CWD: tempDir,
    INIT_CWD: tempDir,
  };

  try {
    const result =
      args.fixture.kind === "skill"
        ? await runLocalSkill({
            skillPath: args.targetPath,
            runId: receiptIds.runId,
            runner: args.fixture.runner,
            inputs: args.fixture.inputs,
            caller,
            env,
            receiptDir,
            runxHome,
            registryStore: args.options.registryStore,
            skillCacheDir: args.options.skillCacheDir,
            toolCatalogAdapters: args.options.toolCatalogAdapters,
            adapters: args.options.adapters,
            voiceProfilePath: args.options.voiceProfilePath,
          })
        : await runLocalGraph({
            graphPath: args.targetPath,
            runId: receiptIds.runId,
            stepRunIds: receiptIds.stepRunIds,
            inputs: args.fixture.inputs,
            caller,
            env,
            receiptDir,
            runxHome,
            registryStore: args.options.registryStore,
            skillCacheDir: args.options.skillCacheDir,
            toolCatalogAdapters: args.options.toolCatalogAdapters,
            adapters: args.options.adapters,
            voiceProfilePath: args.options.voiceProfilePath,
          });

    const assertionErrors = assertHarnessResult(args.fixture, result);
    return {
      source: args.source,
      fixture: args.fixture,
      fixturePath: args.fixturePath,
      targetPath: args.targetPath,
      receiptDir,
      runxHome,
      status: result.status,
      receipt: skillReceipt(result),
      graphReceipt: graphReceipt(result),
      trace,
      assertionErrors,
    };
  } finally {
    if (!args.options.keepFiles) {
      await rm(tempDir, { recursive: true, force: true });
    }
  }
}

async function deterministicHarnessReceiptIds(
  fixture: HarnessFixture,
  targetPath: string,
  env?: NodeJS.ProcessEnv,
): Promise<HarnessReceiptIds> {
  if (fixture.kind === "skill") {
    const skillName = await targetSkillName(targetPath);
    return { runId: harnessReceiptId(fixture.name, skillName) };
  }

  const graph = await validateGraphYamlViaParser(await readFile(targetPath, "utf8"), { env });
  return {
    runId: harnessReceiptId(graph.name, "graph"),
    stepRunIds: Object.fromEntries(
      graph.steps.map((step) => [step.id, harnessReceiptId(graph.name, step.id)]),
    ),
  };
}

async function targetSkillName(skillPath: string): Promise<string> {
  const resolved = await resolveSkillFilePath(skillPath);
  const raw = parseSkillFrontmatter(await readFile(resolved, "utf8"));
  const name = raw.frontmatter.name;
  return typeof name === "string" && name.trim().length > 0
    ? name.trim()
    : path.basename(path.dirname(resolved));
}

async function resolveSkillFilePath(skillPath: string): Promise<string> {
  return (await stat(skillPath)).isDirectory() ? path.join(skillPath, "SKILL.md") : skillPath;
}

function harnessReceiptId(...parts: readonly string[]): string {
  return `hrn_rcpt_${parts.map(safeHarnessSegment).join("_")}`;
}

function safeHarnessSegment(value: string): string {
  const segment = value
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9._-]+/g, "-")
    .replace(/^-+|-+$/g, "");
  return segment.length > 0 ? segment : "unnamed";
}

function createInlineHarnessFixture(entry: RunnerHarnessCase, skillPath: string): HarnessFixture {
  return {
    name: entry.name,
    kind: "skill",
    target: skillPath,
    runner: entry.runner,
    inputs: entry.inputs,
    env: entry.env,
    caller: entry.caller,
    expect: entry.expect as HarnessResultExpectation,
  };
}

async function resolveInlineHarnessTarget(targetPath: string): Promise<ResolvedInlineHarnessTarget> {
  const resolvedTargetPath = path.resolve(targetPath);
  const targetStat = await stat(resolvedTargetPath);
  const skillPath = targetStat.isDirectory() ? path.join(resolvedTargetPath, "SKILL.md") : resolvedTargetPath;
  const basename = path.basename(skillPath).toLowerCase();
  if (basename !== "skill.md") {
    throw new Error(`Inline harness target must be a skill directory or SKILL.md: ${resolvedTargetPath}`);
  }

  const markdown = await readFile(skillPath, "utf8");
  const raw = parseSkillFrontmatter(markdown);
  const skillName = requiredString(raw.frontmatter.name, "frontmatter.name");
  const profile = await resolveLocalSkillProfile(skillPath, skillName);
  if (!profile.profileDocument || !profile.profileSourcePath) {
    throw new Error(`Inline harness target does not have a execution profile: ${resolvedTargetPath}`);
  }

  return {
    skillPath: path.dirname(skillPath),
    profileDocument: profile.profileDocument,
    profileSourcePath: profile.profileSourcePath,
  };
}

function isInlineHarnessTarget(targetPath: string, targetStat: Awaited<ReturnType<typeof stat>>): boolean {
  if (targetStat.isDirectory()) {
    return true;
  }
  const basename = path.basename(targetPath).toLowerCase();
  return basename === "skill.md";
}

function assertHarnessResult(
  fixture: HarnessFixture,
  result: RunLocalSkillResult | RunLocalGraphResult,
): readonly string[] {
  const errors: string[] = [];

  if (fixture.expect.status && result.status !== fixture.expect.status) {
    errors.push(`Expected status ${fixture.expect.status}, got ${result.status}.`);
  }

  const receipt = skillReceipt(result) ?? graphReceipt(result);
  if (fixture.expect.receipt) {
    if (!receipt) {
      errors.push("Expected a receipt, but run did not produce one.");
    } else if (fixture.expect.receipt.schema === harnessReceiptSchema) {
      errors.push(...assertHarnessReceiptShape(fixture.expect.receipt, receipt));
    } else {
      errors.push(`Expected receipt schema ${harnessReceiptSchema}.`);
    }
  }

  if (fixture.expect.steps) {
    const actualSteps =
      hasHistoricalReceiptSteps(receipt)
        ? receipt.steps.map((step) => step.step_id)
        : "steps" in result
          ? result.steps.map((step) => step.stepId)
          : [];
    if (JSON.stringify(actualSteps) !== JSON.stringify(fixture.expect.steps)) {
      errors.push(`Expected steps ${fixture.expect.steps.join(", ")}, got ${actualSteps.join(", ")}.`);
    }
  }

  return errors;
}

function hasHistoricalReceiptSteps(receipt: unknown): receipt is { readonly steps: readonly { readonly step_id: string }[] } {
  return isRecord(receipt)
    && Array.isArray(receipt.steps)
    && receipt.steps.every((step) => isRecord(step) && typeof step.step_id === "string");
}

function assertHarnessReceiptShape(
  expected: HarnessReceiptShapeExpectation,
  receipt: SkillReceipt | Extract<RunLocalGraphResult, { readonly receipt: unknown }>["receipt"],
): readonly string[] {
  if (!isHarnessReceiptShape(receipt)) {
    return [];
  }

  const errors: string[] = [];
  if (receipt.schema !== expected.schema) {
    errors.push(`Expected receipt schema ${expected.schema}, got ${receipt.schema}.`);
  }
  if (expected.body_digest && hasPseudoLocalSignature(receipt) && receipt.seal.digest !== expected.body_digest) {
    errors.push(`Expected receipt body_digest to equal ${expected.body_digest}, got ${receipt.seal.digest}.`);
  }
  if (expected.receipt_digest && hasPseudoLocalSignature(receipt) && receipt.signature?.value === expected.receipt_digest) {
    errors.push("Expected receipt_digest must be a canonical receipt digest, not the signature value.");
  }
  if (expected.harness_id && receipt.harness.harness_id !== expected.harness_id) {
    errors.push(`Expected receipt harness_id to equal ${expected.harness_id}.`);
  }
  if (expected.state && receipt.harness.state !== expected.state) {
    errors.push(`Expected receipt state to equal ${expected.state}.`);
  }
  if (expected.disposition && receipt.seal.disposition !== expected.disposition) {
    errors.push(`Expected receipt disposition to equal ${expected.disposition}.`);
  }
  if (expected.reason_code && receipt.seal.reason_code !== expected.reason_code) {
    errors.push(`Expected receipt reason_code to equal ${expected.reason_code}.`);
  }
  if (expected.act_ids) {
    const actualActIds = (receipt.harness.acts ?? []).map((act) => act.act_id).filter((actId): actId is string => typeof actId === "string");
    if (JSON.stringify(actualActIds) !== JSON.stringify(expected.act_ids)) {
      errors.push(`Expected receipt act_ids ${expected.act_ids.join(", ")}, got ${actualActIds.join(", ")}.`);
    }
  }
  if (expected.child_receipt_refs) {
    const actualChildRefs = (receipt.harness.child_harness_receipt_refs ?? [])
      .map((ref) => ref.uri)
      .filter((uri): uri is string => typeof uri === "string");
    if (JSON.stringify(actualChildRefs) !== JSON.stringify(expected.child_receipt_refs)) {
      errors.push(`Expected receipt child_receipt_refs ${expected.child_receipt_refs.join(", ")}, got ${actualChildRefs.join(", ")}.`);
    }
  }
  return errors;
}

function hasPseudoLocalSignature(receipt: HarnessReceiptShape): boolean {
  return typeof receipt.signature?.value === "string" && receipt.signature.value.startsWith("sig:");
}

function isHarnessReceiptShape(value: unknown): value is HarnessReceiptShape {
  if (!isRecord(value) || value.schema !== harnessReceiptSchema || !isRecord(value.harness) || !isRecord(value.seal)) {
    return false;
  }
  return (
    typeof value.id === "string"
    && typeof value.harness.harness_id === "string"
    && typeof value.harness.state === "string"
    && typeof value.seal.disposition === "string"
    && typeof value.seal.reason_code === "string"
  );
}

function createTrace(): CallerTrace {
  return {
    resolutions: [],
    events: [],
  };
}

function createReplayCaller(fixture: HarnessCallerFixture, trace: CallerTrace): Caller {
  return {
    resolve: async (request) => {
      const response = resolveHarnessRequest(request, fixture);
      (trace.resolutions as { request: ResolutionRequest; response?: ResolutionResponse }[]).push({
        request,
        response,
      });
      return response;
    },
    report: (event) => {
      (trace.events as ExecutionEvent[]).push(event);
    },
  };
}

function resolveHarnessRequest(
  request: ResolutionRequest,
  fixture: HarnessCallerFixture,
): ResolutionResponse | undefined {
  if (request.kind === "input") {
    const payload = Object.fromEntries(
      request.questions
        .filter((question) => fixture.answers?.[question.id] !== undefined)
        .map((question) => [question.id, fixture.answers?.[question.id]]),
    );
    return Object.keys(payload).length === 0 ? undefined : { actor: "human", payload };
  }
  if (request.kind === "approval") {
    const approved = fixture.approvals?.[request.gate.id];
    return approved === undefined ? undefined : { actor: "human", payload: approved };
  }
  const payload = fixture.answers?.[request.id];
  return payload === undefined ? undefined : { actor: "agent", payload };
}

type SkillReceipt = Extract<RunLocalSkillResult, { readonly status: "sealed" | "failure" }>["receipt"];

function skillReceipt(result: RunLocalSkillResult | RunLocalGraphResult): SkillReceipt | undefined {
  if ("receipt" in result && "skill" in result && !("graph" in result)) {
    return result.receipt as SkillReceipt | undefined;
  }
  return undefined;
}

function graphReceipt(result: RunLocalSkillResult | RunLocalGraphResult): Extract<RunLocalGraphResult, { readonly receipt: unknown }>["receipt"] | undefined {
  if ("receipt" in result && "graph" in result) {
    return result.receipt;
  }
  return undefined;
}

function validateCaller(value: Record<string, unknown>): HarnessCallerFixture {
  return {
    answers: optionalRecord(value.answers, "caller.answers"),
    approvals: validateApprovals(optionalRecord(value.approvals, "caller.approvals") ?? {}),
  };
}

function validateApprovals(value: Record<string, unknown>): Readonly<Record<string, boolean>> {
  return Object.fromEntries(
    Object.entries(value).map(([key, entry]) => {
      if (typeof entry !== "boolean") {
        throw new Error(`caller.approvals.${key} must be a boolean.`);
      }
      return [key, entry];
    }),
  );
}

function validateExpectation(value: Record<string, unknown>): HarnessResultExpectation {
  return {
    status: optionalStatus(value.status, "expect.status"),
    receipt: validateReceiptExpectation(optionalRecord(value.receipt, "expect.receipt")),
    steps: optionalStringArray(value.steps, "expect.steps"),
  };
}

function validateReceiptExpectation(value: Record<string, unknown> | undefined): HarnessReceiptShapeExpectation | undefined {
  if (!value) {
    return undefined;
  }
  const expectation: Record<string, unknown> = {
    kind: optionalReceiptKind(value.kind, "expect.receipt.kind"),
    status: optionalSuccessFailure(value.status, "expect.receipt.status"),
    source_type: optionalString(value.source_type, "expect.receipt.source_type"),
    owner: optionalString(value.owner, "expect.receipt.owner"),
    schema: optionalHarnessReceiptSchema(value.schema, "expect.receipt.schema"),
    body_digest: optionalString(value.body_digest, "expect.receipt.body_digest"),
    receipt_digest: optionalString(value.receipt_digest, "expect.receipt.receipt_digest"),
    harness_id: optionalString(value.harness_id, "expect.receipt.harness_id"),
    state: optionalString(value.state, "expect.receipt.state"),
    disposition: optionalString(value.disposition, "expect.receipt.disposition"),
    reason_code: optionalString(value.reason_code, "expect.receipt.reason_code"),
    act_ids: optionalStringArray(value.act_ids, "expect.receipt.act_ids"),
    child_receipt_refs: optionalStringArray(value.child_receipt_refs, "expect.receipt.child_receipt_refs"),
  };
  return expectation as HarnessReceiptShapeExpectation;
}

function validateEnv(value: Record<string, unknown>): Readonly<Record<string, string>> {
  return Object.fromEntries(
    Object.entries(value).map(([key, entry]) => {
      if (typeof entry !== "string") {
        throw new Error(`env.${key} must be a string.`);
      }
      return [key, entry];
    }),
  );
}

function requiredString(value: unknown, field: string): string {
  if (typeof value !== "string" || value.length === 0) {
    throw new Error(`${field} is required.`);
  }
  return value;
}

function optionalString(value: unknown, field: string): string | undefined {
  if (value === undefined || value === null) {
    return undefined;
  }
  if (typeof value !== "string") {
    throw new Error(`${field} must be a string.`);
  }
  return value;
}

function optionalRecord(value: unknown, field: string): Record<string, unknown> | undefined {
  if (value === undefined || value === null) {
    return undefined;
  }
  if (!isRecord(value)) {
    throw new Error(`${field} must be an object.`);
  }
  return value;
}

function optionalStringArray(value: unknown, field: string): readonly string[] | undefined {
  if (value === undefined || value === null) {
    return undefined;
  }
  if (!Array.isArray(value) || value.some((entry) => typeof entry !== "string")) {
    throw new Error(`${field} must be an array of strings.`);
  }
  return value;
}

function optionalStatus(value: unknown, field: string): HarnessExpectation["status"] {
  if (value === undefined || value === null) {
    return undefined;
  }
  if (
    value === "sealed" ||
    value === "failure" ||
    value === "needs_agent" ||
    value === "policy_denied" ||
    value === "escalated"
  ) {
    return value;
  }
  throw new Error(`${field} must be sealed, failure, needs_agent, policy_denied, or escalated.`);
}

function optionalSuccessFailure(value: unknown, field: string): "sealed" | "failure" | undefined {
  if (value === undefined || value === null) {
    return undefined;
  }
  if (value === "sealed" || value === "failure") {
    return value;
  }
  throw new Error(`${field} must be sealed or failure.`);
}

function optionalReceiptKind(value: unknown, field: string): HarnessReceiptExpectation["kind"] {
  if (value === undefined || value === null) {
    return undefined;
  }
  if (value === "harness_receipt") {
    return value as HarnessReceiptExpectation["kind"];
  }
  throw new Error(`${field} must be harness_receipt.`);
}

function optionalHarnessReceiptSchema(value: unknown, field: string): typeof harnessReceiptSchema | undefined {
  if (value === undefined || value === null) {
    return undefined;
  }
  if (value === harnessReceiptSchema) {
    return value;
  }
  throw new Error(`${field} must be ${harnessReceiptSchema}.`);
}
