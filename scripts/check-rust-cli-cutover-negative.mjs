#!/usr/bin/env node
import { spawnSync } from "node:child_process";
import { lstat, readdir, readFile, realpath, stat } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const workspaceRoot = path.resolve(fileURLToPath(new URL("..", import.meta.url)));
const maxFileBytes = 25 * 1024 * 1024;

const forbiddenPathRules = [
  {
    rule: "js_runtime_path",
    pattern: /(^|\/)(bin\/runx\.(?:js|mjs|cjs)|dist\/|src\/.*\.(?:ts|tsx|js|mjs|cjs)|packages\/cli\/src\/)/u,
    message: "candidate package surface contains a JavaScript or TypeScript runtime path",
  },
  {
    rule: "hidden_node_runtime_path",
    pattern: /(^|\/)(node_modules\/|tools\/.*\.(?:js|mjs|cjs|ts|tsx))/u,
    message: "candidate package surface contains hidden Node runtime/tool files",
  },
];

const forbiddenTextRules = [
  {
    group: "js_fallback",
    rule: "js_fallback_token",
    patterns: [
      /\bRUNX_JS_BIN\b/u,
      /\bRUNX_RUST_CLI\b/u,
      /\bRUNX_RUST_HARNESS\b/u,
      /\bnpm\s+exec\b/u,
      /\bpnpm\s+exec\b/u,
      /\btsx\b/u,
      /\bprocess\.execPath\b/u,
      /\bregistry-fallback\b/u,
      /\bpackages\/cli\b/u,
      /\bpackages\/runtime-local\b/u,
    ],
    message: "candidate contains JavaScript fallback or candidate-gate token",
  },
  {
    group: "legacy_shape",
    rule: "legacy_shape_token",
    patterns: [
      /\bskill_execution\b/u,
      /\bgraph_execution\b/u,
      /\bgx_[0-9a-fA-F]{8,}\b/u,
      /\brx_[0-9a-fA-F]{8,}\b/u,
      /\breceiptPath\b/u,
      /\blocalReceipt\b/u,
    ],
    message: "candidate contains retired receipt or legacy contract shape token",
  },
  {
    group: "v2_alias",
    rule: "v2_alias_token",
    patterns: [
      /\bRUNX_V2\b/u,
      /(^|[\s"'`])--v2\b/u,
      /\brunx\s+v2\b/u,
      /\bschema_version["']?\s*:\s*["']v2["']/u,
      /\bversion["']?\s*:\s*["']v2["']/u,
    ],
    message: "candidate contains v2 alias or v2 mode token",
  },
  {
    group: "hidden_package_reference",
    rule: "hidden_package_reference_token",
    patterns: [
      /@runxhq\/(?:adapters|core|runtime-local)\b/u,
      /\bworkspace:/u,
      /\bfrom\s+["']@runxhq\//u,
      /\brequire\(["']@runxhq\//u,
    ],
    message: "candidate contains hidden TypeScript workspace package reference",
  },
];

const packageDependencySections = [
  "dependencies",
  "devDependencies",
  "optionalDependencies",
  "peerDependencies",
];

const bannedPackageDependencies = new Set([
  "@runxhq/adapters",
  "@runxhq/core",
  "@runxhq/runtime-local",
]);

const findings = [];
let candidate = "";

try {
  const args = parseArgs(process.argv.slice(2));
  if (args.help) {
    printUsage();
    process.exit(0);
  }
  candidate = args.candidate ? path.resolve(args.candidate) : "";
  if (!candidate) {
    findings.push(finding("candidate_missing", "<argument>", "missing --candidate path"));
  } else {
    const entries = await loadCandidateEntries(candidate);
    if (entries.length === 0) {
      findings.push(finding("candidate_empty", displayPath(candidate), "candidate contains no inspectable files"));
    }

    for (const entry of entries) {
      inspectPath(entry, findings);
      if (entry.packageJson) {
        inspectPackageJson(entry, findings);
      }
      inspectContent(entry, findings);
    }

    emit({
      status: findings.length === 0 ? "passed" : "blocked",
      candidate: displayPath(candidate),
      scanned_entries: entries.length,
      findings: sortFindings(findings),
    });
    process.exit(findings.length === 0 ? 0 : 1);
  }
} catch (error) {
  findings.push(finding("candidate_unreadable", displayPath(candidate || process.cwd()), error instanceof Error ? error.message : String(error)));
}

emit({
  status: "blocked",
  candidate: candidate ? displayPath(candidate) : null,
  scanned_entries: 0,
  findings: sortFindings(findings),
});
process.exit(1);

async function loadCandidateEntries(candidatePath) {
  const candidateStat = await stat(candidatePath);
  if (candidateStat.isDirectory()) {
    return loadDirectoryEntries(candidatePath);
  }
  if (candidateStat.isFile() && candidatePath.endsWith(".tgz")) {
    return loadTgzEntries(candidatePath);
  }
  if (candidateStat.isFile()) {
    return [await fileEntry(candidatePath, path.basename(candidatePath))];
  }
  throw new Error(`unsupported candidate path type: ${candidatePath}`);
}

async function loadDirectoryEntries(root) {
  const rootRealPath = await realpath(root);
  const files = [];

  async function visit(directory) {
    const children = (await readdir(directory)).sort((left, right) => left.localeCompare(right));
    for (const child of children) {
      const absolutePath = path.join(directory, child);
      const relativePath = toPosix(path.relative(root, absolutePath));
      const childStat = await lstat(absolutePath);
      if (childStat.isSymbolicLink()) {
        const linkedPath = await realpath(absolutePath);
        if (!isInside(linkedPath, rootRealPath)) {
          throw new Error(`candidate symlink escapes root: ${relativePath}`);
        }
        throw new Error(`candidate symlinks are not accepted for cutover verification: ${relativePath}`);
      }
      if (childStat.isDirectory()) {
        await visit(absolutePath);
        continue;
      }
      if (childStat.isFile()) {
        files.push(await fileEntry(absolutePath, relativePath));
      }
    }
  }

  await visit(root);
  return files.sort((left, right) => left.path.localeCompare(right.path));
}

async function fileEntry(absolutePath, relativePath) {
  const fileStat = await stat(absolutePath);
  if (fileStat.size > maxFileBytes) {
    throw new Error(`candidate file exceeds ${maxFileBytes} bytes: ${relativePath}`);
  }
  const buffer = await readFile(absolutePath);
  return {
    path: toPosix(relativePath),
    bytes: buffer,
    text: buffer.toString("utf8"),
    packageJson: toPosix(relativePath) === "package.json",
  };
}

function loadTgzEntries(archivePath) {
  const list = spawnSync("tar", ["-tzf", archivePath], {
    cwd: workspaceRoot,
    encoding: "utf8",
    maxBuffer: 10 * 1024 * 1024,
  });
  if (list.status !== 0) {
    throw new Error(`tar could not list archive: ${(list.stderr || list.stdout || "").trim()}`);
  }

  const paths = list.stdout
    .split(/\r?\n/u)
    .map((entry) => entry.trim())
    .filter(Boolean)
    .filter((entry) => !entry.endsWith("/"))
    .sort((left, right) => left.localeCompare(right));

  return paths.map((entryPath) => {
    if (entryPath.includes("..")) {
      throw new Error(`archive entry contains parent traversal: ${entryPath}`);
    }
    const extract = spawnSync("tar", ["-xOzf", archivePath, entryPath], {
      cwd: workspaceRoot,
      encoding: "buffer",
      maxBuffer: maxFileBytes + 1,
    });
    if (extract.status !== 0) {
      throw new Error(`tar could not read archive entry ${entryPath}: ${(extract.stderr || extract.stdout || "").toString("utf8").trim()}`);
    }
    if (extract.stdout.length > maxFileBytes) {
      throw new Error(`archive entry exceeds ${maxFileBytes} bytes: ${entryPath}`);
    }
    const normalizedPath = normalizeArchiveEntryPath(entryPath);
    return {
      path: normalizedPath,
      bytes: extract.stdout,
      text: extract.stdout.toString("utf8"),
      packageJson: normalizedPath === "package.json",
    };
  });
}

function inspectPath(entry, findings) {
  for (const rule of forbiddenPathRules) {
    if (rule.pattern.test(entry.path)) {
      findings.push(finding(rule.rule, entry.path, rule.message));
    }
  }
}

function inspectPackageJson(entry, findings) {
  let manifest;
  try {
    manifest = JSON.parse(entry.text);
  } catch (error) {
    findings.push(finding("package_json_malformed", entry.path, error instanceof Error ? error.message : String(error)));
    return;
  }

  const bin = normalizeRunxBin(manifest?.bin);
  if (bin === null) {
    findings.push(finding("package_bin_missing", entry.path, "package.json must declare a non-empty bin.runx candidate entry"));
  } else if (/\.(?:js|mjs|cjs)$/u.test(bin)) {
    findings.push(finding("package_bin_js_entry", entry.path, `package.json bin.runx points at JavaScript: ${bin}`));
  }

  for (const section of packageDependencySections) {
    const dependencies = manifest?.[section];
    if (!dependencies || typeof dependencies !== "object") {
      continue;
    }
    for (const [name, spec] of Object.entries(dependencies).sort(([left], [right]) => left.localeCompare(right))) {
      if (bannedPackageDependencies.has(name)) {
        findings.push(finding("package_hidden_dependency", entry.path, `${section}.${name} is forbidden in a Rust CLI cutover package`));
      }
      if (typeof spec === "string" && spec.startsWith("workspace:")) {
        findings.push(finding("package_workspace_dependency", entry.path, `${section}.${name} still uses ${spec}`));
      }
    }
  }
}

function inspectContent(entry, findings) {
  for (const rule of forbiddenTextRules) {
    for (const pattern of rule.patterns) {
      if (pattern.test(entry.text)) {
        findings.push(finding(rule.rule, entry.path, rule.message, { group: rule.group }));
        break;
      }
    }
  }
}

function parseArgs(argv) {
  const parsed = {};
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--help" || arg === "-h") {
      parsed.help = true;
      continue;
    }
    if (arg === "--candidate") {
      parsed.candidate = argv[index + 1];
      index += 1;
      continue;
    }
    throw new Error(`unknown argument: ${arg}`);
  }
  return parsed;
}

function printUsage() {
  console.log("Usage: node scripts/check-rust-cli-cutover-negative.mjs --candidate <path>");
}

function finding(rule, file, message, extra = {}) {
  return {
    rule,
    file,
    message,
    ...extra,
  };
}

function sortFindings(items) {
  return items.sort((left, right) => {
    const byFile = String(left.file).localeCompare(String(right.file));
    if (byFile !== 0) return byFile;
    return String(left.rule).localeCompare(String(right.rule));
  });
}

function emit(payload) {
  console.log(JSON.stringify(payload, null, 2));
}

function toPosix(value) {
  return value.split(path.sep).join("/");
}

function normalizeArchiveEntryPath(value) {
  return toPosix(value)
    .replace(/^\.\//u, "")
    .replace(/^package\//u, "");
}

function normalizeRunxBin(bin) {
  if (typeof bin === "string" && bin.trim() !== "") {
    return bin;
  }
  if (bin && typeof bin === "object" && typeof bin.runx === "string" && bin.runx.trim() !== "") {
    return bin.runx;
  }
  return null;
}

function displayPath(value) {
  const relative = path.relative(workspaceRoot, value);
  return relative && !relative.startsWith("..") ? toPosix(relative) : value;
}

function isInside(candidatePath, rootPath) {
  const relative = path.relative(rootPath, candidatePath);
  return relative === "" || (!relative.startsWith("..") && !path.isAbsolute(relative));
}
