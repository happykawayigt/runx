import { readFile } from "node:fs/promises";
import { execFile } from "node:child_process";
import path from "node:path";
import { promisify } from "node:util";
import { fileURLToPath, pathToFileURL } from "node:url";

const workspaceRoot = path.resolve(fileURLToPath(new URL("..", import.meta.url)));
const defaultFixturePath = path.join(workspaceRoot, "tests", "fixtures", "clean-kernel-prs.json");
const execFileAsync = promisify(execFile);

export type CleanKernelPrReason =
  | "ts_kernel"
  | "kernel_fixture_refresh"
  | "rust_only"
  | "parser_only"
  | "missing_passing_evidence"
  | "outside_advisory_window"
  | "outside_kernel_promotion_scope";

export interface CleanKernelPrReportEntry {
  readonly number?: number;
  readonly title: string;
  readonly reason: CleanKernelPrReason;
  readonly files: readonly string[];
  readonly passing_evidence: boolean;
  readonly merged_at?: string;
}

export interface CleanKernelPrReport {
  readonly advisory_start: unknown;
  readonly advisory_start_source: "cli" | "fixture";
  readonly minimum: number;
  readonly count: number;
  readonly meets_minimum: boolean;
  readonly counting: readonly CleanKernelPrReportEntry[];
  readonly non_counting: readonly CleanKernelPrReportEntry[];
}

interface CliOptions {
  readonly fixturePath: string;
  readonly source: "fixture" | "github";
  readonly advisoryStart?: string;
  readonly minimum: number;
  readonly repo?: string;
  readonly limit: number;
}

interface CliRunOptions {
  readonly cwd?: string;
}

interface CliRunResult {
  readonly exitCode: number;
  readonly stdout: string;
  readonly stderr: string;
}

export async function runCountCleanKernelPrsCli(
  args: readonly string[],
  options: CliRunOptions = {},
): Promise<CliRunResult> {
  try {
    const parsed = parseCliArgs(args, options.cwd ?? workspaceRoot);
    const fixture = parsed.source === "github"
      ? await readGitHubFixture(parsed)
      : await readJsonFile(parsed.fixturePath);
    const report = analyzeCleanKernelPrs(fixture, {
      advisoryStart: parsed.advisoryStart,
      minimum: parsed.minimum,
    });
    return {
      exitCode: report.meets_minimum ? 0 : 1,
      stdout: `${JSON.stringify(report, null, 2)}\n`,
      stderr: report.meets_minimum
        ? ""
        : `clean kernel PR count ${report.count} is below required minimum ${report.minimum}\n`,
    };
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    return {
      exitCode: 1,
      stdout: "",
      stderr: `${message}\n`,
    };
  }
}

export function analyzeCleanKernelPrs(
  fixture: unknown,
  options: {
    readonly advisoryStart?: unknown;
    readonly minimum?: number;
  } = {},
): CleanKernelPrReport {
  const fixtureObject = asRecord(fixture, "fixture");
  const advisory = resolveAdvisoryStart(fixtureObject, options.advisoryStart);
  const prs = readPullRequests(fixtureObject);
  const minimum = options.minimum ?? readFixtureMinimum(fixtureObject) ?? 1;
  const advisoryStartInstant = resolveAdvisoryStartInstant(advisory.value, prs);

  if (!Number.isInteger(minimum) || minimum < 0) {
    throw new Error(`--min must be a non-negative integer`);
  }

  const entries = prs.map((pr) => classifyPullRequest(pr, advisoryStartInstant));
  const counting = entries.filter((entry) => entry.reason === "ts_kernel" || entry.reason === "kernel_fixture_refresh");
  const nonCounting = entries.filter((entry) => !counting.includes(entry));

  return {
    advisory_start: advisory.value,
    advisory_start_source: advisory.source,
    minimum,
    count: counting.length,
    meets_minimum: counting.length >= minimum,
    counting,
    non_counting: nonCounting,
  };
}

function classifyPullRequest(
  rawPr: unknown,
  advisoryStartInstant?: number,
): CleanKernelPrReportEntry {
  const pr = asRecord(rawPr, "pull request");
  const files = readFiles(pr);
  const passingEvidence = hasPassingEvidence(pr);
  const mergedAt = readMergedAt(pr);
  const base = {
    number: readNumber(pr),
    title: readTitle(pr),
    files,
    passing_evidence: passingEvidence,
    ...(mergedAt ? { merged_at: mergedAt } : {}),
  };

  if (!isInsideAdvisoryWindow(pr, advisoryStartInstant)) {
    return { ...base, reason: "outside_advisory_window" };
  }
  if (isParserOnly(files)) {
    return { ...base, reason: "parser_only" };
  }
  if (isRustOnly(files)) {
    return { ...base, reason: "rust_only" };
  }
  if (!passingEvidence) {
    return { ...base, reason: "missing_passing_evidence" };
  }
  if (isTsKernelOnly(files)) {
    return { ...base, reason: "ts_kernel" };
  }
  if (isDeliberateKernelFixtureRefresh(pr, files)) {
    return { ...base, reason: "kernel_fixture_refresh" };
  }
  return { ...base, reason: "outside_kernel_promotion_scope" };
}

