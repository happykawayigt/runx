import { mkdtemp, mkdir, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";

import { describe, expect, it } from "vitest";

import { createDefaultSkillAdapters } from "@runxhq/adapters";
import {
  isRegistryRef,
  parseRegistryRef,
  runLocalGraph,
  type Caller,
  type RegistrySkillVersion as GraphRegistrySkillVersion,
  type RegistryStore as GraphRegistryStore,
} from "@runxhq/runtime-local";
import {
  createFileRegistryStore,
  seedRegistrySkill,
  type RegistrySkillVersion as FixtureRegistrySkillVersion,
  type RegistryStore as FixtureRegistryStore,
} from "./registry-fixtures.js";

const caller: Caller = {
  resolve: async () => undefined,
  report: () => undefined,
};
const adapters = createDefaultSkillAdapters();

const ECHO_MARKDOWN = `---
name: echo
description: Minimal echo skill for registry-resolution fixtures.
---

Echo a message.
`;

const ECHO_PROFILE = `skill: echo
runners:
  echo:
    default: true
    type: cli-tool
    command: node
    args:
      - -e
      - "process.stdout.write(process.env.RUNX_INPUT_MESSAGE || '')"
    inputs:
      message:
        type: string
        required: true
`;

describe("graph registry refs", () => {
  describe("isRegistryRef", () => {
    it("accepts owner/name and owner/name@version", () => {
      expect(isRegistryRef("runx/echo")).toBe(true);
      expect(isRegistryRef("runx/echo@0.1.0")).toBe(true);
      expect(isRegistryRef("aster/skill-lab@2025-04-20")).toBe(true);
    });

    it("rejects filesystem paths", () => {
      expect(isRegistryRef("./scafld")).toBe(false);
      expect(isRegistryRef("../scafld")).toBe(false);
      expect(isRegistryRef("../../skills/echo")).toBe(false);
      expect(isRegistryRef("/abs/skills/echo")).toBe(false);
    });

    it("rejects bare names without an owner", () => {
      expect(isRegistryRef("echo")).toBe(false);
      expect(isRegistryRef("")).toBe(false);
    });
  });

  describe("parseRegistryRef", () => {
    it("splits owner and name", () => {
      expect(parseRegistryRef("runx/echo")).toEqual({
        kind: "registry",
        skillId: "runx/echo",
        owner: "runx",
        name: "echo",
        version: undefined,
        raw: "runx/echo",
      });
    });

    it("captures the version when present", () => {
      expect(parseRegistryRef("runx/echo@1.2.3")).toEqual({
        kind: "registry",
        skillId: "runx/echo",
        owner: "runx",
        name: "echo",
        version: "1.2.3",
        raw: "runx/echo@1.2.3",
      });
    });

    it("throws on bad input", () => {
      expect(() => parseRegistryRef("./local/path")).toThrow();
      expect(() => parseRegistryRef("not-a-ref")).toThrow();
    });
  });

  it("resolves a graph step skill via the registry store", async () => {
    const tempDir = await mkdtemp(path.join(os.tmpdir(), "runx-graph-registry-"));

    try {
      const store = createFileRegistryStore(path.join(tempDir, "registry"));
      await seedRegistrySkill(store, ECHO_MARKDOWN, {
        owner: "testorg",
        version: "0.1.0",
        createdAt: "2026-04-20T00:00:00.000Z",
        profileDocument: ECHO_PROFILE,
      });

      const graphPath = path.join(tempDir, "graph.yaml");
      await writeFile(
        graphPath,
        `name: graph-registry-ref
steps:
  - id: echo
    skill: testorg/echo
    inputs:
      message: hello from registry
`,
      );

      const result = await runLocalGraph({
        graphPath,
        caller,
        receiptDir: path.join(tempDir, "receipts"),
        runxHome: path.join(tempDir, "home"),
        env: process.env,
        registryStore: store,
        skillCacheDir: path.join(tempDir, "skill-cache"),
        adapters,
      });

      expect(result.status).toBe("success");
      if (result.status !== "success") {
        return;
      }
      expect(result.steps[0]).toMatchObject({
        skill: "testorg/echo",
        stdout: "hello from registry",
      });
    } finally {
      await rm(tempDir, { recursive: true, force: true });
    }
  });

  it("resolves a pinned version from the registry", async () => {
    const tempDir = await mkdtemp(path.join(os.tmpdir(), "runx-graph-registry-pin-"));

    try {
      const store = createFileRegistryStore(path.join(tempDir, "registry"));
      await seedRegistrySkill(store, ECHO_MARKDOWN, {
        owner: "testorg",
        version: "0.1.0",
        createdAt: "2026-04-20T00:00:00.000Z",
        profileDocument: ECHO_PROFILE,
      });
      await seedRegistrySkill(store, ECHO_MARKDOWN, {
        owner: "testorg",
        version: "0.2.0",
        createdAt: "2026-04-21T00:00:00.000Z",
        profileDocument: ECHO_PROFILE,
      });

      const graphPath = path.join(tempDir, "graph.yaml");
      await writeFile(
        graphPath,
        `name: graph-registry-pinned
steps:
  - id: echo
    skill: testorg/echo@0.1.0
    inputs:
      message: pinned version
`,
      );

      const result = await runLocalGraph({
        graphPath,
        caller,
        receiptDir: path.join(tempDir, "receipts"),
        runxHome: path.join(tempDir, "home"),
        env: process.env,
        registryStore: store,
        skillCacheDir: path.join(tempDir, "skill-cache"),
        adapters,
      });

      expect(result.status).toBe("success");
      if (result.status !== "success") {
        return;
      }
      expect(result.steps[0]?.stdout).toBe("pinned version");
    } finally {
      await rm(tempDir, { recursive: true, force: true });
    }
  });

  it("fails with a clear message when no registry store is configured", async () => {
    const tempDir = await mkdtemp(path.join(os.tmpdir(), "runx-graph-registry-missing-"));

    try {
      const graphPath = path.join(tempDir, "graph.yaml");
      await writeFile(
        graphPath,
        `name: graph-registry-missing-store
steps:
  - id: echo
    skill: testorg/echo
    inputs:
      message: should fail
`,
      );

      await expect(
        runLocalGraph({
          graphPath,
          caller,
          receiptDir: path.join(tempDir, "receipts"),
          runxHome: path.join(tempDir, "home"),
          env: process.env,
          adapters,
        }),
      ).rejects.toThrow(/Registry ref 'testorg\/echo' used in graph step/);
    } finally {
      await rm(tempDir, { recursive: true, force: true });
    }
  });

  it("fails with a clear message when the skill is not in the registry", async () => {
    const tempDir = await mkdtemp(path.join(os.tmpdir(), "runx-graph-registry-notfound-"));

    try {
      const store = createFileRegistryStore(path.join(tempDir, "registry"));
      const graphPath = path.join(tempDir, "graph.yaml");
      await writeFile(
        graphPath,
        `name: graph-registry-missing-skill
steps:
  - id: echo
    skill: testorg/missing
    inputs:
      message: should fail
`,
      );

      await expect(
        runLocalGraph({
          graphPath,
          caller,
          receiptDir: path.join(tempDir, "receipts"),
          runxHome: path.join(tempDir, "home"),
          env: process.env,
          registryStore: store,
          skillCacheDir: path.join(tempDir, "skill-cache"),
          adapters,
        }),
      ).rejects.toThrow(/Registry skill 'testorg\/missing' not found in registry/);
    } finally {
      await rm(tempDir, { recursive: true, force: true });
    }
  });

  it("fails with available versions when a pinned version is missing", async () => {
    const tempDir = await mkdtemp(path.join(os.tmpdir(), "runx-graph-registry-badpin-"));

    try {
      const store = createFileRegistryStore(path.join(tempDir, "registry"));
      await seedRegistrySkill(store, ECHO_MARKDOWN, {
        owner: "testorg",
        version: "0.1.0",
        createdAt: "2026-04-20T00:00:00.000Z",
        profileDocument: ECHO_PROFILE,
      });

      const graphPath = path.join(tempDir, "graph.yaml");
      await writeFile(
        graphPath,
        `name: graph-registry-missing-pin
steps:
  - id: echo
    skill: testorg/echo@9.9.9
    inputs:
      message: should fail
`,
      );

      await expect(
        runLocalGraph({
          graphPath,
          caller,
          receiptDir: path.join(tempDir, "receipts"),
          runxHome: path.join(tempDir, "home"),
          env: process.env,
          registryStore: store,
          skillCacheDir: path.join(tempDir, "skill-cache"),
          adapters,
        }),
      ).rejects.toThrow(/Registry skill 'testorg\/echo@9\.9\.9' not found \(available: 0\.1\.0\)\./);
    } finally {
      await rm(tempDir, { recursive: true, force: true });
    }
  });

  it("fetches a graph step skill from a remote-backed registry store", async () => {
    const tempDir = await mkdtemp(path.join(os.tmpdir(), "runx-graph-registry-http-"));

    try {
      let fetches = 0;
      const fetchImpl: typeof fetch = async (input, init) => {
        fetches += 1;
        const url = typeof input === "string" ? input : input instanceof URL ? input.toString() : input.url;
        if (!url.includes("/v1/skills/testorg/echo/acquire") || init?.method !== "POST") {
          return new Response("bad request", { status: 400 });
        }
        return new Response(
          JSON.stringify({
            status: "success",
            install_count: 1,
            acquisition: {
              skill_id: "testorg/echo",
              owner: "testorg",
              name: "echo",
              version: "0.1.0",
              digest: "a".repeat(64),
              markdown: ECHO_MARKDOWN,
              profile_document: ECHO_PROFILE,
              profile_digest: "b".repeat(64),
              trust_tier: "community",
              publisher: {
                id: "testorg",
                kind: "publisher",
                handle: "testorg",
              },
              attestations: [
                {
                  kind: "publisher",
                  id: "publisher:testorg",
                  status: "declared",
                  summary: "testorg",
                },
              ],
              runner_names: ["echo"],
            },
          }),
          { status: 200, headers: { "content-type": "application/json" } },
        );
      };

      const cache = createFileRegistryStore(path.join(tempDir, "cache"));
      const store = new FixtureRemoteRegistryStore({
        remoteBaseUrl: "https://registry.example",
        installationId: "inst_test",
        cache,
        fetchImpl,
      });

      const graphPath = path.join(tempDir, "graph.yaml");
      await writeFile(
        graphPath,
        `name: graph-registry-http
steps:
  - id: echo
    skill: testorg/echo@0.1.0
    inputs:
      message: hello from http
`,
      );

      const result = await runLocalGraph({
        graphPath,
        caller,
        receiptDir: path.join(tempDir, "receipts"),
        runxHome: path.join(tempDir, "home"),
        env: process.env,
        registryStore: store,
        skillCacheDir: path.join(tempDir, "skill-cache"),
        adapters,
      });

      expect(result.status).toBe("success");
      if (result.status !== "success") {
        return;
      }
      expect(result.steps[0]?.stdout).toBe("hello from http");
      expect(fetches).toBe(1);

      const second = await runLocalGraph({
        graphPath,
        caller,
        receiptDir: path.join(tempDir, "receipts-2"),
        runxHome: path.join(tempDir, "home-2"),
        env: process.env,
        registryStore: store,
        skillCacheDir: path.join(tempDir, "skill-cache"),
        adapters,
      });
      expect(second.status).toBe("success");
      expect(fetches).toBe(1);
    } finally {
      await rm(tempDir, { recursive: true, force: true });
    }
  });

  it("accepts filesystem-relative skill refs", async () => {
    const tempDir = await mkdtemp(path.join(os.tmpdir(), "runx-graph-registry-fs-"));

    try {
      const skillDir = path.join(tempDir, "skills", "echo");
      await mkdir(skillDir, { recursive: true });
      await writeFile(path.join(skillDir, "SKILL.md"), ECHO_MARKDOWN);
      await writeFile(path.join(skillDir, "X.yaml"), ECHO_PROFILE);

      const graphPath = path.join(tempDir, "graph.yaml");
      await writeFile(
        graphPath,
        `name: graph-registry-fs
steps:
  - id: echo
    skill: ./skills/echo
    inputs:
      message: filesystem still works
`,
      );

      const result = await runLocalGraph({
        graphPath,
        caller,
        receiptDir: path.join(tempDir, "receipts"),
        runxHome: path.join(tempDir, "home"),
        env: process.env,
        adapters,
      });

      expect(result.status).toBe("success");
      if (result.status !== "success") {
        return;
      }
      expect(result.steps[0]?.stdout).toBe("filesystem still works");
    } finally {
      await rm(tempDir, { recursive: true, force: true });
    }
  });
});

interface FixtureRemoteRegistryStoreOptions {
  readonly remoteBaseUrl: string;
  readonly installationId: string;
  readonly cache: FixtureRegistryStore;
  readonly fetchImpl: typeof fetch;
  readonly now?: () => Date;
}

class FixtureRemoteRegistryStore implements GraphRegistryStore {
  constructor(private readonly options: FixtureRemoteRegistryStoreOptions) {}

  async getVersion(skillId: string, version?: string): Promise<GraphRegistrySkillVersion | undefined> {
    const cached = await this.options.cache.getVersion(skillId, version);
    if (cached && version) {
      return cached;
    }

    const acquired = await this.acquire(skillId, version);
    if (!acquired) {
      return cached;
    }

    return await this.options.cache.putVersion(
      acquiredRegistrySkillToVersion(acquired, this.options.now?.() ?? new Date()),
      { upsert: true },
    );
  }

  async listVersions(skillId: string): Promise<readonly GraphRegistrySkillVersion[]> {
    return await this.options.cache.listVersions(skillId);
  }

  private async acquire(skillId: string, version?: string): Promise<FixtureAcquiredRegistrySkill | undefined> {
    const [owner, name] = splitRegistrySkillId(skillId);
    const response = await this.options.fetchImpl(
      `${this.options.remoteBaseUrl.replace(/\/$/, "")}/v1/skills/${encodeURIComponent(owner)}/${encodeURIComponent(name)}/acquire`,
      {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          installation_id: this.options.installationId,
          version,
          channel: "graph-fixture",
        }),
      },
    );
    if (response.status === 404) {
      return undefined;
    }
    if (!response.ok) {
      throw new Error(`Registry acquire failed for ${skillId}: HTTP ${response.status}`);
    }
    const payload = await response.json() as { readonly status?: string; readonly acquisition?: FixtureAcquiredRegistrySkill };
    if (payload.status !== "success" || !payload.acquisition) {
      throw new Error(`Registry acquire returned an invalid payload for ${skillId}.`);
    }
    return payload.acquisition;
  }
}

