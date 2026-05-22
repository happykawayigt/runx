---
spec_version: '2.0'
task_id: oss-local-credential-provision-v1
created: '2026-05-22T11:05:00+10:00'
updated: '2026-05-22T11:05:00+10:00'
status: draft
harden_status: not_run
size: medium
risk_level: high
---

# Restore local credential provision in the OSS runtime

## Current State

Status: draft
Current phase: planning
Next: harden
Reason: `connect-auth-mit-boundary-v1` correctly removed OSS OAuth brokerage but
also removed the local credential *establishment* path, leaving the MIT CLI with
no way to provide a credential to a skill at all. This restores the local
BYO/offline path without reopening brokerage.
Blockers: coordinate with in-flight `credential-envelope-opaque-reference-v1`,
which touches the same `runx-core` policy/credential vocabulary.
Allowed follow-up command: `scafld harden oss-local-credential-provision-v1`
Latest runner update: 2026-05-22T11:05:00+10:00 drafted from an over-cut
diagnosis.
Review gate: not_started

## Summary

The license-boundary refactor cut one notch too deep. Removing the hosted OAuth
brokerage from OSS was right; removing the local credential *establishment* path
was not. Today the MIT CLI cannot supply a credential to a skill: `runx connect`
refuses and points at the private distribution, the runtime's scoped env is a
non-secret allowlist, the CLI run path never constructs a `MaterialResolver`,
and there is no token-intake verb. So the open runtime cannot do authenticated
work standalone, which breaks the doctrine's offline/zero-dependency promise and
north-star's "BYO credential delivery unlocks the portfolio" order, for no moat
gain (the brokerage secrets were always in `cloud/packages/auth`).

This spec restores a local, no-network credential-provision path in the OSS CLI
that feeds the existing opaque `MaterialResolver` and seals `grant_type: local`.
It does not reopen OAuth brokerage, Nango, hosted connect, or secret custody.

## Context

- `crates/runx-cli/src/main.rs:44` returns "runx connect is not available in the
  MIT OSS CLI; use the hosted/private CLI distribution".
- `crates/runx-runtime/src/execution/runner.rs:48` defines `safe_default_env()`,
  a strict allowlist (`PATH`, `SystemRoot`, `PATHEXT`, `RUNX_RECEIPT_DIR`,
  `RUNX_PROJECT_DIR`, `RUNX_CWD`) with no secret passthrough.
- `crates/runx-runtime/src/credentials.rs:106` defines `MaterialResolver` and
  `InMemoryMaterialResolver`, populated only programmatically; the CLI run path
  does not construct one (`rg MaterialResolver crates/runx-cli/src` is empty).
- The doctrine ("runs stay local, zero-dependency") and `plans/runx.md` "Offline
  mode: `runx connect --token`, no browser, `grant_type: local`" both assume a
  local establishment path that no longer exists in OSS.
- `connect-auth-mit-boundary-v1` (archived) banned `NangoConnection`, `oauth_*`,
  `RUNX_CONNECT_*` and kept the opaque `MaterialResolver`. That stays.

## Objectives

- Add a local, no-network credential-provision surface to the OSS CLI: a way to
  supply a token/secret for a run (for example `runx grant`/`--secret`, a local
  config file, or a scope-declared env allowlist) that populates a
  `MaterialResolver` for that run.
- Seal `grant_type: local` for locally-provided credentials (declared, not
  verified), with the secret redacted from receipts, output, and metadata via the
  existing `CredentialDelivery` redaction.
- Wire the resolver into the CLI run path so a provided credential reaches the
  adapter through the existing delivery channel.
- Keep the boundary intact: no OAuth brokerage, Nango, hosted calls, or custody.
  Add only the local-provision identifiers to the boundary manifest allowlist;
  reintroduce none of the banned brokerage identifiers.

## Scope

In scope:
- `crates/runx-cli/src` (credential-provision surface + run-path resolver wiring).
- `crates/runx-runtime/src` (a file/CLI-backed `MaterialResolver` if the in-memory
  one is insufficient; `grant_type: local` sealing).
- `oss/docs/license-boundary.manifest.json` allowlist update.
- Tests: an offline run that consumes a locally-provided credential; redaction;
  and a no-network assertion (sibling to `locality.rs`).

Out of scope:
- OAuth brokerage, hosted connect, Nango, or secret custody (stay private).
- The credential-envelope vocabulary migration (`credential-envelope-opaque-reference-v1`).
- Browser loopback/PKCE establishment (a possible later add, not v1).
- Any cloud change.

## Acceptance

- [ ] `dod1` The OSS CLI can provide a credential for a run with no network and no
  hosted dependency.
- [ ] `dod2` A skill consuming that credential runs and seals `grant_type: local`,
  with the secret redacted from receipts, captured output, and metadata.
- [ ] `dod3` The license-boundary guard passes; only local-provision identifiers
  are added to the allowlist and no banned brokerage identifier is reintroduced.
- [ ] `dod4` A no-network test proves the provision + run path makes no outbound
  calls.

Validation:
- [ ] `v1` Scafld validates this spec.
  - Command: `scafld validate oss-local-credential-provision-v1 --json`
  - Expected kind: `exit_code_zero`
  - Status: pending
- [ ] `v2` Offline credential-provision run + redaction tests pass.
  - Command: `cargo test --manifest-path crates/Cargo.toml -p runx-cli -p runx-runtime local_credential`
  - Expected kind: `exit_code_zero`
  - Status: pending
- [ ] `v3` The license-boundary guard passes on the changed tree.
  - Command: `cargo test --manifest-path crates/Cargo.toml -p runx-runtime --test license_boundary`
  - Expected kind: `exit_code_zero`
  - Status: pending
- [ ] `v4` The CLI locality guard still passes.
  - Command: `cargo test --manifest-path crates/Cargo.toml -p runx-cli --test locality`
  - Expected kind: `exit_code_zero`
  - Status: pending

## Phase 1: Provision Surface And Semantics

Design the local provision surface and the `grant_type: local` semantics; record
the exact UX/wire (intake form, where material lives for the run, redaction
guarantees) before implementation. No source remediation.

## Phase 2: Implementation

Implement the CLI provision surface, wire the `MaterialResolver` into the run
path, seal `grant_type: local`, and apply redaction through the existing
`CredentialDelivery` channel.

## Phase 3: Boundary And Tests

Add the local-provision identifiers to the manifest allowlist, add the offline
run + redaction + no-network tests, and confirm the boundary guard and locality
guard both pass with no brokerage reintroduced.

## Rollback

Revert the CLI surface and run-path wiring; remove the allowlist additions. The
offline path returns to its current absent state with no secret material
persisted. The boundary guard must still pass after rollback.

## Origin

Conversation on 2026-05-22: grounded code review showed the MIT CLI cannot
provide a credential at all after the boundary refactor, contradicting the
project's own doctrine and north-star order. The fix is additive (restore local
establishment), not an unwind of the boundary.
