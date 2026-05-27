import {
  type DeepReadonly,
  type UnknownRecord,
  generatedSchema,
} from "../internal.js";

export type FixtureLaneContract = "deterministic" | "agent" | "repo-integration";

export type FixtureContract = DeepReadonly<{
  name: string;
  lane: FixtureLaneContract;
  target: UnknownRecord;
  inputs?: UnknownRecord;
  env?: UnknownRecord;
  agent?: UnknownRecord;
  repo?: UnknownRecord;
  execution?: UnknownRecord;
  permissions?: UnknownRecord;
  expect: UnknownRecord;
}>;

export const fixtureV1Schema = generatedSchema<FixtureContract>("fixture.schema.json");
