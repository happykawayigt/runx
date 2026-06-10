---
spec_version: '2.0'
task_id: runx-cli-operator-ux-hardening-v2
created: '2026-06-10T12:53:20Z'
updated: '2026-06-10T15:51:57Z'
status: completed
harden_status: passed
size: medium
risk_level: medium
---

# CLI operator UX hardening v2

## Current State

Status: completed
Current phase: final
Next: done
Reason: task completed
Blockers: none
Allowed follow-up command: `none`
Latest runner update: 2026-06-10T15:51:57Z
Review gate: pass

## Summary

Raise daily operator CLI quality from "works" to "confident": command-local help
must work everywhere, `--json` must be reliable for automation even on errors,
registry search must show useful human results, pending runs must offer
copy-paste resume commands, and doctor diagnostics must explain concrete
repairs rather than only naming missing configuration.

This spec deliberately avoids runtime semantics. It is the operator surface
layer that sits on top of the existing Rust core.

## Objectives

- Support `--help` for nested command surfaces such as `runx skill` and
  `runx history`.
- Make `--json` errors machine-readable and stderr-clean for commands that
  advertise JSON mode.
- Render human `registry search` results with names, versions, trust/digest
  signal, and a next action.
- Replace placeholder resume guidance with concrete copy-paste commands when a
  skill run needs agent input.
- Populate doctor diagnostic repair hints with exact env vars or command steps,
  without printing secret values.

## Scope

In scope:

- `crates/runx-cli/src/launcher.rs`
- `crates/runx-cli/src/main.rs`
- `crates/runx-cli/src/{skill,history,registry,doctor}.rs`
- CLI tests under `crates/runx-cli/tests/**`
- CLI docs and fixture snapshots for help/error output.

Out of scope:

- Registry trust enforcement and multi-version install behavior
  (`runx-registry-skill-trust-cutover-v1`).
- New commands, shells, TUI surfaces, or hosted dashboard work.
- Changing JSON success schemas except to add non-breaking repair/resume fields
  already modeled by contracts.

## Grounding Evidence

- `runx skill --help` currently fails as a missing skill argument.
- `runx history --help` currently fails as an unknown history flag.
- `runx verify --receipt missing.json --json` emits plain human error text
  instead of a JSON failure envelope.
- `runx registry search` has JSON results, but the human renderer hides the
  actual result list.
- `runx doctor authority --json` emits warning diagnostics with empty
  `repairs` arrays.

## Assumptions

- The executor may be Codex. Record evidence through
  `scafld build runx-cli-operator-ux-hardening-v2`.
- Prefer additive, command-local rendering helpers over a broad CLI framework
  rewrite.
- Use targeted Cargo tests first. Reserve `pnpm verify:fast` for final
  integration.

## Risks

- Error-shape changes can break scripts. This spec only changes commands that
  explicitly requested `--json`, where machine-readable output is the contract.
- Resume command rendering can leak local absolute paths. Use repo-relative or
  shell-safe values where possible, and keep secrets out of rendered commands.
- Doctor repairs must be actionable but not mutate state automatically.

## Rollback And Repair

- Each phase is output-only and can be reverted independently if a fixture or
  script contract regresses.
- JSON failure-envelope changes are guarded by `--json`; if automation breaks,
  revert the JSON-mode dispatcher change and rerun the exact failing command.
- Help, search, resume, and doctor rendering failures are repaired by updating
  the renderer and fixture together; no runtime state migration is involved.

## Acceptance

Profile: standard

Validation:
- `crates/target/debug/runx skill --help | rg "runx skill .*--input key=value.*--runner name"`
- `crates/target/debug/runx history --help | rg "runx history .*--receipt-dir.*--json"`
- `crates/target/debug/runx verify --receipt missing.json --json >/tmp/runx-error.json 2>/tmp/runx-error.err; test $? -ne 0`
- `test ! -s /tmp/runx-error.err`
- `jq -e '.status == "failure" and .error.message' /tmp/runx-error.json`
- `crates/target/debug/runx doctor authority --json | jq -e 'all(.diagnostics[]; (.severity != "warning") or (.repairs | length > 0))'`
- `cargo test --manifest-path crates/Cargo.toml -p runx-cli --test integration launcher -- --nocapture`
- `cargo test --manifest-path crates/Cargo.toml -p runx-cli --test integration verify -- --nocapture && cargo test --manifest-path crates/Cargo.toml -p runx-cli --test integration registry -- --nocapture && cargo test --manifest-path crates/Cargo.toml -p runx-cli --test integration skill -- --nocapture && cargo test --manifest-path crates/Cargo.toml -p runx-cli --test integration doctor -- --nocapture && cargo test --manifest-path crates/Cargo.toml -p runx-cli history -- --nocapture`
- `pnpm fixtures:cli-help:check`
- `pnpm verify:fast`

