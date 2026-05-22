import { Type, type Static, type TSchema } from "@sinclair/typebox";
import { Value } from "@sinclair/typebox/value";

export const JSON_SCHEMA_DRAFT_2020_12 = "https://json-schema.org/draft/2020-12/schema" as const;

export const RUNX_SCHEMA_BASE_URL = "https://schemas.runx.dev" as const;

export const RUNX_CONTRACT_IDS = {
  doctor: `${RUNX_SCHEMA_BASE_URL}/runx/doctor/v1.json`,
  dev: `${RUNX_SCHEMA_BASE_URL}/runx/dev/v1.json`,
  list: `${RUNX_SCHEMA_BASE_URL}/runx/list/v1.json`,
  runSummary: `${RUNX_SCHEMA_BASE_URL}/runx/run-summary/v1.json`,
  receipt: `${RUNX_SCHEMA_BASE_URL}/runx/receipt/v1.json`,
  fixture: `${RUNX_SCHEMA_BASE_URL}/runx/fixture/v1.json`,
  toolManifest: `${RUNX_SCHEMA_BASE_URL}/runx/tool/manifest/v1.json`,
  packetIndex: `${RUNX_SCHEMA_BASE_URL}/runx/packet/index/v1.json`,
  actAssignment: `${RUNX_SCHEMA_BASE_URL}/runx/act-assignment/v1.json`,
  externalAdapterManifest: `${RUNX_SCHEMA_BASE_URL}/runx/external-adapter/manifest/v1.json`,
  externalAdapterInvocation: `${RUNX_SCHEMA_BASE_URL}/runx/external-adapter/invocation/v1.json`,
  externalAdapterResponse: `${RUNX_SCHEMA_BASE_URL}/runx/external-adapter/response/v1.json`,
  externalAdapterHostResolution: `${RUNX_SCHEMA_BASE_URL}/runx/external-adapter/host-resolution/v1.json`,
  externalAdapterCancellation: `${RUNX_SCHEMA_BASE_URL}/runx/external-adapter/cancellation/v1.json`,
  externalAdapterCredentialRequest: `${RUNX_SCHEMA_BASE_URL}/runx/external-adapter/credential-request/v1.json`,
  credentialDeliveryProfile: `${RUNX_SCHEMA_BASE_URL}/runx/credential-delivery/profile/v1.json`,
  credentialDeliveryRequest: `${RUNX_SCHEMA_BASE_URL}/runx/credential-delivery/request/v1.json`,
  credentialDeliveryBrokerResponse: `${RUNX_SCHEMA_BASE_URL}/runx/credential-delivery/broker-response/v1.json`,
  credentialDeliveryObservation: `${RUNX_SCHEMA_BASE_URL}/runx/credential-delivery/observation/v1.json`,
  threadOutboxProviderManifest: `${RUNX_SCHEMA_BASE_URL}/runx/thread-outbox-provider/manifest/v1.json`,
  threadOutboxProviderPush: `${RUNX_SCHEMA_BASE_URL}/runx/thread-outbox-provider/push/v1.json`,
  threadOutboxProviderFetch: `${RUNX_SCHEMA_BASE_URL}/runx/thread-outbox-provider/fetch/v1.json`,
  threadOutboxProviderObservation: `${RUNX_SCHEMA_BASE_URL}/runx/thread-outbox-provider/observation/v1.json`,
  reference: `${RUNX_SCHEMA_BASE_URL}/runx/reference/v1.json`,
  authority: `${RUNX_SCHEMA_BASE_URL}/runx/authority/v1.json`,
  authoritySubsetProof: `${RUNX_SCHEMA_BASE_URL}/runx/authority/subset-proof/v1.json`,
  signal: `${RUNX_SCHEMA_BASE_URL}/runx/signal/v1.json`,
  decision: `${RUNX_SCHEMA_BASE_URL}/runx/decision/v1.json`,
  act: `${RUNX_SCHEMA_BASE_URL}/runx/act/v1.json`,
  verification: `${RUNX_SCHEMA_BASE_URL}/runx/verification/v1.json`,
  harness: `${RUNX_SCHEMA_BASE_URL}/runx/harness/v1.json`,
  target: `${RUNX_SCHEMA_BASE_URL}/runx/target/v1.json`,
  opportunity: `${RUNX_SCHEMA_BASE_URL}/runx/opportunity/v1.json`,
  thesisAssessment: `${RUNX_SCHEMA_BASE_URL}/runx/thesis-assessment/v1.json`,
  selection: `${RUNX_SCHEMA_BASE_URL}/runx/selection/v1.json`,
  skillBinding: `${RUNX_SCHEMA_BASE_URL}/runx/skill-binding/v1.json`,
  targetTransitionEntry: `${RUNX_SCHEMA_BASE_URL}/runx/target-transition-entry/v1.json`,
  selectionCycle: `${RUNX_SCHEMA_BASE_URL}/runx/selection-cycle/v1.json`,
  reflectionEntry: `${RUNX_SCHEMA_BASE_URL}/runx/reflection-entry/v1.json`,
  feedEntry: `${RUNX_SCHEMA_BASE_URL}/runx/feed-entry/v1.json`,
  artifact: `${RUNX_SCHEMA_BASE_URL}/runx/artifact/v1.json`,
  redaction: `${RUNX_SCHEMA_BASE_URL}/runx/redaction/v1.json`,
  ledgerEntry: `${RUNX_SCHEMA_BASE_URL}/runx/ledger-entry/v1.json`,
  handoffSignal: `${RUNX_SCHEMA_BASE_URL}/runx/handoff-signal/v1.json`,
  handoffState: `${RUNX_SCHEMA_BASE_URL}/runx/handoff-state/v1.json`,
  suppressionRecord: `${RUNX_SCHEMA_BASE_URL}/runx/suppression-record/v1.json`,
  operationalPolicy: `${RUNX_SCHEMA_BASE_URL}/runx/operational-policy/v1.json`,
} as const;

