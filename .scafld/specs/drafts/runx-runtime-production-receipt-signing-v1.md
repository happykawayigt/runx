---
spec_version: '2.0'
task_id: runx-runtime-production-receipt-signing-v1
created: '2026-05-22T12:04:12+10:00'
updated: '2026-05-22T12:04:12+10:00'
status: draft
harden_status: not_run
size: large
risk_level: high
---

# Runx runtime production receipt signing v1

## Current State

Status: implementing
Current phase: Phase 3 (verification surfaces) done; Phase 2 partial
Next: dod1 only — thread `RuntimeOptions::signature_policy()` (production when an
Ed25519 signer + verifier are configured, local-development otherwise) into every
runtime receipt-build site so the live runtime emits production-signed receipts.
Reason: R2 from `runx-security-hardening-v1`. The signing/verification spine is
landed and green; the live runtime still seals with local-development unless a
signer is configured.
Done (green, 2026-05-22):
- Ed25519 signer/verifier primitive + `RuntimeReceiptSignaturePolicy`
  (local/production/production_signing) and `seal_receipt` application.
- Strict-proof verification + production key resolver shared by store, journal,
  and CLI history (`history.rs` reads `RUNX_RECEIPT_VERIFY_*`).
- History/journal trust labeling: `verified` reserved for production-verified
  (dod3), `unverified` for local/missing-verifier, `invalid` for tamper, with
  `journal_history` tests migrated to these semantics (dod4, dod5).
- Tamper + production-signature fixture tests (`receipt_signing`,
  `journal_history`) green (dod6, dod7).
Remaining (dod1, dod8):
- dod1 live signing wiring across ~12 sites:
  `execution/runner/steps.rs` (4 `step_receipt`),
  `execution/runner.rs` (3 `graph_receipt`/`graph_receipt_with_disposition`),
  `execution/harness/runner.rs` (3 `step_receipt_with_disposition` + 2
  `graph_receipt_with_disposition`), `execution/skill_run.rs` (1),
  `adapters/mcp/server_skill.rs` (1). Add owned signer/verifier +
  `signature_policy(&self)` to `RuntimeOptions`; carry a policy on
  `StepReceiptWithDisposition` and `graph_receipt_with_disposition`
  (fold into the closure struct to stay under the clippy arg limit); resolve a
  signer from new `RUNX_RECEIPT_SIGN_*` env keys in the CLI; add a production-path
  runtime test. Keep `local_development` as the default so existing goldens are
  byte-identical.
- dod8 docs: production signing configuration + local-development downgrade.
Blockers: none. Signer/verifier boundary settled (see Settled Architecture).
Allowed follow-up command: `scafld harden runx-runtime-production-receipt-signing-v1`
Latest runner update: 2026-05-22 spine landed and green; dod1/dod8 remain as a
scoped follow-up with sites enumerated above.
Review gate: not_started

## Settled Architecture

- The runtime is the only trust boundary. A receipt is production-trusted only
  when the runtime signs `canonical_receipt_body_digest` with a real Ed25519
  key and the bound verifier confirms it. `placeholder_signature()` and
  `sig:<digest>` pseudo signatures are local-development only and must never be
  reported as production-verified.
- A single `RuntimeReceiptSignaturePolicy` flows from `RuntimeOptions` through
  step/graph sealing, the receipt store, journal projection, and CLI history,
  so one verifier/key-resolver governs every consumer. Production-vs-local is
  decided once, from configured key material (env-resolved), never per call
  site.
- Key material is resolved outside receipts: private seed never enters a
  receipt, log, metadata, snapshot, or committed fixture. Public `kid` +
  `public_key_sha256` are the only key facts bound into issuer metadata.

## A+ Coding Invariants

This work must hold the runx Rust core invariants (enforced by
`scripts/check-rust-core-style.mjs`, `crates/deny.toml`, and
`[workspace.lints]`):

- Typed errors only via `thiserror`; no `anyhow`, `eyre`, or `Box<dyn Error>` in
  library code. Signing/verification failures use typed
  `RuntimeReceiptSigningError` / verifier finding variants.
- No `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg`, or `print` in
  runtime/library code. Fail-closed paths return typed errors.
- No `serde_json::Value` in public API surfaces; typed structs in, typed structs
  out. `BTreeMap`/`BTreeSet` for determinism, never `HashMap`.
- No wildcard re-exports; `unsafe` forbidden.
- Parse-don't-validate: production-vs-local is a sum type, not a bool pair; an
  unsigned-but-claimed-production receipt is unrepresentable.
- File <=350 lines / fn <=60 lines, with documented `// rust-style-allow:`
  escape hatches naming the reason where a security transaction genuinely needs
  it.
- Determinism: signing inputs and fixture keys are deterministic; golden
  regeneration is reproducible.