## Phase 1: Command-Local Help

Status: completed
Dependencies: none

Objective: Make nested command help available before required argument parsing

Changes:
- Teach launcher parsing to intercept `--help`/`-h` for nested `skill` and `history` surfaces.
- Ensure help text names the canonical commands and common flags.
- Add launcher tests for `runx skill --help`, `runx history --help`, and the existing top-level help path.

Acceptance:
- [x] `ac1` command - Skill help renders the nested skill command surface
  - Command: `crates/target/debug/runx skill --help | rg "runx skill .*--input key=value.*--runner name"`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-6
- [x] `ac2` command - History help renders the nested history command surface
  - Command: `crates/target/debug/runx history --help | rg "runx history .*--receipt-dir.*--json"`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-7
- [x] `ac3` command - Launcher tests pass
  - Command: `cargo test --manifest-path crates/Cargo.toml -p runx-cli --test integration launcher -- --nocapture`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-8

## Phase 2: JSON Failure Envelope

Status: completed
Dependencies: phase1

Objective: Make commands that advertise `--json` emit JSON on failure.

Changes:
- Introduce one CLI failure envelope for JSON mode: `{ "status": "failure", "error": { "message": "...", "code": "..." } }`.
- Route parse/runtime failures for `verify`, `registry`, and `skill` JSON mode through that envelope.
- Keep human stderr behavior unchanged when JSON mode is not requested.

Acceptance:
- [x] `ac4` command - JSON verify failure writes a failure envelope
  - Command: `crates/target/debug/runx verify --receipt missing.json --json >/tmp/runx-error.json 2>/tmp/runx-error.err; test $? -ne 0`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-13
- [x] `ac5` command - JSON verify failure keeps stderr clean
  - Command: `test ! -s /tmp/runx-error.err`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-14
- [x] `ac6` command - JSON failure has a message
  - Command: `jq -e '.status == "failure" and .error.message' /tmp/runx-error.json`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-15
- [x] `ac7` command - JSON-capable command tests pass
  - Command: `cargo test --manifest-path crates/Cargo.toml -p runx-cli --test integration verify -- --nocapture && cargo test --manifest-path crates/Cargo.toml -p runx-cli --test integration registry -- --nocapture && cargo test --manifest-path crates/Cargo.toml -p runx-cli --test integration skill -- --nocapture`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-16

## Phase 3: Human Results And Resume Commands

Status: completed
Dependencies: phase2

Objective: Turn human command output into useful operator guidance.

Changes:
- Render registry search result rows with skill id, version, digest/trust signal, and the canonical next action.
- When a skill run needs agent input, render a concrete copy-paste resume command using available skill ref, runner, receipt dir, run id, and answers file path.
- Render pending-run history entries with the same resume command when the recorded metadata is sufficient.
- Avoid absolute local paths in output unless the operator explicitly supplied them and no safer relative form exists.

Acceptance:
- [x] `ac8` command - Registry search human output names results
  - Command: `cargo test --manifest-path crates/Cargo.toml -p runx-cli --test integration registry_human_output_names_search_results -- --nocapture`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-21
- [x] `ac9` command - Skill text output includes a resume command
  - Command: `cargo test --manifest-path crates/Cargo.toml -p runx-cli --test integration native_skill_text_output_includes_copy_paste_resume_command -- --nocapture`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-22
- [x] `ac10` command - History pending run includes a resume command
  - Command: `cargo test --manifest-path crates/Cargo.toml -p runx-cli history_human_pending_run_includes_resume_command -- --nocapture`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-23

## Phase 4: Doctor Repair Hints

Status: completed
Dependencies: phase3

