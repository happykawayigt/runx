---
spec_version: '2.0'
task_id: runx-registry-skill-trust-cutover-v1
created: '2026-06-10T12:53:20Z'
updated: '2026-06-10T15:42:58Z'
status: completed
harden_status: passed
size: large
risk_level: high
---

# Registry skill trust cutover

## Current State

Status: completed
Current phase: final
Next: done
Reason: task completed
Blockers: none
Allowed follow-up command: `none`
Latest runner update: 2026-06-10T15:42:58Z
Review gate: pass

## Summary

Make the registry and skill execution story true end to end: every executable
registry skill, including graph-nested skills, must enter through one signed
manifest admission path; first-party trust must be granted by scoped keys and
policy, not by owner strings; installed versions must have a deterministic
filesystem contract; and every registry-backed run must tell the operator
exactly what code, version, digest, key, trust tier, and source executed.

This is the highest-priority follow-up because it is a governance boundary, not
a cosmetic registry polish pass.

## Objectives

- Route all executable registry materialization through the same signed install
  verifier and admission path.
- Add explicit trust policy binding registry source, owner namespace, trust
  tier, and signing key authority.
- Preserve official shorthand ergonomics only for policy-authorized official
  skills; third-party execution remains explicit `owner/name@version`.
- Replace split install semantics with a clear multi-version filesystem
  contract that never lets two versions compete for the same physical package
  path.
- Remove generated guidance for non-canonical install aliases; the public
  command is `runx registry install`.
- Emit concise execution provenance for every registry-backed skill run in
  human and JSON output.

## Scope

In scope:

- `crates/runx-runtime/src/registry/**`
- `crates/runx-runtime/src/execution/graph.rs`
- `crates/runx-runtime/tests/skill_run.rs`
- `crates/runx-cli/src/skill/**`
- `crates/runx-cli/src/registry.rs`
- `crates/runx-cli/src/official_skills.rs`
- `crates/runx-cli/tests/{registry,skill,doctor}.rs`
- Registry and skill documentation that names install, resolution, trust, and
  version layout behavior.

Out of scope:

- Hosted registry API changes in `../cloud`; this spec is the OSS client/runtime
  trust cutover only.
- New marketplace browsing UX.
- Supporting hidden compatibility aliases for old commands.
- Changing the skill contract format unless needed to record provenance already
  available from registry resolution.

## Grounding Evidence

- Graph execution currently materializes registry skills directly in
  `crates/runx-runtime/src/execution/graph.rs`, while top-level installs verify
  signed manifests through `crates/runx-runtime/src/registry/install.rs`.
- Registry trust tiers live as metadata in
  `crates/runx-runtime/src/registry/types.rs`, while local registry first-party
  classification is derived from owner naming in
  `crates/runx-runtime/src/registry/local/build.rs`.
- File registries store versions as `<owner>/<name>/<version>.json`, while
  explicit install writes the active package to `<owner>/<name>/SKILL.md`,
  collapsing version into one mutable path.
- Registry link output still emits `runx skill add ...` from
  `crates/runx-runtime/src/registry/local.rs`, while native help advertises
  `runx registry install`.
- Registry-backed skill output currently loses resolved source/version/digest
  metadata before rendering.

## Assumptions

- The executor may be Codex. Record evidence phase by phase through
  `scafld build runx-registry-skill-trust-cutover-v1`.
- Use dedicated Cargo target output when running long Rust gates if another
  agent is active: `CARGO_TARGET_DIR=crates/target-registry-trust`.
- No cloud code is required to complete this OSS trust boundary.
- Existing official skills should keep bare-name ergonomics only when resolved
  through an official registry/mirror and official key scope.

## Risks

- Tightening registry trust may reveal fixture registries that never signed
  nested executable skills. Fix fixtures; do not weaken admission.
- Filesystem layout changes can break local installs. This spec is a clean
  cutover: update docs/tests and avoid dual-read compatibility paths.
- Provenance output must be concise; dumping full manifests into command output
  is not acceptable.

## Rollback And Repair

- If Phase 1 or Phase 2 fails after partial implementation, revert registry
  materialization and trust-policy admission together; a verifier-only change
  without all call sites using it is not a valid intermediate state.
- If Phase 3 fails, remove the new side-by-side install materialization and
  reinstall affected local registry skills from the signed registry source.
- If Phase 4 output changes break fixtures, regenerate only the canonical help
  and registry fixtures after confirming they no longer advertise
  non-canonical install aliases.
- This spec intentionally has no compatibility fallback. Repair is by
  completing the clean cutover or reverting the whole phase.

## Acceptance

Profile: strict

