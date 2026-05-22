---
spec_version: '2.0'
task_id: canonical-json-fingerprint-contract-v1
created: '2026-05-21T12:19:24Z'
updated: '2026-05-22T06:24:11Z'
status: completed
harden_status: not_run
size: medium
risk_level: high
---

# Canonical JSON fingerprint contract v1

## Current State

Status: completed
Current phase: final
Next: done
Reason: task completed
Blockers: none
Allowed follow-up command: `none`
Latest runner update: 2026-05-22T06:24:11Z
Review gate: pass

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

Status: completed
Dependencies: none

Objective: Complete this phase.

Changes:
- Inventory `runx.stable-json.v1`, `runx.harness-receipt.c14n.v1`, `runx.signal-source-event.c14n.v1`, act-assignment idempotency hashes, ledger hashes, and internal `push_outbox` IDs/cursors.
- Record whether each hash is full `sha256:`, unprefixed hex, truncated hex, or internal suffix material.
- Decide which tags are built on `runx.stable-json.v1` and which remain separate projection-specific contracts.
- Classification:
  - `runx.stable-json.v1` is the structured JSON byte contract now owned by
    `@runxhq/contracts`. Ledger schemas advertise this label; ledger chain
    hashing in `packages/core/src/artifacts/index.ts` is a contractual caller.
  - `runx.harness-receipt.c14n.v1` is owned by Rust receipt canonicalization in
    `crates/runx-receipts`; TypeScript parity is pinned through shared oracle
    fixtures for the covered harness-spine receipts.
  - `runx.signal-source-event.c14n.v1` is a cloud source-event fingerprint over
    a stable JSON subset and now uses the shared helper in the completed cloud
    slice.
  - `runx.stdout-hash.v1` and `runx.stderr-hash.v1` are raw string SHA-256
    contracts, not `runx.stable-json.v1` callers.
  - `runx.input-hash.v1` is a TypeScript runtime-local structured input hash
    and remains a sunset surface outside this slice.
  - `runx.fingerprint.c14n.v1` covers projection fixtures and is not
    automatically equivalent to `runx.stable-json.v1`.
  - `push_outbox` IDs and cursors are internal opaque identifiers, not
    `sha256:` commitments.
  - Tool file/binary hashes, release artifact hashes, skill markdown digests,
    and profile digests are raw byte/string hashes and remain out of scope.

Acceptance:
- none

## Phase 2: Contract Helper

Status: completed
Dependencies: Phase 1

Objective: Complete this phase.

Changes:
- Implemented canonical JSON and SHA-256 helpers under `oss/packages/contracts`.
- Exported helpers from the package root.
- Added explicit unsupported-value tests for `undefined`, array holes, functions, symbols, `BigInt`, non-finite numbers, and unpaired surrogates.

Acceptance:
- none

## Phase 3: Cross-Runtime Conformance

Status: completed
Dependencies: Phase 2

Objective: Complete this phase.

Changes:
- Added `fixtures/contracts/canonical-json/runx-harness-receipt-c14n-v1.oracles.json` for the covered harness-spine receipt fixtures.
- Added Rust tests that assert full receipt canonical JSON, full digest, body canonical JSON, and body digest against the oracle.
- Added TypeScript tests that read the same oracle and compare `canonicalJsonStringify(fixture.expected)`, `sha256Prefixed`, and body-stripped canonical JSON/digests against it.

Acceptance:
- none

## Phase 4: Cloud and Tool Replacement

Status: completed
Dependencies: Phase 3

Objective: Complete this phase.

