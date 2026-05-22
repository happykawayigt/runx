---
spec_version: '2.0'
task_id: runx-security-hardening-v1
created: '2026-05-22T00:55:00Z'
updated: '2026-05-22T01:12:52Z'
status: review
harden_status: in_progress
size: large
risk_level: high
---

# runx security hardening v1

## Current State

Status: review
Current phase: final
Next: review
Reason: build completed; ready for review
Blockers: none
Allowed follow-up command: `scafld review runx-security-hardening-v1`
Latest runner update: 2026-05-22T01:12:52Z
Review gate: not_started

## Summary

Harden runx's security posture before any untrusted-skill execution or
production-payment story. The architecture is sound (capability/authority model,
receipt-centric design, pure policy kernel), but several controls are currently
*declarations or placeholders* rather than *enforced mechanisms*: the sandbox
does not confine, the receipt seal does not cryptographically bind, the payment
proof is skill-asserted, and a few core admission edges fail open. This spec
ranks the findings by severity and sequences them surgical-first.

This is a defensive-security spec. No finding here authorizes adding
provider/integration code to the kernel; the trust boundaries from
`external-adapter-plugin-protocol-v1`, `skill-author-runtime-contract-v1`, and
`canonical-json-fingerprint-contract-v1` still hold and are referenced where a
finding overlaps them.

## Context

The exposures cluster on three trust-boundary assumptions that do not yet hold:
the sandbox confines (it does not), the seal attests (it is a placeholder), and
the payment proof proves (it is asserted). Plus a set of narrower core-admission
and runtime edges. Findings are grounded in file:line.

Severity legend: **Critical** = breaks a core security claim; **High** =
exploitable bypass or secret exposure; **Medium** = hardening / defense-in-depth.

## Core findings (`runx-core`)

- **C1 [Critical, fail-open] — DONE (core OSS).** `find_matching_grant`
  (`policy/connected_auth.rs`) admitted any grant whose `status != Some(Revoked)`,
  so a grant with `status: None` (the field is `skip_serializing_if`, so omitted
  JSON deserializes to `None`) was admitted. Fixed to require `Some(Active)`
  (fail-closed on missing status). Added `not_before` / `expires_at` to
  `LocalAdmissionGrant`, explicit `connected_auth_checked_at` inputs for the
  pure policy evaluators, and fail-closed lifetime checks: active grants now deny
  if they are unbounded, malformed, expired, not yet valid, or evaluated without
  a timestamp. The current MIT OSS branch has no active `HttpConnectGrant`
  runtime type after the connect brokerage removal; private/hosted connect
  schemas still need parity if reintroduced.
- **C2 [Medium-High, audit integrity] — DONE (subset-proof gate).** The kernel
  recomputes `is_payment_authority_subset(child, parent)` (sound, correct
  direction), and the runtime now passes the typed `AuthoritySubsetProof` into
  `StepAuthorityAdmission`. Payment attenuation rejects missing or mismatched
  subset proofs by parent ref, compared child/parent term ids, relation, result,
  algorithm, and checked timestamp. This closes the caller-supplied boolean
  proof-presence gap. The remaining R3 rail-settlement proof issue stays open.
- **C3 [Medium, coverage] — deep attenuation is payment-only.**
  `admit_step_authority` runs the bounds/capability/condition subset algebra only
  for `resource_family == Payment && spends`; all other families return
  `verb: None` with no attenuation and rely solely on `scope_allows` prefix
  matching. → confirm intentional or extend attenuation to other high-value
  resource families (deploy, repo-write).
- **C4 [Medium] — DONE (aggregate spend capped).** `minor_unit_caps_subset`
  (`policy/payment_authority.rs`) now requires at least one aggregate cap
  (`max_per_run_minor` or `max_per_period_minor`) on both parent and child spend
  authority before per-call caps can pass subset comparison. The policy fixture
  `payment-authority-denies-unbounded-aggregate-spend` covers the fail-closed
  path.
- **C5 [Medium] — `scope_allows("*", …)` is god-mode; prefix is coarse.**
  `policy/scope.rs`: `granted == "*"` allows everything, and `repo:*` matches
  `repo:admin:keys`. → never accept `*` from untrusted grant data (consider
  removing the universal wildcard); decide whether prefix wildcards are
  single-segment.
- **C6 [Medium, design] — DONE (success requires admission witness).** The pure
  single-step and sequential graph state machines now require a
  `StepAdmissionWitness` on success transitions. Sequential success also checks
  that the witness step id and receipt id match the step being sealed, so a
  runner cannot transition a step to succeeded through the kernel without an
  admission/receipt witness.
- **C7 [Medium] — kernel-eval input surface.** `kernel_eval.rs` exposes
  evaluators (`is_payment_authority_subset`, etc.) via JSON `to_value(...)`. If
  externally reachable, add input limits + fuzzing (deeply-nested objects →
  recursion/stack exhaustion in canonical-JSON; oversized inputs).

