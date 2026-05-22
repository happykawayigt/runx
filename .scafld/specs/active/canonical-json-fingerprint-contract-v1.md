---
spec_version: '2.0'
task_id: canonical-json-fingerprint-contract-v1
created: '2026-05-21T12:19:24Z'
updated: '2026-05-22T01:25:00Z'
status: active
harden_status: not_run
size: medium
risk_level: high
---

# Canonical JSON fingerprint contract v1

## Current State

Status: active
Current phase: Phase 4 cloud API/db, core ledger, bundled `push_outbox`, and
focused CLI/script canonical helper replacement are complete in the current
tree; remaining runtime-local/adapter/core legacy helper decisions still
pending
Next: decide broader runtime-local, adapter, and core legacy helper survivorship
cleanup in the owning slice
Reason: the shared `@runxhq/contracts` helper is now executable and verified,
and TypeScript receipt canonical JSON is pinned to Rust receipt canonicalization
through a shared oracle fixture. Cloud now uses the shared helper for stable
JSON and full `sha256:` commitments where harness routes had same-contract
hashers or truncated digest labels. Core ledger chain hashing now uses the same
helper for its `runx.stable-json.v1` label, and `push_outbox`/file-thread short
IDs are explicitly internal truncated fragments rather than `sha256:`
commitments.
Blockers: remaining phases still need non-cloud survivorship cleanup owned by
runtime-local, adapter, and core legacy helper follow-up specs.
Allowed follow-up command: `scafld handoff canonical-json-fingerprint-contract-v1`
Latest runner update: 2026-05-21T23:10:00+10:00 added
`packages/contracts/src/canonical-json.ts`, exported it from the package root,
and added fixtures under `fixtures/contracts/canonical-json/`. Focused tests,
typecheck, schema generation check, and spec validation passed.
Follow-up: 2026-05-21T13:21:35Z completed Phase 1 call-site/tag inventory.
The inventory classifies stable JSON, harness receipt, signal source event,
ledger, runtime-local stdout/stderr/input hashes, projection fingerprints,
`push_outbox` IDs, file hashes, and known truncated `sha256:` labels before any
helper rewiring.
Follow-up: 2026-05-21T13:39:10Z canonicalized
`fixtures/contracts/canonical-json/runx-stable-json-v1.cases.json` key order so
the global contract fixture key-order check passes.
Follow-up: 2026-05-21T13:59:43Z completed Phase 3 for covered
`fixtures/contracts/harness-spine/*` harness receipt fixtures by adding a shared
oracle under `fixtures/contracts/canonical-json/`; Rust asserts full/body
canonical JSON and digests against the oracle, and TypeScript compares
`canonicalJsonStringify(fixture.expected)` plus body-stripped digests against
the same oracle.
Follow-up: 2026-05-21T14:05:00Z fixed external-adapter fixture key order so the
global contract fixture key-order gate is green again.
Follow-up: 2026-05-22T00:12:23+10:00 completed the cloud API/db slice:
`cloud/packages/db/src/stable-json.ts` is a compatibility export of
`@runxhq/contracts` canonical JSON, cloud DB comparison payloads omit
`undefined` optional fields before canonical comparison, and
`cloud/packages/api/src/harness-routes.ts` now uses `canonicalJsonStringify` and
`sha256Prefixed` for signal source fingerprints, harness proof packet hashes,
and formerly truncated `sha256:` harness commitments. Short hashes remain only
for opaque IDs.
Follow-up: 2026-05-22T00:42:00+10:00 completed the focused canonical non-cloud
and `push_outbox` lane. `packages/core/src/artifacts/index.ts` now hashes
ledger chain payloads with `canonicalJsonStringify` plus `sha256Hex` for the
advertised `runx.stable-json.v1` contract. `packages/core/src/knowledge/file-thread.ts`
and both `push_outbox` tool copies use an `opaqueCanonicalJsonHashFragment`
helper for short `entry_` IDs and `push:` cursors, with comments documenting
that those fragments are internal and not `sha256:` commitments. The two
`push_outbox` manifests have refreshed `source_hash` values.
Follow-up: 2026-05-22T11:25:00+10:00 reviewed the focused CLI/script hash
slice that was already present in the worktree before this runner started.
`packages/cli/src/authoring-utils.ts` routes `sha256Stable` and deep structural
comparison through `@runxhq/contracts` canonical JSON; `packages/cli/src/commands/tool.ts`,
`packages/cli/src/commands/doctor.ts`, and `packages/cli/src/scaffold.ts` use
that helper for schema hashes while raw source-content hashing uses
`sha256Prefixed` over byte chunks; `scripts/check-contract-fixture-key-order.ts`
and `scripts/generate-rust-harness-fixtures.ts` now use the contracts canonical
helper. No additional safe code edits remained in this ownership slice.
Review gate: not_started

