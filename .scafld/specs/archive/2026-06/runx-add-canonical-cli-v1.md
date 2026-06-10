---
spec_version: '2.0'
task_id: runx-add-canonical-cli-v1
created: '2026-06-10T15:53:53Z'
updated: '2026-06-10T16:27:53Z'
status: completed
harden_status: passed
size: small
risk_level: medium
---

# Canonical runx add CLI

## Current State

Status: completed
Current phase: final
Next: done
Reason: task completed
Blockers: none
Allowed follow-up command: `none`
Latest runner update: 2026-06-10T16:27:53Z
Review gate: pass

## Summary

Make `runx add <ref>` the canonical human-facing command for adding a skill
from a registry ref or indexing a GitHub skill repository, while preserving
`runx skill <ref>` as the direct-run path for registry-resolved skills.

This is a clean cutover, not a compatibility alias. The legacy wrapper shape
`runx skill add ...` must be removed from live behavior, help, tests, and
operator-facing output. The lower-level `runx registry install ...` primitive
may remain for registry administration and internal delegation, but human
operator hints should prefer `runx add ...`.

## Objectives

- Preserve direct registry execution:
  `runx skill owner/name@version --registry <url|path> ...` continues to
  resolve, verify, cache, and run without a prior install step.
- Introduce top-level `runx add <skill-ref>` for persistent skill install,
  reusing the existing Rust registry install implementation rather than
  duplicating install logic.
- Introduce top-level `runx add <github-repo-url>` for hosted GitHub repository
  indexing, reusing the existing URL indexing implementation rather than
  duplicating HTTP or response-shaping logic.
- Remove the legacy `runx skill add ...` live path. It must fail closed as an
  unsupported command shape instead of silently routing through install.
- Update help, user-facing hints, TypeScript wrapper parsing, and parity
  fixtures so the operator story is one clear shape:
  - `runx skill <ref>` = run now
  - `runx add <ref>` = add/install/index for later use
  - `runx registry ...` = lower-level registry administration

## Scope

- Rust CLI launcher routing, plans, help text, and tests under
  `crates/runx-cli/src/**` and `crates/runx-cli/tests/**`.
- TypeScript npm wrapper argument parsing, dispatch, presentation hints, and
  tests under `packages/cli/src/**`.
- Shared CLI parity fixtures under `fixtures/cli-parity/**` when generated
  snapshots require the new public command surface.
- User-facing docs/hints in `README.md`, `docs/**`, and URL indexing renderers
  when they advertise the old command.

Out of scope:
- Hosted registry API behavior.
- Registry trust/security semantics already covered by
  `runx-registry-skill-trust-cutover-v1`.
- Removing `runx registry install` as a low-level registry primitive.
- Adding compatibility aliases for `runx skill add`.

## Dependencies

- `runx-registry-skill-trust-cutover-v1` completed.
- `runx-cli-operator-ux-hardening-v2` completed.

## Assumptions

- `runx add` should be the operator command. `runx registry install` remains a
  lower-level primitive because registry commands still own registry
  administration, JSON envelopes, and internal acquisition/install plumbing.
- A GitHub URL add is an index/publish-to-registry action, not a local install.
  It should continue to reject install-only flags such as `--to` and `--digest`.
- For GitHub URL indexing, `--ref` is the git ref flag. The old wrapper use of
  `--version` for GitHub ref should not be carried forward into the new command.
- The TypeScript package remains a launcher/wrapper; trusted install and direct
  execution behavior stay in the Rust binary.

## Touchpoints

- `crates/runx-cli/src/launcher.rs`
- `crates/runx-cli/src/main.rs`
- `crates/runx-cli/src/registry.rs`
- `crates/runx-cli/src/url_add.rs`
- `crates/runx-cli/tests/launcher.rs`
- `crates/runx-cli/tests/registry.rs`
- `packages/cli/src/args.ts`
- `packages/cli/src/dispatch.ts`
- `packages/cli/src/commands/url-add.ts`
- `packages/cli/src/commands/url-add.test.ts`
- `packages/cli/src/index.test.ts`
- `packages/cli/src/help.ts`
- `packages/cli/src/skill-refs.ts`
- `README.md`
- `fixtures/cli-parity/**`

