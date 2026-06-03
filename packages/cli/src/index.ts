#!/usr/bin/env node

export const cliPackage = "@runxhq/cli";

import { realpathSync } from "node:fs";
import { stdin as processStdin, stdout as processStdout } from "node:process";
import { pathToFileURL } from "node:url";

import { errorMessage } from "./cli-util.js";

import { isSupportedCommand, parseArgs } from "./args.js";
import { dispatchCli, writeCliError } from "./dispatch.js";
import { isHelpRequest, writeUsage } from "./help.js";

export { parseArgs } from "./args.js";
export type { ParsedArgs } from "./args.js";
export { resolveSkillReference, resolveRunnableSkillReference, createOfficialSkillResolver } from "./skill-refs.js";

export interface CliIo {
  readonly stdout: NodeJS.WriteStream;
  readonly stderr: NodeJS.WriteStream;
  readonly stdin: NodeJS.ReadStream;
}

export interface CliServices {}

export async function runCli(
  argv: readonly string[] = process.argv.slice(2),
  io: CliIo = { stdin: process.stdin, stdout: process.stdout, stderr: process.stderr },
  env: NodeJS.ProcessEnv = process.env,
  services: CliServices = {},
): Promise<number> {
  if (isHelpRequest(argv)) {
    writeUsage(io.stdout, env);
    return 0;
  }

  const parsed = parseArgs(argv);
  if (!isSupportedCommand(parsed)) {
    writeUsage(io.stderr, env);
    return 64;
  }

  try {
    return await dispatchCli(parsed, io, env, services);
  } catch (error) {
    const message = errorMessage(error);
    if (parsed.json) {
      return writeCliJsonError(io, message);
    }
    return writeCliError(io, message);
  }
}

function writeCliJsonError(io: CliIo, message: string): number {
  io.stdout.write(`${JSON.stringify({ status: "failure", error: { message } }, null, 2)}\n`);
  return 1;
}

if (process.argv[1] && import.meta.url === pathToFileURL(realpathSync(process.argv[1])).href) {
  const exitCode = await runCli(process.argv.slice(2), {
    stdin: processStdin,
    stdout: processStdout,
    stderr: process.stderr,
  });
  process.exitCode = exitCode;
}
