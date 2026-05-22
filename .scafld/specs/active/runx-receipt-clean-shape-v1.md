---
spec_version: '2.0'
task_id: runx-receipt-clean-shape-v1
created: '2026-05-22T07:36:10Z'
updated: '2026-05-22T10:35:30Z'
status: review
harden_status: in_progress
size: large
risk_level: high
---

# runx-receipt-clean-shape-v1

## Current State

Status: review
Current phase: final
Next: repair
Reason: review gate fail: 6 finding(s), 3 completion blocker(s)
Blockers: none
Allowed follow-up command: `scafld handoff runx-receipt-clean-shape-v1`
Latest runner update: 2026-05-22T12:37:23Z
Review gate: fail

## Summary

Replace the `runx.harness_receipt.v1` contract with a single, clean
`runx.receipt.v1` shape and cut the entire Rust+cloud surface over to it in one
coordinated move. This is a **clean cutover with zero back-compat**: no
`harness_receipt` aliases, no dual-shape support, no compat shims, no old-shape
glue. The old schema, Rust types, fixtures, and stored-data URIs are gone, not kept
alongside.

Naming resolution: `runx.receipt.v1` is currently taken by a CLI *run-summary*
(`packages/contracts/src/schemas/receipt.ts`: `run_id`/`command`/`status`/`steps`).
That is a report, not a proof artifact. The signed governance receipt is the moat
("a verifiable receipt of exactly what it did"), so it reclaims the canonical name
`runx.receipt.v1`; the run-summary is renamed `runx.run-summary.v1` and carries a
`receipt_ref` to the real receipt (a projection beside it, not a competing receipt).
The durable URI prefix moves `runx:harness_receipt:` -> `runx:receipt:`, and
`ReferenceType::HarnessReceipt` -> `ReferenceType::Receipt`. Per the clean-break
decision there is no stored-data migration: pre-cutover receipts and the
payment-ledger projection are re-seeded from source, not read by new code.

The current shape conflates three artifacts into one: durable proof, planner
deliberation, and per-act detail. The receipt nests a `harness` sub-object that
forces a mandatory duplicated `seal` (`verify.rs` checks `seal == harness.seal`),
inlines planner deliberation (`decisions[]`) that no consumer reads, and restates
the skill's declared contract in every act (`intent.success_criteria`) instead of
recording result bindings. The clean shape flattens the `harness` wrapper (which
removes the seal duplication by construction), references deliberation and per-act
detail instead of inlining them, and keeps the load-bearing proof inspectable.

## Context

Grounded in the committed schemas, a generated runtime receipt, and a consumer
sweep (2026-05-22):

- The contract is `runx.harness_receipt.v1` (`schemas/harness-receipt.schema.json`,
  ~14k inlined lines). The runtime emits it; cloud and the ledger read parts of it.
- `crates/runx-receipts/src/canonical.rs` commits a body digest over everything
  except `signature` and each seal's `digest`/`verification_summary`. So
  `decisions[]`, `acts[]`, `authority`, `enforcement`, `idempotency` are all in the
  signed body.
- `crates/runx-receipts/src/verify.rs` enforces `receipt.seal == receipt.harness.seal`
  (the duplication is mandatory only because of the nested `harness` object),
  validates `decisions[].selected_act_id` references a real act, validates act form
  consistency, binds seal criteria to `acts[].criterion_bindings`, and validates the
  authority attenuation against `authority-subset-proof`.
- Consumer sweep: cloud reads only `idempotency.{intent_key,trigger_fingerprint,content_hash}`
  (dedup) and top-level `seal.{disposition,closed_at}` (status). `decisions[]`
  deliberation content is read by nothing; it is only hashed plus the act-id
  integrity check.
- The payment authority bound (the magnet) lives in
  `authority.terms[].bounds.payment` (`currency`, `max_per_call_minor`,
  `max_per_run_minor`, `max_per_period_minor`, `single_use_spend`,
  `reservation_required`, `receipt_before_success`, `idempotency_required`). It is
  the provable bound and must stay fully inspectable, never collapsed to a hash.
- Composition targets already exist as separate schemas: `act-receipt`,
  `review-receipt-output`, `signal`, `authority-subset-proof`.

## The runx.receipt.v1 shape

Flat top-level keys, each answering one question; everything optional-when-empty:

- envelope: `schema`, `id`, `created_at`, `canonicalization`, `issuer`,
  `signature`, `digest`. `issuer.type` keeps the four issuers (`local`, `hosted`,
  `ci`, `verifier`); `signature.alg` stays `Ed25519`.
- `idempotency`: `{intent_key, trigger_fingerprint, content_hash}` (cloud dedup +
  payment safety; top-level).
- `subject`: `{kind: skill|graph, ref, commitments[]}` where each commitment is
  `{scope, algorithm, value, canonicalization}` and `scope` is one of
  `input|output|stdout|stderr|error`. This unifies the old `seal.hash_commitments`
  and `enforcement.stdout_hash`/`stderr_hash` into one scoped list, so there is one
  way to commit a byte stream, not three.
- `authority`: `actor_ref`, `grant_refs`, `scope_refs`, `attenuation`
  (`{parent_authority_ref, subset_proof?}`), `authority_proof_refs`, `mandate_ref`,
  `terms[]`, `enforcement: {profile_hash, redaction_refs, setup_refs[],
  teardown_refs[]}`. Every `terms[]` entry keeps its full RBAC shape inspectably:
  `principal_ref`, `resource_ref`, `resource_family` (10 families), `verbs` (17),
  `capabilities` (9), `conditions` (9 predicates), `approvals`, `bounds` (incl.
  `bounds.payment` with the cap/rail/single_use_spend magnet fields). Nothing here is
  collapsed to a hash; the policy *profile* is hashed (`enforcement.profile_hash`),
  the granted *authority* stays readable.
- `acts[]`: `{id, form, summary, criteria[]: {criterion_id, status, evidence_refs,
  verification_refs}, by?, artifact_refs[], detail_ref}`. `form` is one of the five
  act forms (`observation`, `reply`, `review`, `revision`, `verification`); `status`
  is one of the five criterion statuses (`verified|failed|pending|not_applicable|
  unknown`). `by` is optional runner provenance (`{provider, model, prompt_version}`)
  for agent acts. `detail_ref` points to an `act-receipt` for the full
  intent/target/source/surface refs and the form-specific bodies (`RevisionDetails`,
  `VerificationDetails`). The restated `intent.success_criteria` declaration is gone:
  the receipt records *result bindings*, not the skill's declared contract.
- `seal`: `{disposition, reason_code, summary, closed_at, criteria}`. One seal, no
  `harness.seal` twin. `disposition` is the eight-value closure enum
  (`closed|deferred|superseded|declined|blocked|failed|killed|timed_out`). `criteria`
  rolls up the per-act bindings. **`verification_summary` is removed from the signed
  body**: signature/attenuation/redaction validity is *computed by a verifier at read
  time* (it is verifier output, not issuer-asserted proof), so the receipt no longer
  carries a self-graded report card.
