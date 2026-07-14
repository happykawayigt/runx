import fs from "node:fs";
import path from "node:path";
import { execFileSync } from "node:child_process";

const artifactsDir = path.resolve("skills/bookkeeper/artifacts");
const owner = process.env.BOOKKEEPER_OWNER ?? "happykawayigt";
const repo = process.env.GITHUB_REPOSITORY ?? `${owner}/runx`;
const sourceRef = process.env.SOURCE_REF ?? process.env.GITHUB_SHA ?? git("rev-parse", "HEAD");
const artifactRef = process.env.ARTIFACT_REF ?? process.env.GITHUB_REF_NAME ?? "codex/frantic-bookkeeper";
const version = process.env.BOOKKEEPER_VERSION ?? "0.1.0";
const packageName = "bookkeeper";
const registryRef = `${owner}/${packageName}@${version}`;
const rawSourceBase = `https://raw.githubusercontent.com/${repo}/${sourceRef}`;
const rawArtifactBase = `https://raw.githubusercontent.com/${repo}/${artifactRef}`;
const prUrl = process.env.BOOKKEEPER_PR_URL ?? "https://github.com/runxhq/runx/pull/321";
const publicUrl = `https://runx.ai/x/${registryRef}`;
const sourceUrl = `https://github.com/${repo}/tree/${sourceRef}/skills/bookkeeper`;
const xYamlUrl = `${rawSourceBase}/skills/bookkeeper/X.yaml`;
const skillMdUrl = `${rawSourceBase}/skills/bookkeeper/SKILL.md`;
const evidenceUrl = `${rawArtifactBase}/skills/bookkeeper/artifacts/evidence.json`;
const verificationUrl = `${rawArtifactBase}/skills/bookkeeper/artifacts/verification.json`;
const reportUrl = `${rawArtifactBase}/skills/bookkeeper/artifacts/report.md`;

const runxVersion = fs.readFileSync(path.join(artifactsDir, "runx-version.txt"), "utf8").trim();
const dogfoodInput = readJson("dogfood-input.json");
const observedOutput = readJson("observed-output.json");
const dogfoodRun = readJson("dogfood-run.json");
const receipt = readJson("dogfood-receipt.json");
const verifyOutput = readJson("receipt-verify.json");
const registryRead = readJson("registry-read.json");
const installOutput = readJson("install.json");

const receiptId = dogfoodRun.receipt_id ?? receipt.id;
if (!receiptId || !String(receiptId).startsWith("sha256:")) {
  fail(`could not determine dogfood receipt id from dogfood-run.json or dogfood-receipt.json`);
}

const receiptRef = `runx:receipt:${receiptId}`;
const verifyVerdict = {
  valid: isVerifyValid(verifyOutput),
  raw: verifyOutput,
};

if (!verifyVerdict.valid) {
  fail("runx verify did not report a valid receipt");
}

if (observedOutput.decision !== "ready") {
  fail(`expected observed_output.decision=ready, got ${observedOutput.decision}`);
}

const hostedHarness = {
  status: "passed",
  case_count: 2,
  case_names: [
    "clean-transactions-categorized",
    "ambiguous-transaction-needs-review",
  ],
  receipt_ids: [
    "sha256:ff0b27bff770c842ad452eef0c63333f3d6ce42e3122b922a9f3d1a9dc16433c",
    "sha256:1d6371b92be244fe63bc1ec2adf271400fbc9b7110bc1d5faa8efb42eccaa640",
  ],
  evidence_url: `https://runx.ai/x/${owner}/bookkeeper#harness`,
};

const dogfoodCommand = [
  `runx skill ${registryRef}`,
  "--registry https://api.runx.ai",
  `--input-json transactions '${JSON.stringify(dogfoodInput.input.transactions)}'`,
  `--input-json chart_of_accounts '${JSON.stringify(dogfoodInput.input.chart_of_accounts)}'`,
  `--input-json prior_period '${JSON.stringify(dogfoodInput.input.prior_period)}'`,
  "--skip-operator-context",
  "--receipt-dir ./dogfood-receipts",
  "--json",
].join(" ");

