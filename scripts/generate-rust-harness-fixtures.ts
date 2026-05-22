import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import path from "node:path";
import process from "node:process";

import { canonicalJsonStringify, sha256Prefixed } from "@runxhq/contracts";

const repoRoot = path.resolve(import.meta.dirname, "..");
const oracleDir = path.join(repoRoot, "fixtures/harness/oracle");
const createdAt = "2026-05-18T00:00:00Z";

type Json = null | boolean | number | string | Json[] | { [key: string]: Json };

interface OracleReceipt {
  readonly fixture: string;
  readonly name: string;
  readonly bodyDigest: string;
  readonly receiptDigest: string;
  readonly canonicalJson: string;
}

interface ActRefs {
  readonly sourceRefs: Json[];
  readonly surfaceRefs: Json[];
  readonly artifactRefs: Json[];
  readonly verificationRefs: Json[];
}

const write = process.argv.includes("--write") || process.argv.includes("--generate");
const check = process.argv.includes("--check") || !write;

const receipts = [
  oracleReceipt("echo-skill", "receipt", stepReceipt("echo-skill", "echo", "hello from harness")),
  ...sequentialGraphOracle(),
  ...paymentApprovalGraphOracle(),
];

if (write) {
  mkdirSync(oracleDir, { recursive: true });
  for (const receipt of receipts) {
    writeFileSync(oraclePath(receipt), `${receipt.canonicalJson}\n`);
  }
}

if (check) {
  let failed = false;
  for (const receipt of receipts) {
    let expected = "";
    try {
      expected = readFileSync(oraclePath(receipt), "utf8");
    } catch {
      console.error(`missing harness oracle ${relative(oraclePath(receipt))}`);
      failed = true;
      continue;
    }
    if (expected !== `${receipt.canonicalJson}\n`) {
      console.error(`stale harness oracle ${relative(oraclePath(receipt))}`);
      failed = true;
    }
  }
  for (const fixture of ["echo-skill", "sequential-graph", "payment-approval-graph"]) {
    const receipt = receipts.find((candidate) => candidate.fixture === fixture && candidate.name === "receipt");
    if (!receipt) {
      throw new Error(`missing generated receipt for ${fixture}`);
    }
    const contents = readFileSync(path.join(repoRoot, `fixtures/harness/${fixture}.yaml`), "utf8");
    if (!contents.includes(`body_digest: ${receipt.bodyDigest}`)) {
      console.error(`stale body_digest in fixtures/harness/${fixture}.yaml`);
      failed = true;
    }
    if (!contents.includes(`receipt_digest: ${receipt.receiptDigest}`)) {
      console.error(`stale receipt_digest in fixtures/harness/${fixture}.yaml`);
      failed = true;
    }
  }
  if (failed) {
    process.exitCode = 1;
  }
}

if (!check) {
  for (const receipt of receipts.filter((candidate) => candidate.name === "receipt")) {
    console.log(`${receipt.fixture} body_digest=${receipt.bodyDigest}`);
    console.log(`${receipt.fixture} receipt_digest=${receipt.receiptDigest}`);
  }
}

function sequentialGraphOracle(): OracleReceipt[] {
  const first = stepReceipt("sequential-echo", "first", "hello from graph");
  const second = stepReceipt("sequential-echo", "second", "hello from graph");
  const graph = graphReceipt("sequential-echo", [first, second]);
  return [
    oracleReceipt("sequential-graph", "receipt", graph),
    oracleReceipt("sequential-graph", "first", first),
    oracleReceipt("sequential-graph", "second", second),
  ];
}

function paymentApprovalGraphOracle(): OracleReceipt[] {
  const approval = stepReceipt("x402-pay-approval", "approve-spend", approvalStdout());
  const fulfill = stepReceipt("x402-pay-approval", "fulfill", paymentRailStdout(), paymentRailRefs());
  const graph = graphReceipt("x402-pay-approval", [approval, fulfill]);
  return [
    oracleReceipt("payment-approval-graph", "receipt", graph),
    oracleReceipt("payment-approval-graph", "approve-spend", approval),
    oracleReceipt("payment-approval-graph", "fulfill", fulfill),
  ];
}

function oracleReceipt(fixture: string, name: string, receipt: Record<string, Json>): OracleReceipt {
  refreshProof(receipt);
  const canonicalJson = canonicalJsonStringify(receipt);
  return {
    fixture,
    name,
    bodyDigest: bodyDigest(receipt),
    receiptDigest: sha256Prefixed(canonicalJson),
    canonicalJson,
  };
}