- `lineage` (optional): `{parent, previous?, children[], sync[], signal_refs[],
  journal_ref, resume_ref?}`. `kind: graph` + `lineage.children/sync` makes skill and
  graph one model. `lineage.previous` is the prior receipt on a resume (former
  `revision.previous_ref`); `revision.sequence` is dropped (orderable from the
  `previous` chain). `signal_refs[]` is the triggering-signal lineage (signal detail,
  authenticity, and trust level live in the referenced `signal` artifact).
  `journal_ref` commits the planner deliberation (former `decisions[]`) by reference
  instead of inlining it. `resume_ref` carries the open resolution request when
  `seal.disposition = deferred` (see non-terminal receipts below).

### Non-terminal (suspended) receipts

The runtime harness has an eleven-state machine (`forming, admitted, running,
waiting, delegated, sealing, sealed, killed, timed_out, failed, superseded`). Most of
those are *live* states that never produce a durable artifact — they are exactly the
runtime bookkeeping the journal owns, not the receipt. A durable receipt is emitted at
only two moments:

- **Termination** — `seal.disposition` is one of the eight closure values.
- **Suspension** — a `waiting`/`delegated` harness emits a receipt with
  `seal.disposition = deferred` and `lineage.resume_ref` pointing at the open
  resolution request (input questions / approval gate / agent-act invocation, the
  existing `act-receipt` resolution shape). Resuming seals a new revision whose
  `lineage.previous` is the deferred receipt.

This collapses the eleven-state machine + conditional-`seal`-nullability rule into one
invariant: **every durable receipt has exactly one `seal`; `deferred` is how
"not done yet" is expressed.** No `seal: null` special case, no separate top-level
`state` field.

### Journal verification contract

Moving `decisions[]` out must not weaken the `selected_act_id` integrity guarantee.
The journal is a local artifact written beside the receipt in the receipt dir,
committed by `lineage.journal_ref` (a hash). Two-tier verify, fail-closed:

- `verify(receipt)` — checks shape, signature, digest, seal↔act criteria binding, and
  authority attenuation against `authority_proof_refs`. Journal-dependent checks
  (every `decision.selected_act_id` resolves to a real `acts[].id`) are reported
  `unverified`, never silently passed.
- `verify_with_journal(receipt, journal)` — additionally loads the journal, asserts
  its hash equals `journal_ref`, and runs the act-id integrity check. This is the only
  path that yields a fully `verified` verdict. A present-but-missing or
  hash-mismatched journal is `invalid`, not `unverified`.

The journal shares the receipt's trust boundary (same dir, same issuer); there is no
cross-trust-boundary journal read.

## Edge and variant coverage

The cutover must not silently drop a variant. Every enum and structure in the current
`runx.harness_receipt.v1` model (verified against `crates/runx-contracts/src/` and
`packages/contracts/src/schemas/spine.ts`, which agree with each other — the cloud
`receipt.schema.json` is the lone simplified outlier this cutover also corrects) maps
to the clean shape as follows. "Kept" = inspectable in the receipt; "Journal" =
moved to the referenced planner journal; "Act-receipt" = moved behind `detail_ref`;
"Projection" = computed at read time, not in the signed body.

Enums (every variant preserved unless noted):

| Current enum | Variants | Clean shape location |
| --- | --- | --- |
| HarnessState | 11: forming, admitted, running, waiting, delegated, sealing, sealed, killed, timed_out, failed, superseded | Collapsed. Live states are runtime-only (never durable); `sealed/killed/timed_out/failed/superseded` map to `seal.disposition`; `waiting/delegated` map to `seal.disposition=deferred` + `resume_ref`. |
| ClosureDisposition (seal) | 8: closed, deferred, superseded, declined, blocked, failed, killed, timed_out | Kept verbatim as `seal.disposition`. |
| ActForm | 5: observation, reply, review, revision, verification | Kept as `acts[].form`; form-specific bodies (RevisionDetails, VerificationDetails) → Act-receipt. |
| CriterionStatus | 5: verified, failed, pending, not_applicable, unknown | Kept as `acts[].criteria[].status`; rolled up in `seal.criteria`. |
| DecisionChoice | 8: open, continue, spawn_child, escalate, defer, close, decline, monitor | Journal. The receipt records outcomes (acts+seal), not deliberation; `selected_act_id` integrity preserved via `verify_with_journal`. |
| AuthorityResourceFamily | 10 (github_repo … publication) | Kept in `authority.terms[].resource_family`. |
| AuthorityVerb | 17 (read … spawn_child) | Kept in `authority.terms[].verbs`. |
| AuthorityCapability | 9 | Kept in `authority.terms[].capabilities`. |
| AuthorityConditionPredicate | 9 | Kept in `authority.terms[].conditions[].predicate`. |
| SignalType / SignalTrustLevel | 11 / 5 | Referenced via `lineage.signal_refs`; detail in the `signal` artifact. |
| ReferenceType | ~35 | Becomes the URI scheme of self-describing `runx:<type>:<id>` ref strings (`HarnessReceipt`→`Receipt`). |
| Fanout strategy / decision | 3 (all,any,quorum) / 4 (proceed,halt,pause,escalate) | Kept in `lineage.sync[]` (incl. `gate`). |
| IssuerType / SignatureAlgorithm | 4 (local,hosted,ci,verifier) / Ed25519 | Kept in `envelope.issuer.type` / `signature.alg`. |

Structures (where each current field goes):

| Current field | Disposition |
| --- | --- |
| `harness` wrapper | Removed; its members flatten to top level (this is what kills the seal twin). |
| `harness.seal` (duplicate of top `seal`) | Removed; one `seal` only. |
| `seal.{disposition,reason_code,summary,closed_at,criteria}` | Kept. |
| `seal.{last_observed_at}` | Dropped (live-state telemetry, journal if needed). |
| `seal.{canonicalization,digest}` | Moved to envelope (`canonicalization`) + `digest`. |
| `seal.verification_summary` (6 booleans) | Projection (verifier-computed at read time). |
| `seal.{redaction_refs,artifact_refs,hash_commitments}` | `redaction_refs`→`authority.enforcement.redaction_refs`; `artifact_refs`→`acts[].artifact_refs`; `hash_commitments`→`subject.commitments[]`. |
| `harness.{decisions[]}` (+proposed_intent, inputs, justification, closure) | Journal, committed by `lineage.journal_ref`. |
| `harness.acts[].{intent.success_criteria,constraints,derived_from,legitimacy,purpose}` | Act-receipt (`detail_ref`); receipt keeps result bindings only. |
| `harness.acts[].{criterion_bindings}` | Kept as `acts[].criteria[]`. |
| `harness.acts[].{source_refs,target_refs,surface_refs,harness_refs,verification_refs,closure,performed_at}` | Act-receipt (`detail_ref`); `artifact_refs`/`verification_refs` summary kept on the act. |
| `harness.authority.*` (terms, attenuation, proof refs, mandate) | Kept inspectably under flat `authority` (incl. `terms[].bounds.payment`). |
| `harness.enforcement.{sandbox,profile_hash,version,redaction_refs,std*_hash,setup/teardown}` | `profile_hash`+`redaction_refs`+`setup/teardown_refs`→`authority.enforcement`; `std*_hash`→`subject.commitments[]`; `sandbox`/`version` → committed inside `profile_hash`. |
| `harness.idempotency.*` | Kept top-level (`idempotency`). |
| `harness.revision.{sequence,previous_ref}` | `previous_ref`→`lineage.previous`; `sequence` dropped (orderable from chain). |
| `harness.{host_ref,harness_ref,parent_harness_ref}` | `parent_harness_ref`→`lineage.parent`; `host_ref`/`harness_ref` → journal (runtime identity, not proof). |
| `harness.{signal_refs,child_harness_receipt_refs}` | `lineage.signal_refs` / `lineage.children`. |
| `sync_points[]` | `lineage.sync[]`. |
| Reference object `{schema,type,uri,provider,locator,label,observed_at}` | Self-describing `runx:<type>:<id>` URI string; rich metadata (provider/locator/label/observed_at) lives in the referenced artifact. |