interface FixtureAcquiredRegistrySkill {
  readonly skill_id: string;
  readonly owner: string;
  readonly name: string;
  readonly version: string;
  readonly digest: string;
  readonly markdown: string;
  readonly profile_document?: string;
  readonly profile_digest?: string;
  readonly runner_names: readonly string[];
  readonly trust_tier: "first_party" | "verified" | "community";
  readonly publisher: FixtureRegistrySkillVersion["publisher"];
  readonly attestations: FixtureRegistrySkillVersion["attestations"];
}

function acquiredRegistrySkillToVersion(
  acquired: FixtureAcquiredRegistrySkill,
  now: Date,
): FixtureRegistrySkillVersion {
  const timestamp = now.toISOString();
  return {
    skill_id: acquired.skill_id,
    owner: acquired.owner,
    name: acquired.name,
    version: acquired.version,
    digest: acquired.digest,
    markdown: acquired.markdown,
    profile_document: acquired.profile_document,
    profile_digest: acquired.profile_digest,
    runner_names: acquired.runner_names,
    source_type: "runx-registry",
    trust_tier: acquired.trust_tier,
    attestations: acquired.attestations,
    required_scopes: [],
    tags: [],
    publisher: acquired.publisher,
    created_at: timestamp,
    updated_at: timestamp,
  };
}

function splitRegistrySkillId(skillId: string): readonly [string, string] {
  const parts = skillId.split("/");
  if (parts.length !== 2 || !parts[0] || !parts[1]) {
    throw new Error(`Invalid registry skill id '${skillId}'. Expected '<owner>/<name>'.`);
  }
  return [parts[0], parts[1]];
}
