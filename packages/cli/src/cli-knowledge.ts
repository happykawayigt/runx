import { readFile } from "node:fs/promises";
import path from "node:path";

import { isNotFound, isRecord } from "./cli-util.js";

export interface LocalKnowledgeProjectionEntry {
  readonly entry_id: string;
  readonly entry_kind: "projection";
  readonly project: string;
  readonly scope: string;
  readonly key: string;
  readonly value: unknown;
  readonly source: string;
  readonly confidence: number;
  readonly freshness: string;
  readonly receipt_id?: string;
  readonly created_at: string;
}

export interface LocalKnowledgeStore {
  readonly listProjections: (filter?: { readonly project?: string }) => Promise<readonly LocalKnowledgeProjectionEntry[]>;
}

export function createFileKnowledgeStore(knowledgeDir: string): LocalKnowledgeStore {
  const indexPath = path.join(knowledgeDir, "index.json");

  return {
    listProjections: async (filter) => {
      const projections = await readProjectionEntries(indexPath);
      const project = filter?.project;
      return project ? projections.filter((projection) => path.resolve(projection.project) === path.resolve(project)) : projections;
    },
  };
}

async function readProjectionEntries(indexPath: string): Promise<readonly LocalKnowledgeProjectionEntry[]> {
  let parsed: unknown;
  try {
    parsed = JSON.parse(await readFile(indexPath, "utf8")) as unknown;
  } catch (error) {
    if (isNotFound(error)) {
      return [];
    }
    throw error;
  }

  if (!isRecord(parsed) || parsed.schema_version !== "runx.knowledge.v1" || !Array.isArray(parsed.entries)) {
    return [];
  }
  return parsed.entries.filter(isLocalKnowledgeProjectionEntry);
}

function isLocalKnowledgeProjectionEntry(value: unknown): value is LocalKnowledgeProjectionEntry {
  return isRecord(value)
    && value.entry_kind === "projection"
    && typeof value.entry_id === "string"
    && typeof value.project === "string"
    && typeof value.scope === "string"
    && typeof value.key === "string"
    && typeof value.source === "string"
    && typeof value.confidence === "number"
    && typeof value.freshness === "string"
    && typeof value.created_at === "string";
}
