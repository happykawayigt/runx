import { spawnSync } from "node:child_process";
import path from "node:path";

import { beforeAll, describe, expect, it } from "vitest";
import {
  extractSkillQualityProfileViaParser,
  validateGraphYamlViaParser,
  validateSkillMarkdownViaParser,
} from "./parser-bridge.js";

const workspaceRoot = process.cwd();
const cargo = process.platform === "win32" ? "cargo.exe" : "cargo";
const cargoTargetDir = process.env.CARGO_TARGET_DIR
  ? path.resolve(workspaceRoot, process.env.CARGO_TARGET_DIR)
  : path.join(workspaceRoot, "crates", "target");
const runxBinary = path.join(
  cargoTargetDir,
  "debug",
  process.platform === "win32" ? "runx.exe" : "runx",
);

describe("Rust parser CLI JSON bridge", () => {
  beforeAll(() => {
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
  }, 120_000);

  it("validates skill markdown through the Rust parser", async () => {
    const skill = await validateSkillMarkdownViaParser(
      [
        "---",
        "name: portable-agent",
        "description: Portable agent skill",
        "inputs:",
        "  prompt:",
        "    type: string",
        "    required: true",
        "---",
        "# Portable agent",
        "",
      ].join("\n"),
      { mode: "strict" },
      { command: runxBinary, cwd: workspaceRoot, timeoutMs: 30_000 },
    );

    expect(skill.name).toBe("portable-agent");
    expect(skill.source.type).toBe("agent");
    expect(skill.inputs.prompt?.required).toBe(true);
  }, 30_000);

  it("validates graph yaml through the Rust parser", async () => {
    const graph = await validateGraphYamlViaParser(
      [
        "name: gx",
        "steps:",
        "  - id: one",
        "    run:",
        "      type: cli-tool",
        "      command: node",
        "      args: [\"-e\", \"process.stdout.write('{}')\"]",
        "",
      ].join("\n"),
      { command: runxBinary, cwd: workspaceRoot },
    );

    expect(graph.name).toBe("gx");
    expect(graph.steps.map((step) => step.id)).toEqual(["one"]);
  });

  it("extracts skill quality profile through the Rust parser", async () => {
    const profile = await extractSkillQualityProfileViaParser(
      "# Skill\n\n## Quality Profile\n\nKeep the output stable.\n",
      { command: runxBinary, cwd: workspaceRoot },
    );

    expect(profile).toEqual({
      heading: "Quality Profile",
      content: "Keep the output stable.",
    });
  });

  it("requires an explicit Rust parser command", async () => {
    await expect(validateGraphYamlViaParser("name: gx\nsteps: []\n", {
      env: {
        RUNX_PARSER_EVAL_BIN: "",
        RUNX_RUST_CLI_BIN: "",
        RUNX_KERNEL_EVAL_BIN: "",
      },
    })).rejects.toThrow("RUNX_PARSER_EVAL_BIN");
  });
});