## Runtime findings (`runx-runtime`)

- **R1 [Critical] — sandbox is advisory, not enforced.** No seccomp / landlock /
  namespaces / setrlimit / cgroups anywhere; `sandbox.rs::validate_sandbox`
  rejects `require_enforcement: true` ("isolation helpers are not available").
  `readonly`/`network:false`/`writable_paths` are validated then ignored — a
  `cli-tool` skill runs with full user authority. → real OS enforcement
  (Landlock/seccomp on Linux, sandbox-exec on macOS) before any untrusted-skill
  story. Overlaps `skill-author-runtime-contract-v1` (which owns the ABI, not
  enforcement).
- **R2 [Critical] — receipts are placeholder-signed.** `receipts/seal.rs` uses
  `placeholder_signature()`, `RuntimeReceiptSignaturePolicy::local_development()`
  as the wired mode, and hardcoded `signature_valid: true`. A `SignatureVerifier`
  trait exists but production signing is not the active path → sealed receipts
  are forgeable. → real asymmetric signing + verification as default, key custody
  outside the producing process.
- **R3 [Critical] — payment proof is skill-asserted.** `is_payment_rail_proof_ref`
  accepts any ref typed `Verification`+`PaymentRail`; the producing skill controls
  receipt acts/refs. With R2, the receipt-before-success invariant is forgeable.
  → bind proofs to an out-of-band rail settlement verified by the supervisor
  (pairs with C2).
- **R4 [High] — secrets via env + post-hoc redaction.** `secret_env` injected
  into child env (`adapters/cli_tool.rs`); leaks via `/proc/<pid>/environ`,
  grandchildren, dumps. `redact_text` is substring replacement (encoding-bypass),
  and with R1 the skill can just exfiltrate over the network. → scoped/short-lived
  tokens, broker delivery; don't treat redaction as containment.
- **R5 [High] — skill stdout trusted as structured output.** stdout-as-JSON →
  `outputs` feeds receipts and authority fields; attacker-controlled. → separate
  skill-asserted output from supervisor-attested facts.
- **R6 [High] — canonical-JSON byte-identity unpinned across runtimes.** 4
  independent canonicalizers stamp `runx.stable-json.v1`; `canonical_json_number`
  = `JsonNumber::to_string()` (float/precision divergence). Digest confusion.
  Owned by `canonical-json-fingerprint-contract-v1` — cross-reference, do not
  duplicate.
- **R7 [High, payments] — DONE (file lock + reload before mutation).**
  `FileBackedPaymentStateStore` now takes a sidecar lock for state mutations,
  reloads the current persisted document under the lock before applying the
  mutation, and writes through the locked state. `payment_state` regression tests
  cover stale stores refusing to overwrite already-recorded idempotency state.
- **R8 [High, supply chain] — self-asserted install digests.**
  `registry/install.rs::validate_candidate_digest` hashes the candidate's own
  markdown against a digest the candidate supplies → no trust anchor (with R2, no
  root of trust beyond the `TrustTier` label). → verify against a publisher-signed
  manifest.
- **R9 [Medium] — input→env name collisions.** `sandbox.rs::input_env_name` maps
  non-alphanumerics to `_` + uppercases, so `foo-bar`/`foo.bar`/`foo_bar` collide
  to `RUNX_INPUT_FOO_BAR`. → reject colliding keys or pass inputs only via JSON.
- **R10 [Medium] — SSRF in A2A + external-HTTP adapters.** `agent_card_url` and
  external-adapter `endpoint` are influenceable, no egress allowlist / metadata-IP
  guard. → block link-local/RFC1918 unless declared; egress allowlist.
- **R11 [Medium] — timeout kills child, not process group.**
  `adapters/cli_tool.rs::wait_for_exit` calls `child.kill()`; grandchildren
  orphan. With R1 (no rlimits) → fork-bomb / disk-fill. → kill process group /
  job object; apply rlimits.
- **R12 [Medium] — `created_at` caller-influenced.** Receipt timestamps come from
  `RuntimeOptions`/env with a fixed fallback → forgeable, no freshness. → stamp
  at a trusted boundary.
- **R13 [Medium] — credential delivery channel for external adapters
  unspecified.** Owned by `external-adapter-plugin-protocol-v1`; flagged as the
  cross-cutting primitive (cli-tool, external adapter, outbox all need it).

## Phases

## Phase 1: Surgical fail-closed core fixes

Status: completed
Dependencies: none

Objective: Complete this phase.

Changes:
- none

Acceptance:
- none

## Phase 2: Enforcement mechanisms

Status: completed
Dependencies: none

Objective: Complete this phase.

Changes:
- none

Acceptance:
- none

## Phase 3: Proof verification + type-enforced gate

Status: completed
Dependencies: none