## Summary

Create one TypeScript stable JSON plus SHA-256 fingerprint implementation for
surviving TypeScript and cloud code, define the byte contract for
`runx.stable-json.v1`, and pin receipt-specific canonicalization to the Rust
receipt implementation through shared fixtures. Delete duplicate stable-hash
helpers from cloud and bundled tools where they claim a runx contract.

This is a correctness spec. If two runtimes stamp `runx.stable-json.v1` or
derive a receipt, signal, act-assignment, or ledger hash from that byte contract,
their bytes must be provably identical for the covered value domain.

## Context

Current Rust implementation:
- `oss/crates/runx-receipts/src/canonical.rs` implements
  `canonical_receipt_json`, `canonical_receipt_body_json`, and body/full
  digest helpers.
- `oss/crates/runx-contracts/src/fingerprint.rs` implements
  `sha256_hex` and `sha256_prefixed`.

Current TypeScript implementations:
- `oss/packages/core/src/util/hash.ts` implements `stableStringify`,
  `hashStable`, and `hashString`.
- `cloud/packages/db/src/stable-json.ts` implements `stableJsonStringify`.
- `cloud/packages/api/src/harness-routes.ts` uses `stableJsonStringify` and
  inline `createHash("sha256")` call sites for source fingerprints and harness
  receipt packet hashes.
- `cloud/packages/api/src/harness-routes.ts` also builds some prefixed digests
  from a truncated 16-hex `shortHash`, so the current labels are not all
  full-length `sha256:` commitments.
- `oss/tools/thread/push_outbox/src/index.ts` and
  `oss/packages/cli/tools/thread/push_outbox/src/index.ts` carry bundled
  `stableStringify` and `hashStable` copies.

Known risk:
- Rust serializes through `serde_json` and explicitly walks maps in sorted key
  order.
- TypeScript copies rely on `JSON.stringify` for escaping and number rendering,
  and some copies filter `undefined` while others canonicalize via object
  reconstruction.
- Cross-runtime verification can silently fail if both sides stamp the same
  canonicalization tag but hash different bytes.

## Objectives

- Add a surviving TypeScript module that owns stable JSON and `sha256:`
  fingerprint helpers.
- Inventory every canonicalization tag and classify it as contractual, derived
  from `runx.stable-json.v1`, or internal/non-contractual.
- Pin TypeScript outputs to Rust canonical outputs for shared fixtures.
- Replace cloud `stable-json.ts` and inline same-contract hashers with the
  shared helper.
- Replace bundled tool copies where the tool hashes runx structured state rather
  than arbitrary file bytes.
- Make unsupported JSON values explicit: `undefined`, `NaN`, infinities,
  `BigInt`, functions, and symbols must either be rejected or have documented
  canonical treatment.

## Scope

In scope:
- `@runxhq/contracts` TypeScript canonical JSON/fingerprint exports.
- Rust-to-TypeScript conformance fixtures for stable JSON and harness receipt
  canonicalization.
- Cloud imports and deletion of duplicate stable JSON helpers.
- Bundled `push_outbox` stable hash copies.

Out of scope:
- Arbitrary file content hashing helpers for tools such as `fs/write`.
- Rust canonicalization redesign.
- Full contract schema validation; owned by `rust-contract-schema-validation-gate`.
- Broad OSS TypeScript cleanup; owned by `rust-ts-sunset-*` specs.

## Dependencies

- `rust-contract-schema-validation-gate`
- `rust-aplus-cleanup` Class C sha256 helper cleanup
- `rust-ts-sunset-runtime-local`

## Touchpoints

- `oss/packages/contracts/src/`
- `oss/packages/contracts/src/index.test.ts`
- `oss/packages/core/src/util/hash.ts`
- `cloud/packages/db/src/stable-json.ts`
- `cloud/packages/api/src/harness-routes.ts`
- `cloud/packages/db/src/{index,postgres}.ts`
- `oss/packages/core/src/artifacts/index.ts`
- `oss/packages/core/src/knowledge/file-thread.ts`
- `oss/tools/thread/push_outbox/src/index.ts`
- `oss/packages/cli/tools/thread/push_outbox/src/index.ts`
- `oss/crates/runx-receipts/src/canonical.rs`
- `oss/fixtures/contracts/harness-spine/`

## Risks

- Moving hashing into contracts can create an unwanted dependency direction if
  contracts imports runtime-local or core. The helper must stay dependency-light.
