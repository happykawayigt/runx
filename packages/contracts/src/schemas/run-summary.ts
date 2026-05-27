import {
  RUNX_LOGICAL_SCHEMAS,
  type DeepReadonly,
  type UnknownRecord,
  generatedSchema,
} from "../internal.js";
import type { DevStatusContract } from "./dev.js";

export type RunSummaryContract = DeepReadonly<{
  schema: typeof RUNX_LOGICAL_SCHEMAS.runSummary;
  run_id: string;
  command: string;
  status: DevStatusContract;
  started_at: string;
  finished_at?: string;
  root: string;
  unit?: UnknownRecord;
  steps: readonly UnknownRecord[];
  receipt_ref?: string;
}>;

export const runSummaryV1Schema = generatedSchema<RunSummaryContract>("run-summary.schema.json");