function parseCliArgs(args: readonly string[], cwd: string): CliOptions {
  let fixturePath = defaultFixturePath;
  let source: "fixture" | "github" = "fixture";
  let advisoryStart: string | undefined;
  let minimum = 1;
  let repo: string | undefined;
  let limit = 50;

  for (let index = 0; index < args.length; index += 1) {
    const arg = args[index];
    if (arg === "--from-github") {
      source = "github";
      continue;
    }
    if (arg === "--repo") {
      repo = requiredArgValue(args, index, arg);
      index += 1;
      continue;
    }
    if (arg === "--limit") {
      const value = requiredArgValue(args, index, arg);
      limit = Number(value);
      if (!Number.isInteger(limit) || limit <= 0) {
        throw new Error(`--limit must be a positive integer`);
      }
      index += 1;
      continue;
    }
    if (arg === "--fixture") {
      fixturePath = requiredArgValue(args, index, arg);
      index += 1;
      continue;
    }
    if (arg === "--advisory-start") {
      advisoryStart = requiredArgValue(args, index, arg);
      index += 1;
      continue;
    }
    if (arg === "--min") {
      const value = requiredArgValue(args, index, arg);
      minimum = Number(value);
      if (!Number.isInteger(minimum) || minimum < 0) {
        throw new Error(`--min must be a non-negative integer`);
      }
      index += 1;
      continue;
    }
    if (arg === "--help" || arg === "-h") {
      throw new Error("usage: tsx scripts/count-clean-kernel-prs.ts [--fixture path | --from-github [--repo owner/name] [--limit count]] [--advisory-start evidence] [--min count]");
    }
    throw new Error(`unsupported argument: ${arg}`);
  }

  return {
    fixturePath: path.resolve(cwd, fixturePath),
    source,
    advisoryStart,
    minimum,
    repo,
    limit,
  };
}

async function readGitHubFixture(options: CliOptions): Promise<unknown> {
  if (!hasExplicitValue(options.advisoryStart)) {
    throw new Error("--from-github requires --advisory-start so live evidence has an audited start point");
  }

  const args = [
    "pr",
    "list",
    "--state",
    "merged",
    "--limit",
    String(options.limit),
    "--json",
    "number,title,mergedAt,files,statusCheckRollup",
  ];
  if (options.repo) {
    args.splice(2, 0, "--repo", options.repo);
  }

  const { stdout } = await execFileAsync("gh", args, {
    cwd: workspaceRoot,
    maxBuffer: 20 * 1024 * 1024,
  });
  const pullRequests = JSON.parse(stdout) as unknown;
  return {
    advisory_start: options.advisoryStart,
    prs: normalizeGitHubPullRequests(pullRequests),
  };
}

export function normalizeGitHubPullRequests(value: unknown): readonly Record<string, unknown>[] {
  if (!Array.isArray(value)) {
    throw new Error("GitHub PR response must be an array");
  }
  return value.map((item) => {
    const pr = asRecord(item, "GitHub pull request");
    const files = pr.files;
    const statusCheckRollup = pr.statusCheckRollup;
    return {
      number: readNumber(pr),
      title: readTitle(pr),
      merged_at: typeof pr.mergedAt === "string" ? pr.mergedAt : undefined,
      metadata_source: "github",
      files: Array.isArray(files)
        ? files.map((file) => normalizeGitHubFilePath(file))
        : [],
      evidence: {
        require_rust_kernel_parity: true,
        checks: Array.isArray(statusCheckRollup)
          ? statusCheckRollup.map(normalizeGitHubCheck)
          : [],
      },
    };
  });
}

function normalizeGitHubFilePath(value: unknown): string {
  if (typeof value === "string") {
    return normalizePath(value);
  }
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw new Error("GitHub file entry must be a path string or object");
  }
  const record = value as Record<string, unknown>;
  const pathValue = record.path;
  if (typeof pathValue !== "string" || pathValue.trim().length === 0) {
    throw new Error("GitHub file entry must include path");
  }
  return normalizePath(pathValue);
}

