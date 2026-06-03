import { mkdir, readFile, readdir, rm, writeFile } from "node:fs/promises";
import { createHash } from "node:crypto";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { parseDocument } from "yaml";

const workspaceRoot = path.resolve(fileURLToPath(new URL("..", import.meta.url)));
const fixtureRoot = path.join(workspaceRoot, "fixtures", "runtime", "skills");
const check = process.argv.includes("--check");
const generatedAt = "2026-05-18T00:00:00Z";
const skillNames = ["issue-intake", "issue-to-pr"] as const;
const retiredReceiptFields = [
  "kind",
  retiredExecutionShape("skill"),
  retiredExecutionShape("graph"),
  "skill_name",
  "source_type",
  "graph_name",
  "owner",
];

process.chdir(workspaceRoot);

for (const skillName of skillNames) {
  await generateSkillFixtures(skillName);
}

console.log(`${check ? "checked" : "generated"} Rust product skill fixtures`);

function retiredExecutionShape(prefix: string): string {
  return `${prefix}_${"execution"}`;
}

async function generateSkillFixtures(skillName: typeof skillNames[number]): Promise<void> {
  const skillDir = path.join(workspaceRoot, "skills", skillName);
  const skillMarkdownPath = path.join(skillDir, "SKILL.md");
  const profilePath = path.join(skillDir, "X.yaml");
  const skillMarkdown = await readFile(skillMarkdownPath, "utf8");
  const profile = parseYamlObject(await readFile(profilePath, "utf8"), profilePath);
  const declaredSkillName = parseSkillName(skillMarkdown, skillMarkdownPath);
  if (declaredSkillName !== skillName || profile.skill !== skillName) {
    throw new Error(`${skillName}: product skill name drifted from SKILL.md/X.yaml`);
  }
  const cases = harnessCases(profile, profilePath);
  const targetDir = path.join(fixtureRoot, skillName);
  if (!check) {
    await rm(targetDir, { recursive: true, force: true });
  }
  await mkdir(path.join(targetDir, "cases"), { recursive: true });

  await writeOrCheck(path.join(targetDir, "metadata.json"), `${JSON.stringify({
    schema: "runx.runtime.skill_fixture.v1",
    generated_at: generatedAt,
    source: {
      skill: path.posix.join("skills", skillName, "SKILL.md"),
      profile: path.posix.join("skills", skillName, "X.yaml"),
    },
    skill_name: skillName,
    manifest_hash: `sha256:${sha256(`${skillMarkdown}\n${JSON.stringify(profile)}`)}`,
    harness_schema: "runx.receipt.v1",
    case_names: cases.map((entry) => String(entry.name)),
  }, null, 2)}\n`);

  const replaySteps = skillName === "issue-to-pr" ? graphReplaySteps(profile, skillName) : [];
  for (const entry of cases) {
    const normalizedEntry = skillName === "issue-intake" ? withIntakeDecision(entry) : entry;
    const fixture = skillName === "issue-intake"
      ? intakeFixture(normalizedEntry, skillName)
      : issueToPrFixture(normalizedEntry, skillName, replaySteps);
    assertNoRetiredReceiptFields(fixture, `${skillName}.${normalizedEntry.name}`);
    await writeOrCheck(
      path.join(targetDir, "cases", `${normalizedEntry.name}.yaml`),
      yaml(fixture),
    );
  }

  if (check) {
    await assertNoStaleCases(targetDir, cases);
  }
}

function intakeFixture(entry: Record<string, unknown>, skillName: string): Record<string, unknown> {
  return {
    name: entry.name,
    kind: "agent_task",
    runner: "issue-intake",
    inputs: entry.inputs ?? {},
    caller: entry.caller ?? {},
    expect: canonicalExpectation(entry, {
      status: "sealed",
      receiptId: `hrn_rcpt_${entry.name}_${entry.name}`,
      harnessId: `hrn_${entry.name}_${entry.name}`,
      disposition: "closed",
      reasonCode: `${entry.name}_closed`,
      actId: `act_${entry.name}`,
      decisionId: `dec_${entry.name}`,
    }),
    metadata: {
      product_skill: skillName,
      source_case: entry.name,
      runner_kind: "agent_task",
    },
  };
}

