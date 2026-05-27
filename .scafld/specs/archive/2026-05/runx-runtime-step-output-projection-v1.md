---
spec_version: '2.0'
task_id: runx-runtime-step-output-projection-v1
created: '2026-05-27T16:00:00Z'
updated: '2026-05-27T14:49:16Z'
status: completed
harden_status: not_run
size: large
risk_level: high
---

# Runtime step output projection cutover

## Current State

Status: completed
Current phase: final
Next: done
Reason: task completed
Blockers: none
Allowed follow-up command: `none`
Latest runner update: 2026-05-27T14:49:16Z
Review gate: pass

## Summary

Stop parsing and cloning step stdout JSON in multiple runtime layers. A step
should produce one typed projection containing the public outputs, parsed skill
claim, and receipt reference observations. Step execution and receipt sealing
should consume that projection instead of independently re-reading stdout.

This is an architectural cleanup of the hot path: one output projection owner,
no duplicated parse/copy responsibilities, no compatibility shim, and no receipt
contract change.

## Objectives

- Parse stdout JSON at most once per step in the normal runtime path.
- Use a single `StepOutputProjection` or equivalent type for outputs, skill
  claim, and receipt reference observations.
- Keep receipt sealing deterministic and schema-compatible.
- Preserve declared outputs, artifact exposure, skill-claim context exposure,
  payment supervisor proof behavior, and receipt reference extraction.
- Add correctness coverage for representative skill, graph, artifact, and
  receipt paths; do not add perf benchmarks.
- Pass the Claude provider adversarial review gate.

## Scope

- In scope:
  - `crates/runx-runtime/src/execution/output_projection.rs`
  - `crates/runx-runtime/src/execution/runner/steps.rs`
  - `crates/runx-runtime/src/receipts/seal.rs`
  - Focused runtime integration tests around graph skill outputs and receipts.
- Out of scope:
  - Receipt schema changes.
  - Canonical receipt algorithm changes.
  - Public adapter protocol changes.
  - TypeScript hash/canonicalization cleanup.
  - Performance benchmark additions.

## Dependencies

- Should run after `runx-runtime-mcp-concurrency-v1` only to reduce overlapping
  runtime edits. There is no semantic dependency on MCP.

## Assumptions

- `SkillOutput.stdout` must remain available as the raw string for compatibility
  and receipts.
- The projection can borrow or own parsed data internally, but public
  `StepRun.outputs` remains an owned `JsonObject`.
- Receipt reference extraction is a projection concern because it interprets the
  same stdout JSON as output projection.

## Touchpoints

- `crates/runx-runtime/src/execution/output_projection.rs`
- `crates/runx-runtime/src/execution/runner/steps.rs`
- `crates/runx-runtime/src/receipts/seal.rs`
- `crates/runx-runtime/tests/skill_run.rs`
- `crates/runx-runtime/tests/receipt_tree.rs`

## Risks

- A projection split can accidentally omit reserved fields or declared artifact
  wrapping. Mitigation: keep projection behavior tests around declared outputs,
  artifact emits, and graph context.
- Payment proof metadata must bind to the final receipt body. Mitigation:
  preserve existing supervisor proof insertion order and receipt rebind tests.
- Receipt reference extraction is subtle. Mitigation: keep existing receipt-tree
  and signing tests in validation.

## Acceptance

Profile: strict

Validation:
- `cargo test --manifest-path crates/Cargo.toml -p runx-runtime --all-features --lib`
- `cargo test --manifest-path crates/Cargo.toml -p runx-runtime --all-features --test integration skill_run`
- `cargo test --manifest-path crates/Cargo.toml -p runx-runtime --all-features --test integration receipt_tree`
- `cargo test --manifest-path crates/Cargo.toml -p runx-runtime --all-features --test integration receipt_signing`
- `cargo fmt --manifest-path crates/Cargo.toml --all --check`
- `scafld review runx-runtime-step-output-projection-v1 --provider claude --review-depth deep`

## Phase 1: Projection Boundary

Status: completed
Dependencies: none

Objective: Define one runtime-owned projection boundary for step output.

Changes:
- Introduce one projection type that carries raw capture, parsed claim, public outputs, and reference observations needed by receipt sealing.
- Move stdout JSON interpretation out of receipt sealing's independent parse path.
- Keep raw `SkillOutput` stable and available.

Acceptance:
- [x] `p1_ac1` command - Normal step execution computes one projection before
  - Command: `rg -n "StepOutputProjection|project_skill_output|step_output" crates/runx-runtime/src/execution crates/runx-runtime/src/receipts`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-3
