import { generateKeyPairSync, sign } from "node:crypto";
import { existsSync, readFileSync, readdirSync, statSync, writeFileSync } from "node:fs";
import { mkdir, mkdtemp, readFile, rm, stat, writeFile } from "node:fs/promises";
import { spawnSync } from "node:child_process";
import os from "node:os";
import path from "node:path";

import { describe, expect, it } from "vitest";

describe("official skill native fetch", () => {
  it("acquires, caches, and reruns official shorthand through the native resolver", async () => {
    const tempDir = await mkdtemp(path.join(os.tmpdir(), "runx-official-fetch-"));
    const projectDir = path.join(tempDir, "project");
    const globalHomeDir = path.join(tempDir, "home");
    const env = testEnv(projectDir, globalHomeDir);

    try {
      await mkdir(projectDir, { recursive: true });
      const registryDir = path.join(tempDir, "registry");
      const sourceyLock = await officialSkillLock("runx/sourcey");
      publishLocalRegistrySkill({
        registryDir,
        subject: path.resolve("skills/sourcey/SKILL.md"),
        profile: path.resolve("skills/sourcey/X.yaml"),
        owner: "runx",
        version: sourceyLock.version,
        env,
      });

      const first = runNativeSkill(env, [
        "sourcey",
        "--registry",
        registryDir,
        "--json",
        "--non-interactive",
      ]);
      const firstJson = parseJsonOutput(first, 2);
      const firstPath = skillDirectoryFromNeedsAgent(firstJson);
      expect(firstPath).toContain(path.join(globalHomeDir, "official-skills"));
      expect(firstPath).toContain(path.join("runx", "sourcey"));
      expect((await stat(path.join(firstPath, "SKILL.md"))).isFile()).toBe(true);
      expect((await stat(path.join(firstPath, "X.yaml"))).isFile()).toBe(true);

      const second = runNativeSkill(env, [
        "sourcey",
        "--registry",
        registryDir,
        "--json",
        "--non-interactive",
      ]);
      const secondJson = parseJsonOutput(second, 2);
      expect(skillDirectoryFromNeedsAgent(secondJson)).toBe(firstPath);
      expect((await stat(path.join(firstPath, "SKILL.md"))).isFile()).toBe(true);
    } finally {
      await rm(tempDir, { recursive: true, force: true });
    }
  });

  it("rejects an official acquisition with a digest mismatch before caching", async () => {
    const tempDir = await mkdtemp(path.join(os.tmpdir(), "runx-official-fetch-digest-"));
    const projectDir = path.join(tempDir, "project");
    const globalHomeDir = path.join(tempDir, "home");
    const env = testEnv(projectDir, globalHomeDir);

    try {
      await mkdir(projectDir, { recursive: true });
      const registryDir = path.join(tempDir, "registry");
      const wrongSkillDir = path.join(tempDir, "wrong-sourcey");
      const wrongSkillPath = path.join(wrongSkillDir, "SKILL.md");
      const originalMarkdown = await readFile(path.resolve("skills/sourcey/SKILL.md"), "utf8");
      await writeTestFile(
        wrongSkillPath,
        originalMarkdown.replace(
          "description: Generate documentation for a project using Sourcey.",
          "description: Generate different documentation for a project using Sourcey.",
        ),
      );
      const sourceyLock = await officialSkillLock("runx/sourcey");
      publishLocalRegistrySkill({
        registryDir,
        subject: wrongSkillPath,
        profile: path.resolve("skills/sourcey/X.yaml"),
        owner: "runx",
        version: sourceyLock.version,
        env,
      });

      const result = runNativeSkill(env, [
        "sourcey",
        "--registry",
        registryDir,
        "--input",
        `project=${projectDir}`,
        "--json",
        "--non-interactive",
      ]);
      expect(result.status).toBe(1);
      expect(result.stderr).toBe("");
      expect((JSON.parse(result.stdout) as { error?: { message?: string } }).error?.message).toContain("digest mismatch");
      expect(existsSync(path.join(globalHomeDir, "official-skills", "runx", "sourcey", "SKILL.md"))).toBe(false);
    } finally {
      await rm(tempDir, { recursive: true, force: true });
    }
  });

  it("copies packaged stage helpers beside cached official graph skills", async () => {
    const tempDir = await mkdtemp(path.join(os.tmpdir(), "runx-official-fetch-runtime-"));
    const projectDir = path.join(tempDir, "project");
    const globalHomeDir = path.join(tempDir, "home");
    const env = testEnv(projectDir, globalHomeDir);

    try {
      await mkdir(projectDir, { recursive: true });
      const registryDir = path.join(tempDir, "registry");
      const lockEntry = await officialSkillLock("runx/issue-to-pr");
      publishLocalRegistrySkill({
        registryDir,
        subject: path.resolve("skills/issue-to-pr/SKILL.md"),
        profile: path.resolve("skills/issue-to-pr/X.yaml"),
        owner: "runx",
        version: lockEntry.version,
        env,
      });

      const result = runNativeSkill(env, [
        "issue-to-pr",
        "--registry",
        registryDir,
        "--input",
        "task_id=issue-to-pr-native-fetch",
        "--input",
        "thread_title=Fixture smoke test",
        "--input",
        "thread_body=Minimal thread body for the official cache test.",
        "--input",
        "thread_locator=local://fixtures/official-cache",
        "--json",
        "--non-interactive",
      ]);
      const output = parseJsonOutput(result, 2);
      const skillPath = skillDirectoryFromNeedsAgent(output);
      expect(
        (await stat(
          path.join(skillPath, "graph", "scafld", "run.mjs"),
        )).isFile(),
      ).toBe(true);
    } finally {
      await rm(tempDir, { recursive: true, force: true });
    }
  });

  it("copies graph stages beside cached official graph skills", async () => {
    const tempDir = await mkdtemp(path.join(os.tmpdir(), "runx-official-fetch-stages-"));
    const projectDir = path.join(tempDir, "project");
    const globalHomeDir = path.join(tempDir, "home");
    const env = testEnv(projectDir, globalHomeDir);

    try {
      await mkdir(projectDir, { recursive: true });
      const registryDir = path.join(tempDir, "registry");
      const lockEntry = await officialSkillLock("runx/spend");
      publishLocalRegistrySkill({
        registryDir,
        subject: path.resolve("skills/spend/SKILL.md"),
        profile: path.resolve("skills/spend/X.yaml"),
        owner: "runx",
        version: lockEntry.version,
        env,
      });

      const result = runNativeSkill(env, ["spend", "--registry", registryDir, "--json", "--non-interactive"]);
      const output = parseJsonOutput(result, 2);
      const skillPath = officialPackageRootFromSkillDirectory(skillDirectoryFromNeedsAgent(output));
      for (const stage of ["pay-quote", "pay-reserve", "pay-fulfill-rail"]) {
        expect(
          (await stat(
            path.join(skillPath, "graph", stage, "X.yaml"),
          )).isFile(),
          stage,
        ).toBe(true);
      }
    } finally {
      await rm(tempDir, { recursive: true, force: true });
    }
  });
});

