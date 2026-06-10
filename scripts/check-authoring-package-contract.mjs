import { execFile } from "node:child_process";
import { copyFile, mkdir, mkdtemp, readFile, rm, stat, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { promisify } from "node:util";
import { fileURLToPath } from "node:url";

const execFileAsync = promisify(execFile);
const workspaceRoot = path.resolve(fileURLToPath(new URL("..", import.meta.url)));
const authoringPackageRoot = path.join(workspaceRoot, "packages", "authoring");
const contractsPackageRoot = path.join(workspaceRoot, "packages", "contracts");
const distEntry = path.join(authoringPackageRoot, "dist", "index.js");
const npm = process.platform === "win32" ? "npm.cmd" : "npm";
const tar = process.platform === "win32" ? "tar.exe" : "tar";
const exec = { timeout: 60_000, maxBuffer: 1024 * 1024 };

const entry = await stat(distEntry);
if (!entry.isFile()) {
  throw new Error(`Authoring dist entry is missing: ${distEntry}`);
}
const entrySource = await readFile(distEntry, "utf8");
if (entrySource.includes(".build/runtime")) {
  throw new Error("Authoring dist entry still points at .build/runtime instead of the packaged dist tree.");
}

async function packTarball(packageRoot) {
  const pack = await execFileAsync(npm, ["pack", "--json"], { cwd: packageRoot, ...exec });
  const [report] = JSON.parse(pack.stdout);
  if (!report?.filename) {
    throw new Error(`npm pack did not report a tarball for ${packageRoot}`);
  }
  return { tarball: path.join(packageRoot, report.filename), files: report.files ?? [] };
}

const tempRoot = await mkdtemp(path.join(os.tmpdir(), "runx-authoring-pack-"));
let authoringTarball;
let contractsTarball;

try {
  const authoring = await packTarball(authoringPackageRoot);
  authoringTarball = authoring.tarball;

  const files = new Set(authoring.files.map((file) => file.path));
  for (const required of [
    "dist/index.js",
    "dist/index.d.ts",
    "dist/src/index.js",
    "dist/src/index.d.ts",
    "package.json",
  ]) {
    if (!files.has(required)) {
      throw new Error(`Authoring package is missing ${required}`);
    }
  }

  // `@runxhq/contracts` is a workspace dependency that is not published to the
  // registry in this pre-publish smoke test, and `npm install` rejects the
  // `workspace:` protocol. Pack contracts locally and point the authoring
  // tarball's dependency at that file so the install resolves offline.
  const contracts = await packTarball(contractsPackageRoot);
  contractsTarball = contracts.tarball;
  const contractsDependencyPath = path.join(tempRoot, path.basename(contractsTarball));
  await copyFile(contractsTarball, contractsDependencyPath);

  const authoringDir = path.join(tempRoot, "authoring");
  await mkdir(authoringDir);
  await execFileAsync(tar, ["-xzf", authoringTarball, "-C", authoringDir, "--strip-components=1"], exec);
  const manifestPath = path.join(authoringDir, "package.json");
  const manifest = JSON.parse(await readFile(manifestPath, "utf8"));
  if (typeof manifest.dependencies?.["@runxhq/contracts"] !== "string") {
    throw new Error("Authoring package is expected to depend on @runxhq/contracts.");
  }
  manifest.dependencies["@runxhq/contracts"] = `file:${contractsDependencyPath}`;
  await writeFile(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`);

  // Re-pack the rewritten package so `npm install` copies it (a directory
  // install symlinks, which breaks dependency resolution from the linked tree).
  const repack = await packTarball(authoringDir);
  const rewrittenTarball = repack.tarball;

  const consumerDir = path.join(tempRoot, "consumer");
  await mkdir(consumerDir);
  await execFileAsync(npm, ["init", "-y"], { cwd: consumerDir, ...exec });
  await execFileAsync(npm, ["install", rewrittenTarball], { cwd: consumerDir, ...exec });
  const smoke = await execFileAsync(
    process.execPath,
    [
      "--input-type=module",
      "-e",
      'import { defineTool, stringInput } from "@runxhq/authoring";'
        + 'const tool = defineTool({ name: "demo.echo", inputs: { value: stringInput() }, run: ({ inputs }) => ({ value: inputs.value }) });'
        + 'const output = await tool.runWith({ value: "ok" });'
        + 'process.stdout.write(JSON.stringify(output));',
    ],
    { cwd: consumerDir, ...exec },
  );
  if (smoke.stdout.trim() !== '{"value":"ok"}') {
    throw new Error(`Authoring tarball smoke test returned unexpected output: ${smoke.stdout.trim()}`);
  }
} finally {
  if (authoringTarball) {
    await rm(authoringTarball, { force: true });
  }
  if (contractsTarball) {
    await rm(contractsTarball, { force: true });
  }
  await rm(tempRoot, { recursive: true, force: true });
}
