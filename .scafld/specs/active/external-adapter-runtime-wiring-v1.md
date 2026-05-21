---
spec_version: '2.0'
task_id: external-adapter-runtime-wiring-v1
created: '2026-05-22T00:00:00+10:00'
updated: '2026-05-22T01:22:00+10:00'
status: active
harden_status: not_run
size: medium
risk_level: high
---

# External adapter runtime wiring

## Current State

Status: active
Current phase: runtime hardening slice complete
Next: rerun handoff/review after the stale workspace-mutation review finding is cleared
Reason: focused runtime wiring now covers explicit inline/package-relative
manifest discovery, credential delivery/redaction into the supervised process,
host-resolution frame routing through `Host`, and fail-closed response
identity/frame validation. Helper SDKs, conformance adapters, and broad
registry discovery remain owned by `external-adapter-plugin-protocol-v1`.
Blockers: none
Allowed follow-up command: `scafld handoff external-adapter-runtime-wiring-v1`
Latest runner update: 2026-05-22T01:22:00+10:00 added package-relative
`manifest_path`, process-env credential delivery with response-observation
redaction, public credential refs plus delivery-observation metadata,
host-resolution frame normalization, graph-level host routing, and 16 focused
passing runtime tests.
Review gate: stale_failed_workspace_mutation_review; rerun required

## Summary

Expose the existing feature-gated external adapter process supervisor as a
usable Rust runtime adapter path. The adapter path must accept
`source_type: external-adapter`, build the contract invocation frame under Rust
authority, call the supervisor, and convert accepted adapter observations into
normal `SkillOutput` values. It must not add provider-specific integration code
to the runtime kernel.

This spec is intentionally narrower than
`external-adapter-plugin-protocol-v1`. It is a Rust runtime wiring and
hardening slice, not the helper SDK, conformance adapter, registry discovery, or
custom provider package story.

## Context

Primary owner spec:
- `.scafld/specs/active/external-adapter-plugin-protocol-v1.md`

Owned touchpoints:
- `crates/runx-runtime/src/adapters/external_adapter.rs`
- `crates/runx-runtime/tests/external_adapter.rs`
- minimal runtime adapter-selection code when needed

Out of scope:
- x402 payment runtime, tests, and fixtures.
- canonical-json, core, and cloud files.
- provider-specific GitHub, Slack, Sentry, or hosted adapter logic.
- TypeScript helper SDKs.
- registry-backed or remote manifest discovery semantics.

## Objectives

- Add a feature-gated `SkillAdapter` facade for `external-adapter`.
- Keep the supervisor authoritative: the facade only builds contract frames,
  resolves manifests, calls the supervisor, and maps observations into runtime
  outputs.
- Support explicit inline manifests and package-relative manifest files without
  registry lookup or provider-specific runtime behavior.
- Deliver admitted credential material only through the existing private
  `CredentialDelivery` process-env channel, project only public credential refs
  and delivery-observation metadata across the external adapter boundary, and
  redact adapter observations before runtime mapping.
- Normalize external host-resolution frames into existing host protocol
  resolution requests.
- Preserve fail-closed behavior when the source type is unsupported, the
  manifest is absent or malformed, the response identity mismatches, the
  adapter crashes, or the response cannot safely map to runtime output.
- Prove a graph or skill invocation with `source_type: external-adapter`
  reaches the supervisor.

## Acceptance

Profile: strict

Definition of done:
- [x] `dod1` Runtime exposes a feature-gated external adapter `SkillAdapter`
  path for `source_type: external-adapter`.
- [x] `dod2` The path builds `ExternalAdapterInvocation` from runtime
  `SkillInvocation` without importing provider-specific logic.
- [x] `dod3` Manifest discovery remains explicit. Inline manifests or injected
  resolvers are allowed for this slice; package-relative manifest files are
  allowed when they canonicalize below the skill directory; implicit registry
  lookup is blocked until `external-adapter-plugin-protocol-v1` settles it.
