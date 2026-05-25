import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import path from "node:path";

import { describe, expect, it } from "vitest";
import {
  admitGraphStepScopesViaKernel,
  admitRetryPolicyViaKernel,
  authorityProofMetadataViaKernel,
  credentialBindingViaKernel,
  createSingleStepStateViaKernel,
  createSequentialGraphStateViaKernel,
  evaluateFanoutSyncViaKernel,
  fanoutSyncDecisionKeyViaKernel,
  localScopeAdmissionViaKernel,
  localSkillAdmissionViaKernel,
  planSequentialGraphTransitionViaKernel,
  transitionSequentialGraphViaKernel,
  transitionSingleStepViaKernel,
} from "./kernel-bridge.js";
import { resolveRunxBinary } from "../../../../tests/runx-binary.js";

const workspaceRoot = process.cwd();
const runxBinary = resolveRunxBinary();

describe("Rust kernel CLI JSON bridge", () => {
  it("evaluates a policy fixture through process JSON", () => {
    assertKernelFixture("fixtures/kernel/policy/retry-admission-denies-mutating-without-key.json");
  }, 20_000);

  it("evaluates a state-machine fixture through process JSON", () => {
    assertKernelFixture("fixtures/kernel/state-machine/sequential-plan-first-step.json");
  }, 20_000);

  it("creates sequential graph state through the Rust kernel", async () => {
    await expect(createSequentialGraphStateViaKernel(
      "gx_bridge",
      [{ id: "first" }, { id: "second", contextFrom: ["first"] }],
      { command: runxBinary, cwd: workspaceRoot },
    )).resolves.toEqual({
      graphId: "gx_bridge",
      status: "pending",
      steps: [
        { stepId: "first", status: "pending", attempts: 0 },
        { stepId: "second", status: "pending", attempts: 0 },
      ],
    });
  });

  it("transitions single step state through the Rust kernel", async () => {
    let state = await createSingleStepStateViaKernel("echo", { command: runxBinary, cwd: workspaceRoot });
    state = await transitionSingleStepViaKernel(state, { type: "admit" }, { command: runxBinary, cwd: workspaceRoot });
    state = await transitionSingleStepViaKernel(
      state,
      { type: "start", at: "2026-05-22T00:00:00Z" },
      { command: runxBinary, cwd: workspaceRoot },
    );
    expect(state).toEqual({
      stepId: "echo",
      status: "running",
      startedAt: "2026-05-22T00:00:00Z",
    });
    await expect(transitionSingleStepViaKernel(
      state,
      {
        type: "succeed",
        at: "2026-05-22T00:00:01Z",
        admissionWitness: { stepId: "echo", receiptId: "rx_echo" },
      },
      { command: runxBinary, cwd: workspaceRoot },
    )).resolves.toEqual({
      stepId: "echo",
      status: "succeeded",
      startedAt: "2026-05-22T00:00:00Z",
      completedAt: "2026-05-22T00:00:01Z",
    });
  });

  it("plans and transitions sequential graph state through the Rust kernel", async () => {
    const steps = [{ id: "first" }, { id: "second", contextFrom: ["first"] }];
    const state = await createSequentialGraphStateViaKernel("gx_bridge", steps, { command: runxBinary, cwd: workspaceRoot });
    await expect(planSequentialGraphTransitionViaKernel(
      state,
      steps,
      {},
      {},
      { command: runxBinary, cwd: workspaceRoot },
    )).resolves.toEqual({
      type: "run_step",
      stepId: "first",
      attempt: 1,
      contextFrom: [],
    });

    const running = await transitionSequentialGraphViaKernel(
      state,
      { type: "start_step", stepId: "first", at: "2026-05-22T00:00:00Z" },
      { command: runxBinary, cwd: workspaceRoot },
    );
    expect(running).toMatchObject({
      graphId: "gx_bridge",
      status: "running",
      steps: [
        { stepId: "first", status: "running", attempts: 1 },
        { stepId: "second", status: "pending", attempts: 0 },
      ],
    });
    await expect(transitionSequentialGraphViaKernel(
      running,
      {
        type: "step_succeeded",
        stepId: "first",
        at: "2026-05-22T00:00:01Z",
        receiptId: "rx_first",
        admissionWitness: { stepId: "first", receiptId: "rx_first" },
      },
      { command: runxBinary, cwd: workspaceRoot },
    )).resolves.toMatchObject({
      graphId: "gx_bridge",
      status: "running",
      steps: [
        { stepId: "first", status: "succeeded", attempts: 1, receiptId: "rx_first" },
        { stepId: "second", status: "pending", attempts: 0 },
      ],
    });
  });

  it("evaluates fanout sync and decision keys through the Rust kernel", async () => {
    const decision = await evaluateFanoutSyncViaKernel(
      {
        groupId: "branches",
        strategy: "all",
        onBranchFailure: "halt",
      },
      [
        { stepId: "left", status: "succeeded" },
        { stepId: "right", status: "failed" },
      ],
      {},
      { command: runxBinary, cwd: workspaceRoot },
    );
    expect(decision).toMatchObject({
      groupId: "branches",
      decision: "halt",
      ruleFired: "branch_failure.halt",
      branchCount: 2,
      successCount: 1,
      failureCount: 1,
    });
    await expect(fanoutSyncDecisionKeyViaKernel(decision, {
      command: runxBinary,
      cwd: workspaceRoot,
    })).resolves.toBe("branches:branch_failure.halt");
  });

  it("uses the Rust kernel for requested retry admission", async () => {
    await expect(admitRetryPolicyViaKernel({
      stepId: "deploy",
      retry: { maxAttempts: 2 },
      mutating: true,
    }, { command: runxBinary, cwd: workspaceRoot })).resolves.toEqual({
      status: "deny",
      reasons: ["step 'deploy' declares mutating retry without an idempotency key"],
    });
  });

  it("does not require a Rust process when retry policy is not requested", async () => {
    await expect(admitRetryPolicyViaKernel({
      stepId: "noop",
      mutating: true,
    }, { command: "missing-runx-kernel-test-binary" })).resolves.toEqual({
      status: "allow",
      reasons: ["retry policy not requested"],
    });
  });

  it("uses the Rust kernel for requested graph scope admission", async () => {
    await expect(admitGraphStepScopesViaKernel({
      stepId: "echo",
      requestedScopes: ["repo:read"],
      grant: {
        grant_id: "grant_repo",
        scopes: ["repo:*"],
      },
    }, { command: runxBinary, cwd: workspaceRoot })).resolves.toEqual({
      status: "allow",
      reasons: ["graph step scopes allowed"],
      stepId: "echo",
      requestedScopes: ["repo:read"],
      grantedScopes: ["repo:*"],
      grantId: "grant_repo",
    });
  });

  it("uses the Rust kernel for denied graph scope admission", async () => {
    await expect(admitGraphStepScopesViaKernel({
      stepId: "deploy",
      requestedScopes: ["deployments:write"],
      grant: {
        grant_id: "grant_checks",
        scopes: ["checks:read"],
      },
    }, { command: runxBinary, cwd: workspaceRoot })).resolves.toEqual({
      status: "deny",
      reasons: ["step 'deploy' requested scope(s) outside graph grant: deployments:write"],
      stepId: "deploy",
      requestedScopes: ["deployments:write"],
      grantedScopes: ["checks:read"],
      grantId: "grant_checks",
    });
  });

  it("uses the Rust kernel for local skill admission", async () => {
    await expect(localSkillAdmissionViaKernel({
      name: "echo",
      source: {
        type: "cli-tool",
        timeoutSeconds: 10,
      },
    }, {}, { command: runxBinary, cwd: workspaceRoot })).resolves.toEqual({
      status: "allow",
      reasons: ["local admission allowed"],
    });
  });

  it("uses the Rust kernel for local scope admission", async () => {
    await expect(localScopeAdmissionViaKernel({
      type: "connected",
      provider: "github",
      scopes: ["repo:read", "repo:read"],
    }, [{
      grant_id: "grant_repo",
      provider: "github",
      scopes: ["repo:*"],
      status: "active",
      expires_at: "2026-05-23T00:00:00Z",
    }], {
      connectedAuthCheckedAt: "2026-05-22T00:00:00Z",
    }, { command: runxBinary, cwd: workspaceRoot })).resolves.toEqual({
      status: "allow",
      requested_scopes: ["repo:read"],
      granted_scopes: ["repo:*"],
      grant_id: "grant_repo",
      decision_summary: "matching active grant admitted",
    });
  });

  it("uses the Rust kernel for credential binding", async () => {
    await expect(credentialBindingViaKernel({
      auth: {
        type: "connected",
        provider: "github",
        scopes: ["repo:read"],
      },
      grants: [{
        grant_id: "grant_repo",
        provider: "github",
        scopes: ["repo:*"],
        status: "active",
      }],
      scopeAdmission: {
        status: "allow",
        requested_scopes: ["repo:read"],
        granted_scopes: ["repo:*"],
        grant_id: "grant_repo",
        decision_summary: "matching active grant admitted",
      },
      credential: {
        kind: "runx.credential-envelope.v1",
        grant_id: "grant_repo",
        provider: "github",
        auth_mode: "oauth",
        material_kind: "opaque_connection",
        provider_reference: "conn_1",
        scopes: ["repo:read"],
        material_ref: "opaque:github:conn_1",
      },
    }, { command: runxBinary, cwd: workspaceRoot })).resolves.toEqual({
      status: "allow",
      reasons: ["credential material matches admitted grant"],
    });
  });

  it("uses the Rust kernel for authority proof metadata", async () => {
    await expect(authorityProofMetadataViaKernel({
      runId: "run_graph",
      skillName: "governed-echo",
      sourceType: "cli-tool",
      auth: undefined,
      grants: [],
      scopeAdmission: {
        status: "allow",
        requested_scopes: ["repo:read"],
        granted_scopes: ["repo:*"],
        grant_id: "grant_repo",
        decision_summary: "graph step scope admission allowed",
      },
      mutating: true,
    }, { command: runxBinary, cwd: workspaceRoot })).resolves.toMatchObject({
      authority_proof: {
        run_id: "run_graph",
        skill_name: "governed-echo",
        source_type: "cli-tool",
        requested: {
          connected_auth: false,
          scopes: [],
          mutating: true,
        },
        scope_admission: {
          status: "allow",
          requested_scopes: ["repo:read"],
          granted_scopes: ["repo:*"],
          grant_id: "grant_repo",
        },
        credential_material: {
          status: "not_requested",
        },
      },
    });
  });

  it("does not require a Rust process when graph step requests no scopes", async () => {
    await expect(admitGraphStepScopesViaKernel({
      stepId: "noop",
      requestedScopes: [],
      grant: {
        grant_id: "local-default",
        scopes: ["*", "*"],
      },
    }, { command: "missing-runx-kernel-test-binary" })).resolves.toEqual({
      status: "allow",
      reasons: ["graph step requested no scopes"],
      stepId: "noop",
      requestedScopes: [],
      grantedScopes: ["*"],
      grantId: "local-default",
    });
  });

  it("requires an explicit Rust kernel command for requested graph scope admission", async () => {
    await expect(admitGraphStepScopesViaKernel({
      stepId: "deploy",
      requestedScopes: ["deployments:write"],
      grant: { scopes: ["repo:read"] },
    }, { env: { RUNX_KERNEL_EVAL_BIN: "" } })).rejects.toThrow("RUNX_KERNEL_EVAL_BIN");
  });

  it("rejects malformed graph scope admission output", async () => {
    await expect(admitGraphStepScopesViaKernel({
      stepId: "deploy",
      requestedScopes: ["deployments:write"],
      grant: { scopes: ["repo:read"] },
    }, {
      command: process.execPath,
      argsPrefix: [
        "-e",
        "process.stdin.resume(); process.stdout.write(JSON.stringify({status:'success',result:{kind:'output',value:{status:'allow',reasons:[]}}}));",
      ],
      cwd: workspaceRoot,
    })).rejects.toThrow("graph scope admission stepId");
  });

  it("rejects malformed sequential graph state output", async () => {
    await expect(createSequentialGraphStateViaKernel("gx_bad", [{ id: "first" }], {
      command: process.execPath,
      argsPrefix: [
        "-e",
        "process.stdin.resume(); process.stdout.write(JSON.stringify({status:'success',result:{kind:'output',value:{graphId:'gx_bad',status:'pending',steps:[{stepId:'first',status:'pending'}]}}}));",
      ],
      cwd: workspaceRoot,
    })).rejects.toThrow("sequential graph step state attempts");
  });

  it("requires an explicit Rust kernel command for requested retry admission", async () => {
    await expect(admitRetryPolicyViaKernel({
      stepId: "deploy",
      retry: { maxAttempts: 2 },
      mutating: true,
    }, { env: { RUNX_KERNEL_EVAL_BIN: "" } })).rejects.toThrow("RUNX_KERNEL_EVAL_BIN");
  });

  it("rejects malformed authority proof metadata output", async () => {
    await expect(authorityProofMetadataViaKernel({
      skillName: "deploy",
      sourceType: "cli-tool",
      mutating: true,
    }, {
      command: process.execPath,
      argsPrefix: [
        "-e",
        "process.stdin.resume(); process.stdout.write(JSON.stringify({status:'success',result:{kind:'output',value:{not_authority_proof:{}}}}));",
      ],
      cwd: workspaceRoot,
    })).rejects.toThrow("authority_proof");
  });

  it("rejects malformed Rust kernel envelopes", async () => {
    await expect(admitRetryPolicyViaKernel({
      stepId: "deploy",
      retry: { maxAttempts: 2 },
      mutating: true,
    }, {
      command: process.execPath,
      argsPrefix: [
        "-e",
        "process.stdin.resume(); process.stdout.write(JSON.stringify({status:'success',result:{kind:'not_output',value:null}}));",
      ],
      cwd: workspaceRoot,
    })).rejects.toThrow("invalid success envelope");
  });
});