Objective: Make doctor diagnostics immediately actionable without automatic

Changes:
- Populate repair hints for authority diagnostics with exact missing env var names and safe command/docs pointers.
- Do the same for registry trust policy diagnostics introduced by `runx-registry-skill-trust-cutover-v1` if that spec has landed; otherwise keep this phase limited to existing diagnostics.
- Ensure repair hints never include secret values.

Acceptance:
- [x] `ac11` command - Doctor authority JSON warning diagnostics have repairs
  - Command: `crates/target/debug/runx doctor authority --json | jq -e 'all(.diagnostics[]; (.severity != "warning") or (.repairs | length > 0))'`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-28
- [x] `ac12` command - Doctor tests pass
  - Command: `cargo test --manifest-path crates/Cargo.toml -p runx-cli --test integration doctor -- --nocapture`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-29

## Phase 5: Fixture And Fast Verification

Status: completed
Dependencies: phase4

Objective: Prove the CLI output remains stable and documented.

Changes:
- Refresh help fixtures only for intentional UX changes.
- Update docs for nested help, JSON error mode, resume guidance, and doctor repair hints.

Acceptance:
- [x] `ac13` command - CLI help fixtures pass
  - Command: `pnpm fixtures:cli-help:check`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-39
- [x] `ac14` command - Fast verifier passes
  - Command: `pnpm verify:fast`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-40

## Harden Rounds

### round-1

Status: passed
Started: 2026-06-10T12:57:07Z
Ended: 2026-06-10T12:59:51Z

Observations:
- path
  - Result: clean
  - Anchor: code:crates/runx-cli/src/launcher.rs:149
- command
  - Result: clean
  - Anchor: code:package.json:56
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
  - Anchor: code:crates/runx-cli/src/main.rs:86

## Review

Status: completed
Verdict: pass
Mode: verify
Provider: codex
Output: codex.output_file
Summary: No completion-blocking issues found. The task-scoped changes satisfy the reviewed contract areas: nested help, JSON-mode failure envelopes for advertised paths, registry search human output, concrete resume commands, and doctor repair hints without secret leakage. Acceptance commands were not rerun because the review packet explicitly required read-only verification.

Attack log:
- `task scope`: Spec scope and diff classification -> clean (Reviewed task-scoped changes against the contract packet and separated ambient drift from files relevant to launcher, main, registry, skill, history, doctor, and CLI tests.)
- `crates/runx-cli/src/launcher.rs`: Command-local help routing -> clean (Checked launcher routing for nested help on skill, history, and verify. Help requests are intercepted before command parsing and main prints the command-local text.)
- `crates/runx-cli/src/main.rs and crates/runx-cli/src/{launcher,registry,skill}.rs`: JSON failure envelope routing -> clean (Checked registry/skill parse errors through JsonErrorPlan, registry runtime errors through json_failure_output, skill resolution/execution errors through skill_json_failure_output, and verify runtime/parse errors in main. stderr-clean JSON mode is preserved for reviewed paths.)
- `crates/runx-cli/src/registry.rs`: Registry human search output -> clean (Reviewed render_search and underlying RegistrySearchResult command fields. Human output includes id, version, digest, trust tier, install command, and run command.)
- `crates/runx-cli/src/{resume,skill/output,history}.rs`: Resume command rendering -> clean (Reviewed skill text output, history pending run rendering, and shared shell quoting. Commands include skill ref, runner, receipt dir when supplied, run id, and answers path/default. Raw operator-supplied paths are used where required, and default receipt dirs are omitted.)
- `crates/runx-cli/src/doctor.rs`: Doctor repair hints and secret safety -> clean (Reviewed authority and registry diagnostics. Repairs contain env var names and generic setup instructions; evidence avoids printing configured secret key material.)
- `crates/runx-runtime/src/journal.rs and crates/runx-cli/src/skill/resolver.rs`: Regression trace to runtime journal/provenance consumers -> clean (Checked paused run metadata extraction and registry provenance attachment paths to ensure new CLI rendering fields are backed by existing output structures and do not change trusted runtime semantics.)
- `acceptance commands`: Acceptance rerun -> skipped (Skipped by provider instruction: review mode is read-only and recorded acceptance evidence must be treated as already executed.)

Findings:
- none