function issueToPrFixture(
  entry: Record<string, unknown>,
  skillName: string,
  replaySteps: { step_id: string; task: string }[],
): Record<string, unknown> {
  const childSteps = replayedChildSteps(entry, replaySteps);
  const expect = canonicalExpectation(entry, {
    status: "needs_agent",
    receiptId: `hrn_rcpt_${entry.name}`,
    harnessId: `hrn_${entry.name}_graph`,
    disposition: "deferred",
    reasonCode: `${entry.name}_deferred`,
    decisionIds: ["dec_graph"],
    childReceiptRefs: childSteps.map((step) => `runx:receipt:hrn_rcpt_${entry.name}_${step.step_id}`),
  });
  expect.steps = childSteps.map((step) => step.step_id);
  return {
    name: entry.name,
    kind: "graph",
    target: "../../../../../skills/issue-to-pr/X.yaml",
    runner: "issue-to-pr",
    inputs: entry.inputs ?? {},
    caller: entry.caller ?? {},
    expect,
    metadata: {
      product_skill: skillName,
      source_case: entry.name,
      runner_kind: "graph",
      graph_shape: "fixture_replay",
      graph_replay_steps: replaySteps,
    },
  };
}

function graphReplaySteps(
  profile: Record<string, unknown>,
  skillName: string,
): { step_id: string; task: string }[] {
  const runners = record(profile.runners, "runners") ?? {};
  const runner = record(runners[skillName], `runners.${skillName}`) ?? {};
  const graph = record(runner.graph, `runners.${skillName}.graph`) ?? {};
  const steps = Array.isArray(graph.steps) ? graph.steps : [];
  return steps.flatMap((rawStep, index) => {
    const step = record(rawStep, `runners.${skillName}.graph.steps[${index}]`);
    const run = record(step?.run, `runners.${skillName}.graph.steps[${index}].run`);
    if (!step || !run || run.type !== "agent-task" || typeof step.id !== "string" || typeof run.task !== "string") {
      return [];
    }
    return [{ step_id: step.id, task: run.task }];
  });
}

function replayedChildSteps(
  entry: Record<string, unknown>,
  replaySteps: { step_id: string; task: string }[],
): { step_id: string; task: string }[] {
  const answers = record(record(entry.caller, "caller")?.answers, "caller.answers") ?? {};
  const childSteps = [];
  for (const step of replaySteps) {
    childSteps.push(step);
    if (!answers[`agent_task.${step.task}.output`]) {
      break;
    }
  }
  return childSteps;
}

function withIntakeDecision(entry: Record<string, unknown>): Record<string, unknown> {
  const clone = JSON.parse(JSON.stringify(entry)) as Record<string, unknown>;
  const caller = record(clone.caller, "caller");
  const answers = record(caller?.answers, "caller.answers");
  const output = record(answers?.["agent_task.issue-intake.output"], "caller.answers.agent_task.issue-intake.output");
  if (!output || output.decision) {
    return clone;
  }
  const report = record(output.intake_report, "intake_report") ?? {};
  output.decision = {
    schema: "runx.decision.v1",
    decision_id: `dec_${clone.name}`,
    choice: decisionChoice(report.action_decision),
    summary: report.rationale ?? report.summary ?? "issue-intake selected the next governed boundary",
    recommended_lane: report.recommended_lane ?? "manual-review",
  };
  return clone;
}

function decisionChoice(value: unknown): string {
  switch (value) {
    case "proceed_to_build":
    case "proceed_to_plan":
      return "open";
    case "request_review":
      return "defer";
    case "stop":
      return "decline";
    default:
      return "monitor";
  }
}

function canonicalExpectation(
  entry: Record<string, unknown>,
  receipt: {
    status: string;
    receiptId: string;
    harnessId: string;
    disposition: string;
    reasonCode: string;
    actId?: string;
    decisionId?: string;
    decisionIds?: string[];
    childReceiptRefs?: string[];
  },
): Record<string, unknown> {
  const status = record(entry.expect, "expect")?.status ?? receipt.status;
  const receiptExpectation: Record<string, unknown> = {
    schema: "runx.receipt.v1",
    receipt_id: receipt.receiptId,
    harness_id: receipt.harnessId,
    state: "sealed",
    disposition: receipt.disposition,
    reason_code: receipt.reasonCode,
  };
  if (receipt.actId) {
    receiptExpectation.act_ids = [receipt.actId];
  }
  const decisionIds = receipt.decisionIds ?? (receipt.decisionId ? [receipt.decisionId] : []);
  if (decisionIds.length > 0) {
    receiptExpectation.decision_ids = decisionIds;
  }
  if (receipt.childReceiptRefs && receipt.childReceiptRefs.length > 0) {
    receiptExpectation.child_receipt_refs = receipt.childReceiptRefs;
  }
  return {
    status,
    receipt: receiptExpectation,
  };
}

function parseSkillName(markdown: string, sourcePath: string): string {
  const match = /^---\r?\n(?<frontmatter>.*?)\r?\n---/s.exec(markdown);
  if (!match?.groups?.frontmatter) {
    throw new Error(`${sourcePath}: missing SKILL.md frontmatter`);
  }
  return String(parseYamlObject(match.groups.frontmatter, sourcePath).name ?? "");
}