Two edges that previously had no clean home and now do: runner provenance
(`acts[].by` for the trainable-export projection) and suspended runs (`deferred` +
`resume_ref` instead of a `seal: null` carve-out).

## Objectives

- Define `runx.receipt.v1` as the single receipt contract, flat (no `harness`
  wrapper), one seal, idempotency top-level, authority fully inspectable.
- Move planner deliberation (former `decisions[]`) into a referenced journal
  artifact committed by `journal_ref` hash; preserve the act-id integrity property
  through journal verification.
- Reference per-act detail via `act-receipt` (`detail_ref`) instead of inlining
  intent/target/source/surface refs.
- Cut the Rust contracts, the receipts kernel (canonical/verify), signing
  (`runx-runtime/src/receipts/signing.rs`), the runtime emitter, and cloud reads over
  to `runx.receipt.v1` in one move.
- Reclaim the name: rename the existing run-summary contract `runx.receipt.v1` ->
  `runx.run-summary.v1` (and its TS consumers), giving it a `receipt_ref`, so the
  governance receipt can own `runx.receipt.v1`.
- Move the durable identity: `ReferenceType::HarnessReceipt` -> `ReferenceType::Receipt`
  and `HARNESS_RECEIPT_REF_PREFIX = "runx:harness_receipt:"` -> `"runx:receipt:"`
  (`crates/runx-runtime/src/journal.rs`, `payment_ledger.rs`, the tree prefix strips).
