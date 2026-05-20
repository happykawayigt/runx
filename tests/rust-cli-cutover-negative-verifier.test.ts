import { spawnSync } from "node:child_process";
import { mkdir, mkdtemp, rm, symlink, truncate, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { gzipSync } from "node:zlib";

import { describe, expect, it } from "vitest";

const workspaceRoot = process.cwd();
const scriptPath = path.join(workspaceRoot, "scripts", "check-rust-cli-cutover-negative.mjs");
const fixtureRoot = path.join(workspaceRoot, "fixtures", "rust-cli-cutover-negative");

describe("Rust CLI cutover negative verifier", () => {
  it("accepts a clean native candidate surface", () => {
    const result = runVerifier(path.join(fixtureRoot, "clean-candidate"));

    expect(result.status).toBe(0);
    const payload = JSON.parse(result.stdout);
    expect(payload).toMatchObject({
      status: "passed",
      candidate: "fixtures/rust-cli-cutover-negative/clean-candidate",
      scanned_entries: 3,
      findings: [],
    });
  });

  it("blocks JavaScript fallback package surfaces", () => {
    const result = runVerifier(path.join(fixtureRoot, "js-fallback-candidate"));

    expect(result.status).toBe(1);
    const payload = JSON.parse(result.stdout);
    expect(payload.status).toBe("blocked");
    expect(ruleIds(payload)).toEqual([
      "js_fallback_token",
      "js_runtime_path",
      "package_bin_js_entry",
    ]);
    expect(payload.findings[0]).toMatchObject({
      file: "bin/runx.js",
      group: "js_fallback",
    });
  });

  it("blocks legacy shapes, v2 aliases, and hidden package references", () => {
    const result = runVerifier(path.join(fixtureRoot, "legacy-v2-package-candidate"));

    expect(result.status).toBe(1);
    const payload = JSON.parse(result.stdout);
    expect(payload.status).toBe("blocked");
    expect(ruleIds(payload)).toEqual([
      "hidden_package_reference_token",
      "package_hidden_dependency",
      "package_workspace_dependency",
      "legacy_shape_token",
      "v2_alias_token",
    ]);
  });

  it("fails closed when the candidate is missing", () => {
    const missingCandidate = path.join(fixtureRoot, "missing-candidate");
    const result = runVerifier(missingCandidate);

    expect(result.status).toBe(1);
    const payload = JSON.parse(result.stdout);
    expect(payload).toMatchObject({
      status: "blocked",
      scanned_entries: 0,
    });
    expect(ruleIds(payload)).toEqual(["candidate_unreadable"]);
    expect(payload.findings[0].message).toContain("no such file or directory");
  });

  it("fails closed with stable JSON for unknown arguments", () => {
    const result = spawnSync(process.execPath, [scriptPath, "--unknown"], {
      cwd: workspaceRoot,
      encoding: "utf8",
      maxBuffer: 1024 * 1024,
    });

    expect(result.status).toBe(1);
    expect(result.stderr).toBe("");
    const payload = JSON.parse(result.stdout);
    expect(payload.status).toBe("blocked");
    expect(ruleIds(payload)).toEqual(["candidate_unreadable"]);
    expect(payload.findings[0].message).toBe("unknown argument: --unknown");
  });

  it("fails closed for empty candidates and malformed package manifests", async () => {
    const tempDir = await mkdtemp(path.join(os.tmpdir(), "runx-cutover-negative-closed-"));

    try {
      const emptyCandidate = path.join(tempDir, "empty");
      const malformedCandidate = path.join(tempDir, "malformed");
      await mkdir(emptyCandidate);
      await mkdir(malformedCandidate);
      await writeFile(path.join(malformedCandidate, "package.json"), "{bad json", "utf8");

      const emptyResult = runVerifier(emptyCandidate);
      expect(emptyResult.status).toBe(1);
      expect(ruleIds(JSON.parse(emptyResult.stdout))).toEqual(["candidate_empty"]);

      const malformedResult = runVerifier(malformedCandidate);
      expect(malformedResult.status).toBe(1);
      expect(ruleIds(JSON.parse(malformedResult.stdout))).toEqual(["package_json_malformed"]);
    } finally {
      await rm(tempDir, { recursive: true, force: true });
    }
  });

  it("fails closed for candidate symlinks", async () => {
    const tempDir = await mkdtemp(path.join(os.tmpdir(), "runx-cutover-negative-symlink-"));

    try {
      const candidate = path.join(tempDir, "candidate");
      await mkdir(candidate);
      await writeFile(path.join(candidate, "real-package.json"), "{}", "utf8");
      await symlink(path.join(candidate, "real-package.json"), path.join(candidate, "package.json"));

      const result = runVerifier(candidate);

      expect(result.status).toBe(1);
      const payload = JSON.parse(result.stdout);
      expect(ruleIds(payload)).toEqual(["candidate_unreadable"]);
      expect(payload.findings[0].message).toContain("candidate symlinks are not accepted");
    } finally {
      await rm(tempDir, { recursive: true, force: true });
    }
  });

  it("fails closed for oversized candidate files", async () => {
    const tempDir = await mkdtemp(path.join(os.tmpdir(), "runx-cutover-negative-oversize-"));
    const candidate = path.join(tempDir, "runx");

    try {
      await writeFile(candidate, "", "utf8");
      await truncate(candidate, 25 * 1024 * 1024 + 1);

      const result = runVerifier(candidate);

      expect(result.status).toBe(1);
      const payload = JSON.parse(result.stdout);
      expect(ruleIds(payload)).toEqual(["candidate_unreadable"]);
      expect(payload.findings[0].message).toContain("candidate file exceeds");
    } finally {
      await rm(tempDir, { recursive: true, force: true });
    }
  });

  it("fails closed for archive parent traversal entries", async () => {
    const tempDir = await mkdtemp(path.join(os.tmpdir(), "runx-cutover-negative-traversal-"));
    const archivePath = path.join(tempDir, "traversal.tgz");

    try {
      await writeFile(archivePath, gzipSync(tarArchiveEntry("../evil", "not allowed")));

      const result = runVerifier(archivePath);

      expect(result.status).toBe(1);
      const payload = JSON.parse(result.stdout);
      expect(ruleIds(payload)).toEqual(["candidate_unreadable"]);
      expect(payload.findings[0].message).toContain("archive entry contains parent traversal");
    } finally {
      await rm(tempDir, { recursive: true, force: true });
    }
  });

  it("inspects tgz package archives with normalized package paths", async () => {
    const tempDir = await mkdtemp(path.join(os.tmpdir(), "runx-cutover-negative-"));
    const archivePath = path.join(tempDir, "clean-candidate.tgz");

    try {
      const tarResult = spawnSync("tar", [
        "-czf",
        archivePath,
        "-C",
        path.join(fixtureRoot, "clean-candidate"),
        ".",
      ], {
        cwd: workspaceRoot,
        encoding: "utf8",
      });
      expect(tarResult.status).toBe(0);

      const result = runVerifier(archivePath);

      expect(result.status).toBe(0);
      const payload = JSON.parse(result.stdout);
      expect(payload).toMatchObject({
        status: "passed",
        scanned_entries: 3,
        findings: [],
      });
    } finally {
      await rm(tempDir, { recursive: true, force: true });
    }
  });

  it("applies package manifest checks to npm-style package-prefixed tgz archives", async () => {
    const tempDir = await mkdtemp(path.join(os.tmpdir(), "runx-cutover-negative-package-tgz-"));
    const archiveRoot = path.join(tempDir, "archive-root");
    const packageDir = path.join(archiveRoot, "package");
    const archivePath = path.join(tempDir, "legacy-package.tgz");

    try {
      await mkdir(packageDir, { recursive: true });
      await writeFile(path.join(packageDir, "package.json"), JSON.stringify({
        name: "@runxhq/cli",
        version: "0.0.0-cutover-fixture",
        bin: "./bin/runx",
        dependencies: {
          "@runxhq/runtime-local": "workspace:^0.1.1",
        },
      }, null, 2), "utf8");

      const tarResult = spawnSync("tar", ["-czf", archivePath, "-C", archiveRoot, "package"], {
        cwd: workspaceRoot,
        encoding: "utf8",
      });
      expect(tarResult.status).toBe(0);

      const result = runVerifier(archivePath);

      expect(result.status).toBe(1);
      const payload = JSON.parse(result.stdout);
      expect(ruleIds(payload)).toEqual([
        "hidden_package_reference_token",
        "package_hidden_dependency",
        "package_workspace_dependency",
      ]);
    } finally {
      await rm(tempDir, { recursive: true, force: true });
    }
  });
});

function runVerifier(candidate: string) {
  return spawnSync(process.execPath, [scriptPath, "--candidate", candidate], {
    cwd: workspaceRoot,
    encoding: "utf8",
    maxBuffer: 1024 * 1024,
  });
}

function ruleIds(payload: { findings: readonly { rule: string }[] }) {
  return payload.findings.map((finding) => finding.rule);
}

function tarArchiveEntry(name: string, contents: string) {
  const body = Buffer.from(contents, "utf8");
  const header = Buffer.alloc(512, 0);
  header.write(name, 0, 100, "utf8");
  header.write("0000644\0", 100, 8, "ascii");
  header.write("0000000\0", 108, 8, "ascii");
  header.write("0000000\0", 116, 8, "ascii");
  header.write(octal(body.length, 11), 124, 12, "ascii");
  header.write("00000000000\0", 136, 12, "ascii");
  header.fill(0x20, 148, 156);
  header.write("0", 156, 1, "ascii");
  header.write("ustar\0", 257, 6, "ascii");
  header.write("00", 263, 2, "ascii");

  const checksum = header.reduce((sum, byte) => sum + byte, 0);
  header.write(octal(checksum, 6), 148, 8, "ascii");

  return Buffer.concat([
    header,
    body,
    Buffer.alloc((512 - (body.length % 512)) % 512, 0),
    Buffer.alloc(1024, 0),
  ]);
}

function octal(value: number, width: number) {
  return value.toString(8).padStart(width, "0") + "\0";
}