- [x] `dod4` Tests prove a graph/skill invocation reaches the supervisor,
  package-relative manifest paths fail closed on directory escape, credentials
  are delivered/redacted, public credential delivery metadata is projected,
  host-resolution frames reach `Host`, and fail-closed behavior is preserved.

Validation:
- [x] `v1` Focused Rust tests pass.
  - Command:
    `cargo test --manifest-path crates/Cargo.toml -p runx-runtime --features external-adapter --test external_adapter -- --nocapture`
  - Expected kind: `exit_code_zero`
  - Status: passed
  - Evidence: 2026-05-22T01:22:00+10:00 passed 16 tests, including
    `external_adapter_graph_invocation_reaches_process_supervisor`,
    `external_adapter_manifest_path_resolves_below_skill_directory`,
    `external_adapter_manifest_path_rejects_directory_escape`,
    `external_adapter_process_supervisor_delivers_credentials_and_redacts_observations`,
    `external_adapter_skill_adapter_passes_credential_delivery_to_supervisor`,
    `external_adapter_skill_adapter_projects_public_credential_refs_and_observation`,
    `external_adapter_process_supervisor_maps_host_resolution_frame`,
    `external_adapter_graph_host_resolution_frame_reaches_host`,
    `external_adapter_skill_adapter_fails_closed_without_inline_manifest`, and
    `external_adapter_skill_adapter_preserves_supervisor_fail_closed_response_mismatch`.
- [x] `v2` Focused spec validates.
  - Command: `scafld validate external-adapter-runtime-wiring-v1 --json`
  - Expected kind: `exit_code_zero`
  - Status: passed
  - Evidence: 2026-05-22T00:42:00+10:00 returned
    `{"ok":true,"command":"validate","result":{"task_id":"external-adapter-runtime-wiring-v1","path":"/Users/kam/dev/runx/runx/oss/.scafld/specs/active/external-adapter-runtime-wiring-v1.md","valid":true,"errors":null}}`.

## Design Constraints

- Feature-gated code must not be reachable without `features =
  ["external-adapter"]`.
- Runtime-local or `@runxhq/adapters` must not be used as a fallback.
- The Rust runtime may parse an explicit manifest and frame data, but must not
  understand provider APIs.
- If manifest lookup requires a new package or registry convention, this spec
  must record that as a blocker rather than inventing a broad discovery system.

## Remaining Boundary

The current parser/runtime `SkillSource` still carries external-adapter manifest
metadata through `source.raw`; this slice deliberately keeps that explicit and
local. It supports inline manifests and package-relative `manifest_path` files
that canonicalize below the skill directory. It does not claim registry-backed,
remote, signed, or catalog-driven manifest discovery.

## Rollback

Remove the feature-gated adapter facade and keep the process supervisor as an
explicit API if runtime selection cannot preserve Rust authority or requires
provider-specific logic.

## Review

Status: completed
Verdict: fail
Mode: discover
Provider: claude:claude-opus-4-7
Output: claude.mcp_submit_review
Summary: Reviewed external-adapter-runtime-wiring-v1 in discover mode (light depth). The feature-gated `ExternalAdapterSkillAdapter` in `crates/runx-runtime/src/adapters/external_adapter.rs` exposes `source_type: external-adapter` through `SkillAdapter::invoke`, builds an `ExternalAdapterInvocation` from `SkillInvocation` under Rust authority (no provider-specific logic), resolves manifests via inline lookup (`source.external_adapter.manifest` or `source.external_adapter_manifest`) with a generic resolver seam, and delegates to `ExternalAdapterProcessSupervisor` which preserves identity/protocol/transport/timeout/exit-code/response-mismatch/credential-frame/timeout-cancellation fail-closed paths. Module is gated behind the `external-adapter` feature in both `adapters.rs` and `lib.rs`, and the test file `crates/runx-runtime/tests/external_adapter.rs` is gated with `#![cfg(feature = "external-adapter")]`. The graph-level test instantiates `Runtime::new(ExternalAdapterSkillAdapter::default(), ...)` and exercises `run_graph_file` end-to-end, capturing the on-the-wire invocation to assert the supervisor was reached with expected fields. Acceptance criteria dod1–dod4 are satisfied by the existing code and the 9 tests in the validation evidence. No completion blockers identified. Workspace changed during review; review failed closed.

