import { existsSync } from "node:fs";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import os from "node:os";
import path from "node:path";

import { describe, expect, it } from "vitest";

import {
  materializeRegistrySkill,
  type RegistrySkillVersion,
  type RegistryStore,
} from "@runxhq/runtime-local";

const ECHO_MARKDOWN = `---
name: echo
description: Minimal echo skill for resolver boundary fixtures.
---

Echo a message.
`;

describe("runtime-local registry resolver", () => {
  it("materializes a store-backed registry ref without importing core registry resolution", async () => {
    const tempDir = await mkdtemp(path.join(os.tmpdir(), "runx-registry-resolver-"));
    const record: RegistrySkillVersion = {
      markdown: ECHO_MARKDOWN,
      runner_names: [],
      skill_id: "testorg/echo",
      name: "echo",
      version: "0.1.0",
      digest: "a".repeat(64),
      source_type: "local",
      trust_tier: "community",
    };
    const store: RegistryStore = {
      getVersion: async (skillId, version) =>
        skillId === "testorg/echo" && version === "0.1.0" ? record : undefined,
      listVersions: async (skillId) => (skillId === "testorg/echo" ? [record] : []),
    };

    try {
      const materialized = await materializeRegistrySkill({
        ref: "testorg/echo@0.1.0",
        store,
        cacheDir: path.join(tempDir, "cache"),
      });

      expect(materialized.resolution).toMatchObject({
        markdown: ECHO_MARKDOWN,
        skill_id: "testorg/echo",
        name: "echo",
        version: "0.1.0",
        digest: record.digest,
        source: "runx-registry",
        source_label: "runx registry",
        source_type: "local",
        trust_tier: "community",
        add_command: "runx skill add testorg/echo@0.1.0",
        run_command: "runx skill echo",
      });
      expect(existsSync(materialized.skillPath)).toBe(true);
      expect(await readFile(materialized.skillPath, "utf8")).toBe(ECHO_MARKDOWN);
    } finally {
      await rm(tempDir, { recursive: true, force: true });
    }
  });
});
