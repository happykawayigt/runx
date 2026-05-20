import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";

import { describe, expect, it } from "vitest";

import { hashString } from "@runxhq/core/util";

import { ensureOfficialSkillCached, officialSkillCachePath, type OfficialSkillLockEntry } from "./official-cache.js";

describe("official skill cache", () => {
  it("reuses a verified cached official skill without acquiring from the registry", async () => {
    const tempDir = await mkdtemp(path.join(os.tmpdir(), "runx-official-cache-hit-"));
    try {
      const markdown = "# Cached official skill\n";
      const entry = officialEntry(markdown);
      const skillPath = officialSkillCachePath(tempDir, entry);
      await mkdir(skillPath, { recursive: true });
      await writeFile(path.join(skillPath, "SKILL.md"), markdown, "utf8");

      let acquired = false;
      const result = await ensureOfficialSkillCached({
        cacheRoot: tempDir,
        registryBaseUrl: "https://registry.example",
        installationId: "install_1",
        entry,
        fetchImpl: async () => {
          acquired = true;
          return new Response(null, { status: 500 });
        },
      });

      expect(acquired).toBe(false);
      expect(result.fromCache).toBe(true);
      expect(result.skillPath).toBe(skillPath);
      expect(result.acquisition).toMatchObject({
        skill_id: "runx/cache-test",
        owner: "runx",
        name: "cache-test",
        version: "1.0.0",
        digest: entry.digest,
        trust_tier: "first_party",
        publisher: {
          kind: "organization",
          id: "runx",
          handle: "runx",
        },
        install_count: 0,
      });
      expect(result.acquisition.attestations).toEqual([
        expect.objectContaining({
          kind: "publisher",
          id: "publisher:runx",
          status: "verified",
          summary: "runx",
        }),
      ]);
    } finally {
      await rm(tempDir, { recursive: true, force: true });
    }
  });

  it("acquires, verifies, and writes a locked official skill cache miss", async () => {
    const tempDir = await mkdtemp(path.join(os.tmpdir(), "runx-official-cache-miss-"));
    try {
      const markdown = "# Acquired official skill\n";
      const profileDocument = "profile: true\n";
      const entry = officialEntry(markdown);
      const requests: Array<{ readonly url: string; readonly init?: RequestInit }> = [];

      const result = await ensureOfficialSkillCached({
        cacheRoot: tempDir,
        registryBaseUrl: "https://registry.example/",
        installationId: "install_1",
        entry,
        fetchImpl: async (url, init) => {
          requests.push({ url: String(url), init });
          return Response.json({
            status: "success",
            install_count: 7,
            acquisition: {
              skill_id: entry.skill_id,
              owner: "runx",
              name: "cache-test",
              version: entry.version,
              digest: entry.digest,
              markdown,
              profile_document: profileDocument,
              profile_digest: "profile_digest_1",
              runner_names: ["runner-a"],
              trust_tier: "first_party",
              publisher: {
                kind: "organization",
                id: "runx",
                handle: "runx",
              },
              attestations: [
                {
                  kind: "publisher",
                  id: "publisher:runx",
                  status: "verified",
                  summary: "runx",
                },
              ],
            },
          });
        },
      });

      expect(result.fromCache).toBe(false);
      expect(result.acquisition.install_count).toBe(7);
      expect(requests).toHaveLength(1);
      expect(requests[0].url).toBe("https://registry.example/v1/skills/runx/cache-test/acquire");
      expect(requests[0].init?.method).toBe("POST");
      expect(JSON.parse(String(requests[0].init?.body))).toEqual({
        installation_id: "install_1",
        version: "1.0.0",
        channel: "cli",
      });
      await expect(readFile(path.join(result.skillPath, "SKILL.md"), "utf8")).resolves.toBe(markdown);
      const profileState = JSON.parse(await readFile(path.join(result.skillPath, ".runx", "profile.json"), "utf8")) as {
        readonly profile: {
          readonly document: string;
          readonly digest: string;
          readonly runner_names: readonly string[];
        };
      };
      expect(profileState.profile).toEqual({
        document: profileDocument,
        digest: "profile_digest_1",
        runner_names: ["runner-a"],
      });
    } finally {
      await rm(tempDir, { recursive: true, force: true });
    }
  });

  it("rejects acquired official skills that do not match the lock digest", async () => {
    const tempDir = await mkdtemp(path.join(os.tmpdir(), "runx-official-cache-verify-"));
    try {
      const entry = officialEntry("# Expected\n");
      await expect(ensureOfficialSkillCached({
        cacheRoot: tempDir,
        registryBaseUrl: "https://registry.example",
        installationId: "install_1",
        entry,
        fetchImpl: async () => Response.json({
          status: "success",
          acquisition: {
            skill_id: entry.skill_id,
            owner: "runx",
            name: "cache-test",
            version: entry.version,
            digest: "wrong_digest",
            markdown: "# Unexpected\n",
            runner_names: [],
            trust_tier: "first_party",
            publisher: {
              kind: "organization",
              id: "runx",
              handle: "runx",
            },
            attestations: [],
          },
        }),
      })).rejects.toThrow(/Official skill verification failed/);
    } finally {
      await rm(tempDir, { recursive: true, force: true });
    }
  });
});

function officialEntry(markdown: string): OfficialSkillLockEntry {
  return {
    skill_id: "runx/cache-test",
    version: "1.0.0",
    digest: hashString(markdown),
  };
}
