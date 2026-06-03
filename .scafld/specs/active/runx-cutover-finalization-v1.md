---
spec_version: '2.0'
task_id: runx-cutover-finalization-v1
created: '2026-06-03T00:00:00Z'
updated: '2026-06-03T00:00:00Z'
status: active
harden_status: not_run
size: large
risk_level: high
---

# runx cutover finalization: final shape + S-tier

## Current State

Status: active
Current phase: phase1
Next: dedup
Reason: A deep multi-dimension review of the landed cutover (commits 62b5b505
"finalize oss cutover" + f205f676 "finish effect supervisor naming cutover", on
top of 220c5a86) confirms the kernel reached final shape: in-kernel GitHub
provider clients deleted with pure halves relocated, payment fully extracted to
`runx-pay` (kernel is payment-free, the `--final` domain-free gate enforces it),
`@runxhq/core` flipped `private: true` with zero OSS importers, no coverage
regression, and the skill-safety inlining verified behavior-preserving. Cloud's
two headline duplications were already collapsed (cloud commit 313bb5d). What
remains is bounded S-tier polish, doc reconciliation, and ONE genuine final-shape
gap (payment-specific shapes still in the generic `runx-contracts` crate, which
the effect-kernel spec knowingly deferred).
Blockers: phase 3 (contracts genericization) is a `public_api_changes` surface
(`require_approval`) and MUST be additive/behavior-preserving or it breaks
external skills + the `@runxhq/contracts` mirror + canonical fixtures.
Allowed follow-up command: `scafld handoff runx-cutover-finalization-v1`
Latest runner update: 2026-06-03T00:00:00Z
Review gate: not_started

## Summary

The runx OSS cutover to "one governed core, many fronts" is substantially done and
high quality. This spec closes the residual gap between the current tree and the
final ultimate shape + S-tier code, in four phases ordered low-risk-first:

1. S-tier de-duplication + correctness (two MAJOR DRY/parity issues + small nits).
2. Doc reconciliation (one MAJOR stale feature list + minor citation drift).
3. Generic-contracts finalization (migrate payment-specific shapes out of the
   generic `runx-contracts` crate behind the generic effect seam, additively).
4. Green every CI gate and close.

It is deliberately scoped to OSS-now work. The cloud `@runxhq/core` sunset, the
real-rail build, the deferred settlement-receipt wiring, the OpenAPI front, and
the cloud-to-kernel bridge stay out of scope (future / separate specs).

## Objectives

- The generic crates (`runx-contracts`, `runx-runtime`, `runx-core`) carry no
  payment-specific shapes; payment is one opaque effect family. The domain-free
  gate covers `runx-contracts/src` too.
- Every cross-language invariant is single-source or pinned by a test: the tool
  `source_hash` algorithm agrees byte-for-byte across Rust and TS; the
  story-milestone vocabulary has one canonical definition.
- The docs match the code: feature lists, citations, and architecture notes are
  accurate at HEAD.
- All nine CI gates are green, with no skill broken and no contract shape changed
  in a way that breaks an existing skill, receipt, or the `@runxhq/contracts`
  mirror.

## Invariant (must-not-regress)

- **Skill/contract surface is frozen.** No `SourceKind` variant, runner-manifest
  ABI, `external-adapter`/`thread-outbox-provider` protocol, or CLI JSON shape is
  removed or renamed. Phase 3 contract changes are ADDITIVE only (new optional
  fields / new generic variants; old fields stay until a separate, approved
  breaking pass) so existing skills, sealed receipts, and the `@runxhq/contracts`
  mirror keep validating. Canonical JSON / receipt digests must not churn for
  existing fixtures.
- **Kernel stays payment-free.** `runx-runtime/src` + `runx-core/src` keep zero
  payment/spend/settlement/x402/rail identifiers (the `--final` gate in
  `scripts/check-runtime-cutover-legacy.mjs`); after phase 3 the same gate covers
  `runx-contracts/src`.
- **No in-kernel provider HTTP clients return.** Only the sanctioned network
  surfaces stay (governed HTTP front `runtime_http`, registry client, inbound MCP
  HTTP server, the in-binary agent resolver).
- **`@runxhq/core` stays `private: true`** with no new OSS importer; cloud importers
  remain on helper/data subpaths only.

## Scope

