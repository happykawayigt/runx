import {
  RUNX_LOGICAL_SCHEMAS,
  type DeepReadonly,
  type UnknownRecord,
  generatedSchema,
  generatedSchemaAt,
  validateContractSchema,
} from "../internal.js";

export type DoctorDiagnosticSeverityContract = "error" | "warning" | "info";
export type DoctorRepairKindContract =
  | "create_file"
  | "replace_file"
  | "edit_yaml"
  | "edit_json"
  | "add_fixture"
  | "run_command"
  | "manual";
export type DoctorRepairConfidenceContract = "low" | "medium" | "high";
export type DoctorRepairRiskContract = "low" | "medium" | "high" | "sensitive";

export type DoctorRepairContract = DeepReadonly<{
  id: string;
  kind: DoctorRepairKindContract;
  confidence: DoctorRepairConfidenceContract;
  risk: DoctorRepairRiskContract;
  path?: string;
  json_pointer?: string;
  contents?: string;
  patch?: string;
  command?: string;
  requires_human_review: boolean;
}>;

export type DoctorLocationContract = DeepReadonly<{
  path: string;
  json_pointer?: string;
}>;

export type DoctorDiagnosticContract = DeepReadonly<{
  id: string;
  instance_id: string;
  severity: DoctorDiagnosticSeverityContract;
  title: string;
  message: string;
  target: UnknownRecord;
  location: DoctorLocationContract;
  evidence?: UnknownRecord;
  repairs: readonly DoctorRepairContract[];
}>;

export type DoctorSummaryContract = DeepReadonly<{
  errors: number;
  warnings: number;
  infos: number;
}>;

export type DoctorReportContract = DeepReadonly<{
  schema: typeof RUNX_LOGICAL_SCHEMAS.doctor;
  status: "success" | "failure";
  summary: DoctorSummaryContract;
  diagnostics: readonly DoctorDiagnosticContract[];
}>;

export const doctorV1Schema = generatedSchema<DoctorReportContract>("doctor.schema.json");
export const doctorDiagnosticSchema = generatedSchemaAt<DoctorDiagnosticContract>(
  doctorV1Schema,
  ["properties", "diagnostics", "items"],
  "doctor.diagnostics[]",
);
export const doctorLocationSchema = generatedSchemaAt<DoctorLocationContract>(
  doctorDiagnosticSchema,
  ["properties", "location"],
  "doctor.diagnostics[].location",
);
export const doctorRepairSchema = generatedSchemaAt<DoctorRepairContract>(
  doctorDiagnosticSchema,
  ["properties", "repairs", "items"],
  "doctor.diagnostics[].repairs[]",
);
export const doctorSummarySchema = generatedSchemaAt<DoctorSummaryContract>(
  doctorV1Schema,
  ["properties", "summary"],
  "doctor.summary",
);

export function validateDoctorReportContract(value: unknown, label = "doctor_report"): DoctorReportContract {
  return validateContractSchema(doctorV1Schema, value, label);
}
