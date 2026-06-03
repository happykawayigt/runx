import { mkdtemp, rm } from "node:fs/promises";
import os from "node:os";
import path from "node:path";

import { describe, expect, it } from "vitest";

import { createFileRegistryStore, type RegistrySkillVersion, type RegistryStore } from "./registry-fixtures.js";

const ECHO_MARKDOWN = `---
name: echo
description: Echo skill for HTTP cached store tests.
---

Echo a message.
`;

const ECHO_PROFILE = `skill: echo
runners:
  echo:
    default: true
    type: cli-tool
    command: node
`;

function buildAcquirePayload(overrides: {
  readonly skillId?: string;
  readonly owner?: string;
  readonly name?: string;
  readonly version?: string;
  readonly digest?: string;
} = {}) {
  return {
    status: "success",
    install_count: 1,
    acquisition: {
      skill_id: overrides.skillId ?? "acme/echo",
      owner: overrides.owner ?? "acme",
      name: overrides.name ?? "echo",
      version: overrides.version ?? "0.1.0",
      digest: overrides.digest ?? "a".repeat(64),
      markdown: ECHO_MARKDOWN,
      profile_document: ECHO_PROFILE,
      profile_digest: "b".repeat(64),
      trust_tier: "community",
      publisher: {
        id: overrides.owner ?? "acme",
        kind: "publisher",
        handle: overrides.owner ?? "acme",
      },
      attestations: [
        {
          kind: "publisher",
          id: `publisher:${overrides.owner ?? "acme"}`,
          status: "declared",
          summary: overrides.owner ?? "acme",
        },
      ],
      runner_names: ["echo"],
    },
  };
}

function jsonResponse(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" },
  });
}

describe("HttpCachedRegistryStore", () => {
  it("fetches a missing skill over HTTP and caches it locally", async () => {
    const tempDir = await mkdtemp(path.join(os.tmpdir(), "runx-http-cache-"));
    try {
      const cache = createFileRegistryStore(path.join(tempDir, "cache"));
      let fetches = 0;
      const fetchImpl: typeof fetch = async (input, init) => {
        fetches += 1;
        const url = typeof input === "string" ? input : input instanceof URL ? input.toString() : input.url;
        expect(url).toContain("/v1/skills/acme/echo/acquire");
        expect(init?.method).toBe("POST");
        return jsonResponse(buildAcquirePayload());
      };
      const store = new HttpCachedRegistryStore({
        remoteBaseUrl: "https://registry.example",
        installationId: "inst_test",
        cache,
        fetchImpl,
      });

      const first = await store.getVersion("acme/echo");
      expect(first?.skill_id).toBe("acme/echo");
      expect(first?.markdown).toBe(ECHO_MARKDOWN);
      expect(first?.profile_document).toBe(ECHO_PROFILE);
      expect(fetches).toBe(1);

      const second = await store.getVersion("acme/echo", "0.1.0");
      expect(second?.skill_id).toBe("acme/echo");
      expect(fetches).toBe(1);
    } finally {
      await rm(tempDir, { recursive: true, force: true });
    }
  });

  it("returns undefined when the registry responds with 404", async () => {
    const tempDir = await mkdtemp(path.join(os.tmpdir(), "runx-http-cache-404-"));
    try {
      const cache = createFileRegistryStore(path.join(tempDir, "cache"));
      const fetchImpl: typeof fetch = async () => new Response("not found", { status: 404 });
      const store = new HttpCachedRegistryStore({
        remoteBaseUrl: "https://registry.example",
        installationId: "inst_test",
        cache,
        fetchImpl,
      });

      const result = await store.getVersion("acme/missing");
      expect(result).toBeUndefined();
    } finally {
      await rm(tempDir, { recursive: true, force: true });
    }
  });

  it("forwards pinned versions to the acquire endpoint", async () => {
    const tempDir = await mkdtemp(path.join(os.tmpdir(), "runx-http-cache-pin-"));
    try {
      const cache = createFileRegistryStore(path.join(tempDir, "cache"));
      let seenVersion: unknown;
      const fetchImpl: typeof fetch = async (_input, init) => {
        const body = init?.body ? JSON.parse(String(init.body)) : {};
        seenVersion = body.version;
        return jsonResponse(buildAcquirePayload({ version: "1.2.3" }));
      };
      const store = new HttpCachedRegistryStore({
        remoteBaseUrl: "https://registry.example",
        installationId: "inst_test",
        cache,
        fetchImpl,
      });

      const result = await store.getVersion("acme/echo", "1.2.3");
      expect(seenVersion).toBe("1.2.3");
      expect(result?.version).toBe("1.2.3");
    } finally {
      await rm(tempDir, { recursive: true, force: true });
    }
  });

  it("refreshes unpinned latest requests instead of returning a stale cache hit", async () => {
    const tempDir = await mkdtemp(path.join(os.tmpdir(), "runx-http-cache-latest-"));
    try {
      const cache = createFileRegistryStore(path.join(tempDir, "cache"));
      let fetches = 0;
      const fetchImpl: typeof fetch = async () => {
        fetches += 1;
        return jsonResponse(buildAcquirePayload({
          version: fetches === 1 ? "0.1.0" : "0.2.0",
          digest: fetches === 1 ? "a".repeat(64) : "c".repeat(64),
        }));
      };
      const store = new HttpCachedRegistryStore({
        remoteBaseUrl: "https://registry.example",
        installationId: "inst_test",
        cache,
        fetchImpl,
      });

      const first = await store.getVersion("acme/echo");
      const second = await store.getVersion("acme/echo");

      expect(first?.version).toBe("0.1.0");
      expect(second?.version).toBe("0.2.0");
      expect(fetches).toBe(2);
    } finally {
      await rm(tempDir, { recursive: true, force: true });
    }
  });

  it("falls back to the cached latest when a refresh returns 404", async () => {
    const tempDir = await mkdtemp(path.join(os.tmpdir(), "runx-http-cache-latest-404-"));
    try {
      const cache = createFileRegistryStore(path.join(tempDir, "cache"));
      let fetches = 0;
      const fetchImpl: typeof fetch = async () => {
        fetches += 1;
        return fetches === 1
          ? jsonResponse(buildAcquirePayload({ version: "0.1.0" }))
          : new Response("not found", { status: 404 });
      };
      const store = new HttpCachedRegistryStore({
        remoteBaseUrl: "https://registry.example",
        installationId: "inst_test",
        cache,
        fetchImpl,
      });

      await expect(store.getVersion("acme/echo")).resolves.toMatchObject({ version: "0.1.0" });
      await expect(store.getVersion("acme/echo")).resolves.toMatchObject({ version: "0.1.0" });
      expect(fetches).toBe(2);
    } finally {
      await rm(tempDir, { recursive: true, force: true });
    }
  });

  it("persists HTTP fetches in the underlying cache store", async () => {
    const tempDir = await mkdtemp(path.join(os.tmpdir(), "runx-http-cache-persist-"));
    try {
      const cacheRoot = path.join(tempDir, "cache");
      const cache = createFileRegistryStore(cacheRoot);
      let fetches = 0;
      const fetchImpl: typeof fetch = async () => {
        fetches += 1;
        return jsonResponse(buildAcquirePayload());
      };
      const store = new HttpCachedRegistryStore({
        remoteBaseUrl: "https://registry.example",
        installationId: "inst_test",
        cache,
        fetchImpl,
      });

      await store.getVersion("acme/echo");
      expect(fetches).toBe(1);

      const detachedCache = createFileRegistryStore(cacheRoot);
      const persisted = await detachedCache.getVersion("acme/echo");
      expect(persisted?.skill_id).toBe("acme/echo");
      expect(persisted?.markdown).toBe(ECHO_MARKDOWN);
    } finally {
      await rm(tempDir, { recursive: true, force: true });
    }
  });
});