- In scope (OSS):
  - De-dup the story-milestone vocabulary (`tools/thread/story.ts` vs
    `tools/outbox/story.ts`).
  - Unify or pin the tool `source_hash` algorithm across `build.rs` and
    `authoring-utils.ts` (add a cross-language agreement test).
  - Small Rust nits: drop the now-dead `provider.rs` re-export, extract the
    duplicated `EffectReceiptRequest` builder in `authority.rs`, fix the
    write-only `_message` test field, remove two empty leftover dirs.
  - Doc fixes: `rust-kernel-architecture.md` feature lists; two
    `governed-execution-layer.md` citations; refresh the effect-kernel spec's
    stale "Current State" note.
  - Phase 3: migrate payment-specific shapes out of `runx-contracts` behind the
    generic `AuthorityEffectGuard` / `ProofKind::EffectSettlement` seam,
    additively; extend the domain-free gate to `runx-contracts/src`; finish the
    `EffectSupervisor` trait generalization (`naming-1`).
  - Phase 4: run and green all CI gates.
- Out of scope (future / separate specs):
  - Cloud `@runxhq/core` sunset (77 non-test / 101 incl. tests cloud importers
    across `/util`,`/registry`,`/artifacts`,`/parser`,`/config`); the package
    source cannot be deleted until cloud migrates. `cloud/` is edit-restricted.
  - Wiring `EffectSettlementReceipt` runtime emission (needs a real
    async/provisional rail — `payment-rails.md` Phase 1).
  - Real rails (x402/Stripe/MPP), the cloud-to-kernel bridge, the OpenAPI front
    (`governed-execution-layer.md` items 11-15, `payment-rails.md`).

## Dependencies

- The effect-kernel spec `oss/.scafld/specs/archive/2026-05/runx-effect-kernel-v1.md`
  (Phase 4 is explicitly deferred; phase 3 here is that deferred cleanup).
- The boundary source of truth `oss/docs/ts-interop-boundary.md` and
  `../../plans/governed-execution-layer.md`.
- Phase 3 touches public contract schemas; coordinate the `@runxhq/contracts` TS
  mirror + fixture cross-validation, and obtain approval (`public_api_changes`).

## Assumptions

- Verified at HEAD f205f676 (deep review, 2026-06-03): boundary clean, kernel
  payment-free, `@runxhq/core` `private: true` with zero OSS importers, all 10
  `SourceKind` variants intact, no coverage regression, inlining
  behavior-preserving, cloud duplications already collapsed (313bb5d).
- The two MAJOR findings (`dry-1` story duplication, `dry-2` hash parity) and the
  MAJOR `residue-1` (contracts payment shapes) survived adversarial verification.
- `runx-contracts` shapes are additively migratable (the generic
  `AuthorityEffectGuard` + `ProofKind::EffectSettlement` seam already exists
  alongside the payment-specific shapes).

## Risks

- **Canonical-JSON / receipt-digest churn (highest, phase 3).** `AuthorityBounds`,
  `ProofKind`, and the payment bounds are serialized into authorities and sealed
  receipts. A non-additive change re-hashes existing fixtures and breaks external
  skills + the `@runxhq/contracts` mirror. Mitigation: additive-only; keep the old
  fields/variants; cross-validate fixtures (`pnpm fixtures:*:check`); gate on
  unchanged digests for existing receipts.
- **Hash-parity fix masking a real divergence (phase 1, `dry-2`).** If the TS and
  Rust scanners already disagree on some tool, unifying them changes a manifest
  `source_hash`. Mitigation: add the agreement test FIRST against current tools;
  if it fails, that is a pre-existing bug to surface, not silently absorb.
- **Over-eager dedup breaking the invariant map (`dry-1`).** The
  `LEGACY_STORY_MILESTONE_ID_MAP` is invariant-critical. Mitigation: single-source
  it by re-export, not by hand-editing two copies; keep behavior byte-identical.
- **Scope creep into cloud.** Phase 3 must not chase the cloud `core` importers;
  that is a separate cutover. Stop at the OSS boundary.

## Acceptance

Profile: strict

Validation:
- `rg -n "PaymentRailSupervisor" crates/` returns none (already true; keep true).
- The generic crates carry no payment shapes: the `--final` domain-free scan, once
  extended, passes over `runx-contracts/src` as well as `runx-runtime/src` +
  `runx-core/src`.
- `tools/thread/story.ts` has no hand-maintained duplicate of
  `tools/outbox/story.ts`; one canonical milestone vocabulary.
- A cross-language test pins the tool `source_hash` (Rust `build.rs` ==
  TS `authoring-utils.ts`) on a fixture covering escaped/mixed quotes.
