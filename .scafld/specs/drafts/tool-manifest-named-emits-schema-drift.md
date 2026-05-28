---
spec_version: '2.0'
task_id: tool-manifest-named-emits-schema-drift
created: '2026-05-28T23:55:00Z'
updated: '2026-05-28T23:55:00Z'
status: draft
harden_status: not_run
size: small
risk_level: low
---

# Tool manifest schema drift: `named_emits` and `artifact`

## Current State

Status: draft
Next: extend `ToolManifest` / `ToolOutput` Rust contract to model `named_emits` and `artifact` fields
Reason: discovered during A-cutover A1. `doctor --json` reports four stale-asset
errors on origin/main; the underlying cause is that tool manifests
(`tools/outbox/build_feed_entry/manifest.json`,
`tools/outbox/build_pull_request/manifest.json`,
`tools/thread/push_outbox/manifest.json`) use `named_emits` and `artifact`
fields that the Rust `runx-contracts::tools::ToolManifest` parser rejects with
`unknown field`. Runtime code (`runx-runtime/src/list.rs`, `adapters/catalog.rs`)
and integration tests reference `named_emits`, so the field is a real runtime
feature; the contract layer just hasn't modeled it yet.

## Symptoms

`pnpm exec tsx packages/cli/src/index.ts doctor --json` exits non-zero with:

- `runx.skill.lock.stale` on `packages/cli/src/official-skills.lock.json`
- `runx.tool.manifest.stale` on `tools/outbox/build_feed_entry/manifest.json`
- `runx.tool.manifest.stale` on `tools/outbox/build_pull_request/manifest.json`
- `runx.tool.manifest.stale` on `tools/thread/push_outbox/manifest.json`

The repair commands (`runx tool build <path>`) fail closed:
`parsing tool manifest <path>: unknown field 'named_emits' at line 87 column 3`.

## Root cause

`crates/runx-contracts/src/tools.rs` `ToolManifest`/`ToolOutput` carries
`outputs: BTreeMap<String, ToolOutput>` and `artifacts: ...` but does not
model the `named_emits` map that the manifests on disk declare, nor the
`artifact` field on input definitions. The runtime reads `named_emits` from a
loosely-typed JSON layer, but `runx tool build` re-parses the manifest through
`ToolManifest` and fails closed.

## Fix

Extend the Rust contract to model `named_emits` and `artifact` fields. Schema
artifact, TypeScript schema, and the wire-conformance corpus must be updated
together. Once the contract accepts the on-disk manifests, the doctor
diagnostic resolves to one regenerate command per manifest.

## Out of scope

- Changing how `named_emits` is interpreted by the runtime.
- Migrating manifests away from `named_emits` (they're a real feature).

## Related

- A-cutover campaign closed at `c553caca` (2026-05-28).
- Companion follow-up: `runtime-large-file-decomposition`.