Validation:
- `cargo test --manifest-path crates/Cargo.toml -p runx-runtime --features cli-tool --test integration native_graph_skill_run_rejects_unsigned_nested_registry_skill -- --nocapture`
- `cargo test --manifest-path crates/Cargo.toml -p runx-runtime --features cli-tool --test integration native_graph_skill_run_rejects_tampered_nested_registry_skill -- --nocapture`
- `cargo test --manifest-path crates/Cargo.toml -p runx-runtime --test integration registry_install_rejects_out_of_scope_manifest_key -- --nocapture`
- `cargo test --manifest-path crates/Cargo.toml -p runx-runtime --test integration registry_install_rejects_unsigned_or_mismatched_trust_tier -- --nocapture`
- `cargo test --manifest-path crates/Cargo.toml -p runx-cli --test integration registry_install_versions_are_side_by_side -- --nocapture`
- `cargo test --manifest-path crates/Cargo.toml -p runx-cli --test integration native_skill_registry_run_reports_provenance -- --nocapture`
- `cargo test --manifest-path crates/Cargo.toml -p runx-cli --test integration doctor_registry_json_reports_trust_policy_scope_without_key_material -- --nocapture`
- `pnpm vitest run tests/official-skill-fetch.test.ts tests/official-skill-resolution.test.ts`
- `pnpm fixtures:cli-help:check`
- `pnpm verify:fast`

## Phase 1: Single Executable Registry Admission

Status: completed
Dependencies: none

Objective: Make graph-nested executable registry skills obey the same signed

Changes:
- Introduce one runtime helper that resolves a registry ref into an admitted executable install candidate and applies signed-manifest verification.
- Replace direct graph materialization of executable registry skill content with that helper.
- Preserve context-only/advisory registry content behavior only where it is not executable.
- Add negative tests for unsigned and tampered nested registry skills.

Acceptance:
- [x] `ac1` command - Nested unsigned registry skills are rejected
  - Command: `cargo test --manifest-path crates/Cargo.toml -p runx-runtime --features cli-tool --test integration native_graph_skill_run_rejects_unsigned_nested_registry_skill -- --nocapture`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-6
- [x] `ac2` command - Nested tampered registry skills are rejected
  - Command: `cargo test --manifest-path crates/Cargo.toml -p runx-runtime --features cli-tool --test integration native_graph_skill_run_rejects_tampered_nested_registry_skill -- --nocapture`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-7

## Phase 2: Scoped Trust Policy

Status: completed
Dependencies: phase1

Objective: Make trust tier and owner authority enforced policy rather than

Changes:
- Add registry trust policy resolution that binds signing key id, registry source, owner namespace, and allowed trust tiers.
- Ensure the built-in official key authorizes official `runx/*` skills only through official registry/mirror policy.
- Ensure operator or third-party keys cannot mint first-party status by using the owner string `runx`.
- Add doctor diagnostics that report trust policy readiness without printing key material.

Acceptance:
- [x] `ac3` command - Out-of-scope manifest keys are rejected
  - Command: `cargo test --manifest-path crates/Cargo.toml -p runx-runtime --test integration registry_install_rejects_out_of_scope_manifest_key -- --nocapture`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-12
- [x] `ac4` command - Unsigned or mismatched trust tiers are rejected
  - Command: `cargo test --manifest-path crates/Cargo.toml -p runx-runtime --test integration registry_install_rejects_unsigned_or_mismatched_trust_tier -- --nocapture`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-13
- [x] `ac5` command - Doctor reports trust policy scope safely
  - Command: `cargo test --manifest-path crates/Cargo.toml -p runx-cli --test integration doctor_registry_json_reports_trust_policy_scope_without_key_material -- --nocapture`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-14

## Phase 3: Multi-Version Filesystem Contract

Status: completed
Dependencies: phase2

Objective: Make installed registry skills deterministic across versions and

Changes:
- Keep immutable cache entries version/digest-addressed.
- Change explicit install materialization so different versions install side-by-side.
- If a bare local execution pointer is needed, store it as an explicit lock or activation file that names owner, name, version, digest, and registry source.
- Update resolution to read that explicit pointer only when the operator chose to install/activate it.
- Update docs to describe the filesystem contract in operator terms.

Acceptance:
- [x] `ac6` command - Registry installs keep versions side-by-side
  - Command: `cargo test --manifest-path crates/Cargo.toml -p runx-cli --test integration registry_install_versions_are_side_by_side -- --nocapture`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-19
- [x] `ac7` command - Native skill resolution handles installed versions
  - Command: `cargo test --manifest-path crates/Cargo.toml -p runx-cli --test integration native_skill_resolves_registry_versions_side_by_side -- --nocapture`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-20

