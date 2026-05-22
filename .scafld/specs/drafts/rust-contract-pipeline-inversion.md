---
spec_version: '2.0'
task_id: rust-contract-pipeline-inversion
created: '2026-05-21T23:10:00Z'
updated: '2026-05-21T23:10:00Z'
status: draft
harden_status: not_run
size: large
risk_level: high
---

# Rust contract pipeline inversion

## Current State

Status: draft
Current phase: planning
Next: harden
Reason: post-cutover direction. The contract spine is currently TypeScript-first
(TypeBox generates `oss/schemas/*.json`, Rust hand-mirrors ~250 types). Once Rust
is the authoritative runtime, the source of truth should invert so the hand
mirroring stops being a permanent parity tax.
Blockers: `rust-contract-schema-validation-gate` must first prove the Rust types
match the generated schemas; the `rust-ts-sunset-*` specs must land so TypeScript
is no longer the authoritative runtime.
Allowed follow-up command: `scafld harden rust-contract-pipeline-inversion`
Latest runner update: 2026-05-21T23:10:00Z
Review gate: not_started

## Summary

Invert the contract source of truth from TypeScript (TypeBox) to Rust once the
Rust runtime is authoritative. Today the chain is one-directional with a gap:
TypeBox in `oss/packages/contracts/src/schemas/*.ts` is the oracle,
`oss/scripts/generate-contract-schemas.ts` generates `oss/schemas/*.json` under a
`--check` CI gate, and `oss/crates/runx-contracts` is a hand-written
reimplementation (~250 `pub struct`/`enum`, ~969 fields) policed only by example
fixtures. The end state: Rust contract types become the declarative source, emit
JSON Schema, and generate the published TypeScript types that surviving TS
consumers import. The hand-written TypeBox schemas are deleted.

This is a direction/sequencing spec, not a redesign. It does not change wire
shapes; it changes which representation is hand-authored and which is generated.

## Context

- The contract-spine duplication is a three-representation parity burden: JSON
  Schema (generated), TypeBox (authored), Rust structs (authored). Two are
  hand-maintained against each other.
- The mechanical mirroring is exactly what codegen removes: 54 `rename_all`
  attributes, 28 per-field `rename`, `Option<T>` + `skip_serializing_if` vs
  `Type.Optional`, `Vec<T>` + `#[serde(default)]` vs `Type.Array` defaulting.
- `rust-contract-schema-validation-gate` already lists pipeline inversion as
  explicitly out of scope; this spec owns it so the direction is not lost.
- Sequencing is load-bearing: inverting before Rust is authoritative would
  invert the parity tax onto the still-authoritative TS side. This must run
  after the sunset specs, not before.

## Objectives

- Choose the Rust-to-schema mechanism. A feasibility spike (2026-05-22) found
  vanilla `schemars` cannot reproduce the committed schemas (they are fully
  inlined — 1 `$ref` in 60 files — render enums as `anyOf` of `const`, and carry
  `x-runx-schema`/custom `$id`, none of which `schemars` emits), and `typify` is
  schema-to-Rust, the wrong direction. The realistic mechanism is a bespoke Rust
  schema emitter that controls shape directly. The invariant it must hold is
  **wire-compatibility, not byte-identity of the schema document** (see below).
- Express contract constraints at the Rust type level where the schemas do today
  only in JSON: model the ubiquitous `minLength:1` (2,382 occurrences) as a
  `NonEmptyString` newtype and the `const` discriminants as fixed/typed fields,
  so a contract type cannot be constructed in a shape its schema would reject.
- Flip the `--check` CI gate direction: Rust types generate JSON Schema; JSON
  Schema generates published TypeScript types for surviving consumers
  (`@runxhq/contracts`, `host-adapters`).
- Delete the hand-written TypeBox schemas once the generated TS types are the
  consumed artifact.
- Keep one canonicalization/fingerprint contract intact across the flip (depends
  on `canonical-json-fingerprint-contract-v1`).

## Scope

In scope:
- `runx-contracts` (and any contract types still living in `runx-core`, e.g.
  `AuthorityProof` if `rust-contract-schema-validation-gate` decides to relocate
  it) becoming schema-emitting.
- A generation + `--check` gate that replaces `generate-contract-schemas.ts`.
- Generating published TypeScript contract types for surviving TS consumers.

Out of scope:
- Changing any wire shape, casing, or optionality. Pure representation move.
- The runtime/CLI behavior. This is contracts-only.
- Sunsetting TS runtime packages (owned by `rust-ts-sunset-*`).

## Dependencies

