import { Type, type Static } from "@sinclair/typebox";
import {
  JSON_SCHEMA_DRAFT_2020_12,
  RUNX_CONTRACT_IDS,
  RUNX_LOGICAL_SCHEMAS,
  type DeepReadonly,
  stringEnum,
  unknownRecordSchema,
} from "../internal.js";
import { devStatuses } from "./dev.js";

const runSummaryStepSchema = unknownRecordSchema();

export const runSummaryV1Schema = Type.Object(
  {
    schema: Type.Literal(RUNX_LOGICAL_SCHEMAS.runSummary),
    run_id: Type.String(),
    command: Type.String(),
    status: stringEnum(devStatuses),
    started_at: Type.String(),
    finished_at: Type.Optional(Type.String()),
    root: Type.String(),
    unit: Type.Optional(unknownRecordSchema()),
    steps: Type.Array(runSummaryStepSchema),
    // Projection beside the governance receipt, not a competing receipt: links a
    // CLI run summary to the signed runx.receipt.v1 it summarizes.
    receipt_ref: Type.Optional(Type.String()),
  },
  {
    $schema: JSON_SCHEMA_DRAFT_2020_12,
    $id: RUNX_CONTRACT_IDS.runSummary,
    "x-runx-schema": RUNX_LOGICAL_SCHEMAS.runSummary,
    additionalProperties: false,
  },
);

export type RunSummaryContract = DeepReadonly<Static<typeof runSummaryV1Schema>>;
