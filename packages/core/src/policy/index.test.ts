import { describe, expect, it } from "vitest";

import {
  admitGraphStepScopes,
  admitLocalSkill,
  admitRetryPolicy,
  admitSandbox,
  buildAuthorityProofMetadata,
  buildLocalScopeAdmission,
  evaluatePublicCommentOpportunity,
  evaluatePublicPullRequestCandidate,
  sandboxRequiresApproval,
  validateCredentialBinding,
} from "./index.js";

describe("admitLocalSkill", () => {
  it("allows local cli-tool skills", () => {
    expect(admitLocalSkill({ name: "echo", source: { type: "cli-tool", timeoutSeconds: 10 } }).status).toBe("allow");
  });

  it("denies inline cli-tool eval when strict workspace policy is enabled", () => {
    const decision = admitLocalSkill(
      {
        name: "inline-node",
        source: {
          type: "cli-tool",
          command: "node",
          args: ["-e", "process.stdout.write('hi')"],
        },
      },
      {
        executionPolicy: {
          strictCliToolInlineCode: true,
        },
      },
    );

    expect(decision).toEqual({
      status: "deny",
      reasons: [
        "cli-tool source 'node' uses inline code via '-e', which is rejected by strict workspace policy; move the program into a checked-in script and invoke that file instead",
      ],
    });
  });

  it("allows checked-in cli-tool scripts when strict workspace policy is enabled", () => {
    expect(
      admitLocalSkill(
        {
          name: "file-node",
          source: {
            type: "cli-tool",
            command: "node",
            args: ["./run.mjs"],
          },
        },
        {
          executionPolicy: {
            strictCliToolInlineCode: true,
          },
        },
      ).status,
    ).toBe("allow");
  });

  it("catches shell and python inline code wrappers in strict workspace policy", () => {
    const shellDecision = admitLocalSkill(
      {
        name: "inline-shell",
        source: {
          type: "cli-tool",
          command: "bash",
          args: ["-lc", "echo hi"],
        },
      },
      {
        executionPolicy: {
          strictCliToolInlineCode: true,
        },
      },
    );
    const pythonDecision = admitLocalSkill(
      {
        name: "inline-python",
        source: {
          type: "cli-tool",
          command: "/usr/bin/env",
          args: ["python3", "-c", "print('hi')"],
        },
      },
      {
        executionPolicy: {
          strictCliToolInlineCode: true,
        },
      },
    );

    expect(shellDecision.status).toBe("deny");
    expect(shellDecision.reasons[0]).toContain("'bash'");
    expect(shellDecision.reasons[0]).toContain("'-lc'");
    expect(pythonDecision.status).toBe("deny");
    expect(pythonDecision.reasons[0]).toContain("'python3'");
    expect(pythonDecision.reasons[0]).toContain("'-c'");
  });

  it("allows standard skills through the agent runner by default", () => {
    expect(admitLocalSkill({ name: "standard", source: { type: "agent" } }).status).toBe("allow");
  });

  it("denies unsupported source types", () => {
    const decision = admitLocalSkill({ name: "unsupported", source: { type: "unsupported" } });

    expect(decision.status).toBe("deny");
  });

  it("allows local a2a skills", () => {
    expect(admitLocalSkill({ name: "a2a", source: { type: "a2a", timeoutSeconds: 10 } }).status).toBe("allow");
  });

  it("allows local mcp skills", () => {
    expect(admitLocalSkill({ name: "mcp", source: { type: "mcp", timeoutSeconds: 10 } }).status).toBe("allow");
  });

  it("denies connected auth in local offline execution", () => {
    const decision = admitLocalSkill({
      name: "connected",
      source: { type: "cli-tool" },
      auth: { type: "nango" },
    });

    expect(decision.status).toBe("deny");
  });

  it("allows connected auth when a matching active grant is provided", () => {
    const decision = admitLocalSkill(
      {
        name: "connected",
        source: { type: "cli-tool" },
        auth: { type: "nango", provider: "github", scopes: ["repo:read"] },
      },
      {
        connectedGrants: [
          {
            grant_id: "grant_1",
            provider: "github",
            scopes: ["repo:read", "user:read"],
            status: "active",
          },
        ],
      },
    );

    expect(decision.status).toBe("allow");
  });

  it("allows connected auth when a wildcard grant covers the requested scope", () => {
    const skill = {
      name: "connected",
      source: { type: "cli-tool" },
      auth: { type: "nango", provider: "github", scopes: ["repo:read"] },
    } as const;
    const grants = [
      {
        grant_id: "grant_wildcard",
        provider: "github",
        scopes: ["repo:*"],
        status: "active" as const,
      },
    ];

    expect(admitLocalSkill(skill, { connectedGrants: grants }).status).toBe("allow");
    expect(buildLocalScopeAdmission(skill.auth, grants)).toEqual({
      status: "allow",
      requested_scopes: ["repo:read"],
      granted_scopes: ["repo:*"],
      grant_id: "grant_wildcard",
      decision_summary: "matching active grant admitted",
    });
    expect(validateCredentialBinding({
      auth: skill.auth,
      grants,
      scopeAdmission: buildLocalScopeAdmission(skill.auth, grants),
      credential: {
        kind: "runx.credential-envelope.v1",
        grant_id: "grant_wildcard",
        provider: "github",
        auth_mode: "oauth",
        material_kind: "nango_connection",
        connection_id: "conn_1",
        scopes: ["repo:*"],
        material_ref: "nango:github:conn_1",
      },
    }).status).toBe("allow");
  });

  it("does not allow universal wildcard connected auth grants by default", () => {
    const decision = admitLocalSkill(
      {
        name: "connected",
        source: { type: "cli-tool" },
        auth: { type: "nango", provider: "github", scopes: ["repo:read"] },
      },
      {
        connectedGrants: [
          {
            grant_id: "grant_wildcard",
            provider: "github",
            scopes: ["*"],
            status: "active",
          },
        ],
      },
    );

    expect(decision).toEqual({
      status: "deny",
      reasons: ["connected auth grant required for provider 'github'"],
    });
  });

  it("requires targeted connected auth grants to match the requested reference", () => {
    const skill = {
      name: "targeted-connected",
      source: { type: "cli-tool" },
      auth: {
        type: "nango",
        provider: "github",
        scopes: ["repo:read"],
        scope_family: "github_repo",
        authority_kind: "read_only",
        target_repo: "runxhq/aster",
        target_locator: "runxhq/aster#issue/4",
      },
    } as const;

    expect(
      admitLocalSkill(skill, {
        connectedGrants: [
          {
            grant_id: "grant_other_issue",
            provider: "github",
            scopes: ["repo:read"],
            status: "active",
            scope_family: "github_repo",
            authority_kind: "read_only",
            target_repo: "runxhq/aster",
            target_locator: "runxhq/aster#issue/5",
          },
        ],
      }).status,
    ).toBe("deny");
    expect(
      admitLocalSkill(skill, {
        connectedGrants: [
          {
            grant_id: "grant_issue_4",
            provider: "github",
            scopes: ["repo:read"],
            status: "active",
            scope_family: "github_repo",
            authority_kind: "read_only",
            target_repo: "runxhq/aster",
            target_locator: "runxhq/aster#issue/4",
          },
        ],
      }).status,
    ).toBe("allow");
  });

  it("does not satisfy untargeted connected auth with a targeted grant", () => {
    const decision = admitLocalSkill(
      {
        name: "untargeted-connected",
        source: { type: "cli-tool" },
        auth: { type: "nango", provider: "github", scopes: ["repo:read"] },
      },
      {
        connectedGrants: [
          {
            grant_id: "grant_targeted",
            provider: "github",
            scopes: ["repo:read"],
            status: "active",
            scope_family: "github_repo",
            authority_kind: "read_only",
            target_repo: "runxhq/aster",
          },
        ],
      },
    );

    expect(decision.status).toBe("deny");
  });

  it("denies readonly sandbox declarations with writable paths", () => {
    const decision = admitLocalSkill({
      name: "readonly-write",
      source: {
        type: "cli-tool",
        sandbox: {
          profile: "readonly",
          writablePaths: ["out.txt"],
        },
      },
    });

    expect(decision).toEqual({
      status: "deny",
      reasons: ["readonly sandbox cannot declare writable paths"],
    });
  });

  it("allows workspace-write sandbox declarations with safe declared paths", () => {
    const decision = admitSandbox({
      profile: "workspace-write",
      writablePaths: ["{{output_path}}"],
      envAllowlist: ["PATH"],
    });

    expect(decision.status).toBe("allow");
  });

  it("requires approval for unrestricted local development sandbox", () => {
    expect(sandboxRequiresApproval({ profile: "unrestricted-local-dev" })).toBe(true);
    expect(admitSandbox({ profile: "unrestricted-local-dev" }).status).toBe("approval_required");
    expect(admitSandbox({ profile: "unrestricted-local-dev" }, { approvedEscalation: true }).status).toBe("allow");
  });
});