- Pin journal-ref semantics: the planner journal is a local artifact written beside
  the receipt in the receipt dir; `lineage.journal_ref` resolves there; verification
  fails closed if `journal_ref` is present but the journal is missing or its hash
  mismatches; no cross-trust-boundary read (the journal shares the receipt's trust).
- Delete the old shape entirely across all ~250 OSS files plus cloud: no
  `harness_receipt`/`HarnessReceipt`/`harness-receipt`/`harness.seal` token,
  `runx.harness_receipt.v1` schema, fixtures, or `runx:harness_receipt:` URI remains.
  No alias, no dual-shape, no compat code, no stored-data migration shim.

## Scope

In scope (OSS repo, run from `oss/`):
- `crates/runx-contracts/src/*` receipt/act/authority/seal/idempotency types.
- `crates/runx-receipts/src/{canonical,verify,tree}.rs` and the digest body.
- `crates/runx-runtime/src/receipts/{seal,signing}.rs`, the runtime emitter, the
  planner journal (`crates/runx-runtime/src/journal.rs`), and `payment_ledger.rs`
  (re-seeded against the new URI prefix).
- The receipt + run-summary contracts in `packages/contracts/src/schemas/` and
  `schemas/*.json` regeneration; `ReferenceType` in the reference contract.
- Fixtures under `fixtures/contracts/harness-spine/*` and the canonical-json oracle.

In scope (cloud repo, run from `cloud/` separately — `oss`/`cloud` is a documented
repo boundary, not a single working dir):
- Cloud reads in `cloud/packages/{db,api,worker}` that touch `idempotency` and
  `seal` (confirm they resolve against the flat shape; `db` package is `@runx/db`).

Out of scope (and explicitly forbidden):
- Any back-compat: `harness_receipt` aliases, dual-shape readers, compat shims, or
  old-shape glue code. None may remain after this task.
- The contract-pipeline source-of-truth flip (owned by
  `rust-contract-pipeline-inversion`); author the new shape in the current source.
- Redesigning `act-receipt`, `signal`, or `authority` semantics beyond referencing
  them; the spend-capability bounds shape is preserved as-is, only relocated under
  the flat `authority`.

## Dependencies

- `canonical-json-fingerprint-contract-v1` (the canonicalization byte contract; the
  new `runx.receipt.c14n.v1` rides on it).
- The act-model reconciliation: this spec settles the durable receipt shape; the
  schema and the runtime emitter currently disagree on `authority`/`enforcement`,
  and this cutover makes them agree on `runx.receipt.v1`.

## Touchpoints

- `crates/runx-contracts/src/` (receipt, act, authority, seal, idempotency, reference)
- `crates/runx-receipts/src/{canonical,verify,tree}.rs`
- `crates/runx-runtime/src/receipts/{seal,signing}.rs`, `crates/runx-runtime/src/journal.rs`,
  `crates/runx-runtime/src/payment_ledger.rs`
- `packages/contracts/src/schemas/receipt.ts` (run-summary, renamed) + new governance
  receipt; `RUNX_LOGICAL_SCHEMAS`/`RUNX_CONTRACT_IDS`
- `schemas/harness-receipt.schema.json` (deleted); `schemas/receipt.schema.json`
  (now the governance receipt); `schemas/run-summary.schema.json` (new)
- `cloud/packages/db/src/*` (`@runx/db`), `cloud/packages/api`, `cloud/packages/worker`
- `fixtures/contracts/harness-spine/*`, `fixtures/contracts/canonical-json/*`

## Risks

- Digest change: flattening, the single seal, and `decisions[]` -> `journal_ref`
  change what the body commits. This is a deliberate version break
  (`runx.receipt.v1` / `runx.receipt.c14n.v1`), not an in-place mutation. All
  fixtures and the oracle regenerate together.
- Coordinated cutover with no bridge: because there is no compat layer, the emitter,
  verifier, cloud, and fixtures must all land in the same change or receipts fail to
  round-trip. Sequence the build so the kernel and emitter flip together before the
  no-compat sweep asserts.
- Lost proof by over-collapsing: `idempotency`, `authority.terms.bounds.payment`,
  attenuation/proof refs, and the hash commitments are durable proof. The shape keeps
  them; acceptance asserts they survive.
- Journal integrity: moving deliberation out must not weaken the `selected_act_id`
  integrity guarantee; the journal is committed by `journal_ref` and verified.
- Blast radius: the no-compat rename touches ~250 OSS files plus cloud, the
  `ReferenceType` enum, the durable URI prefix, and the payment-ledger projection. It
  must land as one coordinated change; partial application leaves the tree non-compiling
  by design (no compat seam to bridge it).
- Name collision: `runx.receipt.v1` is currently the CLI run-summary. The governance
  receipt cannot take the name until the run-summary is renamed first (Phase 1, before
  any governance-receipt work), or both contracts collide.

## Acceptance

Profile: strict

Definition of done:
- [ ] `dod1` `runx.receipt.v1` is the only receipt contract: flat top level (no
  `harness` wrapper), exactly one `seal`, `idempotency` top-level, `subject` with
  `kind: skill|graph` and `commitments[]`, `authority` carrying
  `terms[].bounds.payment` inspectably, `acts[]` with `criteria` bindings and
  `detail_ref`, optional `lineage` with `journal_ref`.
- [ ] `dod2` No back-compat remains: a scan finds no `harness_receipt`,
  `HarnessReceipt`, `harness-receipt`, `harness.seal`, `runx.harness_receipt.v1`, or
  `runx:harness_receipt:` token in the OSS tree (`crates`, `schemas`, `fixtures`,
  `packages`) and, scanned separately, in `cloud/packages`. No alias, dual-shape
  reader, or stored-data migration shim exists.
- [ ] `dod8` The run-summary is renamed: `runx.receipt.v1` no longer means the CLI
  run-summary; `packages/contracts/src/schemas/receipt.ts` content is
  `runx.run-summary.v1` with a `receipt_ref`, its consumers updated, and
  `schemas/run-summary.schema.json` emitted.
- [ ] `dod9` Durable identity moved: `ReferenceType::Receipt` replaces
  `ReferenceType::HarnessReceipt`, the prefix is `runx:receipt:`, and
  `payment_ledger.rs` projects over the new prefix.
- [ ] `dod10` Journal-ref semantics hold: the journal is written beside the receipt;
  verification of a receipt whose `journal_ref` is missing or hash-mismatched fails
  closed; a tampered `selected_act_id` is caught via the journal.
- [ ] `dod3` The receipts kernel operates on the flat shape: the body digest commits
  `idempotency`/`subject`/`authority`/`acts`/`seal`/`lineage`; `verify.rs` no longer
  has a seal-equality check; the `selected_act_id` integrity property is verified via
  the journal.
- [ ] `dod4` The runtime emits `runx.receipt.v1`, writes the planner journal as a
  separate artifact, and records `lineage.journal_ref`. No seal is written twice.
- [ ] `dod5` Cloud dedup (`idempotency.*`) and status (`seal.disposition`,
  `seal.closed_at`) resolve against the flat shape with no field-path changes beyond
  the wrapper removal.
- [ ] `dod6` Harness-spine fixtures are regenerated under `runx.receipt.v1`, the
  canonical-json oracle covers `runx.receipt.c14n.v1`, and old
  `runx.harness_receipt.v1` fixtures are deleted.
- [ ] `dod7` The payment authority bound survives intact: a spend receipt still
  carries `authority.terms[].bounds.payment` with the cap/rail/single_use_spend
  fields, inspectable (not hashed away), and `idempotency` is present.

Validation:
- [ ] `v1` Scafld validates this spec.
  - Command: `scafld validate runx-receipt-clean-shape-v1 --json`
  - Expected kind: `exit_code_zero`
  - Status: pending
- [ ] `v2` Receipts kernel tests pass on the flat shape.
  - Command: `cargo test --manifest-path crates/Cargo.toml -p runx-receipts`
  - Expected kind: `exit_code_zero`
  - Status: pending
- [ ] `v3` Runtime receipt emission + sealing tests pass.
  - Command: `cargo test --manifest-path crates/Cargo.toml -p runx-runtime receipt`
  - Expected kind: `exit_code_zero`
  - Status: pending
- [ ] `v4` No-compat sweep (OSS): zero old-shape references remain.
  - Command: `! rg -n 'harness_receipt|HarnessReceipt|harness-receipt|harness\.seal|runx:harness_receipt:' crates schemas fixtures packages`
  - Expected kind: `exit_code_zero`
  - Status: pending
- [ ] `v5` Canonical-json oracle covers the new receipt c14n.
  - Command: `cargo test --manifest-path crates/Cargo.toml -p runx-receipts canonical`
  - Expected kind: `exit_code_zero`
  - Status: pending
- [ ] `v6` No-compat sweep (cloud) + cloud db tests pass against the flat shape.
  - Command: `cd ../cloud && rg -n 'harness_receipt|HarnessReceipt|runx:harness_receipt:' packages; test $? -eq 1 && pnpm --filter @runx/db test`
  - Expected kind: `exit_code_zero`
  - Status: pending

## Phase 1: Contract (ground level)

Status: completed
Dependencies: none

Objective: Complete this phase.

Changes:
- none

Acceptance:
- none

## Phase 2: Rust contract types

Status: completed
Dependencies: Phase 1

Objective: Complete this phase.

Changes:
- none

Acceptance:
- none

## Phase 3: Receipts kernel

Status: completed
Dependencies: Phase 2

Objective: Complete this phase.

Changes:
- none

Acceptance:
- none

## Phase 4: Runtime emitter + journal

Status: completed
Dependencies: Phase 3

Objective: Complete this phase.

Changes:
- none

Acceptance:
- none

## Phase 5: Cloud + fixtures + no-compat sweep

Status: completed
Dependencies: Phase 4

Objective: Complete this phase.

Changes:
- none

Acceptance:
- none

## Rollback

The cutover is a version break with no compat layer, so rollback is `git revert` of
the coordinated change, not a feature flag. There is no half-state to leave: either
`runx.receipt.v1` is the governance contract end-to-end, or the change is reverted
whole. Stored pre-cutover receipts/ledger are re-seeded from source either way (no
migration shim exists to roll back).

## Review

Status: completed
Verdict: fail
Mode: discover
Provider: claude:claude-opus-4-7
Output: claude.mcp_submit_review
Summary: Architectural cutover landed (flat `runx.receipt.v1`, single seal, `ReferenceType::Receipt`, `runx:receipt:` prefix, run-summary renamed), and the OSS no-compat sweep on `crates/schemas/fixtures/packages` is clean. But three blockers fall out of an adversarial pass: (1) the runtime computes the planner journal only to derive `lineage.journal_ref` and never persists it beside the receipt, so dod4/dod10 ("journal is written beside the receipt", "tampered selected_act_id is caught via the journal") cannot hold for any emitted receipt; (2) `cloud/packages/api/src/server-persistence.ts` contains duplicate imports and duplicate `receiptStore` keys (interface and both factory branches), which fails TypeScript compilation in a declared in-scope file; (3) `cloud/packages/api` still surfaces the `harness-receipt` token (HTTP routes `/v1/harness-receipts`, file path `harness-receipts.json`, error strings, mirrored in tests), which dod2 forbids — and acceptance v6's sweep regex omits `harness-receipt`, so the gate passes silently. Lower-priority findings: the journal hash is computed with plain `serde_json::to_string` (no canonical-json contract), the `Receipt.metadata` doc comment falsely claims it "never appears in the TS contract" while it does, and the v6 cloud sweep regex is weaker than dod2.

Attack log:
- `OSS no-compat sweep (crates schemas fixtures packages)`: rg the dod2 forbidden-token regex across the four OSS directories -> clean (Zero matches in crates/, schemas/, fixtures/, packages/ — the in-scope OSS cutover is genuinely complete.)
- `cloud/packages no-compat sweep`: rg the dod2 forbidden-token regex across cloud/packages -> finding (F-003: 18+ matches of harness-receipt token across cloud/packages/api (routes, file path, error strings, tests). v6's regex misses this set.)
- `runtime journal-write path`: Grep for journal persistence (write_journal/.journal.json/ReceiptJournal sinks); read seal_receipt() end-to-end -> finding (F-001: seal.rs constructs ReceiptJournal, digests it, sets lineage.journal_ref, then drops the journal. No store.write_journal, no fs::write.)
- `cloud/packages/api/src/server-persistence.ts`: Read in scope file for shape and consistency after cutover -> finding (F-002: duplicate imports and duplicate object/interface keys for receiptStore. File cannot type-check.)
- `Receipt::metadata documentation vs TS contract`: Cross-check Rust struct doc comment against packages/contracts/src/schemas/receipt.ts and schemas/receipt.schema.json -> finding (F-004: doc says metadata never appears in TS contract; it does.)
- `ReceiptJournal digest determinism`: Compare ReceiptJournal::digest() against canonical_receipt_body_digest path -> finding (F-005: digest uses plain serde_json::to_string with unwrap_or_default; not under runx.receipt.c14n.v1.)
- `Acceptance command set (v4/v6)`: Recompute the regex coverage against dod2's forbidden-token list -> finding (F-006: v6 omits harness-receipt and harness.seal. v4 covers dod2 tokens but its negation pattern still trusts that all four paths exist (they do here, but the harden-round critique still holds).)
- `Verify removes seal-equality check`: Read crates/runx-receipts/src/verify.rs for any residual receipt.seal == receipt.harness.seal check -> clean (The nested harness wrapper is gone; no seal-equality assertion remains. dod3 holds.)
- `Reference URI prefix migration`: Grep journal.rs / payment_ledger.rs / reference.rs for the new runx:receipt: prefix and absence of old prefix -> clean (RECEIPT_REF_PREFIX = "runx:receipt:" (journal.rs:26); payment_ledger.rs uses it (lines 661/665); ReferenceType::Receipt is the variant. dod9 holds in code.)
- `Schema fixtures validate against new schema`: Confirm schema_validation.rs maps receipt-success/receipt-abnormal/post-merge-observer to receipt.schema.json -> clean (Mappings present at tests/schema_validation.rs:165-178; fixtures show flat shape; old harness-receipt fixtures are deleted per task_changes.)
- `Run-summary rename`: Confirm runx.run-summary.v1 owns the CLI run-summary and runx.receipt.v1 is reclaimed -> clean (packages/contracts/src/internal.ts:62-63 and schemas/run-summary.schema.json:18 confirm; dod8 holds.)
- `Scope drift inside in-scope files`: Diff observed changes against declared task scope/touchpoints -> clean (Touchpoints align with task_changes (contracts/, receipts/canonical+verify+tree, journal.rs, payment_ledger.rs, schemas/{receipt,run-summary}.schema.json, fixtures/contracts/harness-spine/*). No undeclared in-scope rewrites observed.)
- `Ambient drift classification`: Cross-check the 154 ambient-drift entries against task scope so I don't mis-attribute their state to this task -> clean (Ambient drift in crates/runx-cli/tests, crates/runx-runtime/src/{execution,doctor,dev}, runx-parser, etc. is large but outside this task's declared touchpoints; not scored as findings.)

Findings:
- [critical/blocks completion] `F-001` Runtime never writes the planner journal artifact, so journal_ref points at a non-existent file (dod4/dod10 cannot hold).
  - Location: `crates/runx-runtime/src/receipts/seal.rs:721`
  - Evidence: crates/runx-runtime/src/receipts/seal.rs:714-740 — seal_receipt() builds the journal via receipt_journal(receipt), computes journal.digest(), sets lineage.journal_ref.locator to that digest, and then drops the journal value at the end of the function. No fs::write, no LocalReceiptStore call, no helper anywhere persists ReceiptJournal: a grep for `ReceiptJournal` across `crates/runx-runtime/src` shows only the construction path in seal.rs; `write_journal`/`.journal.json`/`journal.json` find no hits. crates/runx-runtime/src/receipts/store.rs::write_receipt only persists the receipt JSON. Result: every emitted receipt commits a journal_ref pointing to a journal artifact that does not exist on disk; verify_receipt_with_journal can be exercised by unit tests but cannot succeed against any runtime-stored receipt, and the dod10 property "a tampered selected_act_id is caught via the journal" cannot hold because there is no journal to consult.
  - Impact: Violates Acceptance dod4 ("The runtime emits runx.receipt.v1, writes the planner journal as a separate artifact, and records lineage.journal_ref") and dod10 ("the journal is written beside the receipt; verification of a receipt whose journal_ref is missing or hash-mismatched fails closed; a tampered selected_act_id is caught via the journal"). The decisions→act-id integrity guarantee the spec promises to preserve through journal verification is provably absent at the runtime/store boundary, defeating the load-bearing motivation for the journal_ref indirection.
  - Validation: After the fix lands, an integration test should: (a) drive the runtime through one sealed step, (b) assert a journal file exists in the receipt store directory keyed off receipt.id, (c) re-read both artifacts and call verify_receipt_with_journal(receipt, journal) successfully, (d) mutate the journal on disk (or the receipt.acts[].id) and assert verify_receipt_with_journal now returns JournalHashMismatch / DecisionSelectedActMissing respectively. `cargo test --manifest-path crates/Cargo.toml -p runx-runtime receipt journal` should cover the path end-to-end.
- [critical/blocks completion] `F-002` cloud/packages/api/src/server-persistence.ts has duplicate imports and duplicate `receiptStore` keys; the file cannot type-check.
  - Location: `cloud/packages/api/src/server-persistence.ts:18`
  - Evidence: cloud/packages/api/src/server-persistence.ts:18 imports `{ FileHostedReceiptStore, PostgresHostedReceiptStore, type HostedReceiptStore }` from `../../db/src/receipts.js`; lines 19-25 import the same identifiers (`FileHostedReceiptStore`, `PostgresHostedReceiptStore`, `HostedReceiptStore`) from `../../receipts-store/src/index.js`. The `HostedPersistence` interface declares `receiptStore: HostedReceiptStore;` twice (lines 40 and 41). Both factory branches construct it twice — lines 66-67 (memory mode) and lines 112-116 (postgres mode). TypeScript rejects duplicate identifiers in an import list and duplicate property names in an object type; even where compilation may be loose, the second binding silently shadows the first, so the intended cutover wiring is non-deterministic.
  - Impact: cloud/packages/api is declared task scope (Touchpoints). A file that does not type-check (or whose receiptStore is the wrong one at runtime) is a regression introduced or left by the cutover and violates `pnpm typecheck`. dod5 (cloud reads resolve against the flat shape) cannot be verified while the file is in this state.
  - Validation: After fix: `pnpm --dir cloud typecheck` (or the cloud project's typecheck script) passes with no "Duplicate identifier" / "An object literal cannot have multiple properties with the same name" errors. The `HostedPersistence` interface has a single `receiptStore: HostedReceiptStore;` field, and both factory return objects have exactly one `receiptStore:` key constructed from the chosen source module.
- [high/blocks completion] `F-003` cloud/packages still contains `harness-receipt` tokens (HTTP routes, file path, error strings); dod2 forbids them and v6's sweep regex silently misses them.
  - Location: `cloud/packages/api/src/harness-routes.ts:114`
  - Evidence: dod2: "a scan finds no harness_receipt, HarnessReceipt, harness-receipt, harness.seal, runx.harness_receipt.v1, or runx:harness_receipt: token … in cloud/packages." Grep of `cloud/packages` matches the dod2 pattern set in: cloud/packages/api/src/harness-routes.ts:114,161,172 (`app.post("/v1/harness-receipts", …)`, `app.get("/v1/harness-receipts", …)`, `app.get("/v1/harness-receipts/:id", …)`); cloud/packages/api/src/server-persistence.ts:67 (`FileHostedReceiptStore(path.join(config.receiptsDir, "harness-receipts.json"))`); cloud/packages/api/src/index.test.ts:631..847 (12 calls to `/v1/harness-receipts...`). Acceptance v6's command is `rg -n 'harness_receipt|HarnessReceipt|runx:harness_receipt:' packages` — it omits `harness-receipt` and `harness.seal`, so the gate passes despite the dod2 violation.
  - Impact: Violates dod2 directly. The OSS no-compat property is real, but the cloud side of the cutover still surfaces the old name on the wire (HTTP path), in stored data (`harness-receipts.json`), and in user-facing error strings; consumers will continue to compile against /v1/harness-receipts indefinitely. Worse, the acceptance gate (v6) was authored too narrowly and reports green even when the violation is present, so this is also an acceptance-validation gap.
  - Validation: After fix: from cloud/, `rg -n 'harness_receipt|HarnessReceipt|harness-receipt|harness\.seal|runx\.harness_receipt\.v1|runx:harness_receipt:' packages` returns exit code 1 (no matches). The cloud API tests are updated to call the new path and pass. Update v6 in the spec to the full regex so the gate matches dod2.
- [medium/non-blocking] `F-004` Receipt doc comment falsely claims `metadata` "never appears in the TS contract" while it does.
  - Location: `crates/runx-contracts/src/harness.rs:244`
  - Evidence: crates/runx-contracts/src/harness.rs:244-248 states: "`metadata` is a runtime-local read aid … it never appears in the TS contract." But packages/contracts/src/schemas/receipt.ts:181-184 declares `metadata: Type.Optional(unknownRecordSchema())` as part of `receiptV1Schema`, and schemas/receipt.schema.json:6778-6783 exposes it as a top-level optional property. Both contracts agree it exists; the Rust doc is the outlier.
  - Impact: Doc rot creates a contract-ambiguity hazard: a future maintainer reading harness.rs may try to strip `metadata` from the TS contract or refuse to set it, breaking the read-aid that history projection (`crates/runx-runtime/src/journal.rs:369-377`) depends on. Minor severity because the runtime code does the right thing; only the comment is wrong.
- [medium/non-blocking] `F-005` ReceiptJournal::digest() uses plain `serde_json::to_string`; the journal hash is not under `runx.receipt.c14n.v1` canonicalization.
  - Location: `crates/runx-receipts/src/verify/journal.rs:27`
  - Evidence: crates/runx-receipts/src/verify/journal.rs:25-32 — `digest()` returns `sha256_prefixed(serde_json::to_string(self).unwrap_or_default().as_bytes())`. Unlike `canonical_receipt_body_digest`/`canonical_json_value` (canonical.rs:46-81) which sorts object keys, the journal hash relies on Rust's serde struct-field order and the silent default-string fallback (empty string → fixed hash of "") on serialization failure. `ReceiptJournalDecision { #[serde(flatten)] decision: Decision }` (journal.rs:18-23) inherits Decision's many-field order; any external producer (e.g., a TS journal writer) using a different field order would produce a non-matching digest and fail closed even when semantically identical, and a corrupted-on-read journal whose `serde_json::to_string` errors would silently hash the empty string.
  - Impact: The journal is supposed to be the durable proof-by-hash artifact backing the act-id integrity property. Tying its digest to Rust's incidental serializer rather than the agreed `runx.receipt.c14n.v1` byte contract means: (a) only the Rust runtime can author or re-verify it, (b) round-tripping through any other tool can silently invalidate, and (c) the `unwrap_or_default()` swallows a failure path. The risk window is bounded because the journal is currently written by Rust and read by Rust, but it grows the moment another producer (CLI test, cloud) needs to mint or verify one.
- [low/non-blocking] `F-006` Acceptance v6's sweep regex is narrower than dod2; the cloud no-compat gate passes silently when `harness-receipt`/`harness.seal` tokens remain.
  - Location: `.scafld/specs/active/runx-receipt-clean-shape-v1.md:384`
  - Evidence: Spec Acceptance v6: `cd ../cloud && rg -n 'harness_receipt|HarnessReceipt|runx:harness_receipt:' packages; test $? -eq 1 && pnpm --filter @runx/db test`. dod2's forbidden-token set adds `harness-receipt`, `harness.seal`, `runx.harness_receipt.v1` (which are present in cloud/packages — see F-003). v6 cannot fail on those tokens. The harden round-1 round (`harden-3`) already flagged a related shape of this defect with the `@runx/cloud-db` package name; this is the second-order miss.
  - Impact: The acceptance gate that is supposed to enforce the cloud half of dod2 cannot detect three of the five forbidden tokens, so a passing v6 is consistent with a failing dod2. As a strict-profile acceptance, this is a gate-validity problem even when the underlying code is also wrong.

## Self Eval

- none

## Deviations

- none

## Metadata

- created_by: scafld

## Origin

Created by: scafld
Source: plan
Conversation 2026-05-22: receipt shape reconsidered from the ground up. The
`harness_receipt` shape conflates proof, deliberation, and per-act detail; the user
called for the ultimate clean shape and a clean cutover with zero compat. Shape
verified against the schemas, a generated receipt, and a consumer sweep before
spec'ing.

## Harden Rounds

### round-1

Status: needs_revision
Started: 2026-05-22T07:37:56Z
Ended: 2026-05-22T07:37:56Z
Verdict: needs_revision
Provider: claude
Model: claude-opus-4-7
Output format: claude.mcp_submit_harden
Summary: Draft has the right diagnosis (collapsed harness/seal duplication, deliberation inlining, restated success_criteria) and a coherent target shape, but four blockers prevent approval: (1) the logical schema name `runx.receipt.v1` and the filename `schemas/receipt.schema.json` are already taken by the CLI run-summary receipt in `packages/contracts/src/schemas/receipt.ts` and `RUNX_LOGICAL_SCHEMAS.receipt`, so a no-compat cutover collides with an existing in-product contract; (2) the cutover crosses the documented `oss/` ↔ `cloud/` repo boundary (CLAUDE.md), yet acceptance/sweep commands and touchpoints assume a single working directory — `cloud/` is not a sibling of `crates/` from `oss/` and the `pnpm --filter @runx/cloud-db test` filter does not match any package (the package is `@runx/db`); (3) Phase 3 names a kernel file that does not exist (`crates/runx-receipts/src/signing.rs`); signing actually lives in `crates/runx-runtime/src/receipts/signing.rs`; (4) the no-compat assertion underestimates blast radius — 1,416 matches across 250 OSS files plus durable `ReferenceType::HarnessReceipt` / `HARNESS_RECEIPT_REF_PREFIX = "runx:harness_receipt:"` URIs in stored receipts that the spec doesn't address. Journal-ref verification semantics (load path, missing-journal failure mode, cross-trust-boundary reads) also need to be pinned before approval.

Checks:
- path audit
  - Grounded in: code:schemas/receipt.schema.json:1
  - Result: failed
  - Evidence: schemas/receipt.schema.json already exists and binds the logical name runx.receipt.v1 to the CLI run-summary shape ({run_id, command, status, started_at, root, steps}); packages/contracts/src/internal.ts:62 declares receipt: 'runx.receipt.v1' and packages/contracts/src/index.ts:698 maps 'receipt.schema.json' to receiptV1Schema. The spec's claim that it can write the new shape at this filename/name with no compat layer is unsupported. Also, Phase 3 names crates/runx-receipts/src/signing.rs which does not exist; signing lives at crates/runx-runtime/src/receipts/signing.rs (verified via Glob).
- command audit
  - Grounded in: code:cloud/packages/db/package.json:2
  - Result: failed
  - Evidence: Acceptance v6 runs `pnpm --filter @runx/cloud-db test`, but cloud/packages/db/package.json declares the package name as '@runx/db' (line 2). The filter matches zero packages and exits 0 silently, falsely satisfying the gate. Acceptance v4 (`! rg -n '...' crates cloud schemas fixtures`) is also unsafe from the OSS working directory: `cloud` is not a sibling of `crates` inside oss/ (cloud lives at /Users/kam/dev/runx/runx/cloud, sibling of oss/), so rg returns a non-zero exit for the missing path and the negation flips it to success — also a false pass.
- scope/migration audit
  - Grounded in: code:CLAUDE.md
  - Result: failed
  - Evidence: CLAUDE.md explicitly says `oss/` is for user-facing/product-agnostic code and `cloud/` is for hosted operations and 'do **not** belong in `oss/`'. The spec's touchpoints list cloud/packages/{db,api,worker} and fixtures/contracts/harness-spine/* and bundles them into one coordinated cutover. The 1,416 harness_receipt occurrences across 250 oss/ files (rg count) plus durable references — ReferenceType::HarnessReceipt (24 usages) and HARNESS_RECEIPT_REF_PREFIX='runx:harness_receipt:' at crates/runx-runtime/src/journal.rs:27 — extend the no-compat sweep into stored on-disk receipt URIs that the spec never enumerates.
- acceptance timing audit
  - Grounded in: spec_gap:acceptance
  - Result: failed
  - Evidence: v4 sweep depends on cross-repo presence of `cloud`; v6 names a non-existent pnpm filter (see command audit). v2 and v3 are reasonable but cannot pass until Phase 4 lands because runtime emitters in seal.rs construct HarnessReceipt today (crates/runx-runtime/src/receipts/seal.rs:115,228). dod3 says 'selected_act_id integrity is verified via the journal' but no acceptance command exercises that: verify.rs:194-207 (check_decisions) currently runs in-memory against harness.decisions; once decisions move out of the body, the spec doesn't say where verify reads the journal from, what happens if the journal is absent, or which test fixture proves the property.
- rollback/repair audit
  - Grounded in: spec_gap:Rollback
  - Result: failed
  - Evidence: Rollback is declared as `git revert` of 'the coordinated change'. With 1,416 OSS hits + cloud changes across two repositories (oss/ and cloud/, with cloud not in the OSS workspace), the 'one coordinated change' is implausible as a single commit. CLAUDE.md requires the OSS-published CLI not to depend on cloud, yet the cutover requires cloud reads to flip simultaneously, which crosses release boundaries. Rollback also doesn't address durable on-disk receipts under the local store (runx:harness_receipt: URIs) issued before the cutover — git revert restores code but not stored artifacts.
- design challenge
  - Grounded in: code:packages/contracts/src/internal.ts:62
  - Result: failed
  - Evidence: Architecturally the diagnosis is right: verify.rs:85-93 enforces seal==harness.seal solely because of the nested harness wrapper, and decisions[] is content-hashed but only used for the in-memory check_decisions act-id check (verify.rs:194-207). Flattening + journal_ref is sound. But the chosen schema name (runx.receipt.v1) is the third receipt named 'receipt' in this codebase (CLI run-summary, harness receipt, local-receipt schema_version='runx.receipt.v1' in packages/contracts/dist/src/schemas/local-receipt.d.ts:3). Without a distinct logical name (e.g., runx.governed_receipt.v1 or runx.harness_receipt.v2 with the harness wrapper removed), the cutover trades one ambiguity for another and breaks the existing CLI run receipt. This is fixable with a rename and a clear scope split: do the Rust kernel/runtime cutover in OSS first, treat cloud and durable-URI rename as separate, sequenced specs.

Issues:
- [critical/blocks approval] `harden-1` schema name collision - Proposed `runx.receipt.v1` / `schemas/receipt.schema.json` already names the CLI run-summary receipt.
  - Status: open
  - Grounded in: code:packages/contracts/src/internal.ts:62
  - Evidence: packages/contracts/src/internal.ts:62 (`receipt: 'runx.receipt.v1'`), packages/contracts/src/schemas/receipt.ts:14-30 (run_id/command/status/started_at/root/steps shape), packages/contracts/src/index.ts:698 (`'receipt.schema.json': receiptV1Schema`), and packages/contracts/dist/src/schemas/local-receipt.d.ts:3 (`schema_version: 'runx.receipt.v1'`) all bind that name to a different artifact. Spec dod1/Phase 1 mandate the new flat harness shape inherits exactly this name and filename with no compat layer.
  - Recommendation: Pick a distinct logical name (e.g., `runx.governed_receipt.v1`, `runx.execution_receipt.v1`, or `runx.harness_receipt.v2` with the wrapper removed) and a distinct filename. If the team really wants `runx.receipt.v1` to belong to the new shape, treat the CLI run-summary rename as an explicit prerequisite spec — not as silent collateral damage of this one.
  - Question: Which logical schema name and filename will the new flat shape take, given that `runx.receipt.v1` / `schemas/receipt.schema.json` already belong to the CLI run-summary?
  - Recommended answer: Adopt `runx.harness_receipt.v2` (same artifact, new shape) at `schemas/harness-receipt.schema.json` — preserves the CLI receipt and keeps cross-references searchable; the 'no harness_receipt' clause in dod2 then targets `v1` specifically.
  - If unanswered: Default to `runx.harness_receipt.v2` at the existing path and rewrite dod2 to forbid only the v1 logical name and HarnessReceipt Rust type.
- [critical/blocks approval] `harden-2` scope boundary - Cutover crosses the documented `oss/` / `cloud/` boundary and uses commands that cannot run from the OSS working directory.
  - Status: open
  - Grounded in: code:CLAUDE.md
  - Evidence: CLAUDE.md ('oss/ is for user-facing capabilities; cloud/ is for hosted operations; admin/control-plane features do not belong in oss/'). Spec touchpoints include cloud/packages/{db,api,worker}; acceptance v4 references the path `cloud` from oss/ (cloud is /Users/kam/dev/runx/runx/cloud, sibling of oss/); v6 uses `pnpm --filter @runx/cloud-db test` while cloud/packages/db/package.json:2 declares `@runx/db`.
  - Recommendation: Split into two specs: (a) OSS cutover (contracts/kernel/runtime/fixtures/oracle), with sweep paths restricted to crates schemas fixtures and tests inside oss/; (b) cloud cutover (cloud/packages/{db,api,worker}, the harness-receipts.ts dedupe path, the check-harness-data-cutover.mjs script) tracked as a coordinated downstream change. Document the temporal contract between them (versioned schema, atomic rollout, read-side tolerance window).
  - Question: Are cloud changes in-scope for this single spec, or do we split OSS and cloud into sequenced specs sharing the same version identifier?
  - Recommended answer: Split. Land the OSS cutover first under a new logical name, then cut cloud over in a sibling spec; co-version them so the cloud spec can assert against the same schema artifact.
  - If unanswered: Default to OSS-only here and open a follow-up `runx-receipt-cloud-cutover-v1` covering cloud/packages/{db,api,worker}, harness-routes.ts, and check-harness-data-cutover.mjs.
- [critical/blocks approval] `harden-3` acceptance command is a silent no-op - `pnpm --filter @runx/cloud-db test` matches no package and exits 0; the no-compat sweep on `cloud` errors then negates to success.
  - Status: open
  - Grounded in: code:cloud/packages/db/package.json:2
  - Evidence: cloud/packages/db/package.json:2 declares `"name": "@runx/db"`. v4's `! rg -n '...' crates cloud schemas fixtures` runs from oss/, where `cloud/` doesn't exist; rg exits non-zero for a missing path and `!` flips it to 0, falsely satisfying the gate.
  - Recommendation: Rewrite v4 to `! rg -n 'harness_receipt|HarnessReceipt|harness-receipt|harness\.seal' crates schemas fixtures packages` (paths that exist inside oss/, including packages/ for TS), and either drop v6 or, if the cloud cutover is kept in scope, change it to `pnpm --filter @runx/db test` and run it from `cloud/` (not `oss/`).
  - Question: What is the exact command set that will be run from `oss/` to prove no-compat, and how is the cloud side proven separately?
  - Recommended answer: OSS sweep: `! rg -n 'harness_receipt|HarnessReceipt|harness-receipt|harness\.seal' crates schemas fixtures packages tests` from oss/; cloud sweep + tests live in the cloud follow-up.
  - If unanswered: Default to the OSS-only sweep above and remove v6 from this spec.
- [high/blocks approval] `harden-4` phase touchpoint wrong - Phase 3 names `crates/runx-receipts/src/signing.rs` which does not exist; signing lives in the runtime.
  - Status: open
  - Grounded in: code:crates/runx-receipts/src
  - Evidence: Glob of crates/runx-receipts/src/*.rs returns only canonical.rs, lib.rs, tree.rs, verify.rs. Signing is at crates/runx-runtime/src/receipts/signing.rs.
  - Recommendation: Drop `signing.rs` from Phase 3 and either (a) add it to Phase 4 if the runtime signing path changes shape, or (b) leave runtime signing alone if the body-digest contract is the only change. Update touchpoints accordingly.
  - Question: Does the signing flow change beyond consuming the new body digest, and if so does the kernel grow a `signing.rs` or does Phase 4 own that change in the runtime?
  - Recommended answer: Phase 4 owns runtime signing edits; the kernel keeps canonical.rs/verify.rs/tree.rs only.
  - If unanswered: Default to Phase 4 owning runtime signing; remove the file from Phase 3.
- [high/blocks approval] `harden-5` missing migration of durable identifiers - Durable reference identifiers (`ReferenceType::HarnessReceipt`, `runx:harness_receipt:` URI prefix) are embedded in stored receipts and never enumerated.
  - Status: open
  - Grounded in: code:crates/runx-runtime/src/journal.rs:27
  - Evidence: crates/runx-runtime/src/journal.rs:27 (`HARNESS_RECEIPT_REF_PREFIX = 'runx:harness_receipt:'`); ReferenceType::HarnessReceipt has 24 usages across 11 files including verify.rs:1, tree.rs:8, post_merge_observer plan/runtime, target_runner.rs, receipts/seal.rs, plus tests. dod2 forbids any `harness_receipt` token in `crates`/`schemas`/`fixtures` after cutover.
  - Recommendation: Either keep the wire-level reference scheme stable (preserve `runx:harness_receipt:` and `ReferenceType::HarnessReceipt` as the lineage identifier even if the receipt's logical name changes) and narrow dod2 accordingly, or add an explicit migration step for local-store URIs and a rename of the enum variant + prefix, with a fixture that demonstrates pre-cutover stored receipts still verify (or are explicitly invalidated).
  - Question: Do we keep `ReferenceType::HarnessReceipt`/`runx:harness_receipt:` as the durable receipt-reference scheme (decoupled from the schema rename), or rename them and rebuild local stores?
  - Recommended answer: Keep the durable scheme stable; narrow dod2 to forbid `runx.harness_receipt.v1` (logical name) and `HarnessReceipt`-the-Rust-type for the old shape. Reference URIs and the lineage enum variant stay.
  - If unanswered: Default to keeping the durable reference scheme; document explicitly that the receipt schema rename does not touch lineage URIs.
- [high/advisory] `harden-6` journal verification semantics - Moving decisions[] to `journal_ref` removes an in-memory integrity check; the verifier's new contract is undefined.
  - Status: open
  - Grounded in: code:crates/runx-receipts/src/verify.rs:194
  - Evidence: verify.rs:194-207 walks `harness.decisions` against in-memory `acts[]`. Once decisions are committed only as a hash, verify needs the journal artifact to recompute the act-id check. The spec asserts the property is preserved 'through journal verification' without defining where the journal is loaded from, what happens when it is missing, or whether cloud read-side consumers must access it.
  - Recommendation: Add a paragraph under 'The runx.receipt.v1 shape' (or Phase 3) defining: (1) verifier interface for journal access (caller-provided loader vs. discovery via `journal_ref`); (2) verdict when the journal is unavailable (`unknown`/`partial` vs. `invalid`); (3) which consumers verify the journal vs. which just check the hash; (4) a fixture pair (receipt + journal) proving the property.
  - Question: What is the verifier's contract when `journal_ref` is present but the journal artifact is not provided?
  - Recommended answer: Verify treats journal-dependent checks as `unverified` (not `invalid`); receipt-shape and seal-binding checks still run; a dedicated `verify_with_journal(receipt, journal)` API computes the act-id integrity check and is the only path that produces a fully verified verdict.
  - If unanswered: Default to the recommended two-tier verifier and add a fixture under fixtures/contracts/harness-spine/journaled-acts.json plus a kernel test that exercises both paths.
- [medium/advisory] `harden-7` rollback credibility - Single `git revert` rollback is implausible given the cross-repo scope and durable on-disk artifacts.
  - Status: open
  - Grounded in: spec_gap:Rollback
  - Evidence: 1,416 OSS hits + cloud changes spanning two repos (oss/, cloud/); on-disk receipts written before the cutover carry `runx:harness_receipt:` URIs and the old body digest.
  - Recommendation: Tighten the rollback section to: (a) what's revertible by `git revert` (the OSS coordinated change), (b) what is not (durable stored receipts under the local receipt store and any cloud-side dedupe rows), (c) the operational stance for those — quarantine pre-cutover receipts and require re-emission, or keep them readable behind a tagged legacy verifier? This is a real choice the spec needs to make.
  - Question: How are pre-cutover stored receipts (local store + cloud dedupe rows) treated after rollback or after a forward roll?
  - Recommended answer: Forward: re-emit on next sealing; cloud-side keeps reads working by inspecting the schema version. Rollback: pre-cutover receipts already match the reverted code; only receipts emitted during the new-shape window need re-emission and are flagged in the journal.
  - If unanswered: Default to the above and add a paragraph to Rollback describing the durable-artifact stance.

### round-2

Status: in_progress
Started: 2026-05-22T07:51:05Z
Ended: none

Checks:
- none

Issues:
- none


## Planning Log

- none