- [x] `p1_ac2` command - Receipt sealing consumes projection refs rather than
  - Command: `rg -n "output_refs|collect_stdout_refs|StepOutputProjection|projection" crates/runx-runtime/src/receipts/seal.rs crates/runx-runtime/src/execution`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-4

## Phase 2: Behavior Preservation

Status: completed
Dependencies: phase1

Objective: Preserve graph output and receipt behavior.

Changes:
- none

Acceptance:
- [x] `p2_ac1` command - Runtime library tests pass.
  - Command: `cargo test --manifest-path crates/Cargo.toml -p runx-runtime --all-features --lib`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-9
- [x] `p2_ac2` command - Skill-run integration tests pass.
  - Command: `cargo test --manifest-path crates/Cargo.toml -p runx-runtime --all-features --test integration skill_run`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-10
- [x] `p2_ac3` command - Receipt-tree integration tests pass.
  - Command: `cargo test --manifest-path crates/Cargo.toml -p runx-runtime --all-features --test integration receipt_tree`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-11
- [x] `p2_ac4` command - Receipt-signing integration tests pass.
  - Command: `cargo test --manifest-path crates/Cargo.toml -p runx-runtime --all-features --test integration receipt_signing`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-12

## Phase 3: Formatting And Local Gate

Status: completed
Dependencies: phase2

Objective: Complete this phase.

Changes:
- none

Acceptance:
- [x] `p3_ac1` command - Formatting remains clean.
  - Command: `cargo fmt --manifest-path crates/Cargo.toml --all --check`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-17

## Phase 4: Claude Review Gate Preparation

Status: completed
Dependencies: phase3

Objective: Verify the requested provider gate is declared before handing the

Changes:
- none

Acceptance:
- [x] `p4_ac1` command - Claude review command is declared.
  - Command: `rg -n "scafld review runx-runtime-step-output-projection-v1 --provider claude --review-depth deep" .scafld/specs/active/runx-runtime-step-output-projection-v1.md`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-22

## Rollback

Revert the projection commit. Because public receipt and adapter schemas remain
unchanged, rollback is a pure runtime implementation rollback.

## Review

Status: completed
Verdict: pass
Mode: discover
Provider: claude:claude-opus-4-7
Output: claude.mcp_submit_review
Summary: Reviewed the runtime step output projection cutover across `execution/output_projection.rs`, `execution/runner/steps.rs`, and `receipts/seal.rs`, plus the tests in `tests/skill_run.rs` and `tests/receipt_tree.rs`. The projection cutover is clean within the declared scope: every graph step path (regular, replayed payment, agent_task, agent skill, tool, approval, runtime-error) computes one `StepOutputProjection` and forwards it to `step_receipt_with_*_projection_and_signature_policy`. Receipt sealing reads `projection.refs` for the stdout-derived references and adds metadata-derived references (`agent_request_id`, payment supervisor proof/evidence) without re-parsing stdout. Reference extraction continues to populate only `source_refs`, `signal_refs`, `artifact_refs`, `surface_refs` from claim stdout; `verification_refs` and `evidence_refs` remain metadata-sourced, and the new `treats_structured_stdout_as_claim_not_receipt_proof` test pins this. Payment supervisor evidence is attached before sealing and proof is rebound to the re-sealed child digest in `attach_parent_to_child_receipts`, matching the prior contract. Replay parity remains intact: `run_replayed_payment_step` reconstructs metadata, projects, and checks the rebuilt receipt id/digest/rail-proof. The wrapper `step_receipt_with_disposition_and_policy` calls `project_step_output` once before delegating, so the public `step_receipt`/`step_receipt_with_signature_policy` APIs remain stable without re-parsing more than once per call. `JsonObject` is `BTreeMap`, so iteration order in projection and reference construction is deterministic. Out-of-scope parse sites (`skill_run.rs::parse_output_payload`, `harness/runner.rs::skill_output_object`, `payment/ledger.rs::with_step_outputs` fallback) are recognized as outside this spec's bounded scope and were not regressed by the cutover. Acceptance evidence recorded as `pass` for rg/cargo gates; no completion-blocking issues found.

