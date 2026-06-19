import { mkdir, readdir, writeFile } from "node:fs/promises";
import path from "node:path";

import { isNodeError } from "./cli-util.js";

export interface ScaffoldRunxPackageOptions {
  readonly name: string;
  readonly directory: string;
}

export interface ScaffoldRunxPackageResult {
  readonly name: string;
  readonly directory: string;
  readonly files: readonly string[];
  readonly next_steps: readonly string[];
}

export async function scaffoldRunxPackage(options: ScaffoldRunxPackageOptions): Promise<ScaffoldRunxPackageResult> {
  const name = sanitizeRunxPackageName(options.name);
  const root = path.resolve(options.directory);
  await assertWritableScaffoldTarget(root);

  const writes = scaffoldPackageFiles(name);
  await mkdir(root, { recursive: true });
  await Promise.all(writes.map(([relativePath, contents]) => write(root, relativePath, contents)));

  return {
    name,
    directory: root,
    files: writes.map(([relativePath]) => relativePath),
    next_steps: [
      `cd ${root}`,
      "runx harness . --json",
      "runx skill . --input message=hello --json",
    ],
  };
}

export function sanitizeRunxPackageName(value: string): string {
  return value.trim().toLowerCase().replace(/[^a-z0-9_.-]+/g, "-").replace(/^[._-]+|[._-]+$/g, "") || "runx-package";
}

function scaffoldPackageFiles(name: string): ReadonlyArray<readonly [string, string]> {
  return [
    ["SKILL.md", skillMd(name)],
    ["X.yaml", xYaml(name)],
    ["run.mjs", runMjs()],
    ["README.md", readme(name)],
    [".gitignore", "node_modules/\n.runx/\n*.tgz\n"],
  ];
}

function skillMd(name: string): string {
  return `---
name: ${name}
description: ${name} runx skill. Replace this with what the skill does and returns.
source:
  type: cli-tool
  command: node
  args:
    - run.mjs
  timeout_seconds: 30
  sandbox:
    profile: readonly
    cwd_policy: skill-directory
inputs:
  message:
    type: string
    required: true
    description: Input the skill acts on. Replace with the real inputs.
runx:
  input_resolution:
    required:
      - message
---

# ${name}

Describe what this skill does, when an agent should reach for it, and what it
returns. Replace the echo in \`run.mjs\` with the real work, and add cases to
\`X.yaml\` so the behaviour is locked by the harness.
`;
}

function xYaml(name: string): string {
  return `skill: ${name}
version: "0.1.0"

catalog:
  kind: skill
  audience: public
  visibility: public
  role: canonical

harness:
  cases:
    - name: ${name}-smoke
      runner: default
      inputs:
        message: hello
      expect:
        status: sealed
        receipt:
          schema: runx.receipt.v1
          state: sealed
          disposition: closed
          reason_code: process_closed
    - name: ${name}-empty-message-fails
      runner: default
      inputs:
        message: ""
      expect:
        status: failure
        receipt:
          schema: runx.receipt.v1
          state: sealed
          disposition: closed
          reason_code: process_failed

runners:
  default:
    default: true
    type: cli-tool
    command: node
    args:
      - run.mjs
    inputs:
      message:
        type: string
        required: true
        description: Input the skill acts on.
`;
}

function runMjs(): string {
  return `// Inputs arrive as RUNX_INPUT_<NAME> environment variables. Do the work and
// write the result to stdout. Replace this echo with the real logic.
const message = process.env.RUNX_INPUT_MESSAGE ?? "";
if (message.trim().length === 0) {
  process.stderr.write("message is required\\n");
  process.exit(64);
}
process.stdout.write(\`${"${message}"}\\n\`);
`;
}

function readme(name: string): string {
  return `# ${name}

A native runx skill: a \`SKILL.md\` contract, an \`X.yaml\` execution profile, and a
\`run.mjs\` script. No build step and no dependencies.

## Develop

\`\`\`bash
runx harness . --json                       # run the harness cases in X.yaml
runx skill . --input message=hello --json   # run the skill once
runx history                                # inspect the signed receipt
\`\`\`

Edit \`run.mjs\` to do the real work, and keep both harness classes in \`X.yaml\`:
one happy path and one stop, error, or refusal case.

## Publish

\`\`\`bash
runx login --provider github --for publish
runx registry publish .   # the registry runs the harness as the publish gate
\`\`\`
`;
}

async function assertWritableScaffoldTarget(root: string): Promise<void> {
  const entries = await readdir(root).catch((error: unknown) => {
    if (isNodeError(error) && error.code === "ENOENT") {
      return undefined;
    }
    throw error;
  });
  if (entries && entries.length > 0) {
    throw new Error(`Refusing to scaffold into non-empty directory: ${root}`);
  }
}

async function write(root: string, relativePath: string, contents: string): Promise<void> {
  const filePath = path.join(root, relativePath);
  await mkdir(path.dirname(filePath), { recursive: true });
  await writeFile(filePath, contents.endsWith("\n") ? contents : `${contents}\n`);
}
