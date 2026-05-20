import { spawn } from "node:child_process";
import type { Readable, Writable } from "node:stream";

export interface CliIo {
  readonly stdin: Readable;
  readonly stdout: Writable;
  readonly stderr: Writable;
}

export type RunCliLike = (
  argv: readonly string[],
  io?: CliIo,
  env?: NodeJS.ProcessEnv,
) => Promise<number>;

const usageLines = [
  "Usage:",
  "  npm create @runxhq/skill@latest <name> [-- --directory dir]",
  "  runx new <name> [--directory dir]",
  "",
  "Notes:",
  "  runx new is the canonical command.",
  "  The create package is a cold-start entrypoint for the same scaffolder.",
];

export async function runRunxNew(
  argv: readonly string[],
  io: CliIo = { stdin: process.stdin, stdout: process.stdout, stderr: process.stderr },
  env: NodeJS.ProcessEnv = process.env,
): Promise<number> {
  const runxBin = env.RUNX_BIN ?? "runx";
  return await new Promise<number>((resolve) => {
    let settled = false;
    const finish = (code: number) => {
      if (!settled) {
        settled = true;
        resolve(code);
      }
    };
    const child = spawn(runxBin, ["new", ...argv], {
      env,
      stdio: "inherit",
    });
    child.on("error", (error) => {
      io.stderr.write(`create-skill: failed to start runx: ${error.message}\n`);
      finish(127);
    });
    child.on("exit", (code, signal) => {
      if (signal) {
        io.stderr.write(`create-skill: runx exited from signal ${signal}\n`);
        finish(1);
        return;
      }
      finish(code ?? 1);
    });
  });
}

export function writeCreateSkillUsage(stream: Writable): void {
  stream.write(`${usageLines.join("\n")}\n`);
}

export async function runCreateSkill(
  argv: readonly string[] = process.argv.slice(2),
  io: CliIo = { stdin: process.stdin, stdout: process.stdout, stderr: process.stderr },
  env: NodeJS.ProcessEnv = process.env,
  runCliImpl: RunCliLike = runRunxNew,
): Promise<number> {
  if (argv.length === 1 && (argv[0] === "--help" || argv[0] === "-h")) {
    writeCreateSkillUsage(io.stdout);
    return 0;
  }
  if (argv.length === 0) {
    writeCreateSkillUsage(io.stderr);
    return 64;
  }
  return await runCliImpl(argv, io, env);
}

if (import.meta.url === new URL(process.argv[1] ?? "", "file:").href) {
  process.exitCode = await runCreateSkill();
}