describe("admitRetryPolicy", () => {
  it("allows bounded read-only retries", () => {
    expect(
      admitRetryPolicy({
        stepId: "read",
        retry: { maxAttempts: 2 },
        mutating: false,
      }),
    ).toEqual({
      status: "allow",
      reasons: ["retry policy allowed"],
    });
  });

  it("denies mutating retries without idempotency keys", () => {
    expect(
      admitRetryPolicy({
        stepId: "deploy",
        retry: { maxAttempts: 2 },
        mutating: true,
      }),
    ).toEqual({
      status: "deny",
      reasons: ["step 'deploy' declares mutating retry without an idempotency key"],
    });
  });

  it("allows mutating retries with an idempotency key", () => {
    expect(
      admitRetryPolicy({
        stepId: "deploy",
        retry: { maxAttempts: 2 },
        mutating: true,
        idempotencyKey: "deploy-123",
      }).status,
    ).toBe("allow");
  });
});

describe("admitGraphStepScopes", () => {
  it("allows exact grant matches", () => {
    expect(
      admitGraphStepScopes({
        stepId: "read",
        requestedScopes: ["repo:read"],
        grant: { grant_id: "grant_1", scopes: ["repo:read"] },
      }),
    ).toMatchObject({
      status: "allow",
      requestedScopes: ["repo:read"],
      grantedScopes: ["repo:read"],
      grantId: "grant_1",
    });
  });

  it("allows narrowed scopes from wildcard grants", () => {
    expect(
      admitGraphStepScopes({
        stepId: "checks",
        requestedScopes: ["checks:read"],
        grant: { scopes: ["checks:*", "repo:read"] },
      }).status,
    ).toBe("allow");
  });

  it("denies nested scopes under prefix wildcard grants", () => {
    expect(
      admitGraphStepScopes({
        stepId: "admin",
        requestedScopes: ["repo:admin:keys"],
        grant: { scopes: ["repo:*"] },
      }),
    ).toMatchObject({
      status: "deny",
      reasons: ["step 'admin' requested scope(s) outside graph grant: repo:admin:keys"],
    });
  });

  it("allows empty step scopes", () => {
    expect(
      admitGraphStepScopes({
        stepId: "no-scope",
        requestedScopes: [],
        grant: { scopes: [] },
      }),
    ).toMatchObject({
      status: "allow",
      reasons: ["graph step requested no scopes"],
    });
  });

  it("denies scopes outside the graph grant", () => {
    expect(
      admitGraphStepScopes({
        stepId: "deploy",
        requestedScopes: ["deployments:write"],
        grant: { grant_id: "grant_1", scopes: ["checks:read"] },
      }),
    ).toMatchObject({
      status: "deny",
      reasons: ["step 'deploy' requested scope(s) outside graph grant: deployments:write"],
      requestedScopes: ["deployments:write"],
      grantedScopes: ["checks:read"],
    });
  });

  it("deduplicates requested scopes before admission", () => {
    expect(
      admitGraphStepScopes({
        stepId: "read",
        requestedScopes: ["repo:read", "repo:read"],
        grant: { scopes: ["*"] },
      }).requestedScopes,
    ).toEqual(["repo:read"]);
  });
});