function normalizeGitHubCheck(value: unknown): Record<string, unknown> {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    return { conclusion: "unknown" };
  }
  const record = value as Record<string, unknown>;
  const conclusion = record.conclusion ?? record.state ?? record.status;
  return {
    name: record.name ?? record.context ?? record.workflowName,
    conclusion: typeof conclusion === "string" ? conclusion.toLowerCase() : "unknown",
  };
}

function requiredArgValue(args: readonly string[], index: number, flag: string): string {
  const value = args[index + 1];
  if (!value || value.startsWith("--")) {
    throw new Error(`${flag} requires a value`);
  }
  return value;
}

function resolveAdvisoryStart(
  fixture: Record<string, unknown>,
  cliAdvisoryStart: unknown,
): { readonly source: "cli" | "fixture"; readonly value: unknown } {
  if (hasExplicitValue(cliAdvisoryStart)) {
    return { source: "cli", value: cliAdvisoryStart };
  }

  const snake = fixture.advisory_start;
  const camel = fixture.advisoryStart;
  if (hasExplicitValue(snake) && hasExplicitValue(camel) && JSON.stringify(snake) !== JSON.stringify(camel)) {
    throw new Error("fixture advisory_start and advisoryStart disagree");
  }
  if (hasExplicitValue(snake)) {
    return { source: "fixture", value: snake };
  }
  if (hasExplicitValue(camel)) {
    return { source: "fixture", value: camel };
  }
  throw new Error("missing advisory start evidence; pass --advisory-start or set fixture.advisory_start/advisoryStart");
}

function hasExplicitValue(value: unknown): boolean {
  if (typeof value === "string") {
    return value.trim().length > 0;
  }
  return value !== undefined && value !== null;
}

function readPullRequests(fixture: Record<string, unknown>): readonly unknown[] {
  const prs = fixture.prs ?? fixture.pull_requests ?? fixture.pullRequests;
  if (!Array.isArray(prs)) {
    throw new Error("fixture must include a prs, pull_requests, or pullRequests array");
  }
  return prs;
}

function readFixtureMinimum(fixture: Record<string, unknown>): number | undefined {
  const minimum = fixture.minimum ?? fixture.min;
  if (minimum === undefined) {
    return undefined;
  }
  if (typeof minimum !== "number") {
    throw new Error("fixture minimum/min must be a number");
  }
  return minimum;
}

function readFiles(pr: Record<string, unknown>): readonly string[] {
  const files = pr.files ?? pr.changed_files ?? pr.changedFiles;
  if (!Array.isArray(files) || !files.every((file) => typeof file === "string" && file.trim().length > 0)) {
    throw new Error(`pull request ${readTitle(pr)} must include a non-empty files array`);
  }
  return files.map((file) => normalizePath(file as string));
}

function readTitle(pr: Record<string, unknown>): string {
  const title = pr.title;
  return typeof title === "string" && title.trim().length > 0 ? title : "(untitled)";
}

function readNumber(pr: Record<string, unknown>): number | undefined {
  const value = pr.number ?? pr.pr_number ?? pr.prNumber;
  return typeof value === "number" && Number.isInteger(value) ? value : undefined;
}

function readMergedAt(pr: Record<string, unknown>): string | undefined {
  const value = pr.merged_at ?? pr.mergedAt;
  return typeof value === "string" && value.trim().length > 0 ? value : undefined;
}

function isLiveGitHubMetadata(pr: Record<string, unknown>): boolean {
  return pr.metadata_source === "github" || pr.metadataSource === "github";
}

function isInsideAdvisoryWindow(
  pr: Record<string, unknown>,
  advisoryStartInstant?: number,
): boolean {
  const mergedAt = readMergedAt(pr);
  if (!mergedAt) {
    return !isLiveGitHubMetadata(pr);
  }
  const mergedAtInstant = Date.parse(mergedAt);
  if (Number.isNaN(mergedAtInstant) || advisoryStartInstant === undefined) {
    return false;
  }
  return mergedAtInstant > advisoryStartInstant;
}

function resolveAdvisoryStartInstant(value: unknown, prs: readonly unknown[]): number | undefined {
  const hasMergeTimes = prs.some((rawPr) => {
    const pr = asRecord(rawPr, "pull request");
    return readMergedAt(pr) !== undefined || isLiveGitHubMetadata(pr);
  });
  if (!hasMergeTimes) {
    return undefined;
  }

  const timestamp = extractAdvisoryStartTimestamp(value);
  if (!timestamp) {
    throw new Error("advisory start must include a parseable timestamp when PR merge times are present");
  }
  const instant = Date.parse(timestamp);
  if (Number.isNaN(instant)) {
    throw new Error(`advisory start timestamp is not parseable: ${timestamp}`);
  }
  return instant;
}

