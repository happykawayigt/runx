---
spec_version: '2.0'
task_id: runx-graph-skill-issue-to-pr
created: '2026-05-25T07:52:32Z'
updated: '2026-05-26T04:52:05Z'
status: completed
harden_status: passed
size: medium
risk_level: high
---

# Runx graph skill issue-to-pr execution

## Current State

Status: completed
Current phase: final
Next: done
Reason: task completed
Blockers: none
Allowed follow-up command: `none`
Latest runner update: 2026-05-26T04:52:05Z
Review gate: pass

## Summary

Make the Rust `runx skill` surface capable of executing the graph-backed
`issue-to-pr` lane used by Nitrosend live issue intake. The current production
dogfood issue reaches GitHub Actions and then fails before PR creation because
the Rust skill runner rejects `source.type: graph`. This work brings the graph
runner through the same caller-mediated agent loop and deterministic tool
boundaries expected by the latest runx contracts.

## Objectives

- Execute `skills/issue-to-pr` through the Rust `runx skill` command instead
  of failing on `source.type: graph`.
- Preserve caller-mediated `needs_agent` / `--run-id` / `--answers`
  continuation semantics for graph `run.type: agent-step` nodes.
- Execute graph `tool:` nodes through local checked-in tool manifests under the
  Rust runtime sandbox instead of bypassing tool contracts.
- Keep the human merge gate intact: runx may create/update the draft PR but
  must not merge it.
- Prove the shape against the live Nitrosend Slack-origin issue-to-PR flow from
  production credentials.

## Scope

- In scope:
  - `crates/runx-runtime/src/execution/skill_run.rs`
  - `crates/runx-runtime/src/execution/runner.rs`
  - `crates/runx-runtime/src/execution/runner/steps.rs`
  - focused runtime/helper tests under `crates/runx-runtime/tests/`
  - minimal docs/spec updates needed to keep the Rust rewrite story honest
  - Nitrosend workflow pin/config changes only if needed after the upstream
    runx fix is committed
- Out of scope:
  - automatic PR merge
  - broad TypeScript runtime-local rewrites
  - unrelated runx naming or policy refactors
  - provider-specific Nitrosend routing changes unless live dogfood exposes a
    blocker in the existing path

## Dependencies

- scafld 2.4.x is available from `/Users/kam/dev/scafld/bin/scafld`.
- Live Nitrosend credentials are available only from the production Docker
  container and GitHub Actions secrets, not from local shell assumptions.
- The existing `issue-to-pr` graph and tool manifests remain the canonical
  first-party lane contract.

## Assumptions

- It is acceptable for the Rust runtime to supervise external `cli-tool`
  implementations for local tools; the security boundary is the manifest,
  sandbox, env allowlist, and receipt, not a hidden TypeScript runtime fallback.
- Graph continuation state may be stored under the normal runx receipt/run
  evidence area keyed by `run_id`.
- Nitrosend live issue intake should keep publishing Slack follow-ups only as
  replies to the original trigger thread.

## Touchpoints

- Rust skill runner dispatch for graph-backed `X.yaml` runners.
- Graph step execution for inline `agent-step` run nodes.
- Graph step execution for local `tool:` nodes such as
  `fs.write_bundle`, `git.current_branch`, `outbox.build_pull_request`, and
  `thread.push_outbox`.
- GitHub Actions issue-intake workflow pin to the fixed runx commit.
- Production Slack/GitHub dogfood issue that must produce a draft PR.

## Risks

- Re-running mutation steps around agent continuations could duplicate specs,
  branches, comments, or PRs. Continuation must resume from persisted state.
- Tool execution must not inherit ambient secrets beyond each tool manifest's
  sandbox/env allowlist.
- A graph failure must fail closed with a clear status comment, not create a
  partial PR without reviewer context.
- Building JS tool artifacts in CI must not become an implicit trusted runtime
  fallback; Rust still owns graph orchestration and receipts.

## Acceptance

Profile: standard

