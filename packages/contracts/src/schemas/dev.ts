import {
  RUNX_LOGICAL_SCHEMAS,
  type DeepReadonly,
  type UnknownRecord,
  generatedSchema,
  generatedSchemaAt,
  validateContractSchema,
} from "../internal.js";
import type { DoctorReportContract } from "./doctor.js";

export const devStatuses = ["success", "failure", "skipped", "needs_approval"] as const;
export type DevStatusContract = (typeof devStatuses)[number];
export type DevFixtureAssertionKindContract =
  | "subset_miss"
  | "exact_mismatch"
  | "packet_invalid"
  | "status_mismatch"
  | "type_mismatch";

export type DevFixtureAssertionContract = DeepReadonly<{
  path: string;
  expected?: unknown;
  actual?: unknown;
  kind: DevFixtureAssertionKindContract;
  message: string;
}>;

export type DevFixtureResultContract = DeepReadonly<{
  name: string;
  lane: string;
  target: UnknownRecord;
  status: "success" | "failure" | "skipped";
  duration_ms: number;
  assertions: readonly DevFixtureAssertionContract[];
  skip_reason?: string;
  output?: unknown;
  replay_path?: string;
}>;

export type DevReportContract = DeepReadonly<{
  schema: typeof RUNX_LOGICAL_SCHEMAS.dev;
  status: DevStatusContract;
  doctor: DoctorReportContract;
  fixtures: readonly DevFixtureResultContract[];
  receipt_id?: string;
}>;

export const devV1Schema = generatedSchema<DevReportContract>("dev.schema.json");
export const devFixtureResultSchema = generatedSchemaAt<DevFixtureResultContract>(
  devV1Schema,
  ["properties", "fixtures", "items"],
  "dev.fixtures[]",
);
export const devFixtureAssertionSchema = generatedSchemaAt<DevFixtureAssertionContract>(
  devFixtureResultSchema,
  ["properties", "assertions", "items"],
  "dev.fixtures[].assertions[]",
);

export function validateDevReportContract(value: unknown, label = "dev_report"): DevReportContract {
  return validateContractSchema(devV1Schema, value, label);
}