Attack log:
- `crates/runx-runtime/src/execution/output_projection.rs`: Verify stdout JSON parsed exactly once per projection build and reused for claim+outputs+refs without double parse -> clean (`serde_json::from_slice` runs once into `parsed_stdout`; reused via `parsed.clone()` for outputs["skill_claim"] and moved into `claim`; refs collected from `parsed_stdout.as_ref()`.)
- `crates/runx-runtime/src/execution/runner/steps.rs`: Trace every step path (regular skill, replayed payment, agent_task, agent skill, tool, approval, error) to confirm a single projection drives both StepRun.outputs and the sealed receipt with no second stdout parse -> clean (Each path threads `StepOutputProjection` into `step_receipt_with_*_projection_and_signature_policy` and reuses `projection.outputs` for StepRun; approval/error paths construct outputs separately but still build the projection once for receipt sealing.)
- `crates/runx-runtime/src/receipts/seal.rs`: Check that the projection-based seal preserves the prior receipt body (act, criterion bindings, decisions, signals) and that metadata-derived references still flow through `output_refs` -> clean (`output_refs(output, &projection.refs)` clones projection refs, then appends `agent_request_id` reference and supervisor proof/evidence refs from metadata. Act, seal criterion, decisions, and signals consume the merged refs deterministically (BTreeMap-backed JsonObject).)
- `crates/runx-runtime/src/execution/runner/steps.rs::seal_regular_skill_step`: Verify payment supervisor evidence/proof binding order remains: attach evidence before sealing, then enforce gate, record proof in metadata, rebind on re-seal -> clean (Order matches the previous contract; `attach_payment_supervisor_evidence_before_gate` runs before projection, projection feeds receipt sealing, then `enforce_step_authority_receipt_before_success` returns proof, `record_payment_supervisor_proof_metadata` writes it, and graph re-seal rebinds via `rebind_supervisor_proof_to_receipt`.)
- `crates/runx-runtime/src/execution/output_projection.rs::stdout_refs`: Confirm malicious stdout cannot inject verification or evidence references into the receipt -> clean (stdout_refs only writes to artifact/signal/source/surface fields; verification_refs and evidence_refs are populated solely via metadata in seal.rs::output_refs. Pinned by tests/skill_run.rs::native_skill_run_treats_structured_stdout_as_claim_not_receipt_proof.)
- `crates/runx-runtime/src/execution/runner/steps.rs::expose_declared_run_outputs / expose_declared_artifacts / expose_skill_claim_context_fields`: Check reserved-name handling so claim cannot overwrite reserved outputs (raw/skill_claim/stdout/stderr/status) and declared outputs cannot collide with them -> clean (`reject_reserved_step_output_name` errors on declared/named-emit collisions; `expose_skill_claim_context_fields` skips reserved keys and existing entries. Tested by `native_graph_skill_run_rejects_reserved_artifact_output_names`.)
- `crates/runx-runtime/src/execution/runner/steps.rs::run_replayed_payment_step`: Confirm replay path computes projection once and validates rebuilt receipt id/digest/rail proof against sealed payment state -> clean (Replay reconstructs metadata (proof) before projection, projects once, then seals with projection; checks receipt.id, digest, and `receipt_has_payment_rail_proof` mirror prior invariants.)
- `crates/runx-runtime/src/receipts/seal.rs::step_receipt_with_disposition_and_policy`: Verify the wrapper that bridges legacy callers (`step_receipt`, `step_receipt_with_signature_policy`, harness replay) still parses exactly once per receipt call -> clean (Wrapper calls `project_step_output(params.output)` once then forwards to the projection-aware function. Public API contract preserved; not a compatibility shim across runtime contract changes, only an internal convenience over the same code path.)
- `crates/runx-runtime/tests/skill_run.rs`: Confirm tests pin declared output exposure, reserved field rejection, deferred closure, structured-stdout claim isolation, and gate behavior -> clean (Covers exposure of `result` declared outputs, `skill_claim` propagation, deferred disposition preservation, transition-gate rejection of skill_claim fact, and reserved artifact name errors.)
- `crates/runx-runtime/tests/receipt_tree.rs`: Verify graph/child receipt tree validation still rejects tampered digests, parent-mismatch, missing locator, orphan children, and production pseudo-signatures -> clean (Test file unchanged in semantics; resolver, validate, and verify entry points still exercised against the projection-built receipts.)
- `scope: ambient_drift`: Distinguish task-relevant edits from unrelated active specs (mcp concurrency, ts-native passthrough) in the dirty tree -> clean (Other dirty paths (mcp/server.rs, graph.rs, packages/cli/*, tests/mcp_server.rs) belong to sibling active specs and are not attributed to this task. Workspace classification reports 0 ambient drift relative to baseline.)
- `out-of-scope parse sites`: Inventory remaining stdout re-parses outside the cutover scope (`skill_run.rs::parse_output_payload`, `harness/runner.rs::skill_output_object`, `payment/ledger.rs::with_step_outputs` fallback) -> clean (All three live outside the declared touchpoints. They are pre-existing and not regressed by the cutover; the spec explicitly bounds scope to runner step + receipt sealing.)

Findings:
- none