## Risks

- Public CLI surface change: removing `skill add` is intentional, but error
  messages must explain the new command.
- Duplicate command semantics: `runx add` must delegate to existing registry
  install/index code paths rather than implementing a second installer.
- Direct-run regression: parsing top-level `add` must not change
  `runx skill owner/name@version --registry ...`.
- Fixture churn: CLI help/parity snapshots may need regeneration, but generated
  files must reflect the canonical surface exactly.

## Acceptance

Profile: standard

Validation:
- `cargo fmt --all --manifest-path crates/Cargo.toml --check`
- `CARGO_TARGET_DIR=/tmp/runx-codex-target cargo check --manifest-path crates/Cargo.toml -p runx-cli`
- `CARGO_TARGET_DIR=/tmp/runx-codex-target cargo test --manifest-path crates/Cargo.toml -p runx-cli --test integration routes_add_to_native_plan -- --nocapture`
- `CARGO_TARGET_DIR=/tmp/runx-codex-target cargo test --manifest-path crates/Cargo.toml -p runx-cli --test integration rejects_legacy_skill_add_shape -- --nocapture`
- `pnpm vitest run packages/cli/src/index.test.ts packages/cli/src/commands/url-add.test.ts`
- `git diff --check`
- `CARGO_TARGET_DIR=/tmp/runx-codex-target pnpm verify:fast`

## Phase 1: Implementation

Status: completed
Dependencies: none

Objective: Introduce the canonical top-level add plan and remove live

Changes:
- Add a native `runx add` launcher branch that parses:
- The add branch must delegate to the existing native registry install or URL indexing implementation; do not duplicate install, trust, acquire, or HTTP logic.
- Update native help and tests so `runx add` is advertised and `runx skill add` is rejected with actionable guidance.
- Update the TypeScript wrapper to parse and dispatch top-level `add`; remove `skill add` support and update tests accordingly. This includes migrating the old `skillAction: "add"` / `isSkillAdd` / `skillRef` state so the wrapper cannot accidentally keep the stale command alive beside `runx add`.
- Keep native `runx url-add` only as an internal/hidden implementation path if needed for delegation. Do not advertise it in help or operator-facing hints once `runx add <github-url>` exists.
- Update user-facing renderers/hints to prefer `runx add`.

Acceptance:
- [x] `ac1` command - Rust formatting
  - Command: `cargo fmt --all --manifest-path crates/Cargo.toml --check`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-6
- [x] `ac2` command - Rust CLI check
  - Command: `CARGO_TARGET_DIR=/tmp/runx-codex-target cargo check --manifest-path crates/Cargo.toml -p runx-cli`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-7
- [x] `ac3` command - Native add routing regression
  - Command: `CARGO_TARGET_DIR=/tmp/runx-codex-target cargo test --manifest-path crates/Cargo.toml -p runx-cli --test integration routes_add_to_native_plan -- --nocapture`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-8
- [x] `ac4` command - Legacy skill-add rejection
  - Command: `CARGO_TARGET_DIR=/tmp/runx-codex-target cargo test --manifest-path crates/Cargo.toml -p runx-cli --test integration rejects_legacy_skill_add_shape -- --nocapture`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-9
- [x] `ac5` command - Wrapper add and URL rendering tests
  - Command: `pnpm vitest run packages/cli/src/index.test.ts packages/cli/src/commands/url-add.test.ts`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-10
- [x] `ac6` command - Diff hygiene
  - Command: `git diff --check`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-11
- [x] `ac7` command - Fast repo verification
  - Command: `CARGO_TARGET_DIR=/tmp/runx-codex-target pnpm verify:fast`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-12

## Rollback

- Revert the launcher/parser additions and restore the prior help/rendered
  hints. Because `runx add` delegates to existing registry/index code, rollback
  does not require data migration.

## Review

Status: completed
Verdict: pass
Mode: verify
Provider: codex
Output: codex.output_file
Summary: No completion-blocking issues found. The implementation adds top-level `runx add` routing, keeps registry install and URL indexing delegated to existing implementations, rejects legacy `runx skill add`, removes `url-add` from advertised help, and updates scoped hints/tests/fixtures to the canonical command shape. Recorded acceptance evidence covers formatting, Rust check/tests, targeted Vitest, diff hygiene, and fast verification.

