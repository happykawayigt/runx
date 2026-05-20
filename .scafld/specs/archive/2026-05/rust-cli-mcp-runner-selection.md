---
spec_version: '2.0'
task_id: rust-cli-mcp-runner-selection
created: '2026-05-20T08:08:55Z'
updated: '2026-05-20T08:18:49Z'
status: completed
harden_status: not_run
size: small
risk_level: medium
---

# Rust CLI MCP runner selection

## Current State

Status: completed
Current phase: final
Next: done
Reason: task completed
Blockers: none
Allowed follow-up command: `none`
Latest runner update: 2026-05-20T08:18:49Z
Review gate: pass

## Summary

Fail closed for runx mcp serve --runner in native Rust CLI mode so runner selection cannot silently fall back to JS.

## Objectives

- Route `runx mcp serve --runner ...` through the native Rust MCP path when
  `RUNX_RUST_CLI` is enabled.
- Preserve fail-closed behavior until native runner selection is implemented:
  requested runners must produce an explicit native error, not silent JS
  delegation.
- Keep the change scoped to `crates/runx-cli` MCP/launcher files and focused
  tests.

## Scope

In scope:
- `crates/runx-cli/src/mcp.rs`
- `crates/runx-cli/src/launcher.rs`
- `crates/runx-cli/tests/launcher.rs`

Out of scope:
- Full CLI cutover.
- Runtime target runner or post-merge observer files.
- Broad runtime exports or legacy/v2 compatibility surfaces.

## Dependencies

- none

## Assumptions

- none

## Touchpoints

- none

## Risks

- none

## Acceptance

Profile: standard

Validation:
- `cargo test --manifest-path crates/Cargo.toml -p runx-cli launcher mcp`
- `cargo test --manifest-path crates/Cargo.toml -p runx-runtime mcp_server --features mcp -- --nocapture`

## Phase 1: Implementation

Status: completed
Dependencies: none

Objective: Complete the requested change.

Changes:
- Added `runner` to the native MCP plan and parser.
- Routed native `mcp serve` through the Rust parser for supported subcommand shapes so `--runner` and unknown serve flags do not silently delegate.
- Added launcher and binary-level tests proving `--runner` fails in native runtime with an explicit unsupported-runner error even when JS fallback env is configured.
- Added a guard for non-canonical `runx mcp --runner=... serve ...` ordering so it fails in native mode instead of delegating.

Acceptance:
- [x] `ac1` command - Rust CLI launcher/MCP focused tests
  - Command: `cargo test --manifest-path crates/Cargo.toml -p runx-cli launcher`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-6
- [x] `ac1b` command - Rust CLI MCP focused tests
  - Command: `cargo test --manifest-path crates/Cargo.toml -p runx-cli mcp`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-7
- [x] `ac2` command - Runtime MCP server focused tests
  - Command: `cargo test --manifest-path crates/Cargo.toml -p runx-runtime mcp_server --features mcp -- --nocapture`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-8

## Rollback

- none

## Review

Status: completed
Verdict: pass
Mode: discover
Provider: claude:claude-opus-4-7
Output: claude.mcp_submit_review
Summary: Native mcp serve --runner routing is implemented and fails closed. parse_mcp_plan captures --runner into McpPlan; run_native_mcp forwards it to McpServerExecutionOptions; from_skill_paths_with_execution rejects any Some(runner) with UnsupportedRunnerSelection before loading skills. Integration test exercises the compiled binary with RUNX_JS_BIN+RUNX_NPM_PACKAGE set and proves stderr contains the explicit native error and exit code is 1. Unknown serve flags also surface as native errors via parser. Empty/zero RUNX_RUST_CLI still delegates. Acceptance commands recorded as exit-zero. One non-blocking observation: mcp_subcommand_is_native gates on args[1]=="serve", so the non-canonical form `runx mcp --runner=x serve` is delegated to JS; spec scopes the fail-closed objective to the canonical `mcp serve --runner ...` shape, so this is acceptable but worth noting if flag ordering parity becomes a goal.