## Phase 4: Canonical Install Command And Provenance

Status: completed
Dependencies: phase3

Objective: Clean up operator guidance and make registry-backed execution

Changes:
- Replace generated `runx skill add ...` guidance with canonical `runx registry install ...` guidance.
- Remove docs/fixtures that advertise non-canonical install aliases.
- Keep third-party execution explicit as `owner/name@version`.
- Extend human and JSON skill output for registry-backed runs with concise provenance: skill id, version, digest, profile digest, registry source, trust tier, and key id.

Acceptance:
- [x] `ac8` command - Registry-backed runs report provenance
  - Command: `cargo test --manifest-path crates/Cargo.toml -p runx-cli --test integration native_skill_registry_run_reports_provenance -- --nocapture`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-25
- [x] `ac9` command - Official skill resolution still respects policy
  - Command: `pnpm vitest run tests/official-skill-fetch.test.ts tests/official-skill-resolution.test.ts`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-26
- [x] `ac10` command - CLI help fixtures advertise canonical registry install
  - Command: `pnpm fixtures:cli-help:check`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-27

## Phase 5: Release Gate

Status: completed
Dependencies: phase4

Objective: Prove the registry trust cutover did not break the broader OSS

Changes:
- Fix any style, fixture, or fast verifier regressions introduced by the cutover.

Acceptance:
- [x] `ac11` command - Fast verifier passes
  - Command: `pnpm verify:fast`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-32

## Harden Rounds

### round-1

Status: passed
Started: 2026-06-10T12:57:07Z
Ended: 2026-06-10T12:59:51Z

Observations:
- path
  - Result: clean
  - Anchor: code:crates/runx-runtime/src/execution/graph.rs:262
- command
  - Result: clean
  - Anchor: code:package.json:76
- scope
  - Result: clean
  - Anchor: spec_gap:Scope
- timing
  - Result: clean
  - Anchor: spec_gap:Phases
- rollback
  - Result: clean
  - Anchor: spec_gap:Risks
- design
  - Result: clean
  - Anchor: code:crates/runx-runtime/src/registry/install.rs:210

## Review

Status: completed
Verdict: pass
Mode: verify
Provider: codex
Output: codex.output_file
Summary: Verify review found no remaining completion blockers. The previously recorded blockers appear repaired in the current task-scoped source, and the acceptance evidence is treated as already executed per the review packet.

Attack log:
- `.scafld/specs/active/runx-registry-skill-trust-cutover-v1.md`: prior blocker verification -> clean (Checked the active spec's prior review findings against current source: source binding now exists, local official source authority is propagated, post-resolution execution errors include provenance, and the binding artifact now uses `runx registry install`.)
- `crates/runx-runtime/src/execution/graph.rs`: single admission path -> clean (Traced nested graph registry materialization in `crates/runx-runtime/src/execution/graph.rs`; it constructs `InstallCandidate` and delegates admission to `install_local_skill`.)
- `crates/runx-runtime/src/registry/install.rs`: signed manifest verification -> clean (Reviewed `install_local_skill` path: signed manifest is required, key verification happens before writes, identity/digest/profile digest are checked, and trust scope is enforced before filesystem materialization.)
- `crates/runx-runtime/src/registry/trust_anchor.rs`: trust policy scope -> clean (Reviewed third-party and official key scopes. Third-party keys bind owner and source and cannot grant first-party or sign `runx/*`; official key is limited to `runx/*` plus official source authority.)
- `crates/runx-cli/src/registry.rs`: source authority propagation -> clean (Traced CLI registry target resolution into install candidates; remote/local official and local registry source authority are passed into the verifier for install and skill execution paths.)
- `crates/runx-runtime/src/registry/refs.rs`: multi-version filesystem layout -> clean (Checked package path construction and tests. Installed and materialized registry refs include resolved version in physical path, preventing competing versions from sharing the same package path.)
- `crates/runx-cli/src/skill.rs`: execution provenance -> clean (Reviewed success and error branches of `run_native_skill`; registry provenance is attached on successful runtime output and passed into JSON failure output after resolution errors.)
- `README.md docs crates packages tests fixtures bindings`: canonical install guidance -> clean (Searched non-archive operator-facing docs/fixtures/bindings for `runx skill add`; no current public generated guidance remains outside historical scafld archive/spec text.)
- `crates/runx-cli/src/doctor.rs`: doctor diagnostics -> clean (Checked registry doctor evidence path; it reports key ids and policy scope without printing public key material, with tests covering the JSON output.)
- `acceptance criteria`: acceptance rerun -> skipped (Skipped per provider instruction: review mode is read-only and recorded acceptance evidence must be treated as already executed.)

Findings:
- none

