import { chmod, mkdtemp, readFile, readdir, rm, stat, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";

import { describe, expect, it } from "vitest";

import { runCli } from "../packages/cli/src/index.js";
import type { MarketplaceAdapter, SkillSearchResult } from "@runxhq/core/marketplaces";
import { hashString } from "@runxhq/core/util";
import { installLocalSkill } from "@runxhq/runtime-local";
import { createFileRegistryStore, seedRegistrySkill } from "./registry-fixtures.js";
import { resolveRunxBinary } from "./runx-binary.js";

describe("skill-add", () => {
  it("rejects unsigned local registry installs through the OSS CLI", async () => {
    const tempDir = await mkdtemp(path.join(os.tmpdir(), "runx-skill-add-registry-"));
    const registryDir = path.join(tempDir, "registry");
    const skillsDir = path.join(tempDir, "skills");
    const stdout = createMemoryStream();
    const stderr = createMemoryStream();
    const markdown = await readFile(path.resolve("skills/sourcey/SKILL.md"), "utf8");

    try {
      await seedRegistrySkill(createFileRegistryStore(registryDir), markdown, {
        owner: "acme",
        version: "1.0.0",
        createdAt: "2026-04-10T00:00:00.000Z",
        profileDocument: await readFile(path.resolve("skills/sourcey/X.yaml"), "utf8"),
      });

      const exitCode = await runCli(
        ["skill", "add", "registry:sourcey", "--to", skillsDir, "--json"],
        { stdin: process.stdin, stdout, stderr },
        {
          ...process.env,
          RUNX_CWD: process.cwd(),
          RUNX_REGISTRY_DIR: registryDir,
        },
      );

      expect(exitCode).toBe(1);
      expect(stdout.contents()).toBe("");
      expect(stderr.contents()).toContain("unsigned_manifest");
      await expect(stat(path.join(skillsDir, "sourcey", "SKILL.md"))).rejects.toThrow();
    } finally {
      await rm(tempDir, { recursive: true, force: true });
    }
  });

  it("does not fall back to the TypeScript marketplace installer", async () => {
    const tempDir = await mkdtemp(path.join(os.tmpdir(), "runx-skill-add-fixture-"));
    const stdout = createMemoryStream();
    const stderr = createMemoryStream();

    try {
      const exitCode = await runCli(
        ["skill", "add", "fixture-marketplace:sourcey-docs", "--to", path.join(tempDir, "skills"), "--json"],
        { stdin: process.stdin, stdout, stderr },
        {
          ...process.env,
          RUNX_CWD: process.cwd(),
          RUNX_REGISTRY_DIR: path.join(tempDir, "registry"),
          RUNX_ENABLE_FIXTURE_MARKETPLACE: "1",
        },
      );

      expect(exitCode).toBe(1);
      expect(stdout.contents()).toBe("");
      expect(stderr.contents()).toContain("registry skill not found");
      await expect(stat(path.join(tempDir, "skills", "sourcey-docs", "SKILL.md"))).rejects.toThrow();
    } finally {
      await rm(tempDir, { recursive: true, force: true });
    }
  });

  it("uses native registry install for registry installs", async () => {
    const tempDir = await mkdtemp(path.join(os.tmpdir(), "runx-skill-add-rust-"));
    const registryBin = path.join(tempDir, "registry-install");
    const skillsDir = path.join(tempDir, "skills");
    const stdout = createMemoryStream();
    const stderr = createMemoryStream();

    try {
      await writeNodeCommand(
        registryBin,
        `
const args = process.argv.slice(2);
process.stdout.write(JSON.stringify({
  status: "success",
  registry: {
    action: "install",
    source: "local",
    ref: "acme/sourcey@1.0.0",
    install: {
      status: "installed",
      destination: ${JSON.stringify(path.join(skillsDir, "acme", "sourcey", "SKILL.md"))},
      skill_name: "sourcey",
      source: "runx-registry",
      source_label: "runx registry",
      skill_id: "acme/sourcey",
      version: "1.0.0",
      digest: "sha256:abcd",
      profile_digest: "sha256:profile",
      profile_state_path: ${JSON.stringify(path.join(skillsDir, "acme", "sourcey", ".runx", "profile.json"))},
      runner_names: ["agent"],
      trust_tier: "community"
    },
    receipt_metadata: { observed_args: args }
  }
}, null, 2) + "\\n");
`,
      );

      const exitCode = await runCli(
        ["skill", "add", "acme/sourcey@1.0.0", "--to", skillsDir, "--digest", "sha256:abcd", "--json"],
        { stdin: process.stdin, stdout, stderr },
        {
          ...process.env,
          RUNX_CWD: process.cwd(),
          RUNX_RUST_CLI_BIN: registryBin,
          RUNX_REGISTRY_DIR: path.join(tempDir, "unused-registry"),
        },
      );

      expect(exitCode).toBe(0);
      expect(stderr.contents()).toBe("");
      const report = JSON.parse(stdout.contents()) as {
        registry: {
          receipt_metadata: {
            observed_args: string[];
          };
          install: {
            status: string;
            destination: string;
            skill_id: string;
            profile_digest: string;
            profile_state_path: string;
            runner_names: string[];
          };
        };
      };
      expect(report.registry.receipt_metadata.observed_args).toEqual([
        "registry",
        "install",
        "acme/sourcey@1.0.0",
        "--json",
        "--to",
        skillsDir,
        "--digest",
        "abcd",
      ]);
      expect(report.registry.install).toMatchObject({
        status: "installed",
        destination: path.join(skillsDir, "acme", "sourcey", "SKILL.md"),
        skill_id: "acme/sourcey",
        profile_digest: "sha256:profile",
        profile_state_path: path.join(skillsDir, "acme", "sourcey", ".runx", "profile.json"),
        runner_names: ["agent"],
      });
    } finally {
      await rm(tempDir, { recursive: true, force: true });
    }
  });

  it("routes marketplace installs through the native registry boundary", async () => {
    const tempDir = await mkdtemp(path.join(os.tmpdir(), "runx-skill-add-marketplace-rust-"));
    const registryBin = path.join(tempDir, "registry-install");
    const stdout = createMemoryStream();
    const stderr = createMemoryStream();

    try {
      await writeNodeCommand(
        registryBin,
        `
process.stderr.write("native registry install should not run for fixture marketplace\\n");
process.exit(2);
`,
      );

      const exitCode = await runCli(
        ["skill", "add", "fixture-marketplace:sourcey-docs", "--to", path.join(tempDir, "skills"), "--json"],
        { stdin: process.stdin, stdout, stderr },
        {
          ...process.env,
          RUNX_CWD: process.cwd(),
          RUNX_REGISTRY_DIR: path.join(tempDir, "registry"),
          RUNX_ENABLE_FIXTURE_MARKETPLACE: "1",
          RUNX_RUST_CLI_BIN: registryBin,
        },
      );

      expect(exitCode).toBe(2);
      expect(stdout.contents()).toBe("");
      expect(stderr.contents()).toContain("native registry install should not run for fixture marketplace");
    } finally {
      await rm(tempDir, { recursive: true, force: true });
    }
  });

  it("installs runx links into decoded namespace folder packages", async () => {
    const tempDir = await mkdtemp(path.join(os.tmpdir(), "runx-skill-add-link-"));
    const registryDir = path.join(tempDir, "registry");
    const skillsDir = path.join(tempDir, "skills");
    const markdown = await readFile(path.resolve("skills/sourcey/SKILL.md"), "utf8");

    try {
      const version = await seedRegistrySkill(createFileRegistryStore(registryDir), markdown, {
        owner: "acme",
        version: "1.0.0",
        createdAt: "2026-04-10T00:00:00.000Z",
        profileDocument: await readFile(path.resolve("skills/sourcey/X.yaml"), "utf8"),
      });

      const install = await installLocalSkill({
        ref: "runx://skill/acme%2Fsourcey@1.0.0",
        registryStore: createFileRegistryStore(registryDir),
        destinationRoot: skillsDir,
        expectedDigest: version.digest,
        env: nativeEnv(),
      });

      expect(install.destination).toBe(path.join(skillsDir, "acme", "sourcey", "SKILL.md"));
      expect(install.profileStatePath).toBe(path.join(skillsDir, "acme", "sourcey", ".runx", "profile.json"));
      expect(install.runnerNames).toEqual(["agent", "sourcey"]);
      await expect(readFile(path.join(skillsDir, "acme", "sourcey", "SKILL.md"), "utf8")).resolves.toBe(markdown);
      await expect(readFile(path.join(skillsDir, "acme", "sourcey", ".runx", "profile.json"), "utf8")).resolves.toContain("tool: sourcey.build");
    } finally {
      await rm(tempDir, { recursive: true, force: true });
    }
  });

  it("rejects TypeScript SDK installs without an explicit digest anchor", async () => {
    const tempDir = await mkdtemp(path.join(os.tmpdir(), "runx-skill-add-explicit-digest-"));
    const registryDir = path.join(tempDir, "registry");
    const skillsDir = path.join(tempDir, "skills");
    const markdown = await readFile(path.resolve("skills/sourcey/SKILL.md"), "utf8");

    try {
      await seedRegistrySkill(createFileRegistryStore(registryDir), markdown, {
        owner: "acme",
        version: "1.0.0",
        createdAt: "2026-04-10T00:00:00.000Z",
      });

      await expect(
        installLocalSkill({
          ref: "acme/sourcey@1.0.0",
          registryStore: createFileRegistryStore(registryDir),
          destinationRoot: skillsDir,
        }),
      ).rejects.toThrow("Trusted skill install requires an expected digest");
      await expect(stat(path.join(skillsDir, "sourcey", "SKILL.md"))).rejects.toThrow();
    } finally {
      await rm(tempDir, { recursive: true, force: true });
    }
  });

  it("fails digest mismatch without writing a partial file", async () => {
    const tempDir = await mkdtemp(path.join(os.tmpdir(), "runx-skill-add-digest-"));
    const registryDir = path.join(tempDir, "registry");
    const skillsDir = path.join(tempDir, "skills");
    const stdout = createMemoryStream();
    const stderr = createMemoryStream();

    try {
      await seedRegistrySkill(createFileRegistryStore(registryDir), await readFile(path.resolve("skills/sourcey/SKILL.md"), "utf8"), {
        owner: "acme",
        version: "1.0.0",
      });

      const exitCode = await runCli(
        ["skill", "add", "acme/sourcey@1.0.0", "--to", skillsDir, "--digest", "sha256:0000", "--json"],
        { stdin: process.stdin, stdout, stderr },
        {
          ...process.env,
          RUNX_CWD: process.cwd(),
          RUNX_REGISTRY_DIR: registryDir,
        },
      );

      expect(exitCode).toBe(1);
      expect(stderr.contents()).toContain("unsigned_manifest");
      await expect(stat(path.join(skillsDir, "sourcey", "SKILL.md"))).rejects.toThrow();
    } finally {
      await rm(tempDir, { recursive: true, force: true });
    }
  });

  it("fails invalid marketplace markdown without writing a partial file", async () => {
    const tempDir = await mkdtemp(path.join(os.tmpdir(), "runx-skill-add-invalid-"));
    const adapter = createInvalidMarketplaceAdapter();

    try {
      await expect(
        installLocalSkill({
          ref: "invalid:sourcey",
          registryStore: createFileRegistryStore(path.join(tempDir, "registry")),
          marketplaceAdapters: [adapter],
          destinationRoot: path.join(tempDir, "skills"),
          expectedDigest: hashString("not a skill"),
          env: nativeEnv(),
        }),
      ).rejects.toThrow("Skill markdown must start with YAML frontmatter");
      await expect(readdir(path.join(tempDir, "skills"))).rejects.toThrow();
    } finally {
      await rm(tempDir, { recursive: true, force: true });
    }
  });
});

function createInvalidMarketplaceAdapter(): MarketplaceAdapter {
  const result: SkillSearchResult = {
    skill_id: "invalid/sourcey",
    name: "sourcey",
    owner: "invalid",
    source: "invalid",
    source_label: "Invalid Fixture",
    source_type: "cli-tool",
    trust_tier: "community",
    required_scopes: [],
    tags: [],
    profile_mode: "portable",
    runner_names: [],
    add_command: "runx skill add invalid:sourcey",
    run_command: "runx skill sourcey",
  };

  return {
    source: "invalid",
    label: "Invalid Fixture",
    search: async () => [result],
    resolve: async () => ({
      markdown: "not a skill",
      result,
    }),
  };
}

function createMemoryStream(): NodeJS.WriteStream & { contents: () => string } {
  let buffer = "";
  return {
    write: (chunk: string | Uint8Array) => {
      buffer += chunk.toString();
      return true;
    },
    contents: () => buffer,
  } as NodeJS.WriteStream & { contents: () => string };
}

async function writeNodeCommand(commandPath: string, source: string): Promise<void> {
  const scriptPath = `${commandPath}.mjs`;
  await writeFile(scriptPath, source, "utf8");
  await writeFile(commandPath, `#!/bin/sh\nexec ${JSON.stringify(process.execPath)} ${JSON.stringify(scriptPath)} "$@"\n`, "utf8");
  await chmod(commandPath, 0o755);
}

function nativeEnv(): NodeJS.ProcessEnv {
  const runxBinary = resolveRunxBinary();
  return {
    ...process.env,
    RUNX_KERNEL_EVAL_BIN: runxBinary,
    RUNX_RUST_CLI_BIN: runxBinary,
  };
}