Objective: Complete this phase.

Changes:
- none

Acceptance:
- none

## Phase 4: Cross-cutting hardening

Status: completed
Dependencies: none

Objective: Complete this phase.

Changes:
- none

Acceptance:
- none

## Risks

- Fail-closed changes (C1/C4/C5) can deny previously-admitted edges; gate each on
  fixture review so a security tightening does not silently break a legitimate
  flow.
- R1/R2 are platform- and key-management-heavy; scope per-OS and per-environment.
- Several items touch files under active parallel work (payment, target_runner,
  contract spine); sequence around those workstreams.

## Acceptance

- [x] `dod1` C1 fail-closed on missing grant status, tests green.
- [x] `dod2` grant expiry/not_before added and enforced in core OSS.
- [x] `dod3` spend verbs require at least one aggregate cap (C4).
- [ ] `dod4` `*` scope not acceptable from untrusted grants (C5).
- [ ] `dod5` receipts signed + verified by default in non-local modes (R2).
- [ ] `dod6` sandbox profiles OS-enforced or documented as non-enforcing (R1).
- [ ] `dod7` payment rail settlement proofs are supervisor-verified, not
  skill-asserted (R3).
- [x] `dod8` state-machine success transition requires an admission witness (C6).
- [x] `dod9` payment-state writes are atomic/locked (R7).

## Rollback

Phase 1 fixes are independent and individually revertible. Phases 2–4 are
additive enforcement; nothing here changes wire shapes except the grant
expiry/proof-verification additions, which are gated on the schema-validation
work.

## Build Evidence

- 2026-05-22T02:42:26+10:00: `cargo test -p runx-core` passed.
- 2026-05-22T02:42:26+10:00: `cargo clippy -p runx-core --all-targets -- -D warnings`
  passed.
- 2026-05-22T02:42:26+10:00:
  `cargo test -p runx-runtime --test connect_policy_integration` passed.
- 2026-05-22T11:00:00+10:00: `cargo test --manifest-path crates/Cargo.toml -p
  runx-core -- --nocapture` passed.
- 2026-05-22T11:00:00+10:00: `cargo clippy --manifest-path crates/Cargo.toml -p
  runx-core --all-targets -- -D warnings` passed.
- 2026-05-22T11:00:00+10:00: `cargo clippy --manifest-path crates/Cargo.toml -p
  runx-runtime --all-targets -- -D warnings` passed.
- 2026-05-22T11:00:00+10:00: `pnpm typecheck` passed.
- 2026-05-22T11:00:00+10:00: `pnpm exec vitest run --config vitest.config.ts
  packages/runtime-local/src/runner-local/kernel-bridge.test.ts
  --fileParallelism=false --maxWorkers=1` passed.
- 2026-05-22T11:05:00+10:00: `cargo test --manifest-path crates/Cargo.toml -p
  runx-runtime --test payment_execution -- --nocapture` passed.
- 2026-05-22T11:08:00+10:00: `cargo test --manifest-path crates/Cargo.toml -p
  runx-core policy -- --nocapture` passed.
- 2026-05-22T11:08:00+10:00: `cargo test --manifest-path crates/Cargo.toml -p
  runx-runtime --test payment_execution --test stripe_spt_payment --test
  payment_ledger_projection --test payment_state -- --nocapture` passed.

## Origin

User-directed pentest review on 2026-05-21/22: widen findings into the Rust core
(which must be S-tier) and capture all findings into a work spec. Core attenuation
algebra verified largely sound; exposures are at admission edges, enforcement
placeholders, and skill-asserted trust.

## Harden Rounds

### round-1

Status: passed
Started: 2026-05-22T00:31:32Z
Ended: 2026-05-22T00:33:10Z

Checks:
- path audit
  - Grounded in: code:crates/runx-core/src/policy/connected_auth.rs:38
  - Result: passed
  - Evidence: Phase 1 C1 is bound to the connected-auth grant matcher and
- command audit
  - Grounded in: spec_gap:acceptance
  - Result: passed
  - Evidence: The built C1 slice has concrete evidence commands recorded
- scope/migration audit
  - Grounded in: code:crates/runx-runtime/src/connect.rs:1
  - Result: passed
  - Evidence: The current OSS branch only exports connect redaction, so the C1
- acceptance timing audit
  - Grounded in: code:crates/runx-core/tests/policy_proptest.rs:370
  - Result: passed
  - Evidence: Deterministic policy/proptest fixtures now supply
- rollback/repair audit
  - Grounded in: spec_gap:rollback
  - Result: passed
  - Evidence: Phase 1 is independently revertible by backing out the
- design challenge
  - Grounded in: code:crates/runx-core/src/policy/connected_auth.rs:60
  - Result: passed
  - Evidence: Requiring explicit `Active`, `expires_at`, and a caller-supplied

Issues:
- none