Changes:
- Replaced cloud `stable-json.ts` with a contracts compatibility export.
- Replaced inline `createHash` plus stable JSON call sites where they produce runx canonical fingerprints in `cloud/packages/api/src/harness-routes.ts`.
- Corrected cloud harness route truncated `sha256:` commitments to full `sha256:` values from the shared canonical JSON helper.
- Replaced core ledger chain hashing with `canonicalJsonStringify` and `sha256Hex` for the `runx.stable-json.v1` canonicalization label.
- Replaced bundled `push_outbox` vendored `stableStringify`/`hashStable` helpers with documented internal opaque fragments derived from contracts canonical JSON.
- Reclassified `packages/core/src/knowledge/file-thread.ts` `entry_` IDs and `push:` cursors as internal opaque truncated fragments, not `sha256:` commitments.
- Reviewed already-present focused CLI/script replacements: `packages/cli/src/authoring-utils.ts` owns CLI `sha256Stable` on top of contracts canonical JSON, CLI tool/doctor/scaffold schema hashes route through that helper, and script-level canonical JSON consumers import the contracts helper directly.
- Closure scan (current tree): the same-contract scan finds only the contracts owner package (`canonical-json.ts`, `schemas/ledger.ts`) and the out-of-scope sunset surfaces (`runtime-local` graph-governance label use, legacy `core/util/hash.ts` exports). No in-scope duplicate stamps the contract with divergent bytes; v1 (24 TS) and v2 (5 Rust) canonical tests pass on this tree.

Acceptance:
- none

## Rollback

If the shared TypeScript helper changes existing published hashes, stop before
replacement and record a compatibility plan. Do not leave two helpers stamping
the same canonicalization tag with divergent bytes.

## Review

Status: completed
Verdict: pass
Mode: discover
Provider: claude:claude-opus-4-7
Output: claude.mcp_submit_review
Summary: Spec satisfies its declared scope. `@runxhq/contracts/canonical-json.ts` is the single TS owner of `runx.stable-json.v1`, depends only on `node:crypto`, exports `canonicalJsonStringify`/`sha256Hex`/`sha256Prefixed`, and rejects undefined/array-holes/functions/symbols/BigInt/NaN/Infinity/unpaired surrogates. The Rust receipt canonicalizer in `crates/runx-receipts/src/canonical.rs` and the TS contracts helper both consume the shared `runx-harness-receipt-c14n-v1.oracles.json` oracle for the three covered harness-spine fixtures (success/abnormal/post-merge-observer). Cloud `stable-json.ts` is reduced to a compatibility re-export, `harness-routes.ts` uses `sha256Prefixed(canonicalJsonStringify(...))` for `runx.signal-source-event.c14n.v1`/`runx.harness-receipt.c14n.v1`/idempotency content hashes (no remaining inline `createHash`+stableStringify same-contract callers), the only surviving `createHash` is the opaque `shortHash` for `sig_`/`hr_`/`h_`/`dec_` ID derivation. Bundled `tools/thread/push_outbox` and `packages/cli/tools/thread/push_outbox` import contracts and document their truncated digests as opaque, core `file-thread.ts` does the same, and ledger chain hashing in `packages/core/src/artifacts/index.ts` routes through `canonicalJsonStringify`+`sha256Hex` against `runx.stable-json.v1`. Out-of-scope sunset surfaces (`runtime-local/runner-local/graph-governance.ts`, `runtime-local/sdk/act-assignment.ts`, `adapters/a2a`, `core/util/hash.ts`) still stamp `sha256:`-prefixed digests via the legacy `hashStable` (`localeCompare` sort + undefined-filter), but the spec explicitly defers them. Ambient drift (53 paths) is dominated by unrelated runx-runtime kernel work (skill, payment supervisor, credentials, target_runner, seal.rs) and does not retouch the canonical JSON owner files; `Task-scoped changes since baseline: 0` is consistent with the spec evidence that v1 (24 TS) and v2 (5 Rust) canonical tests already pass on this tree. Findings below are low-severity observations only; none block completion.