- `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --all-features
  -- -D warnings`, `cargo nextest run --workspace --all-features`,
  `cargo test --workspace --all-features --doc`, `pnpm verify:fast`,
  `pnpm fixtures:harness:check`, and the license-boundary checks are all green.
- The `issue-to-pr` graph still runs end-to-end; no `SourceKind`/protocol changed.

## Phase 1: S-tier de-duplication and correctness

Objective: remove the two MAJOR duplications and the small Rust nits the review
found, with no behavior change.

Changes:
- `dry-1` (MAJOR): make `oss/tools/thread/story.ts` re-export the milestone
  vocabulary from `oss/tools/outbox/story.ts` (or hoist both to one shared
  tool-local module). Delete the verbatim 51-line copy. Keep
  `LEGACY_STORY_MILESTONE_ID_MAP` single-source.
- `dry-2` (MAJOR): pick ONE canonical tool-source hasher. Preferred: Rust
  (`crates/runx-runtime/src/tool_catalogs/build.rs:253-419`) is canonical; the TS
  doctor (`packages/cli/src/authoring-utils.ts`) shells out to `runx` for the hash
  instead of re-scanning. If two implementations must remain, align the TS
  import-specifier scanner to the Rust char-scanner's escape/quote semantics and
  add a cross-language fixture test that pins them byte-for-byte.
- `indirection-1` (nit): delete `provider.rs:3`
  (`pub(super) use super::{github_pull_request_number, github_repository};`) and
  import directly from `super::` at the consumer (`target_runner/pull_request.rs:17`);
  consider folding the 25-line `provider.rs` into `target_runner.rs`.
- `dry-3` (nit): in `crates/runx-runtime/src/execution/runner/authority.rs:87-124`
  extract the identical 8-field `EffectReceiptRequest` builder shared by
  `finalize_effect_output_before_success` and `persist_effect_state_for_step`.
- `test-1` (nit): in `crates/runx-runtime/src/receipts/seal.rs:967-991` drop the
  write-only `_message` field (or assert on it).
- `kernel-2` cleanup (nit): remove the empty leftover dirs
  `crates/runx-runtime/src/payment` and `crates/runx-core/src/policy/payment_authority`.

Acceptance:
- [ ] `p1_ac1` command - story vocabulary is single-source
  - Command: `node -e "const a=require('fs').readFileSync('tools/outbox/story.ts','utf8');const b=require('fs').readFileSync('tools/thread/story.ts','utf8');process.exit(b.includes('LEGACY_STORY_MILESTONE_ID_MAP')&&!b.includes('export { ')&&!b.includes('from \"../outbox/story')?1:0)"`
  - Expected kind: `exit_code_zero`
- [ ] `p1_ac2` command - tool source-hash parity is pinned
  - Command: `pnpm vitest run packages/cli/src/authoring-utils` (or the new cross-language hash test)
  - Expected kind: `exit_code_zero`
- [ ] `p1_ac3` command - clippy clean after the Rust nits
  - Command: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  - Expected kind: `exit_code_zero`
- [ ] `p1_ac4` command - issue-to-pr graph still seals (story tools unaffected)
  - Command: `runx harness skills/issue-to-pr/<harness-case>.yaml --json`
  - Expected kind: `exit_code_zero`

## Phase 2: Doc reconciliation

Objective: docs match code at HEAD.

Changes:
- `drift-1` (MAJOR): `oss/docs/rust-kernel-architecture.md:124,:204,:690` — replace
  the stale `cli-tool, mcp, a2a, agent, catalog` lists with the real set
  (`cli-tool, mcp, mcp-http-server, a2a, agent, catalog, external-adapter, http`),
  flagging `a2a` as contract-defined but not enabled in `runx-cli`.
- `drift-2` (minor): `../../plans/governed-execution-layer.md:32` — repoint the
  `finalize_output` citation to `runx-pay/src/runtime.rs:331-362`.
- `drift-3` (nit): `../../plans/governed-execution-layer.md:131` — fix the Cargo
  anchor to `runx-cli/Cargo.toml:29`.
- Refresh the effect-kernel spec's "Current State" note where it is stale on the
  completed `PaymentRailSupervisor` removal (`kernel-1`).
- Do NOT "fix" the cloud agent-runner "single-shot" wording: `tool-loop.ts` was
  deleted (313bb5d), so single-shot is now correct (`drift-ok-1`).

Acceptance:
- [ ] `p2_ac1` command - no stale feature list remains
  - Command: `rg -n "cli-tool, mcp, a2a, agent, catalog" docs/`
  - Expected kind: `no_matches`