function stepReceipt(
  graphName: string,
  stepId: string,
  stdout: string,
  refs: ActRefs = emptyRefs(),
): Record<string, Json> {
  const disposition = "closed";
  const act = observationAct(stepId, stdout, disposition, refs);
  const receiptSeal = seal(disposition, "process_closed", `step ${stepId} completed`);
  return {
    schema: "runx.harness_receipt.v1",
    id: `hrn_rcpt_${graphName}_${stepId}`,
    created_at: createdAt,
    issuer: localIssuer(),
    signature: { alg: "Ed25519", value: "sig:pending" },
    harness: harness(graphName, stepId, "sealed", [act], [], receiptSeal),
    seal: receiptSeal,
  };
}

function graphReceipt(graphName: string, steps: Record<string, Json>[]): Record<string, Json> {
  const parentHarnessRef = reference("harness", `${graphName}_graph`);
  for (const step of steps) {
    const stepHarness = step.harness;
    if (stepHarness !== null && typeof stepHarness === "object" && !Array.isArray(stepHarness)) {
      stepHarness.parent_harness_ref = parentHarnessRef;
    }
    refreshProof(step);
  }
  const receiptSeal = seal("closed", "graph_closed", `graph ${graphName} completed`);
  return {
    schema: "runx.harness_receipt.v1",
    id: `hrn_rcpt_${graphName}`,
    created_at: createdAt,
    issuer: localIssuer(),
    signature: { alg: "Ed25519", value: "sig:pending" },
    harness: harness(
      graphName,
      "graph",
      "sealed",
      [],
      steps.map(childReceiptReference),
      receiptSeal,
    ),
    seal: receiptSeal,
  };
}

function childReceiptReference(step: Record<string, Json>): Json {
  const sealValue = step.seal;
  const locator =
    sealValue !== null && typeof sealValue === "object" && !Array.isArray(sealValue) ? sealValue.digest : null;
  return {
    type: "harness_receipt",
    uri: `runx:harness_receipt:${String(step.id)}`,
    locator: typeof locator === "string" ? locator : null,
  };
}

function harness(
  graphName: string,
  nodeId: string,
  state: string,
  acts: Json[],
  childRefs: Json[],
  receiptSeal: Json,
): Record<string, Json> {
  return {
    harness_id: `hrn_${graphName}_${nodeId}`,
    parent_harness_ref: null,
    state,
    host_ref: reference("host", "cli"),
    harness_ref: reference("harness", `${graphName}_${nodeId}`),
    authority: {
      actor_ref: reference("principal", "local_runtime"),
      authority_proof_refs: [],
      grant_refs: [],
      scope_refs: [],
      policy_refs: [],
      terms: [],
      attenuation: { parent_authority_ref: null, subset_proof: null },
    },
    enforcement: {
      version: "runtime-skeleton",
      enforcement_profile_hash: "sha256:runtime-skeleton-enforcement",
      sandbox: {
        profile: "process-boundary",
        cwd_policy: "skill-directory",
        network: "declared-by-skill",
        filesystem: "declared-by-skill",
      },
      redaction_refs: [],
    },
    idempotency: {
      intent_key: `sha256:${graphName}-${nodeId}-intent`,
      trigger_fingerprint: `sha256:${graphName}-${nodeId}-trigger`,
      content_hash: `sha256:${graphName}-${nodeId}-content`,
    },
    revision: { sequence: 1, previous_ref: null },
    signal_refs: [],
    decisions: decision(nodeId, acts),
    acts,
    child_harness_receipt_refs: childRefs,
    artifact_refs: [],
    seal: receiptSeal,
  };
}

function observationAct(
  stepId: string,
  stdout: string,
  disposition: string,
  refs: ActRefs,
): Record<string, Json> {
  const summary = "cli-tool exited successfully";
  return {
    act_id: `act_${stepId}`,
    form: "observation",
    intent: {
      purpose: `Run graph step ${stepId}`,
      legitimacy: "Runtime graph execution was admitted by the local harness",
      success_criteria: [{ criterion_id: "process_exit", statement: "cli-tool exits successfully", required: true }],
      constraints: [],
      derived_from: [],
    },
    summary: `Executed graph step ${stepId}`,
    closure: {
      disposition,
      reason_code: "process_exit",
      summary,
      closed_at: createdAt,
    },
    criterion_bindings: [
      {
        criterion_id: "process_exit",
        status: "verified",
        evidence_refs: refs.sourceRefs,
        verification_refs: refs.verificationRefs,
        summary,
      },
    ],
    source_refs: refs.sourceRefs,
    target_refs: [],
    surface_refs: refs.surfaceRefs,
    artifact_refs: refs.artifactRefs,
    verification_refs: refs.verificationRefs,
    harness_refs: [],
    performed_at: createdAt,
  };
}

function emptyRefs(): ActRefs {
  return {
    sourceRefs: [],
    surfaceRefs: [],
    artifactRefs: [],
    verificationRefs: [],
  };
}

