import { describe, expect, it } from "vitest";

import type {
  ActReceiptEnvelope,
  AdapterActInvocation,
  CredentialEnvelope,
  SkillAdapter,
} from "./adapter-types.js";
import { executeSkill } from "./execute-skill.js";
import type { ValidatedSkill } from "../parser-types.js";

describe("runtime-local executeSkill", () => {
  it("dispatches to the adapter matching the skill source", async () => {
    let received: AdapterActInvocation | undefined;
    const adapter: SkillAdapter = {
      type: "agent",
      invoke: async (request) => {
        received = request;
        return successReceipt("ok");
      },
    };

    const credential = canonicalCredential();
    const result = await executeSkill({
      skill: fixtureSkill(),
      inputs: { topic: "dispatch" },
      resolvedInputs: { topic: "dispatch" },
      skillDirectory: "/tmp/skill",
      adapters: [
        {
          type: "process",
          invoke: async () => successReceipt("wrong"),
        },
        adapter,
      ],
      env: { RUNX_TEST: "1" },
      credential,
      runId: "rx_1",
      stepId: "step_1",
    });

    expect(result).toEqual(successReceipt("ok"));
    expect(received).toMatchObject({
      skillName: "Example skill",
      skillBody: "Run the example.",
      allowedTools: ["tool.alpha"],
      inputs: { topic: "dispatch" },
      resolvedInputs: { topic: "dispatch" },
      skillDirectory: "/tmp/skill",
      env: { RUNX_TEST: "1" },
      runId: "rx_1",
      stepId: "step_1",
    });
    expect(received?.source).toEqual(fixtureSkill().source);
    expect(received?.credential).toEqual({
      ...credential,
      grant_reference: {
        ...credential.grant_reference,
        target_locator: undefined,
      },
    });
  });

  it("returns a failure receipt when no adapter matches", async () => {
    await expect(executeSkill({
      skill: fixtureSkill("unknown"),
      inputs: {},
      skillDirectory: "/tmp/skill",
      adapters: [],
    })).resolves.toMatchObject({
      status: "failure",
      exitCode: null,
      signal: null,
      durationMs: 0,
      errorMessage: "No adapter registered for source type 'unknown'.",
    });
  });

  it("validates credentials before invoking an adapter", async () => {
    let invoked = false;
    const adapter: SkillAdapter = {
      type: "agent",
      invoke: async () => {
        invoked = true;
        return successReceipt("unexpected");
      },
    };

    await expect(executeSkill({
      skill: fixtureSkill(),
      inputs: {},
      skillDirectory: "/tmp/skill",
      adapters: [adapter],
      credential: {
        ...canonicalCredential(),
        kind: "github",
      } as unknown as CredentialEnvelope,
    })).rejects.toThrow(/credential-envelope\.schema\.json/);
    expect(invoked).toBe(false);
  });
});

function fixtureSkill(sourceType = "agent"): ValidatedSkill {
  return {
    name: "Example skill",
    body: "Run the example.",
    source: {
      type: sourceType,
      args: [],
      raw: { type: sourceType },
    },
    inputs: {},
    allowedTools: ["tool.alpha"],
    raw: {
      frontmatter: {},
      rawFrontmatter: "",
      body: "Run the example.",
    },
  };
}

function canonicalCredential(): CredentialEnvelope {
  return {
    kind: "runx.credential-envelope.v1",
    grant_id: "grant_1",
    provider: "github",
    auth_mode: "oauth",
    material_kind: "opaque_connection",
    provider_reference: "conn_1",
    scopes: ["repo:read"],
    grant_reference: {
      grant_id: "grant_1",
      scope_family: "github_repo",
      authority_kind: "read_only",
      target_repo: "runxhq/aster",
    },
    material_ref: "opaque:github:conn_1",
  };
}

function successReceipt(stdout: string): ActReceiptEnvelope {
  return {
    status: "sealed",
    stdout,
    stderr: "",
    exitCode: 0,
    signal: null,
    durationMs: 1,
  };
}