describe("authority proof", () => {
  it("summarizes matching grants and opaque credential material without raw secrets", () => {
    const scopeAdmission = buildLocalScopeAdmission(
      {
        type: "nango",
        provider: "github",
        scopes: ["repo:read"],
      },
      [
        {
          grant_id: "grant_1",
          provider: "github",
          scopes: ["repo:read", "user:read"],
          status: "active",
        },
      ],
    );

    expect(scopeAdmission).toEqual({
      status: "allow",
      requested_scopes: ["repo:read"],
      granted_scopes: ["repo:read", "user:read"],
      grant_id: "grant_1",
      decision_summary: "matching active grant admitted",
    });

    const metadata = buildAuthorityProofMetadata({
      runId: "rx_abc",
      skillName: "connected-review",
      sourceType: "agent-step",
      auth: {
        type: "nango",
        provider: "github",
        scopes: ["repo:read"],
        scope_family: "github_repo",
        authority_kind: "read_only",
        target_repo: "runxhq/aster",
        target_locator: "runxhq/aster#issue/4",
      },
      scopeAdmission,
      credential: {
        kind: "runx.credential-envelope.v1",
        grant_id: "grant_1",
        provider: "github",
        auth_mode: "oauth",
        material_kind: "nango_connection",
        connection_id: "conn_1",
        scopes: ["repo:read"],
        material_ref: "nango:github:conn_1",
      },
    });

    expect(metadata).toMatchObject({
      authority_proof: {
        schema_version: "runx.authority-proof.v1",
        requested: {
          connected_auth: true,
          scopes: ["repo:read"],
          scope_family: "github_repo",
          authority_kind: "read_only",
          target_repo: "runxhq/aster",
          target_locator: "runxhq/aster#issue/4",
        },
        credential_material: {
          status: "resolved",
          material_ref_hash: expect.any(String),
        },
        redaction: {
          secret_material: "omitted",
          stdout: "hashed",
          stderr: "hashed",
        },
      },
    });
    expect(JSON.stringify(metadata)).not.toContain("super-secret-token");
    expect(JSON.stringify(metadata)).not.toContain("sk-contract-test");
    expect(JSON.stringify(metadata)).not.toContain("nango:github:conn_1");
  });

  it("records denied connected auth without resolving credential material", () => {
    const metadata = buildAuthorityProofMetadata({
      skillName: "connected-review",
      sourceType: "agent-step",
      auth: {
        type: "nango",
        provider: "github",
        scopes: ["repo:write"],
        scope_family: "github_repo",
        authority_kind: "constructive",
        target_repo: "runxhq/aster",
        target_locator: "runxhq/aster#issue/4",
      },
      grants: [],
    });

    expect(metadata).toMatchObject({
      authority_proof: {
        scope_admission: {
          status: "deny",
          requested_scopes: ["repo:write"],
          granted_scopes: [],
        },
        credential_material: {
          status: "denied",
          provider: "github",
          scopes: ["repo:write"],
          scope_family: "github_repo",
          authority_kind: "constructive",
          target_repo: "runxhq/aster",
          target_locator: "runxhq/aster#issue/4",
        },
      },
    });
    expect(JSON.stringify(metadata)).not.toContain("material_ref");
  });

  it("summarizes sandbox enforcement rather than copying ambient environment", () => {
    const metadata = buildAuthorityProofMetadata({
      skillName: "write-file",
      sourceType: "cli-tool",
      mutating: true,
      sandboxDeclaration: {
        profile: "workspace-write",
        cwdPolicy: "workspace",
        network: false,
        requireEnforcement: true,
      },
      sandboxMetadata: {
        profile: "workspace-write",
        cwd: "/private/workspace",
        workspace_root: "/private/workspace",
        env: { mode: "allowlist", allowlist: ["PATH", "HOME"] },
        network: { declared: false, enforcement: "isolated-namespace" },
        filesystem: {
          enforcement: "bubblewrap-mount-namespace",
          readonly_paths: true,
          writable_paths_enforced: true,
          private_tmp: true,
        },
        runtime: { enforcer: "bubblewrap" },
        approval: { required: false, approved: false },
      },
    });

    expect(metadata).toMatchObject({
      authority_proof: {
        requested: {
          mutating: true,
          sandbox_profile: "workspace-write",
        },
        sandbox: {
          profile: "workspace-write",
          cwd_policy: "workspace",
          require_enforcement: true,
          network: {
            declared: false,
            enforcement: "isolated-namespace",
          },
          filesystem: {
            enforcement: "bubblewrap-mount-namespace",
            writable_paths_enforced: true,
          },
          runtime: {
            enforcer: "bubblewrap",
          },
        },
      },
    });
    expect(JSON.stringify(metadata)).not.toContain("/private/workspace");
    expect(JSON.stringify(metadata)).not.toContain("allowlist");
  });

  it("denies credential envelopes that do not bind to the admitted grant", () => {
    const auth = {
      type: "nango",
      provider: "github",
      scopes: ["repo:read"],
      scope_family: "github_repo",
      authority_kind: "read_only",
      target_repo: "runxhq/aster",
      target_locator: "runxhq/aster#issue/4",
    };
    const grants = [
      {
        grant_id: "grant_expected",
        provider: "github",
        scopes: ["repo:read"],
        status: "active" as const,
        scope_family: "github_repo",
        authority_kind: "read_only" as const,
        target_repo: "runxhq/aster",
        target_locator: "runxhq/aster#issue/4",
      },
    ];
    const scopeAdmission = buildLocalScopeAdmission(auth, grants);

    expect(validateCredentialBinding({
      auth,
      grants,
      scopeAdmission,
      credential: {
        kind: "runx.credential-envelope.v1",
        grant_id: "grant_other",
        provider: "github",
        auth_mode: "oauth",
        material_kind: "nango_connection",
        connection_id: "conn_1",
        scopes: ["repo:read"],
        grant_reference: {
          grant_id: "grant_other",
          scope_family: "github_repo",
          authority_kind: "read_only",
          target_repo: "runxhq/aster",
          target_locator: "runxhq/aster#issue/4",
        },
        material_ref: "nango:github:conn_1",
      },
    })).toEqual({
      status: "deny",
      reasons: [
        "credential grant_id 'grant_other' does not match admitted grant 'grant_expected'",
        "credential grant_reference.grant_id does not match admitted grant",
      ],
    });
  });

  it("denies admitted connected auth when credential material is missing", () => {
    const auth = {
      type: "nango",
      provider: "github",
      scopes: ["repo:read"],
    };
    const grants = [
      {
        grant_id: "grant_1",
        provider: "github",
        scopes: ["repo:read"],
        status: "active" as const,
      },
    ];
    const scopeAdmission = buildLocalScopeAdmission(auth, grants);

    expect(validateCredentialBinding({
      auth,
      grants,
      scopeAdmission,
    })).toEqual({
      status: "deny",
      reasons: ["credential material was not resolved for admitted connected auth grant"],
    });
  });
});