Attack log:
- `review context`: initialization -> clean (Started read-only review of supplied context and task contract.)
- `README.md docs fixtures/cli-parity crates/runx-cli packages/cli/src`: stale command search -> clean (Searched task-scoped docs, fixtures, Rust CLI, and TypeScript wrapper for legacy `runx skill add`, hidden `url-add`, canonical `runx add`, and parser state names.)
- `crates/runx-cli/src/launcher.rs`: Rust launcher routing -> clean (Reviewed `plan_launcher`, `parse_add_plan`, legacy `skill add` rejection, JSON error planning, and help text changes.)
- `crates/runx-cli/src/main.rs crates/runx-cli/src/registry.rs crates/runx-cli/src/url_add.rs`: Rust execution path -> clean (Traced `LauncherAction::RunRegistry`, `RunUrlAdd`, and `JsonError` through `main.rs`, and checked registry install rendering for low-level primitive behavior.)
- `packages/cli/src/args.ts packages/cli/src/dispatch.ts packages/cli/src/skill-refs.ts packages/cli/src/help.ts`: TypeScript parser and dispatch -> clean (Reviewed top-level `add` parsing, retired `skill add` handling, GitHub URL indexing validation, registry install delegation to native `registry install`, and search result hint canonicalization.)
- `crates/runx-cli/tests packages/cli/src/index.test.ts packages/cli/src/commands/url-add.test.ts fixtures/cli-parity`: test and fixture coverage -> clean (Reviewed changed Rust tests, TypeScript tests, and CLI parity command fixture entries for coverage of the canonical command surface and legacy rejection.)
- `task scope and configured invariants`: scope and invariant check -> clean (Compared changed files against declared task scope and checked for duplicated install/HTTP logic, legacy fallback aliases, and user-facing old-command hints.)

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

Status: error
Started: 2026-06-10T15:55:36Z
Ended: 2026-06-10T15:55:36Z
Verdict: needs_revision
Provider: codex
Output format: codex.output_file
Summary: invalid provider dossier evidence: observation "command": invalid anchor prefix "/Users/kam/dev/runx/runx/oss/Cargo.toml" (expected "Anchor: spec_gap:<field>", "Anchor: code:<path>:<line>", or "Anchor: archive:<task-id>"); observation "design": invalid anchor prefix "/Users/kam/dev/runx/runx/oss/crates/runx-cli/src/launcher.rs:277" (expected "Anchor: spec_gap:<field>", "Anchor: code:<path>:<line>", or "Anchor: archive:<task-id>"); observation "scope": invalid anchor prefix "/Users/kam/dev/runx/runx/oss/packages/cli/src/args.ts:145" (expected "Anchor: spec_gap:<field>", "Anchor: code:<path>:<line>", or "Anchor: archive:<task-id>"); observation "path": invalid anchor prefix "/Users/kam/dev/runx/runx/oss/packages/cli/src/commands/url-add.ts:128" (expected "Anchor: spec_gap:<field>", "Anchor: code:<path>:<line>", or "Anchor: archive:<task-id>"); observation "timing": invalid anchor prefix "/Users/kam/dev/runx/runx/oss/crates/runx-cli/tests/integration.rs" (expected "Anchor: spec_gap:<field>", "Anchor: code:<path>:<line>", or "Anchor: archive:<task-id>"); observation "rollback": invalid anchor prefix "/Users/kam/dev/runx/runx/oss/.scafld/specs/drafts/runx-add-canonical-cli-v1.md:186" (expected "Anchor: spec_gap:<field>", "Anchor: code:<path>:<line>", or "Anchor: archive:<task-id>")

Observations:
- command
  - Result: blocks
  - Anchor: /Users/kam/dev/runx/runx/oss/Cargo.toml
  - Note: Blocking harden question: `cargo fmt --all` is listed as ac1, but there is no repo-root Cargo.toml; the Rust workspace manifest is `crates/Cargo.toml`. Recommended answer: change ac1 to `cargo fmt --all --manifest-path crates/Cargo.toml`. If unanswered: the build agent should use the manifest-qualified command and record the deviation.
  - Status: open
