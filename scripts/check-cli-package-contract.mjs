import { execFile } from "node:child_process";
import { readFile, stat } from "node:fs/promises";
import path from "node:path";
import { promisify } from "node:util";
import { fileURLToPath } from "node:url";

const execFileAsync = promisify(execFile);
const workspaceRoot = path.resolve(fileURLToPath(new URL("..", import.meta.url)));
const cliPackageRoot = path.join(workspaceRoot, "packages", "cli");
const cliPackageJson = path.join(cliPackageRoot, "package.json");
const cliBinEntry = path.join(cliPackageRoot, "bin", "runx");
const npm = process.platform === "win32" ? "npm.cmd" : "npm";
const supportedPlatforms = [
  "darwin-arm64",
  "darwin-x64",
  "linux-arm64",
  "linux-x64",
  "win32-x64",
];

const manifest = JSON.parse(await readFile(cliPackageJson, "utf8"));
assertEqual(manifest.name, "@runxhq/cli", "CLI selector package name changed");
assertEqual(manifest.bin?.runx, "./bin/runx", "CLI selector bin.runx must point at ./bin/runx");
assertEqual(manifest.runx?.nativeSelector?.schema, "runx.rust_cli_selector_topology.v1", "native selector schema changed");
assertEqual(
  manifest.runx?.nativeSelector?.nativePackagePattern,
  "@runxhq/cli-${platform}",
  "native selector package pattern changed",
);
assertArrayEqual(
  manifest.runx?.nativeSelector?.supportedPlatforms ?? [],
  supportedPlatforms,
  "native selector supported platform list changed",
);
assertArrayEqual(
  manifest.files ?? [],
  ["LICENSE", "bin/runx", "native/supported-platforms.json"],
  "CLI selector package must pack only selector artifacts",
);
for (const field of ["main", "types", "exports", "dependencies", "devDependencies", "peerDependencies", "scripts"]) {
  if (Object.hasOwn(manifest, field)) {
    throw new Error(`CLI selector package must not declare ${field}`);
  }
}
for (const platform of supportedPlatforms) {
  const packageName = `@runxhq/cli-${platform}`;
  assertEqual(
    manifest.optionalDependencies?.[packageName],
    manifest.version,
    `optionalDependencies.${packageName} must match the selector version`,
  );
}
assertArrayEqual(
  Object.keys(manifest.optionalDependencies ?? {}).sort(),
  supportedPlatforms.map((platform) => `@runxhq/cli-${platform}`).sort(),
  "CLI selector optional dependencies changed",
);

const topology = JSON.parse(await readFile(path.join(cliPackageRoot, "native", "supported-platforms.json"), "utf8"));
assertEqual(topology.schema, "runx.rust_cli_selector_topology.v1", "topology manifest schema changed");
assertEqual(topology.selectorPackage, "@runxhq/cli", "topology manifest selector package changed");
assertArrayEqual(Object.keys(topology.nativePackages ?? {}).sort(), supportedPlatforms, "topology manifest platform list changed");
for (const platform of supportedPlatforms) {
  const entry = topology.nativePackages?.[platform];
  assertEqual(entry?.package, `@runxhq/cli-${platform}`, `topology package changed for ${platform}`);
  assertEqual(entry?.binary, platform.startsWith("win32-") ? "bin/runx.exe" : "bin/runx", `topology binary changed for ${platform}`);
}

const entry = await stat(cliBinEntry);
if (!entry.isFile() || (process.platform !== "win32" && (entry.mode & 0o111) === 0)) {
  throw new Error(`CLI selector entry is missing or not executable: ${cliBinEntry}`);
}
const selector = await readFile(cliBinEntry, "utf8");
for (const token of [
  "packages/cli/src",
  "packages/cli/dist",
  "RUNX_JS_BIN",
  "RUNX_NPM_PACKAGE",
  "RUNX_RUST_CLI",
  "RUNX_RUST_HARNESS",
  "npm exec",
  "process.execPath",
]) {
  if (selector.includes(token)) {
    throw new Error(`CLI selector contains forbidden delegation token ${token}`);
  }
}

const pack = await execFileAsync(npm, ["pack", "--dry-run", "--json"], {
  cwd: cliPackageRoot,
  timeout: 30_000,
  maxBuffer: 1024 * 1024,
});
const [report] = JSON.parse(pack.stdout);
const files = report.files.map((file) => file.path).sort();
assertArrayEqual(files, ["LICENSE", "bin/runx", "native/supported-platforms.json", "package.json"], "CLI package pack list changed");
for (const file of files) {
  if (/^(dist|src|tools|node_modules|\.runx)\//u.test(file) || /^bin\/runx\.(?:js|mjs|cjs)$/u.test(file)) {
    throw new Error(`CLI package unexpectedly ships ${file}`);
  }
}

function assertEqual(actual, expected, message) {
  if (actual !== expected) {
    throw new Error(`${message}: expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`);
  }
}

function assertArrayEqual(actual, expected, message) {
  const actualJson = JSON.stringify(actual);
  const expectedJson = JSON.stringify(expected);
  if (actualJson !== expectedJson) {
    throw new Error(`${message}: expected ${expectedJson}, got ${actualJson}`);
  }
}