Validation:
- `cargo test --manifest-path crates/Cargo.toml -p runx-runtime --features cli-tool,catalog --test skill_run`
- `cargo test --manifest-path crates/Cargo.toml -p runx-runtime --features cli-tool,catalog --test hello_graph`
- `./crates/target/debug/runx doctor skills/issue-to-pr --json`
- Nitrosend wrapper tests with `RUNX_BIN` pointing at the fixed Rust binary
- Live Nitrosend issue intake workflow dispatch for the Slack-origin Stripe
  checkout bug creates a GitHub issue update and draft PR

## Phase 1: Implementation

Status: completed
Dependencies: none

Objective: Add graph-backed skill execution to the Rust `runx skill` path.

Changes:
- Dispatch graph-source skill runners to the Rust graph runtime.
- Persist and reload graph continuation state for caller-mediated agent-step answers.
- Add native graph step support for inline `agent-step` and local tool manifests.
- Execute graph child skills through the Rust adapter without falling back to fixture catalogs.
- Package final graph outputs so callers can find draft PR and source-thread publication results.

Acceptance:
- [x] `ac1` command - Runtime skill tests
  - Command: `cargo test --manifest-path crates/Cargo.toml -p runx-runtime --features cli-tool,catalog --test skill_run`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-7
- [x] `ac2` command - Graph runtime smoke tests
  - Command: `cargo test --manifest-path crates/Cargo.toml -p runx-runtime --features cli-tool,catalog --test hello_graph`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-8
- [x] `ac3` command - Issue-to-PR manifest validation
  - Command: `cargo run --manifest-path crates/Cargo.toml -p runx-cli -- doctor skills/issue-to-pr --json`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-9
- [x] `ac4` manual - Live Nitrosend dogfood
  - Command: `gh workflow run issue-intake.yml --repo nitrosend/nitrosend --ref runx-dogfood-pin-5c73651 -f issue_number=187 -f publish=true -f source_repo=nitrosend/nitrosend`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-10

## Rollback

- Revert the runx runtime commit and restore Nitrosend `RUNX_REF` to the last
  known-good pin.
- Rerun the issue-intake workflow only after the prior pin is restored; do not
  retry production Slack-origin issues against a known failing graph runner.

## Review

Status: completed
Verdict: pass
Mode: verify
Provider: claude:claude-opus-4-7
Output: claude.mcp_submit_review
Summary: Human-reviewed override accepted: Claude re-review confirmed f1/f2/f3 fixed and found no code blockers; only failure was workspace-mutation guard while concurrent work was active.

Attack log:
- `review gate`: manual human audit -> clean (Claude re-review confirmed f1/f2/f3 fixed and found no code blockers; only failure was workspace-mutation guard while concurrent work was active.)

Findings:
- none

## Self Eval

- none

## Deviations

- none

## Metadata

- created_by: scafld

## Origin

Created by: scafld
Source: plan

## Harden Rounds

### round-1

Status: passed
Started: 2026-05-25T07:53:31Z
Ended: 2026-05-25T07:54:17Z

Checks:
- path audit
  - Grounded in: code:crates/runx-runtime/src/execution/skill_run.rs:45
  - Result: passed
  - Evidence: The blocker is in `execute_skill_run` / `runner_invocation`,
- command audit
  - Grounded in: spec_gap:acceptance
  - Result: passed
  - Evidence: Acceptance is tied to focused Rust runtime tests, the existing
- scope/migration audit
  - Grounded in: code:crates/runx-runtime/src/execution/runner/steps.rs:47
  - Result: passed
  - Evidence: The graph step execution boundary already centralizes skill,
- acceptance timing audit
  - Grounded in: code:crates/runx-runtime/src/execution/runner.rs:104
  - Result: passed
  - Evidence: `GraphCheckpoint` already exists at the runtime boundary, so the
- rollback/repair audit
  - Grounded in: spec_gap:rollback
  - Result: passed
  - Evidence: Rollback is a normal commit revert plus restoring Nitrosend's
- design challenge
  - Grounded in: code:crates/runx-runtime/src/adapters/catalog.rs:63
  - Result: passed
  - Evidence: Reusing manifest-backed `cli-tool` tool execution keeps Rust as

Issues:
- none


## Planning Log

- none