## Summary

Turn runtime harness receipts from local pseudo-signed development artifacts into
production-verifiable evidence. The receipt body digest and strict verifier
already exist in `runx-receipts`, but the runtime still constructs receipts with
`placeholder_signature()`, seals through
`RuntimeReceiptSignaturePolicy::local_development()`, and uses `sig:<digest>`
pseudo signatures in local mode. Production mode currently has a verifier hook
but no real signer path.

This spec builds the missing production path: a real asymmetric signer over the
canonical receipt body commitment, issuer metadata bound to the public
verification key, fail-closed production sealing, and CLI/history verification
that cannot report pseudo signatures as production-trusted receipts.

## Objectives

- Add a production receipt signer boundary that signs the
  `canonical_receipt_body_digest` using Ed25519 and returns a real
  `ReceiptSignature`.
- Bind `ReceiptIssuer.kid` and `ReceiptIssuer.public_key_sha256` to the
  configured verification key; never write private key material into receipts,
  logs, metadata, or fixture output.
- Make production sealing fail closed when a signer, key id, public key hash, or
  verifier is missing or inconsistent.
- Keep local-development pseudo signatures available only for explicit local
  development/test policy and make that status visible to verification callers.
- Route `runx history` and receipt proof checks through the same verifier/key
  resolver used for production receipts.
- Add deterministic fixture keys and tamper tests so body mutation, signature
  mutation, issuer/key mismatch, and missing verifier inputs all fail.

## Scope

In scope:
- Runtime receipt signing and proof context wiring.
- Production signer/verifier abstractions and deterministic fixture
  implementations.
- CLI/history verification status for production-signed receipts.
- Tests and fixtures for positive production signatures and negative tamper
  cases.
- Documentation of production signing configuration and local-development
  downgrade behavior.

Likely files to touch during implementation, not in this draft:
- `crates/runx-runtime/src/receipts/seal.rs`
- `crates/runx-runtime/src/receipts/store.rs`
- `crates/runx-runtime/src/journal.rs`
- `crates/runx-cli/src/history.rs`
- `crates/runx-receipts/src/verify/proof.rs`
- `crates/runx-receipts/src/verify/proof/signature.rs`
- `crates/runx-receipts/src/canonical.rs`
- `crates/runx-contracts/src/fingerprint.rs`
- `crates/runx-runtime/tests/**`
- `crates/runx-receipts/tests/**`
- `fixtures/contracts/**`
- `README.md` and receipt/security docs

Out of scope:
- Sandbox enforcement from R1.
- Payment rail settlement proof verification from R3.
- Hosted KMS/HSM integrations beyond a signer interface that can support them.
- Cloud receipt storage, receipt indexing, or remote attestation services.
- Any runtime code change as part of creating this draft.

## Dependencies

- Parent finding: `runx-security-hardening-v1` R2.
- `rust-receipt-proof-verification`, because strict body-digest and verifier
  semantics must remain the acceptance path.
- `canonical-json-fingerprint-contract-v1`, because production signatures are
  only meaningful if canonical receipt bytes are stable.
- Coordination with R1/R3, but neither blocks this work: production signatures
  can land before OS sandbox enforcement or rail-settlement verification.
- A vetted Rust Ed25519 implementation or signing-provider adapter selected
  before implementation begins.

## Risks

- Signing the wrong bytes would create a false trust signal while leaving
  mutable receipt fields outside the proof.
- Allowing pseudo signatures in production mode would preserve the current
  forgery gap under a new name.
- Key material could leak through env vars, debug output, fixtures, or receipt
  metadata if the signer boundary is sloppy.
- Key rotation and `kid` lookup can break historical verification if old public
  keys are not resolvable.
- History output could overstate trust if it collapses local-development,
  unverified, and production-verified receipts into one status.
- Introducing crypto dependencies in pure crates can widen the dependency graph;
  keep signing in runtime/CLI-adjacent code unless the proof crate needs only
  verifier traits.

## Acceptance

- [ ] `dod1` Production receipt mode signs every runtime step and graph receipt
  with a real Ed25519 signature over `canonical_receipt_body_digest`.
- [ ] `dod2` Production mode rejects missing signer, missing verifier, missing
  key id, malformed key material, issuer/key-hash mismatch, and any `sig:`
  pseudo signature.
- [ ] `dod3` Local-development pseudo signatures remain possible only through an
  explicit local-development policy and are not reported as production-verified.
- [ ] `dod4` `runx history --json` exposes `verified`, `unverified`, or
  `invalid` using strict proof verification and a production key resolver.
- [ ] `dod5` Tampering with the receipt body, `seal.digest`, signature value,
  issuer `kid`, or `issuer.public_key_sha256` fails with specific verifier
  findings.
