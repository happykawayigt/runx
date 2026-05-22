import { readFile, stat } from "node:fs/promises";
import path from "node:path";

import { resolveLocalSkillProfile } from "@runxhq/core/config";

import { runHarnessTarget, type HarnessRunOptions, type HarnessSuiteResult } from "./runner.js";
import { parseSkillFrontmatter } from "./skill-frontmatter.js";
import { validateRunnerManifestYamlViaParser } from "../runner-local/parser-bridge.js";

export interface PublishHarnessSummary {
  readonly status: "passed" | "failed" | "not_declared";
  readonly case_count: number;
  readonly assertion_error_count: number;
  readonly assertion_errors: readonly string[];
  readonly case_names: readonly string[];
  readonly receipt_ids: readonly string[];
  // Cases that exercised the skill inside a graph (produced a graph receipt).
  // Feeds the stable-maturity graph-integration signal.
  readonly graph_case_count: number;
}

export async function validatePublishHarness(
  targetPath: string,
  options: HarnessRunOptions = {},
): Promise<PublishHarnessSummary> {
  const profileDocument = await resolveInlineHarnessProfileDocument(targetPath);
  if (!profileDocument) {
    return emptyHarnessSummary();
  }

  const manifest = await validateRunnerManifestYamlViaParser(profileDocument, { env: options.env });
  if (!manifest.harness || manifest.harness.cases.length === 0) {
    return emptyHarnessSummary();
  }

  const result = await runHarnessTarget(targetPath, options);
  if (!isHarnessSuiteResult(result)) {
    throw new Error(`Expected inline harness suite for publish target ${path.resolve(targetPath)}.`);
  }

  const receiptIds = result.cases.flatMap((entry) => [entry.receipt?.id, entry.graphReceipt?.id].filter(isString));
  return {
    status: result.assertionErrors.length === 0 ? "passed" : "failed",
    case_count: result.cases.length,
    assertion_error_count: result.assertionErrors.length,
    assertion_errors: result.assertionErrors,
    case_names: result.cases.map((entry) => entry.fixture.name),
    receipt_ids: receiptIds,
    graph_case_count: result.cases.filter((entry) => entry.graphReceipt).length,
  };
}

async function resolveInlineHarnessProfileDocument(targetPath: string): Promise<string | undefined> {
  const resolvedTargetPath = path.resolve(targetPath);
  const targetStat = await stat(resolvedTargetPath);
  const skillPath = targetStat.isDirectory() ? path.join(resolvedTargetPath, "SKILL.md") : resolvedTargetPath;
  if (path.basename(skillPath).toLowerCase() !== "skill.md") {
    return undefined;
  }
  const markdown = await readFile(skillPath, "utf8");
  const raw = parseSkillFrontmatter(markdown);
  const skillName = typeof raw.frontmatter.name === "string" ? raw.frontmatter.name : undefined;
  if (!skillName) {
    return undefined;
  }
  const profile = await resolveLocalSkillProfile(skillPath, skillName);
  return profile.profileDocument;
}

function emptyHarnessSummary(): PublishHarnessSummary {
  return {
    status: "not_declared",
    case_count: 0,
    assertion_error_count: 0,
    assertion_errors: [],
    case_names: [],
    receipt_ids: [],
    graph_case_count: 0,
  };
}

function isHarnessSuiteResult(
  value: Awaited<ReturnType<typeof runHarnessTarget>>,
): value is HarnessSuiteResult {
  return "cases" in value;
}

function isString(value: string | undefined): value is string {
  return typeof value === "string" && value.length > 0;
}