- Replacing all `hashStable` call sites blindly would change hashes that are not
  governed by runx canonicalization tags.
- TypeScript and Rust number domains differ. The conformance fixture must state
  the allowed JSON number domain.

## Acceptance

Profile: strict

Definition of done:
- [x] `dod1` `@runxhq/contracts` exports canonical JSON, `sha256Hex`, and
  `sha256Prefixed` helpers without importing `@runxhq/core`.
- [x] `dod2` Every existing stable-hash call site is classified by tag and
  contract ownership.
- [x] `dod3` TypeScript stable JSON matches the declared `runx.stable-json.v1`
  byte contract for covered JSON values.
- [x] `dod4` TypeScript harness receipt canonicalization matches Rust receipt
  canonical JSON for `fixtures/contracts/harness-spine/*` covered by the spec.
- [x] `dod5` Cloud deletes `stable-json.ts` or leaves only a compatibility
  re-export to the shared contracts helper.
- [x] `dod6` Inline same-contract hashers in `harness-routes.ts` are replaced,
  and truncated `sha256:` labels are removed or explicitly reclassified.
- [x] `dod7` Bundled `push_outbox` stable hash copies are replaced or the spec
  records why bundled standalone tools must keep a pinned vendored helper.
- [x] `dod8` Unsupported values fail closed in tests.

Validation:
- [x] `v1` Contracts canonical JSON tests pass.
  - Command: `pnpm vitest run packages/contracts/src/canonical-json.test.ts`
  - Expected kind: `exit_code_zero`
  - Status: passed
  - Evidence: 2026-05-21 local command passed 24 tests after adding
    cross-runtime harness receipt oracle checks.
- [x] `v2` Rust receipt canonicalization tests pass.
  - Command: `cargo test --manifest-path crates/Cargo.toml -p runx-receipts canonical -- --nocapture`
  - Expected kind: `exit_code_zero`
  - Status: passed
  - Evidence: 2026-05-21 local command passed 5 canonical receipt tests,
    including the Rust oracle assertion for full and body receipt canonical JSON
    and digests.
- [x] `v3` Cloud API/db tests covering harness routes and stable payload
  equality pass.
  - Command: `pnpm vitest run packages/api/src/index.test.ts packages/db/src/index.test.ts packages/db/src/postgres.test.ts`
  - Expected kind: `exit_code_zero`
  - Status: passed
  - Evidence: 2026-05-22 local command passed 3 files / 45 tests from
    `/Users/kam/dev/runx/runx/cloud`, including full `sha256:` assertions for
    hosted signal-admission harness receipts and DB idempotency/equality
    coverage.
- [x] `v4` No duplicate same-contract helper remains in completed owner slices.
  - Command: `rg "function stableStringify|function hashStable|stableJsonStringify|runx\\.stable-json\\.v1|runx\\.harness-receipt\\.c14n\\.v1" packages cloud/packages tools`
  - Expected kind: `reviewed_output`
  - Status: passed for completed owner slices; pending broader runtime-local,
    adapter, and core legacy survivorship decisions outside this slice
  - Evidence: 2026-05-22 local scan confirmed cloud `stable-json.ts` is only a
    contracts compatibility export and `harness-routes.ts` has no
    `sha256:${shortHash(...)}` or inline stable-JSON SHA-256 call sites. The
    2026-05-22T00:42+10 scan confirmed the bundled `push_outbox` helper copies
    are gone, `packages/core/src/knowledge/file-thread.ts` uses only documented
    opaque fragments, and core ledger chain hashing uses the contracts helper.
    Remaining reviewed hits are contracts tests/schema labels,
    `packages/core/src/util/hash.ts` legacy exports, non-contract artifact meta
    and projection IDs, runtime-local sunset surfaces, adapter A2A internal
    hashes, and scripts for act-assignment fixture generation. The
    2026-05-22T11:25+10 scan confirmed the focused CLI schema-hash helpers now
    flow through `packages/cli/src/authoring-utils.ts` `sha256Stable`, which
    uses `@runxhq/contracts` canonical JSON.
- [x] `v5` Stable-hash and canonicalization tag inventory is reviewed.
  - Command: `rg "stableStringify|hashStable|sha256Stable|stableJsonStringify|runx\\.stable-json\\.v1|runx\\.harness-receipt\\.c14n\\.v1|runx\\.signal-source-event\\.c14n\\.v1|shortHash" packages tools scripts ../cloud/packages`
  - Expected kind: `reviewed_output`
  - Status: passed
  - Evidence: 2026-05-21 local inventory classified contractual, projection,
    internal, raw string/file, and truncated-digest call sites before rewiring.
