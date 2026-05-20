import { chmod, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";

import { describe, expect, it } from "vitest";

import { createDefaultSkillAdapters } from "@runxhq/adapters";
import { runLocalGraph, type Caller } from "@runxhq/runtime-local";

const caller: Caller = {
  resolve: async () => undefined,
  report: () => undefined,
};
const adapters = createDefaultSkillAdapters();

const ECHO_MARKDOWN = `---
name: echo
description: Minimal echo skill for Rust registry boundary fixtures.
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

describe("graph registry refs through the Rust registry boundary", () => {
  it("materializes a graph registry ref via runx registry resolve without a TS registry store", async () => {
    const tempDir = await mkdtemp(path.join(os.tmpdir(), "runx-graph-registry-rust-"));
    const fakeRegistryBin = path.join(tempDir, "fake-runx-registry.mjs");
    const registryDir = path.join(tempDir, "registry");
    const logPath = path.join(tempDir, "calls.jsonl");
    const graphPath = path.join(tempDir, "graph.yaml");

    const resolution = {
      markdown: ECHO_MARKDOWN,
      profile_document: ECHO_PROFILE,
      profile_digest: "b".repeat(64),
      runner_names: ["echo"],
      skill_id: "testorg/echo",
      name: "echo",
      version: "0.1.0",
      digest: "a".repeat(64),
      source: "runx-registry",
      source_label: "runx registry",
      source_type: "local",
      trust_tier: "community",
      add_command: "runx skill add testorg/echo",
      run_command: "runx run testorg/echo",
    };
    const envelope = {
      status: "success",
      registry: {
        action: "resolve",
        source: "local",
        ref: "testorg/echo",
        resolution: {
          kind: "local",
          ...resolution,
        },
      },
    };
    await writeFile(
      fakeRegistryBin,
      `#!/usr/bin/env node
import { appendFileSync } from "node:fs";

const expected = ["registry", "resolve", "testorg/echo", "--json", "--version", "0.1.0"];
const argv = process.argv.slice(2);
appendFileSync(process.env.FAKE_RUST_REGISTRY_LOG, JSON.stringify({
  argv,
  rustCli: process.env.RUNX_RUST_CLI,
  registryDir: process.env.RUNX_REGISTRY_DIR,
}) + "\\n");

if (JSON.stringify(argv) !== JSON.stringify(expected)) {
  process.stderr.write("unexpected args: " + JSON.stringify(argv));
  process.exit(64);
}
if (process.env.RUNX_RUST_CLI !== "1") {
  process.stderr.write("missing RUNX_RUST_CLI");
  process.exit(64);
}

process.stdout.write(${JSON.stringify(`${JSON.stringify(envelope, null, 2)}\n`)});
`,
      "utf8",
    );
    await chmod(fakeRegistryBin, 0o755);
    await writeFile(
      graphPath,
      `name: graph-registry-rust-boundary
steps:
  - id: echo
    skill: testorg/echo@0.1.0
    inputs:
      message: hello from rust boundary
`,
      "utf8",
    );

    const envOverrides: Record<string, string> = {
      FAKE_RUST_REGISTRY_LOG: logPath,
      RUNX_REGISTRY_DIR: registryDir,
      RUNX_RUST_REGISTRY_BIN: fakeRegistryBin,
      RUNX_RUST_REGISTRY_RESOLVE: "1",
      RUNX_RUST_REGISTRY_TIMEOUT_MS: "5000",
    };
    const previousEnv = new Map<string, string | undefined>();
    for (const [key, value] of Object.entries(envOverrides)) {
      previousEnv.set(key, process.env[key]);
      process.env[key] = value;
    }

    try {
      const result = await runLocalGraph({
        graphPath,
        caller,
        receiptDir: path.join(tempDir, "receipts"),
        runxHome: path.join(tempDir, "home"),
        env: process.env,
        skillCacheDir: path.join(tempDir, "skill-cache"),
        adapters,
      });

      expect(result.status).toBe("success");
      if (result.status !== "success") {
        return;
      }
      expect(result.steps[0]).toMatchObject({
        skill: "testorg/echo@0.1.0",
        stdout: "hello from rust boundary",
      });

      const calls = (await readFile(logPath, "utf8"))
        .trim()
        .split("\n")
        .map((line) => JSON.parse(line) as { argv: string[]; rustCli: string; registryDir: string });
      expect(calls).toHaveLength(2);
      for (const call of calls) {
        expect(call.argv).toEqual(["registry", "resolve", "testorg/echo", "--json", "--version", "0.1.0"]);
        expect(call.rustCli).toBe("1");
        expect(call.registryDir).toBe(registryDir);
      }
    } finally {
      for (const [key, value] of previousEnv) {
        if (value === undefined) {
          delete process.env[key];
        } else {
          process.env[key] = value;
        }
      }
      await rm(tempDir, { recursive: true, force: true });
    }
  });
});
