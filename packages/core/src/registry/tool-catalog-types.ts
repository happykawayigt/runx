import type { SkillInput } from "../parser/index.js";

export interface RemoteToolCatalogSearchResult {
  readonly tool_id: string;
  readonly name: string;
  readonly summary?: string;
  readonly source: string;
  readonly source_label: string;
  readonly source_type: string;
  readonly namespace: string;
  readonly external_name: string;
  readonly required_scopes: readonly string[];
  readonly tags: readonly string[];
  readonly catalog_ref: string;
}

export interface RemoteToolInspectProvenance {
  readonly origin: "local" | "imported";
  readonly source?: string;
  readonly source_label?: string;
  readonly source_type?: string;
  readonly namespace?: string;
  readonly external_name?: string;
  readonly catalog_ref?: string;
  readonly tool_id?: string;
  readonly tags?: readonly string[];
}

export interface RemoteToolInspectResult {
  readonly ref: string;
  readonly name: string;
  readonly description?: string;
  readonly execution_source_type: string;
  readonly inputs: Readonly<Record<string, SkillInput>>;
  readonly scopes: readonly string[];
  readonly mutating?: boolean;
  readonly runtime?: unknown;
  readonly risk?: unknown;
  readonly runx?: Record<string, unknown>;
  readonly reference_path: string;
  readonly skill_directory: string;
  readonly provenance: RemoteToolInspectProvenance;
}