- design
  - Result: advisory
  - Anchor: /Users/kam/dev/runx/runx/oss/crates/runx-cli/src/launcher.rs:277
  - Note: Current native URL indexing is `runx url-add`, while registry install parsing lives under `runx registry install`. Hardening question: should native `runx add <registry-ref>` synthesize/delegate to an existing `RegistryPlan`, and should `runx add <github-url>` reuse `UrlAddPlan` while retiring public `url-add` help? Recommended answer: yes, use existing plan types/runners and hide/replace public operator hints for `url-add` with `add`.
- scope
  - Result: advisory
  - Anchor: /Users/kam/dev/runx/runx/oss/packages/cli/src/args.ts:145
  - Note: The TypeScript parser has a live `skill add` branch, and dispatch routes it to URL indexing or native registry install. The spec is scoped correctly, but implementation must remove this branch rather than layering `add` beside it, or the clean cutover invariant is not met.
- path
  - Result: clean
  - Anchor: /Users/kam/dev/runx/runx/oss/packages/cli/src/commands/url-add.ts:128
  - Note: Verified current operator-facing hints still emit `runx registry install`; matching old hints also appear in native URL rendering, bundled skill refs, README, tests, and cli-parity fixtures. Declared touchpoints cover these paths.
- timing
  - Result: advisory
  - Anchor: /Users/kam/dev/runx/runx/oss/crates/runx-cli/tests/integration.rs
  - Note: Acceptance filters `routes_add_to_native_plan` and `rejects_legacy_skill_add_shape` are future exact test names; they do not currently exist. That is acceptable if phase 1 creates those integration tests with exact names before running ac3/ac4.
- rollback
  - Result: clean
  - Anchor: /Users/kam/dev/runx/runx/oss/.scafld/specs/drafts/runx-add-canonical-cli-v1.md:186
  - Note: Rollback is credible only if `runx add` remains a routing/parser/help/test change that delegates to existing install/index code. If implementation introduces new cache layout, install metadata, or registry wire behavior, the rollback section must be expanded.

### round-2

Status: error
Started: 2026-06-10T15:57:14Z
Ended: 2026-06-10T15:57:14Z
Verdict: needs_revision
Provider: claude
Model: claude-opus-4-7
Output format: claude.mcp_submit_harden
Summary: invalid provider dossier evidence: observation "command": spec_gap citation does not name a known spec field: spec_gap:phases.phase1.acceptance.ac1

Observations:
- path
  - Result: clean
  - Anchor: code:packages/cli/src/dispatch.ts:212
  - Note: All declared touchpoints exist: crates/runx-cli/src/{launcher.rs,url_add.rs,registry.rs}, crates/runx-cli/tests/{launcher.rs,integration.rs,registry.rs}, packages/cli/src/{args.ts,dispatch.ts,help.ts,skill-refs.ts,index.test.ts,commands/url-add.ts,commands/url-add.test.ts}, README.md, fixtures/cli-parity/. The current live skill-add dispatch is at packages/cli/src/dispatch.ts:212 (GitHub URL branch) and :239 (registry install branch).
- command
  - Result: blocks
  - Anchor: spec_gap:phases.phase1.acceptance.ac1
  - Note: ac1 `cargo fmt --all` runs from the oss/ root, which has no workspace Cargo.toml; the Rust workspace manifest is at crates/Cargo.toml. ac2/ac3/ac4/ac7 already use `--manifest-path crates/Cargo.toml`. Round-1 raised this and the draft is unchanged. Recommended: change ac1 to `cargo fmt --all --manifest-path crates/Cargo.toml --check` (matching prior specs such as runx-rust-95-release-readiness:168) or `cd crates && cargo fmt --all --check`.
  - Default: Default to `cargo fmt --all --manifest-path crates/Cargo.toml --check`.
  - Status: open
- scope
  - Result: advisory
  - Anchor: code:packages/cli/src/index.test.ts:1009
  - Note: Clean cutover requires more than removing the args.ts `skillAction === "add"` branches: the existing rejection test at index.test.ts:1006-1023 covers `["add", "acme/sourcey@1.0.0"]` as a retired alias and asserts exit code 64 with `Usage:` on stderr. After cutover that case must move from the rejection loop to a success case, and the `skillAction` literal `"add"` plus the `skillRef`/`installVersion`/`installTo` ParsedArgs fields that today key off `isSkillAdd` need an explicit removal/migration note in the spec to prevent a partial cutover.