describe("public work policy", () => {
  it("blocks dependency churn and bots by default", () => {
    expect(
      evaluatePublicPullRequestCandidate({
        authorLogin: "dependabot[bot]",
        title: "Bump react from 19.0.0 to 19.0.1",
        labels: ["dependencies"],
        headRefName: "dependabot/npm_and_yarn/react-19.0.1",
      }),
    ).toEqual({
      blocked: true,
      reasons: ["bot_authored_pull_request", "dependency_update_pull_request", "internal_or_build_only_pull_request"],
    });
  });

  it("requires a welcome signal before issue-triage comments on cold external PRs", () => {
    expect(
      evaluatePublicCommentOpportunity({
        source: "github_pull_request",
        lane: "issue-triage",
        authorLogin: "stranger",
        authorAssociation: "NONE",
        title: "Clarify docs wording",
        labels: [],
        headRefName: "docs/fix-wording",
        commentsCount: 0,
        reviewCommentsCount: 0,
      }),
    ).toMatchObject({
      blocked: true,
      reasons: ["comment_without_welcome_signal"],
      welcome_signal: false,
    });
  });

  it("respects operator-supplied trust recovery statuses", () => {
    expect(
      evaluatePublicCommentOpportunity(
        {
          source: "github_pull_request",
          lane: "issue-triage",
          authorLogin: "maintainer",
          authorAssociation: "CONTRIBUTOR",
          title: "Improve onboarding docs",
          labels: [],
          headRefName: "docs/onboarding",
          commentsCount: 1,
          reviewCommentsCount: 0,
          recentOutcomes: [{ status: "cooldown" }],
        },
        {
          trust_recovery_statuses: ["cooldown"],
        },
      ),
    ).toMatchObject({
      blocked: true,
      reasons: ["comment_lane_in_trust_recovery"],
    });
  });
});
