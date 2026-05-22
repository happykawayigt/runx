import { Type, type Static } from "@sinclair/typebox";
import {
  JSON_SCHEMA_DRAFT_2020_12,
  RUNX_CONTRACT_IDS,
  RUNX_LOGICAL_SCHEMAS,
  type DeepReadonly,
  dateTimeStringSchema,
  stringEnum,
  unknownRecordSchema,
  validateContractSchema,
} from "../internal.js";
import {
  actFormSchema,
  authorityTermSchema,
  criterionStatusSchema,
  fanoutReceiptSyncPointSchema,
  harnessAuthorityAttenuationSchema,
  receiptIssuerSchema,
  receiptSignatureSchema,
  redactionCommitmentAlgorithmSchema,
  referenceSchema,
} from "./spine.js";

/**
 * runx.receipt.v1 — the single signed governance receipt.
 *
 * One flat artifact, each top-level key answering one question: integrity
 * (envelope), dedup (idempotency), what ran (subject), what allowed it
 * (authority), what was done (acts), the outcome (seal), and graph/resume
 * lineage. Planner deliberation and full per-act detail are referenced
 * (`lineage.journal_ref`, `acts[].detail_ref`), not inlined. Verification is a
 * read-time projection, never part of the signed body.
 */

/** The canonicalization byte contract this receipt's digest commits under. */
export const RECEIPT_CANONICALIZATION = "runx.receipt.c14n.v1" as const;

const nonEmptyString = () => Type.String({ minLength: 1 });

/** Scoped byte commitment; unifies the old hash_commitments + enforcement.std*_hash. */
export const receiptCommitmentSchema = Type.Object(
  {
    scope: stringEnum(["input", "output", "stdout", "stderr", "error"] as const),
    algorithm: redactionCommitmentAlgorithmSchema,
    value: nonEmptyString(),
    canonicalization: nonEmptyString(),
  },
  { additionalProperties: false },
);

export const receiptSubjectSchema = Type.Object(
  {
    kind: stringEnum(["skill", "graph"] as const),
    ref: referenceSchema,
    commitments: Type.Array(receiptCommitmentSchema),
  },
  { additionalProperties: false },
);

/** Enforcement profile is hashed; the granted authority stays readable. */
export const receiptEnforcementSchema = Type.Object(
  {
    profile_hash: nonEmptyString(),
    redaction_refs: Type.Array(referenceSchema),
    setup_refs: Type.Array(referenceSchema),
    teardown_refs: Type.Array(referenceSchema),
  },
  { additionalProperties: false },
);

export const receiptAuthoritySchema = Type.Object(
  {
    actor_ref: referenceSchema,
    grant_refs: Type.Array(referenceSchema),
    scope_refs: Type.Array(referenceSchema),
    authority_proof_refs: Type.Array(referenceSchema),
    attenuation: harnessAuthorityAttenuationSchema,
    mandate_ref: Type.Optional(referenceSchema),
    terms: Type.Array(authorityTermSchema),
    enforcement: receiptEnforcementSchema,
  },
  { additionalProperties: false },
);

export const receiptIdempotencySchema = Type.Object(
  {
    intent_key: nonEmptyString(),
    trigger_fingerprint: nonEmptyString(),
    content_hash: nonEmptyString(),
  },
  { additionalProperties: false },
);

/** Runner provenance for agent acts (drives the trainable-export projection). */
export const receiptRunnerProvenanceSchema = Type.Object(
  {
    provider: Type.Union([Type.String(), Type.Null()]),
    model: Type.Union([Type.String(), Type.Null()]),
    prompt_version: Type.Union([Type.String(), Type.Null()]),
  },
  { additionalProperties: false },
);

/** Result binding only: criterion_id -> status. The skill declares the criteria. */
export const receiptCriterionSchema = Type.Object(
  {
    criterion_id: nonEmptyString(),
    status: criterionStatusSchema,
    evidence_refs: Type.Array(referenceSchema),
    verification_refs: Type.Array(referenceSchema),
    summary: Type.Optional(nonEmptyString()),
  },
  { additionalProperties: false },
);

export const receiptActSchema = Type.Object(
  {
    id: nonEmptyString(),
    form: actFormSchema,
    summary: nonEmptyString(),
    criteria: Type.Array(receiptCriterionSchema),
    by: Type.Optional(receiptRunnerProvenanceSchema),
    artifact_refs: Type.Array(referenceSchema),
    // Full intent/target/source/surface refs and form-specific bodies live here.
    detail_ref: Type.Optional(referenceSchema),
  },
  { additionalProperties: false },
);

/** Exactly one seal. `deferred` expresses a suspended (waiting/delegated) run. */
export const receiptSealSchema = Type.Object(
  {
    disposition: stringEnum([
      "closed",
      "deferred",
      "superseded",
      "declined",
      "blocked",
      "failed",
      "killed",
      "timed_out",
    ] as const),
    reason_code: nonEmptyString(),
    summary: nonEmptyString(),
    closed_at: dateTimeStringSchema(),
    criteria: Type.Array(receiptCriterionSchema),
  },
  { additionalProperties: false },
);

export const receiptLineageSchema = Type.Object(
  {
    parent: Type.Optional(referenceSchema),
    previous: Type.Optional(referenceSchema),
    children: Type.Array(referenceSchema),
    sync: Type.Array(fanoutReceiptSyncPointSchema),
    signal_refs: Type.Array(referenceSchema),
    // Commits the planner deliberation (former decisions[]) by reference.
    journal_ref: Type.Optional(referenceSchema),
    // Open resolution request when seal.disposition === "deferred".
    resume_ref: Type.Optional(referenceSchema),
  },
  { additionalProperties: false },
);

export const receiptV1Schema = Type.Object(
  {
    schema: Type.Literal(RUNX_LOGICAL_SCHEMAS.receipt),
    id: nonEmptyString(),
    created_at: dateTimeStringSchema(),
    canonicalization: nonEmptyString(),
    issuer: receiptIssuerSchema,
    signature: receiptSignatureSchema,
    digest: nonEmptyString(),
    idempotency: receiptIdempotencySchema,
    subject: receiptSubjectSchema,
    authority: receiptAuthoritySchema,
    acts: Type.Array(receiptActSchema),
    seal: receiptSealSchema,
    lineage: Type.Optional(receiptLineageSchema),
    // Runtime-local read aid (history projection labels); never part of the
    // signed body. The canonicalizer strips it before the digest.
    metadata: Type.Optional(unknownRecordSchema()),
  },
  {
    $schema: JSON_SCHEMA_DRAFT_2020_12,
    $id: RUNX_CONTRACT_IDS.receipt,
    "x-runx-schema": RUNX_LOGICAL_SCHEMAS.receipt,
    additionalProperties: false,
  },
);

export type ReceiptContract = DeepReadonly<Static<typeof receiptV1Schema>>;

export function validateReceiptContract(value: unknown, label = "receipt"): ReceiptContract {
  return validateContractSchema(receiptV1Schema, value, label);
}