function testEnv(projectDir: string, globalHomeDir: string): NodeJS.ProcessEnv {
  return {
    ...process.env,
    RUNX_CWD: projectDir,
    RUNX_HOME: globalHomeDir,
    RUNX_DEV_RUST_CLI_BIN: nativeRunxBinaryForTest(),
    RUNX_RECEIPT_SIGN_KID: process.env.RUNX_RECEIPT_SIGN_KID ?? "official-skill-native-fetch-test-key",
    RUNX_RECEIPT_SIGN_ED25519_SEED_BASE64:
      process.env.RUNX_RECEIPT_SIGN_ED25519_SEED_BASE64 ?? "QkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkI=",
    RUNX_RECEIPT_SIGN_ISSUER_TYPE: process.env.RUNX_RECEIPT_SIGN_ISSUER_TYPE ?? "hosted",
  };
}

async function officialSkillLock(skillId: string): Promise<{
  readonly skill_id: string;
  readonly version: string;
  readonly digest: string;
}> {
  const officialLock = JSON.parse(
    await readFile(path.resolve("packages/cli/src/official-skills.lock.json"), "utf8"),
  ) as ReadonlyArray<{
    readonly skill_id: string;
    readonly version: string;
    readonly digest: string;
  }>;
  const entry = officialLock.find((candidate) => candidate.skill_id === skillId);
  if (!entry) {
    throw new Error(`Missing ${skillId} entry in official-skills.lock.json.`);
  }
  return entry;
}

function runNativeSkill(env: NodeJS.ProcessEnv, args: readonly string[]): {
  readonly status: number | null;
  readonly stdout: string;
  readonly stderr: string;
} {
  const result = spawnSync(env.RUNX_DEV_RUST_CLI_BIN ?? "runx", ["skill", ...args], {
    cwd: env.RUNX_CWD ?? process.cwd(),
    env,
    encoding: "utf8",
  });
  return {
    status: result.status,
    stdout: result.stdout,
    stderr: result.stderr,
  };
}

function parseJsonOutput(result: {
  readonly status: number | null;
  readonly stdout: string;
  readonly stderr: string;
}, expectedStatus: number): unknown {
  expect(result.status, `stderr=${result.stderr}\nstdout=${result.stdout}`).toBe(expectedStatus);
  expect(result.stderr).toBe("");
  return JSON.parse(result.stdout);
}

function skillDirectoryFromNeedsAgent(value: unknown): string {
  const record = value as {
    requests?: Array<{
      invocation?: {
        envelope?: {
          execution_location?: {
            skill_directory?: string;
          };
        };
      };
    }>;
  };
  const skillDirectory = record.requests?.[0]?.invocation?.envelope?.execution_location?.skill_directory;
  if (!skillDirectory) {
    throw new Error("Missing needs_agent skill directory.");
  }
  return skillDirectory;
}

function officialPackageRootFromSkillDirectory(skillDirectory: string): string {
  const graphMarker = `${path.sep}graph${path.sep}`;
  const index = skillDirectory.indexOf(graphMarker);
  return index === -1 ? skillDirectory : skillDirectory.slice(0, index);
}

