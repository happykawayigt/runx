import { readdir, readFile } from "node:fs/promises";
import path from "node:path";

import { describe, expect, it } from "vitest";

const workspaceRoot = process.cwd();
const cliSourceRoot = path.join(workspaceRoot, "packages", "cli", "src");
const runxScope = "@runxhq";
const adaptersPackage = `${runxScope}/adapters`;
const runtimeLocalPackage = `${runxScope}/runtime-local`;

const ALLOWED_RUNTIME_IMPORTERS = new Map<string, readonly string[]>([
  ["packages/cli/src/agent-runtime.ts", [`${adaptersPackage}/agent`]],
  ["packages/cli/src/commands/dev/skill-fixture.ts", [
    adaptersPackage,
    runtimeLocalPackage,
    `${runtimeLocalPackage}/tool-catalogs`,
  ]],
  ["packages/cli/src/commands/history.ts", [runtimeLocalPackage]],
  ["packages/cli/src/dispatch.ts", [
    adaptersPackage,
    runtimeLocalPackage,
    `${runtimeLocalPackage}/harness`,
    `${runtimeLocalPackage}/sdk`,
    `${runtimeLocalPackage}/tool-catalogs`,
  ]],
  ["packages/cli/src/registry-fallback.ts", [`${runtimeLocalPackage}/sdk`]],
  ["packages/cli/src/skill-refs.ts", [runtimeLocalPackage]],
]);

describe("CLI runtime-local importer boundary", () => {
  it("keeps runtime-local and adapters imports pinned to execution-owned blockers", async () => {
    const importers = await collectRuntimeImporters();

    expect(importers).toEqual(ALLOWED_RUNTIME_IMPORTERS);
  });
});

async function collectRuntimeImporters(): Promise<Map<string, readonly string[]>> {
  const importers = new Map<string, readonly string[]>();
  for (const filePath of await listTypeScriptFiles(cliSourceRoot)) {
    const contents = await readFile(filePath, "utf8");
    const imports = extractRuntimeImportSpecifiers(contents);
    if (imports.length === 0) {
      continue;
    }
    importers.set(toProjectPath(filePath), imports);
  }
  return new Map([...importers].sort(([left], [right]) => left.localeCompare(right)));
}

async function listTypeScriptFiles(directory: string): Promise<readonly string[]> {
  const entries = await readdir(directory, { withFileTypes: true });
  const files: string[] = [];
  for (const entry of entries) {
    const entryPath = path.join(directory, entry.name);
    if (entry.isDirectory()) {
      files.push(...await listTypeScriptFiles(entryPath));
      continue;
    }
    if (entry.isFile() && entry.name.endsWith(".ts")) {
      files.push(entryPath);
    }
  }
  return files.sort((left, right) => left.localeCompare(right));
}

function extractRuntimeImportSpecifiers(contents: string): readonly string[] {
  const imports = new Set<string>();
  for (const pattern of [
    /\bfrom\s+["']([^"'`]+)["']/gm,
    /^\s*import\s+(?:type\s+)?["']([^"'`]+)["'];?/gm,
  ]) {
    for (const match of contents.matchAll(pattern)) {
      const specifier = match[1];
      if (specifier === runtimeLocalPackage || specifier.startsWith(`${runtimeLocalPackage}/`) ||
        specifier === adaptersPackage || specifier.startsWith(`${adaptersPackage}/`)) {
        imports.add(specifier);
      }
    }
  }
  return [...imports].sort((left, right) => left.localeCompare(right));
}

function toProjectPath(filePath: string): string {
  return path.relative(workspaceRoot, filePath).split(path.sep).join("/");
}