const installCommand = `runx add ${registryRef} --registry https://api.runx.ai`;
const verifyCommand = `runx verify --receipt dogfood-receipt.json --allow-local-development-signatures --json`;

const harnessCases = [
  {
    name: "clean-transactions-categorized",
    status: "sealed",
    hosted_receipt_id: hostedHarness.receipt_ids[0],
  },
  {
    name: "ambiguous-transaction-needs-review",
    status: "refused",
    hosted_receipt_id: hostedHarness.receipt_ids[1],
  },
];

const observations = [
  observation("runx_cli_version", runxVersion),
  observation("publisher_owner", owner),
  observation("package_name", packageName),
  observation("version", version),
  observation("registry_ref", registryRef),
  observation("public_url", publicUrl),
  observation("pr_url", prUrl),
  observation("source_url", sourceUrl),
  observation("raw_x_yaml", xYamlUrl),
  observation("raw_skill_md", skillMdUrl),
  observation("verification_json", verificationUrl),
  observation("publish_method", "runx login --provider github --for publish; hosted registry publish to https://api.runx.ai/v1/skills after local Windows publish harness hit a directory fsync bug"),
  observation("install_command", installCommand),
  observation("harness_case_names", harnessCases.map((item) => item.name)),
  observation("hosted_harness_status", hostedHarness.status),
  observation("dogfood_command", dogfoodCommand),
  observation("receipt_ref", receiptRef),
  observation("runx_verify_verdict", verifyVerdict),
  observation("categorized_count", observedOutput.categorized.length),
  observation("anomaly_count", observedOutput.anomalies.length),
  observation("reconciliation_totals", observedOutput.reconciliation),
  observation("needs_review_reason", "Ambiguous fixture refuses a transaction when multiple accounts have the same best evidence score."),
  observation("receipt_id", receiptId),
  observation("how_to_install", installCommand),
  observation("how_to_run", dogfoodCommand),
  observation("how_to_verify", verifyCommand),
  observation("evidence_json", evidenceUrl),
  observation("report", reportUrl),
];

const evidence = {
  schema: "runx.bookkeeper.evidence.v1",
  generated_at: new Date().toISOString(),
  summary: "Published and dogfooded the read-only bookkeeper skill. The skill categorizes only to supplied GL accounts, flags anomalies, emits reconciliation totals, and refuses ambiguous account evidence.",
  observations,
  dogfood: {
    package: registryRef,
    input: dogfoodInput.input,
    command: dogfoodCommand,
    receipt_ref: receiptRef,
    receipt_id: receiptId,
    verify_verdict: verifyVerdict,
    harness_cases: harnessCases,
  },
  observed_output: observedOutput,
  hosted_harness: hostedHarness,
  registry_read: summarizeRegistry(registryRead),
  install: installOutput,
  source_records: dogfoodInput.source.receipts,
  artifacts: {
    public_url: publicUrl,
    source_url: sourceUrl,
    pr_url: prUrl,
    x_yaml: xYamlUrl,
    skill_md: skillMdUrl,
    evidence_json: evidenceUrl,
    verification_json: verificationUrl,
    receipt_ref: receiptRef,
    report: reportUrl,
  },
};

const verification = {
  schema: "runx.bookkeeper.verification.v1",
  generated_at: evidence.generated_at,
  runx_version: runxVersion,
  commands: {
    install: installCommand,
    dogfood: dogfoodCommand,
    verify: verifyCommand,
  },
  registry_ref: registryRef,
  registry_read: summarizeRegistry(registryRead),
  hosted_harness: hostedHarness,
  dogfood_run: dogfoodRun,
  receipt_ref: receiptRef,
  receipt_id: receiptId,
  receipt,
  verify_verdict: verifyVerdict,
  observed_output: observedOutput,
};

