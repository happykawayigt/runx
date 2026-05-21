---
spec_version: '2.0'
task_id: rust-adapter-credential-delivery
created: '2026-05-21T00:00:00Z'
updated: '2026-05-21T00:00:00Z'
status: draft
harden_status: not_run
size: large
risk_level: high
---

# Runtime credential delivery to adapters

## Current State

Status: draft
Current phase: not started
Next: design the material resolver boundary and the secret-carrying channel that
keeps secrets out of `SandboxPlan.metadata` and receipts
Reason: the kernel already admits grants, decides credential binding, and proves
authority over a credential while recording only a hash of the material; the
runtime never resolves that material into a real secret nor delivers it to a
tool process. This is the single missing layer between an admitted credential
and a tool that can use it. It is the keystone for integrations: the catalog
lists hundreds of providers and only `github` is wired end-to-end.
Blockers: depends on the credential envelope v2 shape owned by
`byo-credential-foundations` (see Dependencies). The OAuth-only fields can be
delivered against the current `CredentialEnvelope` first.
Review gate: not_started

## Why this exists

The credential contract and the kernel decisions already exist in Rust:

- `CredentialEnvelope { kind, grant_id, provider, connection_id, scopes,
  grant_reference, material_ref }`
  ([`runx-core/src/policy/types.rs:133`](../../crates/runx-core/src/policy/types.rs#L133)).
- `CredentialBindingRequest` / `CredentialBindingDecision::{Allow,Deny}`
  ([`policy/types.rs:150`](../../crates/runx-core/src/policy/types.rs#L150)):
  the kernel decides whether a credential may bind to a run.
- `AuthorityProofCredentialMaterial { status, ..., material_ref_hash }`
  ([`policy/types.rs:266`](../../crates/runx-core/src/policy/types.rs#L266)):
  the proof records only a hash of `material_ref`, never the material, and
  carries an explicit `AuthorityProofRedaction`.
- `connect_grant_to_local_admission` turns a hosted grant into a
  `LocalAdmissionGrant` of metadata only (grant_id, provider, scopes), no secret
  ([`runx-runtime/src/connect/types.rs`](../../crates/runx-runtime/src/connect/types.rs)).

What is absent is the runtime delivery. The cli-tool adapter builds the child
environment through `prepare_process_sandbox` -> `child_env` ->
`allowed_base_env`, which is a fixed allowlist plus `RUNX_INPUTS_JSON` and
per-input `RUNX_INPUT_*` vars
([`runx-runtime/src/sandbox.rs:33`](../../crates/runx-runtime/src/sandbox.rs#L33),
[`:145`](../../crates/runx-runtime/src/sandbox.rs#L145),
[`:160`](../../crates/runx-runtime/src/sandbox.rs#L160)). No credential material
is ever resolved or injected; the MCP adapter path is the same via
`prepare_mcp_process_sandbox`. Sandbox metadata is `declared-policy-only`
([`sandbox.rs:394`](../../crates/runx-runtime/src/sandbox.rs#L394)). So an
admitted, authority-proven credential cannot power a real tool today.

## Summary

Add a runtime layer that, for a run whose `CredentialBindingDecision` is
`Allow`, resolves the envelope's `material_ref` into concrete secret material,
maps it to the provider's declared environment variables via a delivery
profile, and injects it into the cli-tool and MCP child process at spawn. The
secret travels a dedicated channel that never enters `SandboxPlan.metadata`, the
authority proof, receipts, captured stdout/stderr, or logs. Resolution happens
as late as possible and fails closed.

## Context

CWD: `.` (run cargo from `crates/`).

Packages:
- `crates/runx-runtime` (resolver, delivery, sandbox injection, adapters)
- `crates/runx-core` (binding decision and proof already exist; consume them)
- `crates/runx-contracts` (any new delivery-profile contract type)

Current sources:
- `crates/runx-runtime/src/sandbox.rs`
  (`SandboxPlan`, `prepare_process_sandbox`, `prepare_mcp_process_sandbox`,
  `child_env`, `allowed_base_env`)
- `crates/runx-runtime/src/adapters/cli_tool.rs` (`CliToolAdapter::invoke`)
- `crates/runx-runtime/src/adapters/mcp/adapter.rs` (`McpAdapter`)
- `crates/runx-runtime/src/connect/` (grant retrieval; OAuth handshake)
- `crates/runx-core/src/policy/authority_proof.rs` (proof + redaction)

Files impacted:
- new `crates/runx-runtime/src/credentials/` (resolver trait, delivery profile
  application, redaction guard)
- `crates/runx-runtime/src/sandbox.rs` (a secret env channel separate from the
  audited `env`/`metadata`)
- `crates/runx-runtime/src/adapters/cli_tool.rs` and `.../mcp/adapter.rs`
  (consume the secret channel at spawn only)

Invariants:
- A secret is never serialized into `SandboxPlan.metadata`, the authority proof,
  a receipt, captured stdout/stderr, or any log. Receipts continue to record
  `material_ref_hash` only.
- Material is delivered only when the kernel's `CredentialBindingDecision` for
  the run is `Allow`, scoped to the admitted grant and provider.
- Material is resolved as late as possible (at adapter spawn), held in memory no
  longer than the child process needs it, and the runtime fails closed if the
  resolver is unavailable or the ref does not resolve.
- The injected variable set is exactly the provider delivery profile's declared
  vars. No ambient or broad secret passthrough.
- The cli-tool and MCP paths deliver identically; no path-specific leniency.

## Objectives

- Define a `MaterialResolver` boundary: `material_ref` -> secret material, with a
  local resolver (keychain / env / file for self-host and dev) and a hosted
  resolver (cloud-brokered, behind the connect client) sharing one trait.
- Define per-provider `CredentialDeliveryProfile` (provider, auth_mode, env var
  -> role mapping) and apply it at spawn.
- Carry resolved material on a secret channel distinct from the audited
  `SandboxPlan.env`/`metadata`, and inject only into the child environment.
- Add a redaction guard so a secret value cannot reach metadata, proof, receipt,
  or log surfaces, with tests that assert non-leakage.

## Scope

In scope:
- The Rust runtime resolver boundary, delivery profiles, secret channel,
  injection at the cli-tool and MCP adapters, and the redaction guard.
- Fail-closed behavior and tests proving secrets do not appear in
  metadata/proof/receipt/stdout.

Out of scope:
- Cloud-side secret storage, grant v2, credential envelope v2, BYO connect
  session, and verification. Owned by `byo-credential-foundations`.
- New auth modes beyond what the envelope and delivery profile express.
- File-mount or helper-process delivery if env injection is chosen first (see
  Open Questions); add as a follow-up only if a provider needs it.

## Dependencies

- `rust-connect-client` (archived, completed; grant retrieval and the hosted
  client this resolver extends).
- `rust-policy-authority-proof-parity` (archived; the proof + redaction the
  delivery must not violate).
- `byo-credential-foundations` (cloud; supplies credential envelope v2 with
  optional `connection_id`, `material_kind`, `auth_mode`). The current
  `CredentialEnvelope` requires `connection_id`
  ([`types.rs:137`](../../crates/runx-core/src/policy/types.rs#L137)), so the
  OAuth path can land first and BYO follows envelope v2.

## Open Questions

- Delivery mechanism: environment injection vs mounted file vs helper process.
  Start with env injection (the pre-oauth foundations plan's recommendation) and
  defer the others unless a provider requires them.
- Where the delivery profile is declared: a runx-owned per-provider profile, the
  tool manifest, or both. Lean to a runx-owned profile keyed by provider so
  tools stay credential-agnostic.
- Whether the secret channel is a new field on `SandboxPlan` (excluded from its
  `PartialEq`/serialization) or a separate value passed alongside the plan to
  the adapter. The latter keeps `SandboxPlan` purely auditable.

## References

- [`plans/integrations/pre-oauth-foundations.md`](../../../plans/integrations/pre-oauth-foundations.md)
  §9 ("Runtime Delivery To Adapters")
- [`plans/integrations/tonight-pre-oauth-work.md`](../../../plans/integrations/tonight-pre-oauth-work.md)
  §8 (note: it points at the sunsetting TS adapters; this spec is the
  Rust-canonical owner)
- [`runx-core/src/policy/types.rs`](../../crates/runx-core/src/policy/types.rs)
  (`CredentialEnvelope`, `CredentialBindingDecision`,
  `AuthorityProofCredentialMaterial`)
- [`runx-runtime/src/sandbox.rs`](../../crates/runx-runtime/src/sandbox.rs)
  (the env construction this spec extends)
