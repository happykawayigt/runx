---
spec_version: '2.0'
task_id: rust-approval-gate-parity
created: '2026-05-18T00:00:00Z'
updated: '2026-05-20T05:35:00Z'
status: completed
harden_status: passed
size: small
risk_level: medium
---

# Rust approval gate parity

## Current State

Status: completed
Current phase: final
Next: done
Reason: local-only Rust runtime approval parity is complete, hardened, and
validated.
Cloud approval HTTP routing and deployment behavior remain separate follow-up
scope because this OSS checkout does not contain an executable cloud approval
contract source or pinned artifact.
Blockers: none for the local approval parity slice.
Allowed follow-up command: `none`
Latest runner update: 2026-05-19T13:43:20Z
Review gate: pass

## Summary

Land local approval-gate parity on the Rust runtime. The current TS
shape (`ApprovalGate { id, reason, type?, summary? }` plus
`ResolutionRequest { id, kind: "approval", gate }`,
`ResolutionResponse { actor: "human" | "agent", payload: unknown }`, and
`Caller.report` / `Caller.resolve`) is the cross-language contract. Rust must
not invent a parallel compatibility shape. `runx-runtime` consumes the
contract through the local caller boundary: it reports approval requests,
awaits local resolution, validates the response payload to a boolean, and
dedupes repeated gates by the canonical gate hash.

Cloud approval routing, hosted deployment behavior, and receipt-store
projection are not executable from this OSS checkout and are deferred to their
own contract-stabilization specs. This spec intentionally does not define
approval POST/GET/PUT routes, hosted retry semantics, auth headers, or cloud
error envelopes.

## Context

CWD: `.`

Packages:
- `@runxhq/contracts`
- `@runxhq/runtime-local` (runner-local approval, graph-governance)
- `crates/runx-runtime`
- `crates/runx-contracts`

Current TypeScript sources:
- `packages/contracts/src/schemas/agent-act.ts` (ApprovalGate contract)
- `packages/contracts/src/schemas/resolution.ts` (approval resolution request
  and response contracts)
- `packages/runtime-local/src/runner-local/approval.ts`
- `packages/runtime-local/src/runner-local/graph-governance.ts`
- Cloud approval routes and durable-step code are external to this OSS checkout
  and read-only for this spec unless an exact pinned artifact is added.
- `crates/runx-contracts/src/host_protocol.rs` (existing Rust
  `ApprovalGate`, `ResolutionRequest`, `ResolutionResponse`)

Files impacted:
- `crates/runx-runtime/src/approval.rs`
- `crates/runx-runtime/tests/approval.rs`
- `crates/runx-contracts/src/host_protocol.rs` is consumed as the existing
  approval wire model and is not duplicated by this slice.

Invariants:
- The TS approval contract does not silently change. Any clarification that
  the Rust port forces (enumerated gate types, payload schema) lands in TS
  first via a small clarification spec, not by Rust drift.
- `ApprovalGate.type` and `ApprovalGate.summary` remain optional in Rust until
  TS makes them required. Rust serialization must omit absent optional fields,
  not emit nulls or empty objects to satisfy convenience tests.
- Approval resolution actors are exactly the TS enum values `human` and
  `agent`. Do not broaden to a free string in Rust.
- Approval resolution payload is boolean for approval requests. The Rust caller
  boundary must reject non-boolean payloads. This local Rust spec does not
  change TS runner-local behavior.
- Cloud HTTP routing remains out of scope for this local parity slice. A later
  cloud routing spec must document the HTTP contract before any Rust client
  consumes it.
- No approval bypasses: Rust runtime callers must go through the local approval
  request/report/resolve boundary before treating a gate as approved.
- No legacy/compat readers: do not accept alternate field spellings, null
  optional fields, stringly boolean decisions, or old actor names unless TS
  accepts them in the named source contract first.

## Objectives

- Reuse the existing `runx-contracts::host_protocol` approval types (gate,
  request envelope, resolution envelope, actor identity) or re-export them from
  a single public home without creating a second wire model.
- Implement `runx-runtime` local caller approval handling: gate emission,
  caller reporting, resolution awaiting, pending state, approved/denied
  decisions, and duplicate gate dedupe by canonical idempotency key.
- Reject non-boolean approval response payloads at the caller boundary.
- Add focused local runtime tests for approved, denied, pending, agent actor,
  non-boolean payload rejection, optional-field omission, alternate-shape
  rejection, and resolved-gate dedupe.
- Preserve the existing `runx-contracts::host_protocol` wire contract or move it
  through a single re-export path. Do not leave two public Rust approval models
  that can diverge.

## Scope

In scope:
- Runtime-side local caller approval request/report/resolve behavior.
- Existing host-protocol approval contract consumption.
- Focused Rust tests that prove request, report, resolve, denial, pending,
  idempotency, actor, optional-field, and no-legacy behavior.

Out of scope:
- Aster operator UI consumption of gates (separate spec under aster v1 reset).
- Approval routing logic changes (the cloud rules stay in TS).
- Cloud approval POST/GET/PUT routes and Rust cloud client packaging until
  `cloud-http-contract-stabilization` provides a concrete source or pinned
  artifact.
- Harness receipt approval-round-trip proof, fixture generation, and receipt
  store projection. Those remain owned by receipt/deployment specs.
- Replacing the TS runner-local approval path. Both runners co-exist until a
  TS sunset spec.

## Dependencies

- `rust-runtime-skeleton`, `rust-contracts-parity`.
- `rust-ts-interop-boundary` for the cross-language crossing reference.
- `cloud-http-contract-stabilization` is a follow-up dependency for cloud
  approval routing, not a build blocker for this local runtime parity slice.

## Open Questions

- None for the local approval parity slice. Cloud client packaging is
  explicitly deferred. This spec must not add `runx-cloud-client`,
  `crates/runx-runtime/src/cloud_client.rs`, or HTTP route assumptions.

## Completion State

Verdict: completed for local Rust runtime approval parity. The focused
executable slice is `crates/runx-runtime/src/approval.rs` plus
`crates/runx-runtime/tests/approval.rs`.

Validation run:

```sh
cargo test --manifest-path crates/Cargo.toml -p runx-runtime --test approval -- --nocapture
```

Result: passed on 2026-05-19T13:43:20Z with 8 tests passed, 0 failed.

Remaining non-blocking follow-up scope:
- Cloud HTTP approval routing is not buildable from this checkout and remains
  deferred until a concrete HTTP contract or pinned artifact exists.
- Cloud retry/idempotency semantics are owned by the future cloud routing
  contract.
- Receipt proof integration, approval receipt fixtures, redaction proof
  projection, and deployment packaging are not part of this local approval
  parity completion.

Advisories:
- Prefer extending or re-exporting `crates/runx-contracts/src/host_protocol.rs`
  over adding `approval.rs` as an independent type home. A new module is fine
  only if `host_protocol` and public exports point at the same structs/enums.
- Do not introduce network dependencies in this slice. The runtime must have a
  local caller path that remains fully testable without network access.