- [ ] `dod6` Fixture tests cover at least one production-signed receipt and the
  negative tamper cases.
- [ ] `dod7` No private key bytes appear in receipts, snapshots, logs, or public
  fixture JSON.
- [ ] `dod8` Production signing configuration and local-development downgrade
  semantics are documented.

Validation:
- [ ] `v1` Scafld validates this spec.
  - Command: `scafld validate runx-runtime-production-receipt-signing-v1 --json`
  - Expected kind: `exit_code_zero`
  - Status: pending
- [ ] `v2` Receipt proof verification tests pass.
  - Command: `cargo test --manifest-path crates/Cargo.toml -p runx-receipts`
  - Expected kind: `exit_code_zero`
  - Status: pending
- [ ] `v3` Runtime receipt signing tests pass.
  - Command: `cargo test --manifest-path crates/Cargo.toml -p runx-runtime receipt`
  - Expected kind: `exit_code_zero`
  - Status: pending
- [ ] `v4` CLI history verification tests pass.
  - Command: `cargo test --manifest-path crates/Cargo.toml -p runx-cli history`
  - Expected kind: `exit_code_zero`
  - Status: pending
- [ ] `v5` Rust formatting and lint checks pass.
  - Command: `cargo fmt --check --manifest-path crates/Cargo.toml`
  - Expected kind: `exit_code_zero`
  - Status: pending
- [ ] `v6` Runtime and receipt lint checks pass.
  - Command: `cargo clippy --manifest-path crates/Cargo.toml -p runx-runtime -p runx-receipts --all-targets --all-features -- -D warnings`
  - Expected kind: `exit_code_zero`
  - Status: pending
- [ ] `v7` Patch hygiene passes.
  - Command: `git diff --check`
  - Expected kind: `exit_code_zero`
  - Status: pending

## Phase 1: Signing Contract

Status: pending
Dependencies: `rust-receipt-proof-verification`,
`canonical-json-fingerprint-contract-v1`

Objective: Define exactly what production signs and how keys are identified.

Changes:
- Define a runtime production signer trait that returns issuer metadata and a
  real `ReceiptSignature`.
- Keep `SignatureVerifier` focused on verification and add only the minimum
  resolver hooks needed for history/proof contexts.
- Specify config names, required inputs, failure modes, and local-development
  downgrade behavior.

Acceptance:
- Contract review proves the signature covers the canonical receipt body
  commitment and excludes only derived proof fields by design.
- Production mode has no path that silently falls back to pseudo signatures.

## Phase 2: Runtime Sealing

Status: pending
Dependencies: Phase 1

Objective: Wire the signer into runtime receipt construction.

Changes:
- Replace production `sign_receipt` failure stub with signer-backed Ed25519
  signing.
- Stop hardcoding `local_issuer()` for production receipts; derive issuer data
  from configured public verification material.
- Ensure `step_receipt` and `graph_receipt` can receive a production signature
  policy from runtime options while tests can still opt into local development.

Acceptance:
- Production receipts seal only when signer and verifier inputs are present and
  mutually consistent.
- Local-development fixtures still pass through explicit local policy.

## Phase 3: Verification Surfaces

Status: pending
Dependencies: Phase 2

Objective: Make receipt consumers report trust accurately.

Changes:
- Route runtime proof contexts and CLI history through a production key resolver.
- Distinguish production-verified, local-development verified, unverified, and
  invalid receipts in machine-readable output.
- Add negative tests for missing keys, wrong keys, malformed signatures, and
  body/signature tampering.

Acceptance:
- `runx history --json` cannot label a pseudo-signed receipt as
  production-verified.
- All tamper fixtures fail with stable finding codes.

## Phase 4: Fixtures, Docs, And Gates

Status: pending
Dependencies: Phase 3

Objective: Make the implementation maintainable and auditable.

Changes:
- Add deterministic fixture keys and production-signed receipt fixtures.
- Document key configuration, rotation expectations, and local-development
  downgrade semantics.
- Run the acceptance validation commands and record build evidence before moving
  to review.

Acceptance:
- Fixture regeneration is deterministic and documented.
- No private key material appears in committed public receipt fixtures.

## Rollback

Revert the signer wiring, production key config, verification resolver changes,
and production-signed fixtures. Leave the strict proof verifier and existing
local-development receipt path intact. After rollback, receipts return to the
current local pseudo-signature posture and must not be described as production
verified.

## Origin

Created on 2026-05-22 from `runx-security-hardening-v1` R2:
"receipts are placeholder-signed." The parent finding names
`crates/runx-runtime/src/receipts/seal.rs`, `placeholder_signature()`,
`RuntimeReceiptSignaturePolicy::local_development()`, and hardcoded
`signature_valid: true` as the gap. This draft narrows that critical finding
into a build-ready production receipt signing and verification workstream.
