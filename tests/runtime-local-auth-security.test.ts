import { spawnSync } from "node:child_process";
import { mkdtemp, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";

import { describe, expect, it } from "vitest";

import { createDefaultSkillAdapters } from "@runxhq/adapters";
import { runLocalSkill, type Caller } from "@runxhq/runtime-local";

const caller: Caller = {
  resolve: async () => undefined,
  report: () => undefined,
};
const workspaceRoot = process.cwd();
const cargo = process.platform === "win32" ? "cargo.exe" : "cargo";
const runxBinary = path.join(
  workspaceRoot,
  "crates",
  "target",
  "debug",
  process.platform === "win32" ? "runx.exe" : "runx",
);
let rustRunxBuilt = false;

describe("local runtime auth security", () => {
  it("fails closed when resolved credential material does not match the admitted grant", async () => {
    const tempDir = await mkdtemp(path.join(os.tmpdir(), "runx-auth-binding-"));
    const skillDir = path.join(tempDir, "credential-boundary");
    const receiptDir = path.join(tempDir, "receipts");

    try {
      await mkdir(skillDir, { recursive: true });
      await writeFile(
        path.join(skillDir, "SKILL.md"),
        `---
name: credential-boundary
auth:
  type: nango
  provider: github
  scopes:
    - repo:read
  scope_family: github_repo
  authority_kind: read_only
  target_repo: runxhq/aster
  target_locator: runxhq/aster#issue/4
source:
  type: cli-tool
  command: node
  args:
    - -e
    - "process.stdout.write('executed')"
---

Exercises connected credential binding.
`,
      );

      const result = await runLocalSkill({
        skillPath: skillDir,
        caller,
        receiptDir,
        runxHome: path.join(tempDir, "home"),
        env: process.env,
        adapters: createDefaultSkillAdapters(),
        authResolver: {
          resolveGrants: async () => ({
            grants: [
              {
                grant_id: "grant_expected",
                provider: "github",
                scopes: ["repo:read"],
                status: "active",
                scope_family: "github_repo",
                authority_kind: "read_only",
                target_repo: "runxhq/aster",
                target_locator: "runxhq/aster#issue/4",
              },
            ],
          }),
          resolveCredential: async () => ({
            credential: {
              kind: "runx.credential-envelope.v1",
              grant_id: "grant_other",
              provider: "github",
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
          }),
        },
      });

      expect(result.status).toBe("policy_denied");
      if (result.status !== "policy_denied") {
        return;
      }

      expect(result.reasons).toEqual([
        "credential grant_id 'grant_other' does not match admitted grant 'grant_expected'",
        "credential grant_reference.grant_id does not match admitted grant",
      ]);
      expect(result.receipt?.metadata).toMatchObject({
        authority_proof: {
          requested: {
            connected_auth: true,
            scopes: ["repo:read"],
            mutating: false,
            scope_family: "github_repo",
            authority_kind: "read_only",
            target_repo: "runxhq/aster",
            target_locator: "runxhq/aster#issue/4",
          },
          scope_admission: {
            status: "allow",
            grant_id: "grant_expected",
          },
          credential_material: {
            status: "resolved",
            grant_id: "grant_other",
            material_ref_hash: expect.any(String),
          },
        },
      });

      const receiptContents = await readFile(path.join(receiptDir, `${result.receipt?.id}.json`), "utf8");
      expect(receiptContents).not.toContain("executed");
      expect(receiptContents).not.toContain("nango:github:conn_1");
    } finally {
      await rm(tempDir, { recursive: true, force: true });
    }
  });

  it("passes only the admitted grant into credential resolution", async () => {
    const tempDir = await mkdtemp(path.join(os.tmpdir(), "runx-admitted-grant-"));
    const skillDir = path.join(tempDir, "credential-boundary");
    const seenGrantIds: string[][] = [];

    try {
      await mkdir(skillDir, { recursive: true });
      await writeFile(
        path.join(skillDir, "SKILL.md"),
        `---
name: admitted-grant-only
auth:
  type: nango
  provider: github
  scopes:
    - repo:read
  scope_family: github_repo
  authority_kind: read_only
  target_repo: runxhq/aster
  target_locator: runxhq/aster#issue/4
source:
  type: cli-tool
  command: node
  args:
    - -e
    - "process.stdout.write('ok')"
---

Exercises admitted grant narrowing.
`,
      );

      const result = await runLocalSkill({
        skillPath: skillDir,
        caller,
        receiptDir: path.join(tempDir, "receipts"),
        runxHome: path.join(tempDir, "home"),
        env: process.env,
        adapters: createDefaultSkillAdapters(),
        authResolver: {
          resolveGrants: async () => ({
            grants: [
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
              {
                grant_id: "grant_expected",
                provider: "github",
                scopes: ["repo:read"],
                status: "active",
                scope_family: "github_repo",
                authority_kind: "read_only",
                target_repo: "runxhq/aster",
                target_locator: "runxhq/aster#issue/4",
              },
            ],
          }),
          resolveCredential: async ({ grants }) => {
            seenGrantIds.push(grants.map((grant) => grant.grant_id));
            const grant = grants[0];
            return grant
              ? {
                credential: {
                  kind: "runx.credential-envelope.v1",
                  grant_id: grant.grant_id,
                  provider: grant.provider,
                  connection_id: "conn_1",
                  scopes: grant.scopes,
                  grant_reference: {
                    grant_id: grant.grant_id,
                    scope_family: "github_repo",
                    authority_kind: "read_only",
                    target_repo: "runxhq/aster",
                    target_locator: "runxhq/aster#issue/4",
                  },
                  material_ref: "nango:github:conn_1",
                },
              }
              : undefined;
          },
        },
      });

      expect(result.status).toBe("success");
      expect(seenGrantIds).toEqual([["grant_expected"]]);
    } finally {
      await rm(tempDir, { recursive: true, force: true });
    }
  });

  it("denies admitted connected auth when no credential envelope is resolved", async () => {
    const tempDir = await mkdtemp(path.join(os.tmpdir(), "runx-missing-credential-"));
    const skillDir = path.join(tempDir, "credential-boundary");

    try {
      await mkdir(skillDir, { recursive: true });
      await writeFile(
        path.join(skillDir, "SKILL.md"),
        `---
name: missing-credential
auth:
  type: nango
  provider: github
  scopes:
    - repo:read
source:
  type: cli-tool
  command: node
  args:
    - -e
    - "process.stdout.write('should-not-run')"
---

Exercises missing credential denial.
`,
      );

      const result = await runLocalSkill({
        skillPath: skillDir,
        caller,
        receiptDir: path.join(tempDir, "receipts"),
        runxHome: path.join(tempDir, "home"),
        env: process.env,
        adapters: createDefaultSkillAdapters(),
        authResolver: {
          resolveGrants: async () => ({
            grants: [
              {
                grant_id: "grant_1",
                provider: "github",
                scopes: ["repo:read"],
                status: "active",
              },
            ],
          }),
          resolveCredential: async () => undefined,
        },
      });

      expect(result.status).toBe("policy_denied");
      if (result.status !== "policy_denied") {
        return;
      }
      expect(result.reasons).toEqual(["credential material was not resolved for admitted connected auth grant"]);
      expect(result.receipt?.metadata).toMatchObject({
        authority_proof: {
          scope_admission: {
            status: "allow",
            grant_id: "grant_1",
          },
          credential_material: {
            status: "not_resolved",
            grant_id: "grant_1",
          },
        },
      });

      const receiptContents = await readFile(path.join(tempDir, "receipts", `${result.receipt?.id}.json`), "utf8");
      expect(receiptContents).not.toContain("should-not-run");
    } finally {
      await rm(tempDir, { recursive: true, force: true });
    }
  });

  it("returns signed graph denial receipts when a graph skill source is policy denied", async () => {
    const tempDir = await mkdtemp(path.join(os.tmpdir(), "runx-graph-source-denial-"));
    const wrapperDir = path.join(tempDir, "graph-wrapper");
    const childDir = path.join(wrapperDir, "child");
    const receiptDir = path.join(tempDir, "receipts");

    try {
      await mkdir(childDir, { recursive: true });
      await writeFile(
        path.join(childDir, "SKILL.md"),
        `---
name: child-mutator
source:
  type: cli-tool
  command: node
  args:
    - -e
    - "process.stdout.write('should-not-run')"
---

This child should not execute when retry policy is denied.
`,
      );
      await writeFile(
        path.join(wrapperDir, "SKILL.md"),
        `---
name: graph-denial-wrapper
source:
  type: graph
  graph:
    name: graph-denial-wrapper
    steps:
      - id: deploy
        skill: ./child
        mutation: true
        retry:
          max_attempts: 2
---

Graph wrapper used to verify policy denial receipt propagation.
`,
      );
      ensureRustRunxBinary();

      const result = await runLocalSkill({
        skillPath: wrapperDir,
        caller,
        receiptDir,
        runxHome: path.join(tempDir, "home"),
        env: kernelEnv(),
        adapters: createDefaultSkillAdapters(),
      });

      expect(result.status).toBe("policy_denied");
      if (result.status !== "policy_denied") {
        return;
      }
      expect(result.reasons).toEqual(["step 'deploy' declares mutating retry without an idempotency key"]);
      expect(result.receipt).toMatchObject({
        schema: "runx.harness_receipt.v1",
        metadata: {
          authority_proof: {
            run_id: expect.any(String),
            skill_name: "child-mutator",
            source_type: "cli-tool",
            requested: {
              connected_auth: false,
              scopes: [],
              mutating: true,
            },
            credential_material: {
              status: "not_requested",
            },
          },
        },
      });
      const receiptContents = await readFile(path.join(receiptDir, `${result.receipt?.id}.json`), "utf8");
      const receipt = JSON.parse(receiptContents) as { readonly metadata?: Readonly<Record<string, unknown>> };
      expect(receipt).toMatchObject({ schema: "runx.harness_receipt.v1" });
      expect(graphStepStatusesFromReceipt(receipt)).toEqual([["deploy", "failure"]]);
      expect(receiptContents).not.toContain("should-not-run");
    } finally {
      await rm(tempDir, { recursive: true, force: true });
    }
  });

  it("stamps authority proof metadata on top-level graph skill receipts", async () => {
    const tempDir = await mkdtemp(path.join(os.tmpdir(), "runx-graph-source-authority-"));
    const wrapperDir = path.join(tempDir, "graph-wrapper");
    const childDir = path.join(wrapperDir, "child");

    try {
      await mkdir(childDir, { recursive: true });
      await writeFile(
        path.join(childDir, "SKILL.md"),
        `---
name: child-reader
source:
  type: cli-tool
  command: node
  args:
    - -e
    - "process.stdout.write('ok')"
---

This child executes successfully.
`,
      );
      await writeFile(
        path.join(wrapperDir, "SKILL.md"),
        `---
name: graph-success-wrapper
source:
  type: graph
  graph:
    name: graph-success-wrapper
    steps:
      - id: read
        skill: ./child
---

Graph wrapper used to verify top-level graph authority proof metadata.
`,
      );

      const result = await runLocalSkill({
        skillPath: wrapperDir,
        caller,
        receiptDir: path.join(tempDir, "receipts"),
        runxHome: path.join(tempDir, "home"),
        env: process.env,
        adapters: createDefaultSkillAdapters(),
      });

      expect(result.status).toBe("success");
      if (result.status !== "success") {
        return;
      }
      expect(result.receipt).toMatchObject({
        schema: "runx.harness_receipt.v1",
        metadata: {
          authority_proof: {
            run_id: result.receipt.id,
            skill_name: "graph-success-wrapper",
            source_type: "graph",
            requested: {
              connected_auth: false,
              scopes: [],
              mutating: false,
            },
            credential_material: {
              status: "not_requested",
            },
          },
        },
      });
    } finally {
      await rm(tempDir, { recursive: true, force: true });
    }
  });
});

function graphStepStatusesFromReceipt(receipt: { readonly metadata?: Readonly<Record<string, unknown>> }): Array<[string, string]> {
  const runx = receipt.metadata?.runx as { readonly steps?: unknown } | undefined;
  expect(Array.isArray(runx?.steps)).toBe(true);
  return (runx?.steps as Array<{ readonly step_id?: unknown; readonly status?: unknown }>).map((step) => [
    String(step.step_id),
    String(step.status),
  ]);
}

function kernelEnv(): NodeJS.ProcessEnv {
  return {
    ...process.env,
    RUNX_KERNEL_EVAL_BIN: runxBinary,
  };
}

function ensureRustRunxBinary(): void {
  if (rustRunxBuilt) {
    return;
  }
  const result = spawnSync(
    cargo,
    ["build", "--quiet", "--manifest-path", "crates/Cargo.toml", "-p", "runx-cli", "--bin", "runx"],
    {
      cwd: workspaceRoot,
      encoding: "utf8",
      env: process.env,
      maxBuffer: 8 * 1024 * 1024,
    },
  );

  expect(result.status, result.stderr || result.stdout).toBe(0);
  rustRunxBuilt = true;
}
