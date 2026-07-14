import assert from "node:assert/strict";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

const skillDir = dirname(fileURLToPath(import.meta.url));

const clean = runFixture("clean-batch.json", 0);
assert.equal(clean.decision, "ready");
assert.equal(clean.categorized.length, 3);
assert.deepEqual(clean.anomalies, []);
assert.deepEqual(clean.needs_review, []);
assert.deepEqual(clean.reconciliation, {
  matched: 3,
  unmatched: 0,
  total: 3,
  prior_period_matches: 0,
  debits: 185,
  credits: 300,
  net: 115,
});
assert.deepEqual(
  clean.categorized.map((entry) => entry.account_id),
  ["4000", "6100", "6200"],
);
for (const entry of clean.categorized) {
  assert.equal(typeof entry.confidence, "number");
  assert.ok(entry.confidence >= 0 && entry.confidence <= 1);
  assert.ok(entry.reason.length > 0);
}

const ambiguous = runFixture("ambiguous-batch.json", 2);
assert.equal(ambiguous.decision, "needs_review");
assert.deepEqual(ambiguous.categorized, []);
assert.equal(ambiguous.needs_review.length, 1);
assert.equal(ambiguous.anomalies[0].type, "ambiguous_account");
assert.deepEqual(ambiguous.anomalies[0].candidates, ["6000", "6200"]);
assert.deepEqual(ambiguous.reconciliation, {
  matched: 0,
  unmatched: 1,
  total: 1,
  prior_period_matches: 0,
  debits: 500,
  credits: 0,
  net: -500,
});

const allowedAccounts = new Set(["6000", "6200"]);
for (const entry of ambiguous.categorized) {
  assert.ok(allowedAccounts.has(entry.account_id), "runner invented a GL account");
}

process.stdout.write("bookkeeper fixture tests passed\n");

function runFixture(name, expectedStatus) {
  const fixturePath = join(skillDir, "fixtures", name);
  const execution = spawnSync(process.execPath, [join(skillDir, "run.mjs")], {
    cwd: skillDir,
    env: { ...process.env, RUNX_INPUTS_PATH: fixturePath },
    encoding: "utf8",
  });
  assert.equal(execution.status, expectedStatus, execution.stderr || execution.stdout);
  return JSON.parse(execution.stdout);
}
