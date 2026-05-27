import {
  RUNX_LOGICAL_SCHEMAS,
  type DeepReadonly,
  generatedSchema,
  generatedSchemaAt,
} from "../internal.js";

export type PacketIndexEntryContract = DeepReadonly<{
  id: string;
  package: string;
  version: string;
  path: string;
  sha256: string;
}>;

export type PacketIndexContract = DeepReadonly<{
  schema: typeof RUNX_LOGICAL_SCHEMAS.packetIndex;
  packets: readonly PacketIndexEntryContract[];
}>;

export const packetIndexV1Schema = generatedSchema<PacketIndexContract>(
  "packet-index.schema.json",
);
export const packetIndexEntrySchema = generatedSchemaAt<PacketIndexEntryContract>(
  packetIndexV1Schema,
  ["properties", "packets", "items"],
  "packet-index.packets[]",
);