Attack log:
- `packages/contracts/src/canonical-json.ts`: Validate dod1: contracts owns canonical JSON + sha256 helpers and does not import @runxhq/core. Inspected imports (only node:crypto) and package.json (deps only @sinclair/typebox). -> clean
- `packages/contracts/src/canonical-json.ts unsupported-value gate`: Validate dod8: confirm undefined, array holes, functions, symbols, BigInt, NaN, Infinity, -Infinity, unpaired surrogates all throw. Traced each branch in canonicalJsonValue + assertNoUnpairedSurrogate + canonicalJsonNumber against the it.each table in canonical-json.test.ts. -> clean
- `crates/runx-receipts/src/canonical.rs vs packages/contracts canonical-json`: Validate dod4/v1/v2: confirm the shared oracle fixture runx-harness-receipt-c14n-v1.oracles.json is consumed by both Rust harness_receipt_oracle_matches_rust_canonical_json test and TS canonical-json.test.ts oracle tests with full/body canonical JSON + sha256 assertions on the three covered harness-spine fixtures. -> clean
- `cloud/packages/api/src/harness-routes.ts`: dod6 regression hunt: scan for surviving createHash+stableStringify same-contract callers and truncated sha256: commitments. Confirmed sha256CanonicalJson goes through sha256Prefixed(canonicalJsonStringify(...)) for digest/enforcement_profile_hash/content_hash, and the only remaining createHash is the opaque shortHash used for sig_/hr_/h_/dec_ IDs. -> finding (Identified F3 - non-hash literal still under sha256: shape at line 345.)
- `cloud/packages/db/src/{stable-json,index,postgres}.ts`: dod5: confirm cloud stable-json.ts is a compatibility re-export and all stableJsonStringify callers route to contracts canonicalJsonStringify. -> clean
- `oss/tools/thread/push_outbox + packages/cli/tools/thread/push_outbox + packages/core/src/knowledge/file-thread.ts`: dod7 regression: confirm bundled push_outbox copies use contracts opaque hash and document non-commitment status; cross-compare with core file-thread.ts. -> finding (F1 - bundled tools use {thread_locator, outbox_entry_id, pushed_at} but core uses {thread, outbox_entry, pushed_at} for entry_id; cursors agree.)
- `packages/core/src/artifacts/index.ts ledger chain hashing`: Confirm ledger entry_hash uses canonicalJsonStringify+sha256Hex with the runx.stable-json.v1 canonicalization label, while artifact-meta hashStable use remains classified as non-contract. -> clean
- `Closure scan for stableStringify/hashStable/stableJsonStringify/sha256Stable across oss + cloud`: Verify v4 evidence: enumerate every remaining caller and confirm each is either an out-of-scope sunset surface (runtime-local graph-governance, runtime-local sdk/act-assignment, packages/adapters/src/a2a, packages/core/src/util/hash.ts) or in-scope contracts-routed (cli/authoring-utils sha256Stable, push_outbox bundles, core artifacts/file-thread). -> clean (Sunset surfaces still stamp sha256:-prefixed digests under runx.harness-receipt.c14n.v1 (graph-governance.ts:992) and idempotency tags (act-assignment SDK lines 67/83/90), but spec explicitly defers these and the act-assignment fixtures pin parity to ASCII-only keys.)
- `crates/runx-contracts/src/act_assignment/hash.rs vs packages/runtime-local/src/sdk/act-assignment.ts`: Bytewise parity check for act-assignment idempotency hashes: Rust uses BTreeMap (byte-sorted) keys, TS uses Object.entries.sort(localeCompare). For ASCII-lowercase keys (the only covered domain per fixture description) the two orders agree and U+2028/U+2029 raw emission matches modern V8. Non-ASCII or mixed-case keys would diverge. -> clean (Acknowledged limitation: fixture descriptions explicitly say 'non-ASCII object keys are rejected until hash-stable-codepoint-cutover replaces localeCompare ordering'.)
- `Ambient drift attribution`: Workspace classifier shows Task-scoped changes since baseline: 0 and 53 ambient drift items. Walk the drift to confirm none retouch in-scope owner files (packages/contracts/src/canonical-json.ts, fixtures/contracts/canonical-json/*, cloud/packages/db/src/stable-json.ts, cloud/packages/api/src/harness-routes.ts, oss/tools/thread/push_outbox/*, oss/packages/cli/tools/thread/push_outbox/*, oss/packages/core/src/{artifacts,knowledge/file-thread}, oss/crates/runx-receipts/src/canonical.rs). -> clean (Drift is dominated by runx-runtime kernel rewiring (skill, payment supervisor, credentials, target_runner, seal.rs, skill_run) and new fixture/skills directories; spec owner files are untouched.)
- `covered_number_domain vs canonicalJsonStringify finite-number admission`: Probe whether canonicalJsonStringify allows finite numbers outside the documented covered_number_domain that would diverge from Rust serde_json's default Display (e.g., 1e21, 1e-7, integers >2^53). -> finding (F2 - helper accepts a strictly broader domain than the oracle commits to.)
- `Object key ordering in canonical JSON`: Compare TS compareJsonObjectKeys (code-point iteration via Symbol.iterator + codePointAt(0)) against Rust BTreeMap<String> ordering (UTF-8 byte order) and serde_json::Map sort. For all valid UTF-8 strings code-point order equals UTF-8 byte order, so they agree; arrays guard against string-keyed extras via isArrayElementKey/Object.keys filtering. -> clean
- `Cycle and prototype safety in canonicalJsonStringify`: Inspect assertAcyclic/assertNoEnumerableSymbolKeys/isPlainJsonObject for traversal bugs (Date instances, Object.create(null), class instances, getter throws, sparse arrays). -> clean (Plain object guard rejects non-Object.prototype/non-null prototypes (so Date/Map throw 'non-plain object'); Object.create(null) is accepted; sparse arrays throw 'array hole'; cycle stack is properly add/delete via finally.)

Findings:
- [low/non-blocking] `F1` Bundled push_outbox tools and core file-thread.ts derive the opaque entry_id from different key sets, so the same logical push produces different IDs across code paths.
  - Location: `packages/core/src/knowledge/file-thread.ts:99`
  - Evidence: tools/thread/push_outbox/src/index.ts:319-324 hashes {thread_locator, outbox_entry_id, pushed_at}; packages/cli/tools/thread/push_outbox/src/index.ts uses the identical formula; packages/core/src/knowledge/file-thread.ts:99-103 hashes {thread, outbox_entry, pushed_at}. Both pipe through opaqueCanonicalJsonHashFragment(canonicalJsonStringify(...)) so the canonicalization helper is consistent, but the input shapes diverge. The push cursor formula at file-thread.ts:126 and push_outbox.../index.ts:346 does match ({outbox_entry, pushed_at}), so only entry_id drifts.
  - Impact: Replaying the same thread push through the bundled CLI tool vs core in-process API yields divergent entry_ values. Spec reclassifies these as opaque-only, so this is not a contract violation, but downstream consumers comparing entry_ids across pipelines may be surprised.
- [low/non-blocking] `F2` covered_number_domain in the v1 oracle commits to only six specific finite numbers, but canonicalJsonStringify accepts any finite JS number where Rust serde_json output is known to diverge (1e21, 1e-7, integers beyond Number.MAX_SAFE_INTEGER).
  - Location: `packages/contracts/src/canonical-json.ts:45`
  - Evidence: fixtures/contracts/canonical-json/runx-stable-json-v1.cases.json line covered_number_domain: {"examples":[0,-7,42,12.5,-0.25,0.125]}. canonical-json.ts:45 rejects only NaN/Infinity. JsonNumber Display in crates/runx-contracts/src/json.rs:121 writes whole floats as "{value:.0}" (matching JSON.stringify integer form) but non-whole f64s use the default Rust Display formatter, which emits "0.0000001" where JS emits "1e-7" and "1e+21" where Rust emits "1000000000000000000000". The conformance harness exercises none of these.
  - Impact: A future caller hashing structured data with edge-domain numbers under runx.stable-json.v1 may produce TS bytes that do not match Rust without any test surfacing the drift. Spec acknowledges this in the Risks section but does not narrow the helper.
- [low/non-blocking] `F3` Cloud harness-routes still emits a non-hash literal under the sha256: prefix shape (public_key_sha256: "sha256:hosted-api"), which is the same anti-pattern dod6 set out to eliminate from truncated commitments.
  - Location: `cloud/packages/api/src/harness-routes.ts:345`
  - Evidence: cloud/packages/api/src/harness-routes.ts:345 issuer.public_key_sha256 = "sha256:hosted-api" inside the synthesized hosted source-signal harness receipt; the surrounding shortHash callers (lines 273,274,353,393) are explicitly opaque sig_/hr_/h_/dec_ IDs and are not claimed as sha256 commitments, so the placeholder issuer key is the only remaining non-conformant sha256:-shaped value in the route. The harness receipt schema in packages/contracts/src/schemas/spine.ts:855 only requires minLength:1, so validation does not catch this.
  - Impact: Any verifier that inspects issuer.public_key_sha256 will see a structurally valid but cryptographically meaningless commitment. Likely predates this task, but the dod6 cleanup of harness-routes left it in place.

## Origin

User-provided cross-scan synthesis on 2026-05-21 identified canonical JSON and
`sha256:` fingerprint duplication as the highest-severity cross-runtime drift
risk.