- `rust-contract-schema-validation-gate` (Rust must demonstrably match the
  schemas first).
- `rust-ts-sunset-*` (TS must no longer be the authoritative runtime).
- `canonical-json-fingerprint-contract-v1` (the canonicalization byte contract
  must survive the flip unchanged).
- `ts-extension-survivorship-boundary` (defines which TS consumers still need
  generated contract types).

## Touchpoints

- `oss/crates/runx-contracts/src/*.rs`
- `oss/crates/runx-core/src/policy/types.rs` (if `AuthorityProof` relocates)
- `oss/scripts/generate-contract-schemas.ts` (replaced/inverted)
- `oss/packages/contracts/src/schemas/*.ts` (deleted once generated TS lands)
- `oss/schemas/*.json` (now a Rust-derived artifact)

## Risks

- Codegen drift: the Rust-emitted schema must accept/reject the same value domain
  as the current schemas, or downstream validators and cloud consumers break
  silently. Mitigate with a conformance-corpus wire-compatibility gate (not
  byte-equality) over the committed schemas before flipping.
- Schema-document consumers: a consumer that introspects schema *structure* or
  pins schema-document hashes (rather than validating data) could break on the
  idiomatic-shape change. Inventory these before the flip (dod6).
- Premature flip: running before the sunset specs lands the parity tax on the
  wrong side. Hard-gate on the dependencies above.
- Lost expressiveness: TypeBox constraints (formats, refinements) without a clean
  Rust equivalent must be inventoried; the bespoke emitter and the
  `NonEmptyString`/typed-discriminant model must cover the common cases
  (`minLength:1`, `const`) before commitment.

## Acceptance

- [ ] `dod1` The Rust-emitted JSON Schema is wire-compatible with every committed
  `oss/schemas/*.json` for the covered contract set: a conformance corpus
  validates identically (same accept/reject) against the prior and the
  Rust-emitted schema, and schema identity (`x-runx-schema`, `$id`, version) is
  preserved. The schema *document* shape may change to idiomatic JSON Schema;
  byte-identity is explicitly NOT required.
- [ ] `dod2` The `--check` gate is inverted: editing a Rust contract type and not
  regenerating fails CI; editing a hand-written TypeBox schema is no longer
  possible (files removed).
- [ ] `dod3` Surviving TS consumers (`@runxhq/contracts`, `host-adapters`) build
  against generated types, not hand-authored TypeBox.
- [ ] `dod4` `canonical-json-fingerprint-contract-v1` fixtures still pass
  unchanged across the flip.
- [ ] `dod5` No wire shape, casing, or optionality changed: serialized contract
  data is byte-identical across the flip and the canonical-json fixtures pass.
  (The schema document may change shape; only the validated value domain and the
  serialized wire bytes must be preserved.)
- [ ] `dod6` Before flipping, any consumer that structurally introspects schema
  documents or pins schema-document hashes (vs validating data) is inventoried;
  none break on the idiomatic-shape change.

## Phase 1: Emitter + Wire-Compatibility Drift Detector

Stand up the bespoke Rust schema emitter (the spike ruled out vanilla `schemars`
and `typify`) behind a non-authoritative check that asserts **canonical /
wire-compatibility equality** against the committed `oss/schemas/*.json`: a
conformance corpus must validate identically against the prior and the
Rust-emitted schema, and schema identity must be preserved. This is explicitly
NOT byte-equality of the schema document. Land the `NonEmptyString` /
typed-discriminant constraint model so the constraints become type-level.
Inventory any TypeBox constraint (format, pattern, refinement) without a clean
Rust equivalent. No source-of-truth change yet. This phase is safe to land
independently as a drift detector even if the full inversion stays deferred.

## Phase 2: Flip The Gate

Once byte-equality holds and the sunset dependencies have landed, make Rust the
source: generate JSON Schema from Rust, generate TS types from JSON Schema, and
point surviving TS consumers at the generated types.

## Phase 3: Delete TypeBox

Remove the hand-written TypeBox schemas and the old TS-first generation script.
The contract spine is now single-source from Rust.

## Rollback

The flip is gated on byte-equality; if generated schemas diverge from committed
schemas at any phase, do not flip. Phase 1 is non-authoritative and safe to land
independently as a drift detector even if the full inversion is deferred.

## Origin

User architecture review on 2026-05-21: the TS-first contract spine forces a
permanent hand-mirroring tax onto Rust (~250 types, ~969 fields) and the Rust
side is currently policed only by example fixtures. Once Rust is authoritative,
the source of truth should invert. Captured as the one cross-language abstraction
item not already owned by an existing spec.