class HttpCachedRegistryStore {
  constructor(private readonly options: {
    readonly remoteBaseUrl: string;
    readonly installationId: string;
    readonly cache: RegistryStore;
    readonly fetchImpl: typeof fetch;
  }) {}

  async getVersion(skillId: string, version?: string): Promise<RegistrySkillVersion | undefined> {
    if (version) {
      const cached = await this.options.cache.getVersion(skillId, version);
      if (cached) {
        return cached;
      }
      return await this.fetchAndCache(skillId, version);
    }

    const cachedLatest = await this.options.cache.getVersion(skillId);
    const refreshed = await this.fetchAndCache(skillId);
    return refreshed ?? cachedLatest;
  }

  private async fetchAndCache(skillId: string, version?: string): Promise<RegistrySkillVersion | undefined> {
    const [owner, name] = splitSkillId(skillId);
    const response = await this.options.fetchImpl(
      `${this.options.remoteBaseUrl.replace(/\/$/, "")}/v1/skills/${owner}/${name}/acquire`,
      {
        method: "POST",
        body: JSON.stringify({
          installation_id: this.options.installationId,
          ...(version ? { version } : {}),
          channel: "cli",
        }),
      },
    );
    if (response.status === 404) {
      return undefined;
    }
    if (!response.ok) {
      throw new Error(`Remote registry acquire failed with HTTP ${response.status}`);
    }
    const payload = await response.json() as { readonly acquisition?: RegistrySkillVersion };
    if (!payload.acquisition) {
      throw new Error("Remote registry acquire response did not include acquisition.");
    }
    const acquired = payload.acquisition as Partial<RegistrySkillVersion> & Omit<
      RegistrySkillVersion,
      "created_at" | "required_scopes" | "source_type" | "tags" | "updated_at"
    >;
    const now = new Date().toISOString();
    return await this.options.cache.putVersion({
      ...acquired,
      required_scopes: acquired.required_scopes ?? [],
      tags: acquired.tags ?? [],
      source_type: acquired.source_type ?? "agent",
      created_at: acquired.created_at ?? now,
      updated_at: acquired.updated_at ?? now,
    }, { upsert: true });
  }
}

function splitSkillId(skillId: string): readonly [string, string] {
  const parts = skillId.split("/");
  if (parts.length !== 2 || !parts[0] || !parts[1]) {
    throw new Error(`Invalid registry skill id '${skillId}'. Expected '<owner>/<name>'.`);
  }
  return [parts[0], parts[1]];
}