function publishLocalRegistrySkill(input: {
  readonly registryDir: string;
  readonly subject: string;
  readonly owner: string;
  readonly version: string;
  readonly env: NodeJS.ProcessEnv;
  readonly profile?: string;
  readonly trustTier?: "verified" | "community";
}): void {
  const signingKey = testManifestSigningKey();
  input.env.RUNX_REGISTRY_MANIFEST_TRUST_KEY_ID = signingKey.keyId;
  input.env.RUNX_REGISTRY_MANIFEST_TRUST_KEY_BASE64 = signingKey.publicKeyBase64;
  input.env.RUNX_REGISTRY_MANIFEST_TRUST_OWNER = input.owner;
  if (input.owner === "runx") {
    input.env.RUNX_REGISTRY_SOURCE_AUTHORITY = "official_runx";
  }
  const args = [
    "registry",
    "publish",
    input.subject,
    "--registry-dir",
    input.registryDir,
    "--owner",
    input.owner,
    "--version",
    input.version,
    "--trust-tier",
    input.trustTier ?? "community",
    "--upsert",
    "--json",
  ];
  if (input.profile) {
    args.push("--profile", input.profile);
  }
  const result = spawnSync(input.env.RUNX_DEV_RUST_CLI_BIN ?? "runx", args, {
    cwd: process.cwd(),
    env: input.env,
    encoding: "utf8",
  });
  if (result.status !== 0) {
    throw new Error(`failed to publish local registry fixture: ${result.stderr || result.stdout}`);
  }
  signPublishedRegistryEntry(input.registryDir, signingKey);
}

async function writeTestFile(filePath: string, contents: string): Promise<void> {
  await rm(path.dirname(filePath), { recursive: true, force: true });
  await mkdir(path.dirname(filePath), { recursive: true });
  await writeFile(filePath, contents, "utf8");
}

function nativeRunxBinaryForTest(): string {
  const existing = process.env.RUNX_DEV_RUST_CLI_BIN;
  if (existing) {
    return existing;
  }
  const candidate = path.resolve("crates/target/debug/runx");
  return existsSync(candidate) ? candidate : "runx";
}

interface TestManifestSigningKey {
  readonly keyId: string;
  readonly signerId: string;
  readonly publicKeyBase64: string;
  readonly privateKey: ReturnType<typeof generateKeyPairSync>["privateKey"];
}

let cachedManifestSigningKey: TestManifestSigningKey | undefined;

function testManifestSigningKey(): TestManifestSigningKey {
  if (cachedManifestSigningKey) {
    return cachedManifestSigningKey;
  }
  const keyPair = generateKeyPairSync("ed25519");
  const publicKeyDer = keyPair.publicKey.export({ format: "der", type: "spki" });
  const publicKeyRaw = Buffer.from(publicKeyDer).subarray(-32);
  cachedManifestSigningKey = {
    keyId: "runx-test-registry-ed25519",
    signerId: "runx-test-registry",
    publicKeyBase64: publicKeyRaw.toString("base64"),
    privateKey: keyPair.privateKey,
  };
  return cachedManifestSigningKey;
}

function signPublishedRegistryEntry(registryDir: string, signingKey: TestManifestSigningKey): void {
  const entryPath = findSingleRegistryEntry(registryDir);
  const entry = JSON.parse(readFileSync(entryPath, "utf8")) as {
    skill_id: string;
    version: string;
    digest: string;
    profile_digest?: string;
    signed_manifest?: unknown;
  };
  const payload =
    "runx.registry.signed_manifest.v1\n" +
    `skill_id=${entry.skill_id}\n` +
    `version=${entry.version}\n` +
    `digest=${entry.digest}\n` +
    `profile_digest=${entry.profile_digest ?? ""}\n` +
    `signer_id=${signingKey.signerId}\n` +
    `key_id=${signingKey.keyId}\n`;
  entry.signed_manifest = {
    schema: "runx.registry.signed_manifest.v1",
    skill_id: entry.skill_id,
    version: entry.version,
    digest: entry.digest,
    ...(entry.profile_digest ? { profile_digest: entry.profile_digest } : {}),
    signer: {
      id: signingKey.signerId,
      key_id: signingKey.keyId,
    },
    signature: {
      alg: "ed25519",
      value: `base64:${sign(null, Buffer.from(payload), signingKey.privateKey).toString("base64")}`,
    },
  };
  writeFileSync(entryPath, `${JSON.stringify(entry, null, 2)}\n`, "utf8");
}

function findSingleRegistryEntry(root: string): string {
  const matches: string[] = [];
  const walk = (dir: string): void => {
    for (const entry of readdirSync(dir)) {
      const entryPath = path.join(dir, entry);
      const stats = statSync(entryPath);
      if (stats.isDirectory()) {
        walk(entryPath);
      } else if (entryPath.endsWith(".json")) {
        matches.push(entryPath);
      }
    }
  };
  walk(root);
  if (matches.length !== 1) {
    throw new Error(`expected one registry fixture entry, found ${matches.length}`);
  }
  return matches[0];
}