- [ ] `p2_ac2` manual - citations in governed-execution-layer.md resolve to the cited symbols
  - Expected kind: `manual`

## Phase 3: Generic-contracts finalization (the one real final-shape gap)

Objective: the generic `runx-contracts` crate carries no payment-specific shapes;
payment proves through the generic seam. ADDITIVE and behavior-preserving.

Changes:
- `residue-1` (MAJOR): migrate the payment-specific shapes in
  `crates/runx-contracts/src/authority.rs` (`PaymentAuthorityBounds:67`,
  `AuthorityBounds.max_spend_usd:137`, `AuthorityBounds.payment:139`,
  `AuthorityResourceFamily::Payment:16`, `AuthorityCapability::PaymentSingleUseSpend`,
  the two payment `AuthorityConditionPredicate` variants, `PaymentCredentialForm`)
  and `crates/runx-contracts/src/reference.rs:105` (`ProofKind::PaymentRail`) into
  `runx-pay`, behind the generic `AuthorityEffectGuard` + `ProofKind::EffectSettlement`
  seam. Do it additively: payment expresses its bounds/proof through the generic
  guard/payload; keep the old fields/variants present (deprecated) until a separate
  approved breaking pass, so canonical JSON for existing authorities/receipts does
  not churn.
- `naming-1` (minor): finish the `EffectSupervisor` generalization
  (`crates/runx-pay/src/runtime.rs:42`) so the method/types are family-agnostic
  (`supervise(request) -> evidence` over an opaque payload), not
  payment-settlement-only. (Resolves the "generic name over payment-only types"
  half-abstraction.)
- Extend `scripts/check-runtime-cutover-legacy.mjs` `--final` domain-free scan to
  include `crates/runx-contracts/src`, so the contracts crate cannot reaccumulate
  payment identifiers.

Acceptance:
- [ ] `p3_ac1` command - generic crates are payment-free under the widened gate
  - Command: `node scripts/check-runtime-cutover-legacy.mjs --final`
  - Expected kind: `exit_code_zero`
- [ ] `p3_ac2` command - canonical fixtures unchanged (additive migration)
  - Command: `pnpm fixtures:harness:check && cargo nextest run --workspace --all-features`
  - Expected kind: `exit_code_zero`
- [ ] `p3_ac3` command - the @runxhq/contracts TS mirror + fixtures still cross-validate
  - Command: `pnpm verify:fast`
  - Expected kind: `exit_code_zero`
- [ ] `p3_ac4` manual - no existing receipt/authority digest changed; payment skills (x402/stripe-spt dogfoods) still seal
  - Expected kind: `manual`

## Phase 4: Green all gates and close

Objective: prove the finalized tree green end to end.

Acceptance:
- [ ] `p4_ac1` command - Rust gates
  - Command: `cargo fmt --all --check && cargo clippy --workspace --all-targets --all-features -- -D warnings && cargo nextest run --workspace --all-features && cargo test --workspace --all-features --doc`
  - Expected kind: `exit_code_zero`
- [ ] `p4_ac2` command - TS + fixtures
  - Command: `pnpm verify:fast && pnpm fixtures:harness:check`
  - Expected kind: `exit_code_zero`
- [ ] `p4_ac3` command - license boundary
  - Command: `node .scafld/scripts/check-license-edges.mjs --check manifest-complete && node .scafld/scripts/check-license-edges.mjs --check identifiers`
  - Expected kind: `exit_code_zero`
- [ ] `p4_ac4` manual - the deferred items (cloud core sunset, real rails, settlement-receipt emission, OpenAPI front) recorded as their own specs/handoffs
  - Expected kind: `manual`

## Rollback

- Phases 1, 2, 4 are local and revert cleanly.
- Phase 3 is the only contract-touching step; keep it a single revertible commit
  and additive-only, so reverting restores the exact prior canonical JSON. If any
  fixture digest churns, the migration was not additive — revert and redo.

## Resulting shape (after this spec)

- Generic crates (`runx-contracts`/`runx-runtime`/`runx-core`) are payment-free and
  domain-free under one gate; payment is one opaque effect family in `runx-pay`.
- One canonical tool `source_hash` (or two pinned by a test); one canonical
  story-milestone vocabulary.
- Docs accurate at HEAD; the effect-kernel spec's deferred Phase 4 is closed.
- All nine CI gates green; no skill or contract surface broken.
- Remaining runx work is explicitly future/separate: the cloud `@runxhq/core`
  sunset, real rails + the cloud-to-kernel bridge, settlement-receipt emission,
  and the OpenAPI front.