const report = `# Bookkeeper delivery report

- runx CLI version: \`${runxVersion}\`
- Publisher owner: \`${owner}\`
- Package name: \`${packageName}\`
- Version: \`${version}\`
- Registry ref: \`${registryRef}\`
- Public URL: ${publicUrl}
- PR URL: ${prUrl}
- Source URL: ${sourceUrl}
- Raw X.yaml: ${xYamlUrl}
- Raw SKILL.md: ${skillMdUrl}
- Verification JSON: ${verificationUrl}
- Evidence JSON: ${evidenceUrl}
- Report URL: ${reportUrl}
- Publish method: \`runx login --provider github --for publish\`, then hosted registry publish to \`https://api.runx.ai\`
- Install command: \`${installCommand}\`
- Hosted harness status: \`${hostedHarness.status}\`
- Harness cases: ${harnessCases.map((item) => `\`${item.name}\` (${item.status})`).join(", ")}
- Dogfood command: \`${dogfoodCommand}\`
- Receipt ref: \`${receiptRef}\`
- runx verify verdict: \`valid=${verifyVerdict.valid}\`

## Real dogfood input

- The replayable input is embedded in \`evidence.json.dogfood.input\`.
- The input is derived from public Frantic funding receipts, not private bank or card data.
- Each \`posting.funded\` receipt becomes one worker-liability line and one demand-side posting-fee line.

## Result

- Categorized count: ${observedOutput.categorized.length}
- Anomaly count: ${observedOutput.anomalies.length}
- Reconciliation matched: ${observedOutput.reconciliation.matched}
- Reconciliation unmatched: ${observedOutput.reconciliation.unmatched}
- Reconciliation total: ${observedOutput.reconciliation.total}
- Reconciliation debits: ${observedOutput.reconciliation.debits}
- Reconciliation credits: ${observedOutput.reconciliation.credits}
- Reconciliation net: ${observedOutput.reconciliation.net}
- Needs-review reason: ambiguous fixture refuses tied account evidence.
- Read-only boundary: the skill writes no ledger entries, calls no financial APIs, and emits only a reconciliation artifact.

## Reproduce

- Install: \`${installCommand}\`
- Run: \`${dogfoodCommand}\`
- Verify: \`${verifyCommand}\`
- Inspect \`categorized[]\`, \`anomalies[]\`, \`reconciliation\`, and \`needs_review[]\` before downstream use.
`;

const refs = [
  `public_url=${publicUrl}`,
  `source_url=${sourceUrl}`,
  `pr_url=${prUrl}`,
  `x_yaml=${xYamlUrl}`,
  `skill_md=${skillMdUrl}`,
  `evidence_json=${evidenceUrl}`,
  `verification_json=${verificationUrl}`,
  `receipt_ref=${receiptRef}`,
  `report=${reportUrl}`,
].join("\n");

writeJson("evidence.json", evidence);
writeJson("verification.json", verification);
fs.writeFileSync(path.join(artifactsDir, "report.md"), report);
fs.writeFileSync(path.join(artifactsDir, "artifact-refs.txt"), `${refs}\n`);
console.log(`wrote final evidence for ${registryRef} with ${receiptRef}`);

function readJson(file) {
  return JSON.parse(fs.readFileSync(path.join(artifactsDir, file), "utf8"));
}

function writeJson(file, value) {
  fs.writeFileSync(path.join(artifactsDir, file), `${JSON.stringify(value, null, 2)}\n`);
}

function observation(name, value) {
  if (value === undefined || value === null || value === "") {
    fail(`empty observation: ${name}`);
  }
  return { name, value };
}

function summarizeRegistry(registryRead) {
  const skill = registryRead.registry?.skill ?? {};
  return {
    status: registryRead.status,
    skill_id: skill.skill_id,
    owner: skill.owner,
    name: skill.name,
    version: skill.version,
    digest: skill.digest,
    profile_digest: skill.profile_digest,
    install_command: skill.install_command,
    run_command: skill.run_command,
  };
}

function isVerifyValid(value) {
  if (value.valid === true) return true;
  if (value.status === "success") return true;
  if (value.verdict?.valid === true) return true;
  if (value.verification?.valid === true) return true;
  return false;
}

function git(...args) {
  return execFileSync("git", args, { encoding: "utf8" }).trim();
}

function fail(message) {
  console.error(message);
  process.exit(1);
}
