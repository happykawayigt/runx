import { readdir, readFile, stat } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { canonicalJsonStringify } from "@runxhq/contracts";

const workspaceRoot = path.resolve(fileURLToPath(new URL("..", import.meta.url)));
const [targetArg, ...flags] = process.argv.slice(2);
const allowMissing = flags.includes("--allow-missing");
const target = path.resolve(workspaceRoot, targetArg ?? "fixtures/contracts");

if (!(await exists(target))) {
  if (allowMissing) {
    console.log(`Contract fixture directory is missing, allowed: ${path.relative(workspaceRoot, target)}`);
    process.exit(0);
  }
  throw new Error(`Contract fixture directory does not exist: ${path.relative(workspaceRoot, target)}`);
}

const failures: string[] = [];

for (const filePath of await listJsonFiles(target)) {
  const actual = await readFile(filePath, "utf8");
  const expected = `${canonicalJsonStringify(JSON.parse(actual))}\n`;
  if (actual !== expected) {
    failures.push(path.relative(workspaceRoot, filePath));
  }
}

if (failures.length > 0) {
  console.error(`Contract fixture keys are not canonical:\n${failures.map((file) => `- ${file}`).join("\n")}`);
  process.exit(1);
}

console.log("Contract fixture keys are sorted.");

async function exists(filePath: string): Promise<boolean> {
  try {
    await stat(filePath);
    return true;
  } catch (error) {
    if (isNodeError(error) && error.code === "ENOENT") {
      return false;
    }
    throw error;
  }
}

async function listJsonFiles(directory: string): Promise<readonly string[]> {
  const entries = await readdir(directory, { withFileTypes: true });
  const files: string[] = [];
  for (const entry of entries) {
    const entryPath = path.join(directory, entry.name);
    if (entry.isDirectory()) {
      files.push(...await listJsonFiles(entryPath));
    } else if (entry.isFile() && entry.name.endsWith(".json")) {
      files.push(entryPath);
    }
  }
  return files.sort();
}

function isNodeError(error: unknown): error is NodeJS.ErrnoException {
  return Boolean(error && typeof error === "object" && "code" in error);
}