Attack log:
- `spec acceptance dod1-dod4`: Trace adapter facade, manifest resolution, supervisor call, and fail-closed paths against each definition-of-done item -> clean (Facade is feature-gated, builds invocation contract without provider knowledge, supports inline manifests + injectable resolver, and tests cover graph reach + missing-manifest + response-mismatch fail-closed paths.)
- `feature gating / module exposure`: Check that external_adapter module and tests are unreachable without the external-adapter feature -> clean (adapters.rs gates `pub mod external_adapter` on feature; lib.rs only enables `pub mod adapters` when one of the adapter features (including external-adapter) is set; test file is gated via `#![cfg(feature = "external-adapter")]`.)
- `runtime integration / regression hunt`: Confirm wiring did not perturb other adapters or step execution paths (cli-tool, agent, a2a, mcp, catalog) -> clean (Runtime still dispatches via single `runtime.adapter.invoke` in steps.rs:60; no provider-specific branches added; only impact is a new optional adapter type behind feature flag.)
- `supervisor fail-closed behavior`: Verify identity mismatch, unexpected credential frames, unsupported protocol/schema, transport, timeouts, crashed-process, oversized response, and pre-spawn validation all return errors that surface through the facade as RuntimeError::SkillFailed -> clean (validate_invocation_contract runs before spawn; parse_response rejects credential_request frames and unknown schemas; validate_response_contract enforces schema/protocol/adapter_id/invocation_id; timeout path returns TimedOut with cancellation frame; oversize stdout returns ResponseTooLarge.)
- `subprocess hygiene / dark patterns`: Look for shell injection, env leakage, deadlocks, identifier collisions, or stdin/stdout race conditions -> clean (spawn_process uses env_clear + explicit envs; stdout/stderr captured on dedicated threads to avoid pipe deadlock; identifier_segment sanitizes skill names and defaults to 'skill'; supervisor reads compact JSON terminated by newline to match shell `read -r` semantics; process group + /bin/kill TERM/KILL escalation present on unix with direct child kill fallback.)
- `conventions / AGENTS.md`: Check error envelope, trusted-kernel boundaries, no test logic in production, no hardcoded secrets -> clean (Errors via thiserror; no secret material handled (credential refs are pass-through); no test-only branches in production code; adapter lives in runtime crate, not in core/policy/state-machine.)
- `workspace mutation guard`: compare pre-review and post-review workspace snapshots -> finding (removed .scafld/specs/active/external-adapter-runtime-wiring-v1.md (was M 1bc0cab4af492066e09e9f9cc0be432c6723c32fbc112b27a5716577a9fb3bc8))

Findings:
- [critical/blocks completion] `workspace_mutation` Workspace changed during review.
  - Location: `.scafld/specs/active/external-adapter-runtime-wiring-v1.md (was M 1bc0cab4af492066e09e9f9cc0be432c6723c32fbc112b27a5716577a9fb3bc8)`
  - Evidence: workspace changed during review: removed .scafld/specs/active/external-adapter-runtime-wiring-v1.md (was M 1bc0cab4af492066e09e9f9cc0be432c6723c32fbc112b27a5716577a9fb3bc8)
  - Impact: The review provider changed the workspace while acting as a read-only reviewer, so its verdict is not trustworthy.
  - Validation: Restore the workspace to the expected state, ensure the provider is read-only, then rerun scafld review.
