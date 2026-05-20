import { describe, expect, it } from "vitest";

import {
  RUNX_CONTROL_SCHEMA_REFS,
  validateScopeAdmissionContract,
  type ScopeAdmissionContract,
} from "@runxhq/contracts";

function validateScopeAdmission(value: unknown): ScopeAdmissionContract {
  const admission = validateScopeAdmissionContract(value);
  return {
    status: admission.status,
    requested_scopes: admission.requested_scopes,
    granted_scopes: admission.granted_scopes,
    grant_id: admission.grant_id,
    reasons: admission.reasons,
    decision_summary: admission.decision_summary,
  };
}

function validateGovernance(value: { readonly scope_admission?: unknown }): {
  readonly scope_admission?: ScopeAdmissionContract;
} {
  return {
    scope_admission: value.scope_admission === undefined
      ? undefined
      : validateScopeAdmission(value.scope_admission),
  };
}

describe("receipt governance schema contracts", () => {
  it("exposes the published scope admission schema ref", () => {
    expect(RUNX_CONTROL_SCHEMA_REFS.scope_admission).toBe("https://runx.ai/spec/scope-admission.schema.json");
  });

  it("accepts the canonical scope admission shape", () => {
    expect(validateScopeAdmission({
      status: "allow",
      requested_scopes: ["repo:status"],
      granted_scopes: ["repo:*"],
      grant_id: "grant_1",
      reasons: ["bounded prerelease scope"],
      decision_summary: "Allowed by the parent grant.",
    })).toEqual({
      status: "allow",
      requested_scopes: ["repo:status"],
      granted_scopes: ["repo:*"],
      grant_id: "grant_1",
      reasons: ["bounded prerelease scope"],
      decision_summary: "Allowed by the parent grant.",
    });
  });

  it("normalizes governance wrappers around scope admission", () => {
    expect(validateGovernance({
      scope_admission: {
        status: "deny",
        requested_scopes: ["deployments:write"],
        granted_scopes: [],
        reasons: ["missing grant"],
      },
    })).toEqual({
      scope_admission: {
        status: "deny",
        requested_scopes: ["deployments:write"],
        granted_scopes: [],
        reasons: ["missing grant"],
        grant_id: undefined,
        decision_summary: undefined,
      },
    });
  });

  it("rejects invalid scope admission statuses", () => {
    expect(() => validateScopeAdmission({
      status: "pending",
      requested_scopes: ["repo:status"],
      granted_scopes: ["repo:*"],
    } as never)).toThrow(/scope-admission\.schema\.json/);
  });
});