- [x] `v6` Contract fixture key order remains canonical.
  - Command: `pnpm fixtures:contracts:keys`
  - Expected kind: `exit_code_zero`
  - Status: passed
  - Evidence: 2026-05-21 local command passed after canonicalizing
    `fixtures/contracts/external-adapter/*.json`; the canonical-json oracle was
    also accepted.
- [x] `v7` Focused core ledger, knowledge, contracts, and `push_outbox` tests
  pass.
  - Command: `pnpm vitest run packages/contracts/src/canonical-json.test.ts packages/core/src/artifacts/index.test.ts packages/core/src/knowledge/index.test.ts tests/thread-push-outbox-tool.test.ts`
  - Expected kind: `exit_code_zero`
  - Status: passed
  - Evidence: 2026-05-22 local command passed 4 files / 76 tests, including
    ledger `runx.stable-json.v1` hash assertions and opaque `entry_`/`push:`
    outbox fragment assertions.
- [x] `v8` `push_outbox` manifest source hashes match the edited sources.
  - Command: `node - <<'NODE' ... compute src/index.ts + run.mjs source_hash for tools/thread/push_outbox and packages/cli/tools/thread/push_outbox ... NODE`
  - Expected kind: `reviewed_output`
  - Status: passed
  - Evidence: 2026-05-22 local command reported both manifests match
    `sha256:287d61a66dee03b6e0eb086d6c17d22807f4386fad1ab9ef0e22eeede7ffc48f`.
- [x] `v9` Spec validates after the focused lane update.
  - Command: `scafld validate canonical-json-fingerprint-contract-v1 --json`
  - Expected kind: `exit_code_zero`
  - Status: passed
  - Evidence: 2026-05-22 local command returned
    `{"ok":true,"command":"validate",...,"valid":true,"errors":null}`.
- [x] `v10` Focused CLI/script canonical JSON hash tests pass.
  - Command: `pnpm vitest run packages/contracts/src/canonical-json.test.ts tests/init-command.test.ts`
  - Expected kind: `exit_code_zero`
  - Status: passed
  - Evidence: 2026-05-22T11:23+10 local command passed 2 files / 27 tests.
- [x] `v11` CLI manifest/scaffold/import-adjacent tests pass.
  - Command: `pnpm vitest run packages/cli/src/index.test.ts packages/cli/src/import-boundary.test.ts packages/cli/src/trainable-receipts.test.ts packages/cli/src/cli-presentation.test.ts tests/init-command.test.ts`
  - Expected kind: `exit_code_zero`
  - Status: passed
  - Evidence: 2026-05-22T11:24+10 local command passed 5 files / 54 tests.
- [x] `v12` Script canonical JSON consumers pass focused checks.
  - Command: `pnpm fixtures:contracts:keys`
  - Expected kind: `exit_code_zero`
  - Status: passed
  - Evidence: 2026-05-22T11:23+10 local command printed
    `Contract fixture keys are sorted.`
- [x] `v13` Rust harness fixture generator remains current.
  - Command: `pnpm tsx scripts/generate-rust-harness-fixtures.ts --check`
  - Expected kind: `exit_code_zero`
  - Status: passed
  - Evidence: 2026-05-22T11:23+10 local command exited zero with no stale
    oracle output.

## Phase 1: Tag Inventory

Status: done
Dependencies: none

Objective: Complete this phase.

Changes:
- Inventory `runx.stable-json.v1`, `runx.harness-receipt.c14n.v1`, `runx.signal-source-event.c14n.v1`, act-assignment idempotency hashes, ledger hashes, and internal `push_outbox` IDs/cursors.
- Record whether each hash is full `sha256:`, unprefixed hex, truncated hex, or internal suffix material.
- Decide which tags are built on `runx.stable-json.v1` and which remain separate projection-specific contracts.
- Classification:
  - `runx.stable-json.v1` is the structured JSON byte contract now owned by
    `@runxhq/contracts`. Ledger schemas advertise this label; ledger chain
    hashing in `packages/core/src/artifacts/index.ts` is a contractual caller
    and should migrate only after conformance tests prove byte identity.
  - `runx.harness-receipt.c14n.v1` is owned by Rust receipt canonicalization in
    `crates/runx-receipts`. TypeScript runtime-local and cloud code currently
    stamp this label, but they must be treated as untrusted until Phase 3 proves
    fixture parity.
  - `runx.signal-source-event.c14n.v1` is a cloud source-event fingerprint over
    a stable JSON subset. It can use the shared helper after cloud test coverage
    pins the exact event shape.
  - `runx.stdout-hash.v1` and `runx.stderr-hash.v1` are raw string SHA-256
    contracts, not `runx.stable-json.v1` callers.
  - `runx.input-hash.v1` is a TypeScript runtime-local structured input hash.
    It is a sunset surface; do not rewire it ahead of the runtime-local sunset
    decision.
  - `runx.fingerprint.c14n.v1` covers Aster/target/opportunity projection
    fixtures. It is projection-specific and is not automatically equivalent to
    `runx.stable-json.v1`.
  - `push_outbox` IDs and cursors are internal bundled-tool identifiers. They
    may keep a pinned vendored helper unless the standalone bundle policy
    allows importing `@runxhq/contracts`.
  - Tool file/binary hashes, release artifact hashes, skill markdown digests,
    and profile digests are raw byte/string hashes and remain out of scope.
  - Cloud `harness-routes.ts` `shortHash` call sites and
    `scripts/dogfood-github-issue-to-pr.mjs` produce truncated values under
    `sha256:` labels. Those labels must be reclassified or replaced before any
    helper migration claims full SHA-256 semantics.