function paymentRailRefs(): ActRefs {
  return {
    sourceRefs: [
      {
        type: "credential",
        uri: "credential:mock:x402-pay-approval-001",
        label: "scoped payment credential",
      },
    ],
    surfaceRefs: [],
    artifactRefs: [],
    verificationRefs: [
      {
        type: "verification",
        uri: "receipt-proof:mock:x402-pay-approval-001",
        locator: "payment:x402-pay-approval-001",
        label: "payment rail proof",
        proof_kind: "payment_rail",
      },
    ],
  };
}

function approvalStdout(): string {
  return JSON.stringify({
    payment_approval: {
      data: {
        actor: "human",
        approved: true,
        gate_id: "spend-approval",
        idempotency_key:
          "sha256:96d55a53b82f13d894e6cfc32bedccae12355a9c449c66c5026a9079c668f642",
        status: "approved",
      },
    },
  });
}

function paymentRailStdout(): string {
  return JSON.stringify({
    payment_rail_packet: {
      data: {
        rail_result: {
          status: "fulfilled",
          rail: "mock",
          amount_minor: 125,
          currency: "USD",
        },
        rail_proof: {
          proof_ref: "receipt-proof:mock:x402-pay-approval-001",
          idempotency_key: "payment:x402-pay-approval-001",
        },
        credential_envelope: {
          form: "paid_tool_credential",
          credential_ref: "credential:mock:x402-pay-approval-001",
        },
      },
    },
  });
}

function decision(nodeId: string, acts: Json[]): Json[] {
  const selectedAct = acts.find((act) => act !== null && typeof act === "object" && !Array.isArray(act));
  return [
    {
      decision_id: `dec_${nodeId}`,
      choice: "open",
      inputs: { signal_refs: [], target_ref: null, opportunity_refs: [], selection_ref: null },
      proposed_intent: {
        purpose: `Open runtime harness node ${nodeId}`,
        legitimacy: "Local graph execution requested this harness node",
        success_criteria: [],
        constraints: [],
        derived_from: [],
      },
      selected_act_id: selectedAct && "act_id" in selectedAct ? String(selectedAct.act_id) : null,
      selected_harness_ref: null,
      justification: { summary: "runtime graph planner selected this node", evidence_refs: [] },
      closure: null,
      artifact_refs: [],
    },
  ];
}

function seal(disposition: string, reasonCode: string, summary: string): Record<string, Json> {
  return {
    disposition,
    reason_code: reasonCode,
    summary,
    closed_at: createdAt,
    last_observed_at: createdAt,
    canonicalization: "runx.harness-receipt.c14n.v1",
    digest: "sha256:runtime-skeleton",
    criteria: [],
    verification_summary: {
      signature_valid: true,
      hash_commitments_valid: true,
      authority_attenuation_valid: false,
      criteria_bound: true,
      redaction_valid: true,
      external_attestations_present: false,
    },
    redaction_refs: [],
    artifact_refs: [],
    hash_commitments: [],
  };
}

function localIssuer(): Json {
  return { type: "local", kid: "runtime-skeleton", public_key_sha256: "sha256:runtime-skeleton-public" };
}

function reference(type: string, id: string): Json {
  return { type, uri: `runx:${type}:${id}` };
}

function refreshProof(receipt: Record<string, Json>): void {
  const digest = bodyDigest(receipt);
  (receipt.seal as Record<string, Json>).digest = digest;
  ((receipt.harness as Record<string, Json>).seal as Record<string, Json>).digest = digest;
  (receipt.signature as Record<string, Json>).value = `sig:${digest}`;
}

function bodyDigest(receipt: Record<string, Json>): string {
  return sha256Prefixed(canonicalJsonStringify(stripBodyProofFields(receipt, true)));
}

function stripBodyProofFields(value: Json, isRoot: boolean): Json {
  if (Array.isArray(value)) {
    return value.map((item) => stripBodyProofFields(item, false));
  }
  if (value !== null && typeof value === "object") {
    const output: Record<string, Json> = {};
    for (const [key, child] of Object.entries(value)) {
      if (isRoot && key === "signature") {
        continue;
      }
      if (key === "seal" && child !== null && typeof child === "object" && !Array.isArray(child)) {
        const seal: Record<string, Json> = {};
        for (const [sealKey, sealValue] of Object.entries(child)) {
          if (sealKey !== "digest" && sealKey !== "verification_summary") {
            seal[sealKey] = stripBodyProofFields(sealValue, false);
          }
        }
        output[key] = seal;
        continue;
      }
      output[key] = stripBodyProofFields(child, false);
    }
    return output;
  }
  return value;
}

function oraclePath(receipt: OracleReceipt): string {
  return path.join(oracleDir, `${receipt.fixture}.${receipt.name}.json`);
}

function relative(filePath: string): string {
  return path.relative(repoRoot, filePath);
}