function extractAdvisoryStartTimestamp(value: unknown): string | undefined {
  if (typeof value === "string") {
    return value;
  }
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    return undefined;
  }
  const record = value as Record<string, unknown>;
  for (const key of ["timestamp", "advisory_start", "advisoryStart", "merged_after", "mergedAfter", "start", "at"]) {
    const candidate = record[key];
    if (typeof candidate === "string" && candidate.trim().length > 0) {
      return candidate;
    }
  }
  return undefined;
}

function hasPassingEvidence(pr: Record<string, unknown>): boolean {
  if (pr.passing_evidence === true || pr.passingEvidence === true) {
    return true;
  }
  const evidence = pr.evidence;
  if (!evidence || typeof evidence !== "object" || Array.isArray(evidence)) {
    return false;
  }
  return evidenceContainsPass(evidence as Record<string, unknown>);
}

function evidenceContainsPass(evidence: Record<string, unknown>): boolean {
  const directValues = [evidence.status, evidence.verdict, evidence.conclusion, evidence.result];
  const directEvidence = directValues.filter((value) => value !== undefined && value !== null);
  const hasDirectPass = directValues.some(isPassingToken);

  const checks = evidence.checks ?? evidence.required_checks ?? evidence.requiredChecks;
  if (Array.isArray(checks) && checks.length > 0) {
    const normalizedChecks = checks.map((check) => {
      if (!check || typeof check !== "object" || Array.isArray(check)) {
        return { name: "", passed: false };
      }
      const record = check as Record<string, unknown>;
      return {
        name: [record.name, record.context, record.workflowName]
          .filter((value): value is string => typeof value === "string")
          .join(" ")
          .toLowerCase(),
        passed: [record.status, record.verdict, record.conclusion, record.result].some(isPassingToken),
      };
    });
    const requiredParityChecks = normalizedChecks.filter((check) =>
      check.name.includes("rust") && check.name.includes("kernel") && check.name.includes("parity"),
    );
    if (evidence.require_rust_kernel_parity === true && requiredParityChecks.length === 0) {
      return false;
    }
    const checksToRequire = requiredParityChecks.length > 0 ? requiredParityChecks : normalizedChecks;
    return checksToRequire.every((check) => check.passed)
      && (directEvidence.length === 0 || hasDirectPass);
  }
  return hasDirectPass;
}

function isPassingToken(value: unknown): boolean {
  return typeof value === "string" && ["clean", "pass", "passed", "success", "successful"].includes(value.toLowerCase());
}

function isTsKernelOnly(files: readonly string[]): boolean {
  return files.every(isTsKernelFile);
}

function isDeliberateKernelFixtureRefresh(pr: Record<string, unknown>, files: readonly string[]): boolean {
  const explicit = pr.deliberate_kernel_fixture_refresh === true
    || pr.deliberateKernelFixtureRefresh === true
    || pr.kind === "kernel_fixture_refresh"
    || pr.classification === "kernel_fixture_refresh";
  return explicit
    && files.some(isKernelFixtureFile)
    && files.every((file) => isKernelFixtureFile(file) || isTsKernelFile(file));
}

function isTsKernelFile(file: string): boolean {
  return /^packages\/core\/src\/(?:state-machine|policy)\/.+\.ts$/u.test(file);
}

function isKernelFixtureFile(file: string): boolean {
  return file.startsWith("fixtures/kernel/");
}

function isRustOnly(files: readonly string[]): boolean {
  return files.every((file) => file.startsWith("crates/") || file === "Cargo.toml" || file.endsWith(".rs"));
}

function isParserOnly(files: readonly string[]): boolean {
  return files.every((file) =>
    file.startsWith("packages/core/src/parser/")
    || file.startsWith("crates/runx-parser/")
    || file.startsWith("fixtures/parser/")
    || file === "scripts/generate-rust-parser-fixtures.ts"
    || file.endsWith("-parser.test.ts"),
  );
}

function normalizePath(value: string): string {
  return value.replace(/\\/gu, "/").replace(/^\.\//u, "");
}

function asRecord(value: unknown, label: string): Record<string, unknown> {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw new Error(`${label} must be a JSON object`);
  }
  return value as Record<string, unknown>;
}

async function readJsonFile(filePath: string): Promise<unknown> {
  return JSON.parse(await readFile(filePath, "utf8")) as unknown;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  void runCountCleanKernelPrsCli(process.argv.slice(2)).then((result) => {
    if (result.stdout) {
      process.stdout.write(result.stdout);
    }
    if (result.stderr) {
      process.stderr.write(result.stderr);
    }
    process.exitCode = result.exitCode;
  });
}