function parseYamlObject(source: string, sourcePath: string): Record<string, unknown> {
  const document = parseDocument(source, { prettyErrors: false });
  if (document.errors.length > 0) {
    throw new Error(`${sourcePath}: ${document.errors.map((error: { message: string }) => error.message).join("; ")}`);
  }
  return record(document.toJS(), sourcePath) ?? {};
}

function harnessCases(profile: Record<string, unknown>, sourcePath: string): Record<string, unknown>[] {
  const cases = record(profile.harness, `${sourcePath}.harness`)?.cases;
  if (!Array.isArray(cases)) {
    throw new Error(`${sourcePath}: harness.cases must be an array`);
  }
  return cases.map((entry, index) => {
    const value = record(entry, `${sourcePath}.harness.cases[${index}]`);
    if (!value || typeof value.name !== "string" || value.name.length === 0) {
      throw new Error(`${sourcePath}: harness.cases[${index}].name is required`);
    }
    return value;
  });
}

function assertNoRetiredReceiptFields(value: unknown, label: string): void {
  const findings: string[] = [];
  visit(value, [], (pathSegments, key) => {
    if (retiredReceiptFields.includes(key) && pathSegments.includes("receipt")) {
      findings.push(`${label}:${pathSegments.concat(key).join(".")}`);
    }
  });
  if (findings.length > 0) {
    throw new Error(`retired receipt expectation fields found:\n${findings.join("\n")}`);
  }
}

async function assertNoStaleCases(
  targetDir: string,
  cases: Record<string, unknown>[],
): Promise<void> {
  const expected = new Set(cases.map((entry) => `${entry.name}.yaml`));
  const casesDir = path.join(targetDir, "cases");
  let entries: string[];
  try {
    entries = await readdir(casesDir);
  } catch {
    entries = [];
  }
  const stale = entries.filter((entry) => !expected.has(entry));
  if (stale.length > 0) {
    throw new Error(`${casesDir}: stale generated fixture(s): ${stale.join(", ")}`);
  }
}

async function writeOrCheck(filePath: string, contents: string): Promise<void> {
  if (check) {
    const current = await readFile(filePath, "utf8").catch(() => undefined);
    if (current !== contents) {
      throw new Error(`${path.relative(workspaceRoot, filePath)} is stale; run pnpm tsx scripts/generate-rust-skill-fixtures.ts`);
    }
    return;
  }
  await mkdir(path.dirname(filePath), { recursive: true });
  await writeFile(filePath, contents);
}

function yaml(value: unknown, indent = 0): string {
  if (Array.isArray(value)) {
    if (value.length === 0) {
      return `${" ".repeat(indent)}[]\n`;
    }
    return value.map((entry) => `${" ".repeat(indent)}- ${yamlScalarOrBlock(entry, indent + 2)}`).join("");
  }
  const object = record(value, "yaml") ?? {};
  if (Object.keys(object).length === 0) {
    return `${" ".repeat(indent)}{}\n`;
  }
  return Object.entries(object).map(([key, entry]) => {
    if (entry === undefined) {
      return "";
    }
    if (isScalar(entry)) {
      return `${" ".repeat(indent)}${key}: ${scalar(entry)}\n`;
    }
    return `${" ".repeat(indent)}${key}:\n${yaml(entry, indent + 2)}`;
  }).join("");
}

function yamlScalarOrBlock(value: unknown, indent: number): string {
  if (isScalar(value)) {
    return `${scalar(value)}\n`;
  }
  return `\n${yaml(value, indent)}`;
}

function scalar(value: unknown): string {
  if (value === null) {
    return "null";
  }
  if (typeof value === "number" || typeof value === "boolean") {
    return String(value);
  }
  const stringValue = String(value);
  return JSON.stringify(stringValue);
}

function isScalar(value: unknown): boolean {
  return value === null || ["string", "number", "boolean"].includes(typeof value);
}

function record(value: unknown, _field: string): Record<string, unknown> | undefined {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    return undefined;
  }
  return value as Record<string, unknown>;
}

function visit(
  value: unknown,
  pathSegments: string[],
  onKey: (pathSegments: string[], key: string) => void,
): void {
  if (Array.isArray(value)) {
    value.forEach((entry, index) => visit(entry, pathSegments.concat(String(index)), onKey));
    return;
  }
  const object = record(value, "visit");
  if (!object) {
    return;
  }
  for (const [key, entry] of Object.entries(object)) {
    onKey(pathSegments, key);
    visit(entry, pathSegments.concat(key), onKey);
  }
}

function sha256(value: string): string {
  return createHash("sha256").update(value).digest("hex");
}