export const RUNX_LOGICAL_SCHEMAS = {
  doctor: "runx.doctor.v1",
  dev: "runx.dev.v1",
  list: "runx.list.v1",
  runSummary: "runx.run-summary.v1",
  receipt: "runx.receipt.v1",
  fixture: "runx.fixture.v1",
  toolManifest: "runx.tool.manifest.v1",
  packetIndex: "runx.packet.index.v1",
  actAssignment: "runx.act_assignment.v1",
  externalAdapterManifest: "runx.external_adapter.manifest.v1",
  externalAdapterInvocation: "runx.external_adapter.invocation.v1",
  externalAdapterResponse: "runx.external_adapter.response.v1",
  externalAdapterHostResolution: "runx.external_adapter.host_resolution.v1",
  externalAdapterCancellation: "runx.external_adapter.cancellation.v1",
  externalAdapterCredentialRequest: "runx.external_adapter.credential_request.v1",
  credentialDeliveryProfile: "runx.credential_delivery.profile.v1",
  credentialDeliveryRequest: "runx.credential_delivery.request.v1",
  credentialDeliveryBrokerResponse: "runx.credential_delivery.broker_response.v1",
  credentialDeliveryObservation: "runx.credential_delivery.observation.v1",
  threadOutboxProviderManifest: "runx.thread_outbox_provider.manifest.v1",
  threadOutboxProviderPush: "runx.thread_outbox_provider.push.v1",
  threadOutboxProviderFetch: "runx.thread_outbox_provider.fetch.v1",
  threadOutboxProviderObservation: "runx.thread_outbox_provider.observation.v1",
  reference: "runx.reference.v1",
  authority: "runx.authority.v1",
  authoritySubsetProof: "runx.authority_subset_proof.v1",
  signal: "runx.signal.v1",
  decision: "runx.decision.v1",
  act: "runx.act.v1",
  verification: "runx.verification.v1",
  harness: "runx.harness.v1",
  target: "runx.target.v1",
  opportunity: "runx.opportunity.v1",
  thesisAssessment: "runx.thesis_assessment.v1",
  selection: "runx.selection.v1",
  skillBinding: "runx.skill_binding.v1",
  targetTransitionEntry: "runx.target_transition_entry.v1",
  selectionCycle: "runx.selection_cycle.v1",
  reflectionEntry: "runx.reflection_entry.v1",
  feedEntry: "runx.feed_entry.v1",
  artifact: "runx.artifact.v1",
  redaction: "runx.redaction.v1",
  ledgerEntry: "runx.ledger.entry.v1",
  handoffSignal: "runx.handoff_signal.v1",
  handoffState: "runx.handoff_state.v1",
  suppressionRecord: "runx.suppression_record.v1",
  operationalPolicy: "runx.operational_policy.v1",
} as const;