Attack log:
- `crates/runx-cli/src/mcp.rs::parse_mcp_plan`: Verify --runner=<value> and --runner <value> both capture into McpPlan.runner and unknown serve flags error instead of falling through -> clean (Both inline and space-separated values handled by flag_value; unknown flags return Err per `unknown mcp serve flag {flag}`. Tests rust_cli_signal_routes_mcp_runner_selection_to_native_fail_closed_plan and rust_cli_signal_rejects_unknown_mcp_serve_flags_instead_of_delegating cover this.)
- `crates/runx-runtime/src/adapters/mcp.rs::from_skill_paths_with_execution`: Confirm runner check fires before skill loading so invalid skill paths cannot mask the fail-closed error -> clean (Line 156: `if let Some(runner) = &execution.runner` returns UnsupportedRunnerSelection prior to any load_mcp_server_tool calls.)
- `crates/runx-cli/tests/launcher.rs::native_mcp_runner_selection_fails_closed_without_js_fallback`: Confirm the binary-level test exercises real fail-closed behavior with JS fallback env configured (not just plan-level) -> clean (Test sets RUNX_RUST_CLI=1, RUNX_JS_BIN=/repo/oss/packages/cli/bin/runx.js, RUNX_NPM_PACKAGE=@runxhq/cli@0.5.22 and runs the cargo bin against fixtures/skills/mcp-echo. Asserts exit_code(1) and stderr contains 'runner selection 'default' is not supported by the native runtime yet'. Fixture exists at oss/fixtures/skills/mcp-echo/SKILL.md.)
- `crates/runx-cli/src/launcher.rs::native_signal_requested`: Empty or '0' RUNX_RUST_CLI should not trigger native dispatch and must continue delegating to JS for mcp serve --runner -> clean (rust_cli_zero_signal_still_delegates_to_js and rust_cli_empty_signal_still_delegates_to_js confirm the gate. The native_signal_requested helper filters empty and "0".)
- `crates/runx-cli/src/launcher.rs::mcp_subcommand_is_native`: Probe flag-ordering: `runx mcp --runner=x serve ...` and `runx mcp <not-serve> --runner=x` -> finding (Gate requires args[1] == 'serve'. Non-canonical orderings bypass native parsing and delegate to JS with --runner in argv. Documented as F1-flag-ordering-edge (low, non-blocking).)
- `crates/runx-cli/src/mcp.rs::McpPlan`: Public API addition of `runner: Option<String>` field — verify it does not break existing struct construction in tests/binary call sites -> clean (All five touchpoints (run_native_mcp, parse_mcp_plan, two launcher tests, and rust_cli_signal_routes_mcp_serve_without_runner_to_native_lifecycle) construct McpPlan with `runner` set explicitly. Additive struct field; no construction site missing the field.)
- `crates/runx-cli/src/main.rs::LauncherAction::RunMcp`: Ensure run_native_mcp surfaces RuntimeError::UnsupportedRunnerSelection with non-zero exit and prefixed stderr line -> clean (run_native_mcp writes `runx: {error}` via writeln!(stderr) then returns ExitCode::from(1). thiserror format string is 'runner selection \'{runner}\' is not supported by the native runtime yet'.)
- `crates/runx-cli/src/mcp.rs::parse_mcp_plan empty runner value`: Try --runner= and --runner '' — should still fail closed, not silently treat as None -> clean (flag_value sets value to '' (or owned empty string) and runner becomes Some(""); from_skill_paths_with_execution's `if let Some(runner)` is true for empty strings and returns the unsupported error.)

Findings:
- [resolved] `F1-flag-ordering-edge` Non-canonical flag ordering `runx mcp --runner=x serve ...` bypasses native fail-closed gate
  - Location: `crates/runx-cli/src/launcher.rs:604`
  - Evidence: mcp_subcommand_is_native returns true only when args[1] == "serve". If `--runner` is placed before the `serve` subcommand (e.g. `runx mcp --runner=default serve fixtures/skills/echo` with RUNX_RUST_CLI=1), args[1] is `--runner=default`, the native dispatch is skipped, and delegate_plan forwards the original argv (including --runner) to the JS CLI. The spec objective says requested runners must produce an explicit native error, not silent JS delegation, but the canonical shape it cites is `runx mcp serve --runner ...`.
  - Impact: If the JS CLI happens to accept flags before its subcommand, a user could still smuggle --runner into JS execution by reordering. With commander's default positional handling this is uncommon, so the practical leak is small.
  - Validation: Add a launcher unit test that passes ["mcp", "--runner=default", "serve", "fixtures/skills/echo"] with rust_cli=Some("1") and asserts the action is LauncherAction::Error or LauncherAction::RunMcp (with runner=Some(...)), not Delegate.
  - Resolution: Added `mcp_args_request_runner_selection` and `rust_cli_signal_rejects_mcp_runner_before_serve_instead_of_delegating`; reran `cargo test --manifest-path crates/Cargo.toml -p runx-cli mcp` and `cargo test --manifest-path crates/Cargo.toml -p runx-cli launcher` successfully.

## Self Eval

- none

## Deviations

- Suggested command `cargo test --manifest-path crates/Cargo.toml -p runx-cli
  launcher mcp` is not valid Cargo syntax because Cargo accepts only one test
  filter. Ran `launcher` and `mcp` filters separately.
- Targeted clippy was attempted with `cargo clippy --manifest-path
  crates/Cargo.toml -p runx-cli --all-targets -- -D warnings`; it failed on
  pre-existing runtime warnings outside this slice:
  `runx-runtime/src/harness/runner.rs:417` and
  `runx-runtime/src/adapters/mcp.rs:1089`.

## Metadata

- created_by: scafld

## Origin

Created by: scafld
Source: plan

## Harden Rounds

- none

## Planning Log

- none