- timing
  - Result: advisory
  - Anchor: code:crates/runx-cli/tests/launcher.rs:1
  - Note: ac3 `routes_add_to_native_plan` and ac4 `rejects_legacy_skill_add_shape` are exact test filters that do not exist in crates/runx-cli/tests/launcher.rs or any other module in tests/integration.rs today. That is acceptable only if phase 1 creates those tests under the `integration` binary with those exact names before running ac3/ac4. Recommend the spec note the target module (likely `launcher` mod) so the build agent does not invent a separate test binary.
  - Default: Default to adding the new tests inside crates/runx-cli/tests/launcher.rs so they ride the consolidated `integration` binary.
- rollback
  - Result: clean
  - Anchor: spec_gap:rollback
  - Note: Rollback remains credible because the cutover is parser/help/test/renderer-only and delegates to the existing RegistryPlan (install) and UrlAddPlan (index) runners. No cache layout, install metadata, or wire shape change is introduced. If implementation drifts beyond delegation (e.g. introduces a new add-specific plan struct or persisted state), the rollback section must be expanded; flag during build review.
- design
  - Result: advisory
  - Anchor: code:crates/runx-cli/src/launcher.rs:277
  - Note: Native CLI currently exposes `runx url-add` (launcher.rs:277, advertised in help_text at launcher.rs:320). Objectives say `runx add <ref>` is the one canonical add surface, but Scope/Out-of-scope only retire `runx skill add` and explicitly preserve `runx registry install`. The spec is silent on whether public `runx url-add` becomes hidden, aliased, or removed once `runx add <github-url>` ships. Recommended answer: keep `runx url-add` as an internal/hidden command (reused by `runx add` delegation) but drop it from `help_text()` and operator-facing hints, mirroring the `runx registry install` treatment.
  - Default: Default to hiding `runx url-add` from help and operator hints while keeping it as the underlying plan that `runx add <github-url>` delegates to.

### round-3

Status: passed
Started: 2026-06-10T16:00:57Z
Ended: 2026-06-10T16:00:57Z
Verdict: pass
Provider: claude
Model: claude-opus-4-7
Output format: claude.mcp_submit_harden
Summary: Round-3 review of runx-add-canonical-cli-v1. Verified the round-1/2 corrections landed: ac1 now uses `cargo fmt --all --manifest-path crates/Cargo.toml --check`, the spec calls out the wrapper `skillAction:"add"`/`isSkillAdd`/`skillRef` migration explicitly, and design explicitly hides `runx url-add` from operator help while keeping it as the delegation target. All declared touchpoints exist; the live cutover sites (dispatch.ts:212/239 GitHub vs install branches, args.ts:145 derivation, url-add.ts:128 renderer hint, skill-refs.ts:355 bundled add_command, index.test.ts:1009 rejection case) are reachable from the listed paths. Rollback remains credible because the plan is parser/help/test/renderer-only and delegates to existing RegistryPlan and UrlAddPlan runners. Two advisories remain: (1) ac3/ac4 reference future exact test filters `routes_add_to_native_plan` and `rejects_legacy_skill_add_shape` that do not yet exist, and the spec should pin them to crates/runx-cli/tests/launcher.rs so they ride the consolidated `integration` binary rather than spawning a new test binary; (2) the wrapper-only `--version` flag for GitHub URL adds becomes `--ref`, which is a deliberate but breaking change that operator-facing rejection text should explain. No blocking observations remain.

Observations:
- path
  - Result: clean
  - Anchor: code:packages/cli/src/dispatch.ts:212
  - Note: Verified every declared touchpoint exists and contains the live cutover sites: crates/runx-cli/src/launcher.rs (UrlAddPlan at :55, url-add help line at :320, branch at :277), crates/runx-cli/src/{main.rs,registry.rs,url_add.rs}, crates/runx-cli/tests/{launcher.rs,integration.rs,registry.rs}, packages/cli/src/args.ts (isSkillAdd at :145), packages/cli/src/dispatch.ts (GitHub branch :212, registry install branch :239), packages/cli/src/commands/url-add.ts (operator hint at :128), packages/cli/src/skill-refs.ts (bundled add_command at :355), packages/cli/src/index.test.ts (rejection list at :1006-1023 including the `add` alias at :1009), README.md, and fixtures/cli-parity/. No declared touchpoint is missing.