Acceptance:
- Stable-hash call sites are classified before replacement begins. Done.

## Phase 2: Contract Helper

Goal: add the TypeScript helper in the surviving contracts package.

Status: done for the additive helper slice
Dependencies: Phase 1
Note: executed before full Phase 1 because it does not rewire existing
call sites or change published hashes. Replacement work still depends on tag
inventory.

Changes:
- Implemented canonical JSON and SHA-256 helpers under `oss/packages/contracts`.
- Exported helpers from the package root.
- Added explicit unsupported-value tests for `undefined`, array holes,
  functions, symbols, `BigInt`, non-finite numbers, and unpaired surrogates.

Acceptance:
- The helper has no dependency on `@runxhq/core`, runtime-local, or cloud code.

## Phase 3: Cross-Runtime Conformance

Goal: prove byte identity with Rust for covered fixtures.

Status: done for covered harness-spine receipt fixtures
Dependencies: Phase 2

Changes:
- Added `fixtures/contracts/canonical-json/runx-harness-receipt-c14n-v1.oracles.json`
  for the covered harness-spine receipt fixtures.
- Added Rust tests that assert full receipt canonical JSON, full digest, body
  canonical JSON, and body digest against the oracle.
- Added TypeScript tests that read the same oracle and compare
  `canonicalJsonStringify(fixture.expected)`, `sha256Prefixed`, and body-stripped
  canonical JSON/digests against it.

Acceptance:
- A hash drift in either implementation fails tests. Done for
  `harness-receipt-abnormal`, `harness-receipt-success`, and
  `post-merge-observer-merged-verified`.

## Phase 4: Cloud and Tool Replacement

Goal: remove duplicate same-contract implementations.

Status: in_progress; cloud API/db, core ledger, file-thread, bundled
`push_outbox`, and focused CLI/script replacement complete;
runtime-local/adapter/core legacy follow-up decisions pending
Dependencies: Phase 3

Changes:
- Replaced cloud `stable-json.ts` with a contracts compatibility export.
- Replaced inline `createHash` plus stable JSON call sites where they produce
  runx canonical fingerprints in `cloud/packages/api/src/harness-routes.ts`.
- Corrected cloud harness route truncated `sha256:` commitments to full
  `sha256:` values from the shared canonical JSON helper.
- Replaced core ledger chain hashing with `canonicalJsonStringify` and
  `sha256Hex` for the `runx.stable-json.v1` canonicalization label.
- Replaced bundled `push_outbox` vendored `stableStringify`/`hashStable`
  helpers with documented internal opaque fragments derived from contracts
  canonical JSON.
- Reclassified `packages/core/src/knowledge/file-thread.ts` `entry_` IDs and
  `push:` cursors as internal opaque truncated fragments, not `sha256:`
  commitments.
- Reviewed already-present focused CLI/script replacements:
  `packages/cli/src/authoring-utils.ts` owns CLI `sha256Stable` on top of
  contracts canonical JSON, CLI tool/doctor/scaffold schema hashes route
  through that helper, and script-level canonical JSON consumers import the
  contracts helper directly.

Acceptance:
- Review can identify one owner for the canonicalization label.

## Rollback

If the shared TypeScript helper changes existing published hashes, stop before
replacement and record a compatibility plan. Do not leave two helpers stamping
the same canonicalization tag with divergent bytes.

## Review

Review must inspect byte-level conformance, not just semantic JSON equality.

## Origin

User-provided cross-scan synthesis on 2026-05-21 identified canonical JSON and
`sha256:` fingerprint duplication as the highest-severity cross-runtime drift
risk.
