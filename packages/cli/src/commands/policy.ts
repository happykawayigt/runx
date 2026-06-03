import { readFile } from "node:fs/promises";
import path from "node:path";

import {
  projectOperationalPolicyReadback,
  type OperationalPolicyReadback,
  type OperationalPolicyValidationFinding,
} from "@runxhq/contracts";
import { resolvePathFromUserInput } from "../cli-config.js";

import { renderRows, statusIcon, theme } from "../ui.js";

export type PolicyAction = "inspect" | "lint";

export interface PolicyCommandArgs {
  readonly policyAction?: PolicyAction;
  readonly policyPath?: string;
}

export interface PolicyCommandResult {
  readonly action: PolicyAction;
  readonly status: "success" | "failure";
  readonly path: string;
  readonly policy: OperationalPolicyReadback;
  readonly findings: readonly OperationalPolicyValidationFinding[];
}

export function policyAction(positionals: readonly string[]): PolicyAction | undefined {
  if (positionals[0] === "inspect" || positionals[0] === "lint") {
    return positionals[0];
  }
  return undefined;
}

export async function handlePolicyCommand(
  parsed: PolicyCommandArgs,
  env: NodeJS.ProcessEnv,
): Promise<PolicyCommandResult> {
  if (!parsed.policyAction) {
    throw new Error("policy action is required.");
  }
  if (!parsed.policyPath) {
    throw new Error("policy path is required.");
  }

  const resolvedPath = resolvePathFromUserInput(parsed.policyPath, env);
  const raw = await readFile(resolvedPath, "utf8");
  const value = parseJson(raw, resolvedPath);
  const policy = projectOperationalPolicyReadback(value);
  const findings = policy.findings;

  return {
    action: parsed.policyAction,
    status: findings.length === 0 ? "success" : "failure",
    path: displayPolicyPath(resolvedPath, env),
    policy,
    findings,
  };
}

export function renderPolicyResult(result: PolicyCommandResult, env: NodeJS.ProcessEnv): string {
  const t = theme(process.stdout, env);
  const lines = [
    "",
    `  ${statusIcon(result.status, t)}  ${t.bold}policy ${result.action}${t.reset}  ${t.dim}${result.status}${t.reset}`,
  ];
  lines.push(...renderRows([
    ["path", result.path],
    ["policy", result.policy.policy_id],
    ["schema", result.policy.schema_version],
    ["sources", String(result.policy.sources.length)],
    ["targets", String(result.policy.targets.length)],
    ["runners", String(result.policy.runners.length)],
    ["findings", String(result.findings.length)],
  ], t));

  if (result.policy.sources.length > 0) {
    lines.push("", `  ${t.bold}sources${t.reset}`);
    for (const source of result.policy.sources) {
      lines.push(
        `  - ${source.source_id}: ${source.provider}; locators=${source.locator_count}; thread=${source.source_thread_required ? source.publish_mode : "not-required"}; actions=${source.allowed_actions.join(",")}`,
      );
    }
  }

  if (result.policy.targets.length > 0) {
    lines.push("", `  ${t.bold}targets${t.reset}`);
    for (const target of result.policy.targets) {
      lines.push(
        `  - ${target.repo}: runners=${target.runner_ids.join(",")}; available=${target.available_runner_count}; owners=${target.owner_count}; actions=${target.allowed_actions.join(",")}`,
      );
    }
  }

  if (result.findings.length > 0) {
    lines.push("", `  ${t.bold}findings${t.reset}`);
    for (const finding of result.findings) {
      lines.push(`  - ${finding.code} ${finding.path}: ${finding.message}`);
    }
  }

  lines.push("");
  return `${lines.join("\n")}\n`;
}

function parseJson(raw: string, filePath: string): unknown {
  try {
    return JSON.parse(raw);
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    throw new Error(`Invalid JSON in ${filePath}: ${message}`);
  }
}

function displayPolicyPath(resolvedPath: string, env: NodeJS.ProcessEnv): string {
  const cwd = resolvePathFromUserInput(".", env);
  const relative = path.relative(cwd, resolvedPath);
  if (relative && !relative.startsWith("..") && !path.isAbsolute(relative)) {
    return relative;
  }
  return path.basename(resolvedPath);
}