- command
  - Result: clean
  - Anchor: code:.scafld/specs/drafts/runx-add-canonical-cli-v1.md:125
  - Note: ac1 is now `cargo fmt --all --manifest-path crates/Cargo.toml --check`, matching the workspace manifest at crates/Cargo.toml (the round-1/2 blocker is resolved). ac2/ac3/ac4/ac7 also use `--manifest-path crates/Cargo.toml`. ac5 (`pnpm vitest run packages/cli/src/index.test.ts packages/cli/src/commands/url-add.test.ts`), ac6 (`git diff --check`), and ac7 (`pnpm verify:fast`) are runnable from oss/ as currently scripted.
  - Status: fixed
- scope
  - Result: clean
  - Anchor: code:packages/cli/src/args.ts:145
  - Note: Scope correctly retires `runx skill add` and explicitly preserves `runx registry install` as a low-level primitive. Phase 1 changes now name the wrapper migration of `skillAction:"add"`/`isSkillAdd`/`skillRef`/`installVersion`/`installTo` so the ParsedArgs lineage cannot keep a stale parallel path. Operator-facing renderer in skill-refs.ts:355 (`runx registry install runx/${name}`) and url-add.ts:128 are in-scope via the listed touchpoints. docs/orchestrator-integrations.md:54 also mentions `runx registry install` but in a low-level registry primitive context that the spec explicitly keeps, so it does not require this spec's cutover.
- timing
  - Result: advisory
  - Anchor: code:crates/runx-cli/tests/integration.rs:13
  - Note: ac3 `routes_add_to_native_plan` and ac4 `rejects_legacy_skill_add_shape` are exact `--test integration <filter>` names that do not yet exist anywhere in crates/runx-cli/tests/. That is acceptable only because phase 1 creates them, but the spec does not pin the host module. Recommend the spec explicitly say the new tests live in crates/runx-cli/tests/launcher.rs so they ride the consolidated `integration` binary at tests/integration.rs rather than tempting a separate test binary. If unanswered: default to adding both tests inside crates/runx-cli/tests/launcher.rs alongside `top_level_help_and_version_are_native`.
  - Default: Add the new tests inside crates/runx-cli/tests/launcher.rs so the consolidated integration binary picks them up via `--test integration --filter <name>`.
  - Status: open
- rollback
  - Result: clean
  - Anchor: code:.scafld/specs/drafts/runx-add-canonical-cli-v1.md:189
  - Note: Rollback is credible because the cutover is strictly parser/help/test/renderer and delegates to existing RegistryPlan (install) and UrlAddPlan (index) runners. No new cache layout, install metadata, journal entry, or registry wire shape is introduced. If implementation drifts beyond delegation (e.g. introduces a dedicated AddPlan struct or persists separate state), the rollback section must be expanded; flag during review.
- design
  - Result: advisory
  - Anchor: code:.scafld/specs/drafts/runx-add-canonical-cli-v1.md:85
  - Note: Round-2 design concern about `runx url-add` is resolved: spec now keeps it as an internal/hidden delegation target and drops it from help (launcher.rs:320) and operator-facing hints. One unresolved design nuance: the spec switches GitHub URL adds from the wrapper's `--version` flag to `--ref`, which is the right semantic but a breaking change for anyone scripting today's `runx skill add <github-url> --version <ref>`. Recommend phase 1 explicitly translate `--version` on a GitHub URL into an actionable error message that names `--ref`, rather than the generic unknown-flag/unsupported-command output. If unanswered: default to a single dedicated rejection branch that detects `--version` with a GitHub URL and emits guidance pointing at `runx add <github-url> --ref <git-ref>`.
  - Default: Detect `--version` on a GitHub URL `runx add` invocation and reject with a message pointing at `--ref`, so existing `runx skill add <github-url> --version` scripts get a clear migration hint.
  - Status: open


## Planning Log

- none