function assertKernelFixture(relativeFixturePath: string): void {
  const fixture = readJson(path.join(workspaceRoot, relativeFixturePath));
  const result = runKernelEval(["--input", relativeFixturePath, "--json"]);

  expect(result.status).toBe(0);
  expect(result.stderr).toBe("");
  expect(JSON.parse(result.stdout)).toEqual({
    status: "success",
    result: fixture.expected,
  });
}

function runKernelEval(args: readonly string[]): { status: number | null; stdout: string; stderr: string } {
  const invocation = rustCliInvocation(["kernel", "eval", ...args]);
  const result = spawnSync(invocation.command, invocation.args, {
    cwd: workspaceRoot,
    encoding: "utf8",
    env: {
      ...process.env,
      NO_COLOR: "1",
      RUNX_CWD: workspaceRoot,
      RUNX_RUST_CLI: "1",
    },
    maxBuffer: 8 * 1024 * 1024,
  });

  return {
    status: result.status,
    stdout: result.stdout,
    stderr: result.stderr,
  };
}

function rustCliInvocation(args: readonly string[]): { command: string; args: string[] } {
  return { command: runxBinary, args: [...args] };
}

function readJson(filePath: string): { expected: unknown } {
  return JSON.parse(readFileSync(filePath, "utf8")) as { expected: unknown };
}