export const RUNX_CONTROL_SCHEMA_REFS = {
  output: "https://runx.ai/spec/output.schema.json",
  agent_context_envelope: "https://runx.ai/spec/agent-context-envelope.schema.json",
  agent_act_invocation: "https://runx.ai/spec/agent-act-invocation.schema.json",
  question: "https://runx.ai/spec/question.schema.json",
  approval_gate: "https://runx.ai/spec/approval-gate.schema.json",
  resolution_request: "https://runx.ai/spec/resolution-request.schema.json",
  resolution_response: "https://runx.ai/spec/resolution-response.schema.json",
  act_receipt: "https://runx.ai/spec/act-receipt.schema.json",
  credential_envelope: "https://runx.ai/spec/credential-envelope.schema.json",
  scope_admission: "https://runx.ai/spec/scope-admission.schema.json",
  authority_proof: "https://runx.ai/spec/authority-proof.schema.json",
} as const;

export const RUNX_AUXILIARY_SCHEMA_IDS = {
  registryBinding: "https://runx.ai/schemas/registry-binding.schema.json",
  reviewReceiptOutput: "https://runx.ai/schemas/review-receipt-output.schema.json",
} as const;

export type UnknownRecord = Readonly<Record<string, unknown>>;
export type DeepReadonly<T> =
  T extends (...args: never[]) => unknown ? T
    : T extends readonly (infer TValue)[] ? readonly DeepReadonly<TValue>[]
      : T extends (infer TValue)[] ? readonly DeepReadonly<TValue>[]
        : T extends object ? { readonly [TKey in keyof T]: DeepReadonly<T[TKey]> }
          : T;

export function stringEnum<const TValue extends readonly string[]>(
  values: TValue,
  options: Record<string, unknown> = {},
) {
  const properties = Object.fromEntries(
    values.map((value) => [value, Type.Null()]),
  ) as Record<TValue[number], ReturnType<typeof Type.Null>>;
  return Type.KeyOf(
    Type.Object(properties, { additionalProperties: false }),
    options,
  );
}

export function unknownRecordSchema(options: Record<string, unknown> = {}) {
  return Type.Record(Type.String(), Type.Unknown(), options);
}

export function dateTimeStringSchema(options: Record<string, unknown> = {}) {
  return Type.String({
    minLength: 1,
    pattern: "^\\d{4}-\\d{2}-\\d{2}T\\d{2}:\\d{2}:\\d{2}(?:\\.\\d+)?Z$",
    ...options,
  });
}

export function validateContractSchema<TSchemaValue extends TSchema>(
  schema: TSchemaValue,
  value: unknown,
  label: string,
  references: readonly TSchema[] = [],
): Static<TSchemaValue> {
  const normalizedReferences = [...references];
  const matches = normalizedReferences.length > 0
    ? Value.Check(schema, normalizedReferences, value)
    : Value.Check(schema, value);
  if (matches) {
    return value as Static<TSchemaValue>;
  }
  const firstError = normalizedReferences.length > 0
    ? [...Value.Errors(schema, normalizedReferences, value)][0]
    : [...Value.Errors(schema, value)][0];
  const schemaRef = typeof schema.$id === "string" ? schema.$id : "contract schema";
  const path = firstError?.path ? `${label}${formatSchemaErrorPath(firstError.path)}` : label;
  throw new Error(`${path} must match ${schemaRef}.`);
}

export function formatSchemaErrorPath(path: string): string {
  const segments = path.split("/").filter((segment) => segment.length > 0);
  return segments.map((segment) => {
    const decoded = segment.replace(/~1/g, "/").replace(/~0/g, "~");
    return /^\d+$/u.test(decoded) ? `[${decoded}]` : `.${decoded}`;
  }).join("");
}

export function asUnknownRecord(value: unknown): UnknownRecord | undefined {
  return value && typeof value === "object" && !Array.isArray(value) ? value as UnknownRecord : undefined;
}
