import { readFileSync } from "node:fs";

import { Value } from "@sinclair/typebox/value";
import { describe, expect, it } from "vitest";

import {
  operationalPolicySchema,
  operationalPolicySchemaVersion,
  admitOperationalPolicyRequest,
  lintOperationalPolicyContract,
  projectOperationalPolicyReadback,
  validateOperationalPolicyContract,
  validateOperationalPolicySemantics,
  type OperationalPolicyContract,
} from "./operational-policy.js";

const fixtureRoot = new URL("../../../../fixtures/operational-policy/", import.meta.url);

const validPolicy: OperationalPolicyContract = {
  schema: "runx.operational_policy.v1",
  schema_version: operationalPolicySchemaVersion,
  policy_id: "nitrosend-dev-flow",
  created_at: "2026-05-19T02:00:00Z",
  sources: [
    {
      source_id: "slack-bugs",
      provider: "slack",
      allowed_locators: ["slack://team/T123/channel/CBUGS"],
      allowed_actions: ["reply-only", "issue-intake", "issue-to-pr", "manual-review"],
      source_thread: {
        required: true,
        publish_mode: "reply",
        missing_behavior: "fail_closed",
      },
      minimum_confidence: 0.72,
    },
    {
      source_id: "sentry-production",
      provider: "sentry",
      allowed_locators: ["sentry://nitrosend/production"],
      allowed_actions: ["issue-intake", "issue-to-pr", "manual-review"],
      source_thread: {
        required: true,
        publish_mode: "reply",
        missing_behavior: "fail_closed",
      },
      sentry: {
        production_only: true,
        unresolved_only: true,
        regressed_only: true,
      },
    },
  ],
  runners: [
    {
      runner_id: "aster-primary",
      kind: "aster",
      state: "available",
      allowed_actions: ["issue-to-pr", "pr-review", "pr-fix-up", "merge-assist"],
      target_repos: ["nitrosend/api", "nitrosend/app"],
      scafld_required: true,
    },
  ],
  owner_routes: [
    {
      route_id: "api-kam",
      owners: ["Kam"],
      target_repos: ["nitrosend/api"],
      labels: ["runx", "api"],
      project: "Nitrosend Engineering",
    },
    {
      route_id: "app-chong",
      owners: ["Chong"],
      target_repos: ["nitrosend/app"],
      labels: ["runx", "app"],
    },
  ],
  targets: [
    {
      repo: "nitrosend/api",
      runner_ids: ["aster-primary"],
      allowed_actions: ["issue-to-pr", "pr-review", "pr-fix-up", "merge-assist"],
      default_owner_route: "api-kam",
      scafld_required: true,
      base_branch: "main",
    },
    {
      repo: "nitrosend/app",
      runner_ids: ["aster-primary"],
      allowed_actions: ["issue-to-pr", "pr-review", "pr-fix-up", "merge-assist"],
      default_owner_route: "app-chong",
      scafld_required: true,
      base_branch: "main",
    },
  ],
  dedupe: {
    strategy: "source_fingerprint",
    key_fields: ["source_locator", "fingerprint", "target_repo"],
    on_duplicate: "reuse",
  },
  outcomes: {
    observe_provider: true,
    verification_required: true,
    close_source_issue: "when_verified",
    publish_final_source_thread_update: true,
  },
  permissions: {
    auto_merge: false,
    mutate_target_repo: true,
    require_human_merge_gate: true,
  },
};

