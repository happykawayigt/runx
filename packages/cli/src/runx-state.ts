import { randomUUID } from "node:crypto";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";

import { isNotFound, isRecord } from "./cli-util.js";

export interface RunxInstallState {
  readonly version: 1;
  readonly installation_id: string;
  readonly created_at: string;
}

export interface RunxProjectState {
  readonly version: 1;
  readonly project_id: string;
  readonly created_at: string;
}

export async function readRunxProjectState(
  projectDir: string,
): Promise<RunxProjectState | undefined> {
  const projectPath = path.join(projectDir, "project.json");
  let contents: string;
  try {
    contents = await readFile(projectPath, "utf8");
  } catch (error) {
    if (isNotFound(error)) {
      return undefined;
    }
    throw error;
  }
  const parsed: unknown = JSON.parse(contents);
  if (
    !isRecord(parsed)
    || parsed.version !== 1
    || typeof parsed.project_id !== "string"
    || typeof parsed.created_at !== "string"
  ) {
    throw new Error(`${projectPath} is not a valid Runx project state.`);
  }
  return {
    version: 1,
    project_id: parsed.project_id,
    created_at: parsed.created_at,
  };
}

export async function ensureRunxProjectState(
  projectDir: string,
  now: () => string = () => new Date().toISOString(),
): Promise<{ readonly state: RunxProjectState; readonly created: boolean }> {
  const existing = await readRunxProjectState(projectDir);
  if (existing) {
    return {
      state: existing,
      created: false,
    };
  }
  const state: RunxProjectState = {
    version: 1,
    project_id: `proj_${randomUUID()}`,
    created_at: now(),
  };
  await mkdir(projectDir, { recursive: true });
  await writeFile(path.join(projectDir, "project.json"), `${JSON.stringify(state, null, 2)}\n`, {
    mode: 0o600,
  });
  return {
    state,
    created: true,
  };
}

export async function readRunxInstallState(
  globalHomeDir: string,
): Promise<RunxInstallState | undefined> {
  const installPath = path.join(globalHomeDir, "install.json");
  let contents: string;
  try {
    contents = await readFile(installPath, "utf8");
  } catch (error) {
    if (isNotFound(error)) {
      return undefined;
    }
    throw error;
  }
  const parsed: unknown = JSON.parse(contents);
  if (
    !isRecord(parsed)
    || parsed.version !== 1
    || typeof parsed.installation_id !== "string"
    || typeof parsed.created_at !== "string"
  ) {
    throw new Error(`${installPath} is not a valid Runx install state.`);
  }
  return {
    version: 1,
    installation_id: parsed.installation_id,
    created_at: parsed.created_at,
  };
}

export async function ensureRunxInstallState(
  globalHomeDir: string,
  now: () => string = () => new Date().toISOString(),
): Promise<{ readonly state: RunxInstallState; readonly created: boolean }> {
  const existing = await readRunxInstallState(globalHomeDir);
  if (existing) {
    return {
      state: existing,
      created: false,
    };
  }
  const state: RunxInstallState = {
    version: 1,
    installation_id: `inst_${randomUUID()}`,
    created_at: now(),
  };
  await mkdir(globalHomeDir, { recursive: true });
  await writeFile(
    path.join(globalHomeDir, "install.json"),
    `${JSON.stringify(state, null, 2)}\n`,
    { mode: 0o600 },
  );
  return {
    state,
    created: true,
  };
}
