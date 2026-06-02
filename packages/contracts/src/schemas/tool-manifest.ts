import type { DeepReadonly, JsonSchema, UnknownRecord } from "../internal.js";
import { runxSchemaArtifacts } from "../schema-artifacts.js";

export type ToolManifestSourceTypeContract = "cli-tool" | "mcp" | "a2a" | "catalog" | "http";
export type ToolCommandInputModeContract = "args" | "stdin" | "none";

export type ToolManifestHttpSourceContract = DeepReadonly<{
  url: string;
  method?: string;
  headers?: Readonly<Record<string, string>>;
  allow_private_network?: boolean;
}>;

export type ToolManifestSourceContract = DeepReadonly<{
  type: ToolManifestSourceTypeContract;
  command?: string;
  args?: readonly string[];
  cwd?: string;
  input_mode?: ToolCommandInputModeContract;
  sandbox?: UnknownRecord;
  server?: string;
  catalog_ref?: string;
  tool?: string;
  arguments?: UnknownRecord;
  agent_card_url?: string;
  agent_identity?: string;
  http?: ToolManifestHttpSourceContract;
}>;

export type ToolManifestRuntimeContract = DeepReadonly<{
  command: string;
  args?: readonly string[];
  cwd?: string;
  env?: Readonly<Record<string, string>>;
}>;

export type ToolManifestInputContract = DeepReadonly<{
  type: string;
  required: boolean;
  description?: string;
  default?: unknown;
}>;

export type ToolManifestOutputContract = DeepReadonly<{
  packet?: string;
  wrap_as?: string;
} & UnknownRecord>;

export type ToolRetryPolicyContract = DeepReadonly<{
  max_attempts: number;
}>;

export type ToolIdempotencyPolicyContract = DeepReadonly<{
  key?: string;
}>;

export type ToolManifestContract = DeepReadonly<{
  schema: "runx.tool.manifest.v1";
  name: string;
  version?: string;
  description?: string;
  source: ToolManifestSourceContract;
  inputs?: Readonly<Record<string, ToolManifestInputContract>>;
  scopes?: readonly string[];
  risk?: unknown;
  runx?: UnknownRecord;
  runtime: ToolManifestRuntimeContract;
  output: ToolManifestOutputContract;
  retry?: ToolRetryPolicyContract;
  idempotency?: ToolIdempotencyPolicyContract;
  mutating?: boolean;
  source_hash: string;
  schema_hash: string;
  toolkit_version?: string;
}>;

export const toolManifestV1Schema = runxSchemaArtifacts[
  "tool-manifest.schema.json"
] as JsonSchema<ToolManifestContract>;