describe("operational-policy schema", () => {
  it("accepts a valid multi-source, multi-target policy", () => {
    expect(Value.Check(operationalPolicySchema, validPolicy)).toBe(true);
    expect(validateOperationalPolicyContract(validPolicy)).toMatchObject({
      policy_id: "nitrosend-dev-flow",
      permissions: {
        auto_merge: false,
        require_human_merge_gate: true,
      },
    });
    expect(lintOperationalPolicyContract(validPolicy)).toEqual([]);
    expect(validateOperationalPolicySemantics(validPolicy)).toMatchObject({
      policy_id: "nitrosend-dev-flow",
    });
  });

  it.each([
    "nitrosend-like.json",
    "minimal-single-repo.json",
  ])("accepts positive fixture %s", (fixtureName) => {
    const policy = readPolicyFixture(fixtureName);

    expect(validateOperationalPolicyContract(policy)).toMatchObject({
      schema: "runx.operational_policy.v1",
      schema_version: "runx.operational_policy.v1",
    });
    expect(lintOperationalPolicyContract(policy)).toEqual([]);
    expect(projectOperationalPolicyReadback(policy).valid).toBe(true);
  });

  it.each([
    ["invalid-unknown-runner.json", "unknown_runner"],
    ["invalid-owner-route-mismatch.json", "owner_route_target_mismatch"],
    ["invalid-source-thread-missing.json", "source_thread_required"],
    ["invalid-no-available-runner.json", "target_action_without_runner"],
    ["invalid-not-scafld-target.json", "mutation_without_scafld"],
  ])("reports stable semantic finding for %s", (fixtureName, code) => {
    const policy = readPolicyFixture(fixtureName);

    expect(lintOperationalPolicyContract(policy)).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ code }),
      ]),
    );
    expect(() => validateOperationalPolicySemantics(policy))
      .toThrow(new RegExp(code));
  });

  it.each([
    "invalid-schema-literal.json",
    "invalid-secret-field.json",
  ])("rejects schema-invalid fixture %s", (fixtureName) => {
    const policy = readPolicyFixture(fixtureName);

    expect(Value.Check(operationalPolicySchema, policy)).toBe(false);
    expect(() => validateOperationalPolicyContract(policy)).toThrow();
  });

  it("rejects policy that enables auto-merge", () => {
    expect(Value.Check(operationalPolicySchema, {
      ...validPolicy,
      permissions: {
        ...validPolicy.permissions,
        auto_merge: true,
      },
    })).toBe(false);
  });

  it("rejects source routes that can fall back when the source thread is missing", () => {
    expect(Value.Check(operationalPolicySchema, {
      ...validPolicy,
      sources: [{
        ...validPolicy.sources[0],
        source_thread: {
          ...validPolicy.sources[0].source_thread,
          missing_behavior: "post_to_root",
        },
      }],
    })).toBe(false);
  });

  it("rejects target repos that are not owner/repo slugs", () => {
    expect(Value.Check(operationalPolicySchema, {
      ...validPolicy,
      targets: [{
        ...validPolicy.targets[0],
        repo: "nitrosend",
      }],
    })).toBe(false);
  });

  it("rejects unknown source providers", () => {
    expect(Value.Check(operationalPolicySchema, {
      ...validPolicy,
      sources: [{
        ...validPolicy.sources[0],
        provider: "pager",
      }],
    })).toBe(false);
  });

  it("rejects extra fields so secrets do not drift into policy", () => {
    expect(Value.Check(operationalPolicySchema, {
      ...validPolicy,
      github_token: "ghp_123",
    })).toBe(false);
  });

  it("reports unknown target runners as semantic findings", () => {
    const findings = lintOperationalPolicyContract({
      ...validPolicy,
      targets: [{
        ...validPolicy.targets[0],
        runner_ids: ["missing-runner"],
      }],
    });

    expect(findings).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          code: "unknown_runner",
          path: "/targets/0/runner_ids/0",
        }),
      ]),
    );
  });

  it("reports owner routes that do not cover the target repo", () => {
    const findings = lintOperationalPolicyContract({
      ...validPolicy,
      targets: [{
        ...validPolicy.targets[0],
        default_owner_route: "app-chong",
      }],
    });

    expect(findings).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          code: "owner_route_target_mismatch",
          path: "/targets/0/default_owner_route",
        }),
      ]),
    );
  });

  it("reports target actions with no available runner support", () => {
    const findings = lintOperationalPolicyContract({
      ...validPolicy,
      runners: [{
        ...validPolicy.runners[0],
        state: "maintenance",
      }],
    });

    expect(findings).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          code: "target_action_without_runner",
          path: "/targets/0/allowed_actions",
        }),
      ]),
    );
  });

  it("throws a readable error for semantic validation failures", () => {
    expect(() => validateOperationalPolicySemantics({
      ...validPolicy,
      outcomes: {
        ...validPolicy.outcomes,
        verification_required: false,
      },
    })).toThrow(/close_without_verification/);
  });

  it("projects an admin-safe readback without raw source locators", () => {
    expect(projectOperationalPolicyReadback(validPolicy)).toMatchObject({
      policy_id: "nitrosend-dev-flow",
      valid: true,
      findings: [],
      sources: [
        {
          source_id: "slack-bugs",
          provider: "slack",
          locator_count: 1,
          source_thread_required: true,
          publish_mode: "reply",
        },
        {
          source_id: "sentry-production",
          provider: "sentry",
          locator_count: 1,
          source_thread_required: true,
          publish_mode: "reply",
        },
      ],
      targets: [
        {
          repo: "nitrosend/api",
          default_owner_route: "api-kam",
          owner_count: 1,
          available_runner_count: 1,
        },
        {
          repo: "nitrosend/app",
          default_owner_route: "app-chong",
          owner_count: 1,
          available_runner_count: 1,
        },
      ],
    });
    expect(JSON.stringify(projectOperationalPolicyReadback(validPolicy)))
      .not.toContain("slack://team/T123/channel/CBUGS");
  });

  it("admits a concrete request against target, source, runner, dedupe, and outcome policy", () => {
    expect(admitOperationalPolicyRequest(validPolicy, {
      source_id: "slack-bugs",
      target_repo: "nitrosend/api",
      action: "issue-to-pr",
      runner_id: "aster-primary",
      source_thread_locator: "slack://team/T123/channel/CBUGS/thread/168",
    })).toMatchObject({
      status: "allow",
      findings: [],
      policy_id: "nitrosend-dev-flow",
      source_id: "slack-bugs",
      target_repo: "nitrosend/api",
      runner_id: "aster-primary",
      owner_route_id: "api-kam",
      owners: ["Kam"],
      dedupe_strategy: "source_fingerprint",
      outcome_close_mode: "when_verified",
      source_thread_required: true,
      mutate_target_repo: true,
      require_human_merge_gate: true,
    });
  });

  it("denies request-time admission before unknown target or runner mutation boundaries", () => {
    const admission = admitOperationalPolicyRequest(validPolicy, {
      source_id: "slack-bugs",
      target_repo: "nitrosend/unknown",
      action: "issue-to-pr",
      runner_id: "missing-runner",
      source_thread_locator: "slack://team/T123/channel/CBUGS/thread/168",
    });

    expect(admission.status).toBe("deny");
    expect(admission.findings).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ code: "unknown_target_repo" }),
        expect.objectContaining({ code: "unknown_runner" }),
      ]),
    );
  });

  it("denies PR-producing admission without recoverable source-thread routing", () => {
    const admission = admitOperationalPolicyRequest(validPolicy, {
      source_id: "slack-bugs",
      target_repo: "nitrosend/api",
      action: "issue-to-pr",
      runner_id: "aster-primary",
    });

    expect(admission).toMatchObject({
      status: "deny",
      findings: expect.arrayContaining([
        expect.objectContaining({ code: "source_thread_locator_required" }),
      ]),
    });
  });

  it("denies maintenance-only runner requests even when the runner id exists", () => {
    const admission = admitOperationalPolicyRequest({
      ...validPolicy,
      runners: [{
        ...validPolicy.runners[0],
        state: "maintenance",
      }],
    }, {
      source_id: "slack-bugs",
      target_repo: "nitrosend/api",
      action: "issue-to-pr",
      runner_id: "aster-primary",
      source_thread_locator: "slack://team/T123/channel/CBUGS/thread/168",
    });

    expect(admission).toMatchObject({
      status: "deny",
      findings: expect.arrayContaining([
        expect.objectContaining({ code: "runner_unavailable" }),
      ]),
    });
  });
});

function readPolicyFixture(fixtureName: string): unknown {
  return JSON.parse(readFileSync(new URL(fixtureName, fixtureRoot), "utf8")) as unknown;
}
