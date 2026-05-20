---
spec_version: '2.0'
task_id: rust-cli-rust-cutover
created: '2026-05-18T00:00:00Z'
updated: '2026-05-20T00:00:00Z'
status: draft
harden_status: blocked
size: extra_large
risk_level: very_high
---

# Rust CLI hard cutover

## Current State

Status: draft
Current phase: implementation gap review
Next: implement remaining native CLI and packaging gates before approval
Reason: hard cutover contract for moving the launcher/CLI boundary to Rust.
Blockers: native Rust candidate dispatch exists for several command slices, but
release authority still remains with the npm/TypeScript CLI. The Rust launcher
still delegates by default through `npm exec` or `RUNX_JS_BIN`; native CLI
dispatch is only reachable through `RUNX_RUST_CLI` and native harness replay is
only reachable through `RUNX_RUST_HARNESS`. The npm package still points
`bin.runx` at `./bin/runx.js`, ships `dist`/`tools`, and depends on TS
workspace packages instead of platform native binary packages. Cutover-specific
package/release verifier scripts named below do not exist yet. Do not approve
until the release launcher, platform package pointers, full canonical Rust
command surface, and no-JS-fallback gates are implemented and passing.
Allowed follow-up command: implement missing blockers, then run validation;
do not run `scafld review rust-cli-rust-cutover` until this state changes.
Latest runner update: 2026-05-20T00:00:00Z
Review gate: not_started

## Summary

Flip the npm `@runxhq/cli` package from a Node CLI to a thin platform-aware
native launcher that downloads, verifies, and executes the bundled Rust binary.
Today `crates/runx-cli/src/launcher.rs` still delegates to TypeScript through
`npm exec` or invokes a local `node` against a `RUNX_JS_BIN` path unless
explicit candidate signals are set. `RUNX_RUST_CLI` currently routes native
candidate implementations for `connect`, `config`, supported `doctor`, supported
`list`, `new`, `init`, `history`, `policy inspect|lint`, and
`tool build|search|inspect`, plus supported `mcp serve` shapes without native
runner selection;
`RUNX_RUST_HARNESS` separately routes `runx harness <fixture>` to the Rust
runtime harness runner. This spec removes release-path delegation and makes the
Rust binary the authoritative `runx` invocation.

This is a hard cutover, not a compatibility bridge. The public CLI exposes
simple canonical verbs implemented by Rust. The npm launcher must not parse,
rewrite, alias, upgrade, downgrade, or translate command shapes. Rust owns CLI
parsing, presentation, dispatch, harness execution, receipt creation, receipt
storage, and exit-code mapping through `runx-runtime`.

## Context

CWD: `.`

Packages:
- `@runxhq/cli` (npm package)
- `crates/runx-cli` (native binary)
- `crates/runx-runtime`
- `cloud/packages/api` (release distribution; see existing
  `registry-release-distribution-hardening` draft)

Current TypeScript sources:
- `packages/cli/src/index.ts`
- `packages/cli/src/dispatch.ts`
- `crates/runx-cli/src/launcher.rs` (today: candidate native router plus
  delegate-to-TS launcher)
- `crates/runx-cli/src/main.rs`

Files impacted:
- `crates/runx-cli/src/main.rs` (full native CLI parser and dispatch)
- `crates/runx-cli/src/commands/**` (one module per command)
- `crates/runx-cli/src/presentation/**`
- `packages/cli/bin/runx.js` (shrinks to: detect platform, download binary,
  exec; or even-thinner postinstall download)
- `packages/cli/package.json` (optional platform-specific dependencies for
  the binary, or postinstall script)
- `scripts/package-rust-cli.ts`, `scripts/release-rust-cli.ts`, and
  `scripts/check-rust-cli-release-artifacts.ts` (new: package the Rust binary
  per platform, publish to the binary CDN, verify artifacts, bump npm)
- `crates/runx-cli/build.rs` (if needed for embedding version metadata)

Invariants:
- The npm package still installs `runx` on every platform that TS supports
  today: macOS (x86_64, aarch64), Linux (x86_64, aarch64), Windows
  (x86_64). Other platforms must be explicitly documented as unsupported
  before this cutover lands.
- Every canonical user action currently accepted for the cutover has exactly
  one Rust-owned command spelling. Do not preserve historical aliases, hidden
  admin forms, or nested/top-level synonyms for compatibility.
- Existing consumers must migrate to the canonical simple verbs before this
  spec lands. The launcher must not keep legacy alias rewrites to make old
  scripts appear compatible.
- Every command currently in `dispatch.ts` that survives the canonical-verb
  decision (see `plans/rust-takeover.md` section 4 matrix and the completed
  `rust-cli-feature-parity-matrix`) has a Rust implementation with an oracle
  case or an explicit removal/migration note.
- The Rust CLI must not be treated as authoritative merely because native
  command dispatch exists in `crates/runx-cli/src/main.rs`. Until this cutover
  is executed, native dispatch is candidate implementation only; release
  authority stays with the npm/TypeScript CLI.
- If native command dispatch lands before this cutover, the cutover gate must
  prove it is not reachable from the released npm launcher except under an
  explicit operator-controlled candidate path.
- As of the 2026-05-20 inspection, this guard is intentionally active:
  `crates/runx-cli/tests/launcher.rs` proves packaged Node CLI files do not set
  `RUNX_RUST_CLI`/`RUNX_RUST_HARNESS`, candidate commands delegate without
  native signals, `RUNX_RUST_CLI=0`/`RUNX_RUST_HARNESS=0` and empty signals
  still delegate, and supported native routes only activate when the
  operator-controlled signal is non-empty and non-zero.
- The current TS command set includes `runx policy inspect|lint <policy.json>`;
  the Rust CLI parity matrix must include its exit codes, JSON shape, and
  redacted human readback.
- The launcher exits with the same exit codes the TS CLI did. The current
  `docs/cli-exit-codes.md` is the taxonomy for canonical commands.
- Receipts emitted after the cutover use only the post-contract-spine canonical
  shapes. The Rust CLI must not emit retired `skill_execution`,
  `graph_execution`, pre-spine harness receipt shapes, or compatibility
  projections to match older TypeScript output.
- No v2 surface is introduced. This cutover must not add `runx v2`,
  `runx --v2`, `RUNX_V2`, `schema_version: "v2"`, dual v1/v2 output modes, or
  "new CLI behind v2" packaging. External HTTP route versions owned by other
  specs are not part of this rule.
- Rust runtime owns harness execution and receipts. `runx harness <path>` must
  execute through the native runtime harness path and write through the Rust
  receipt store APIs, not through `packages/runtime-local`.
- Rollback is an npm-level package rollback to a previous known-good release.
  Rollback must not be implemented as a permanent JS fallback switch in the
  new launcher.

## Objectives

- Build the full native Rust CLI on top of `runx-runtime` with canonical simple
  verbs and Rust-owned help, parser, presentation, dispatch, and exit mapping.
- Replace the launcher's "npm exec / js-bin" path with "download + verify +
  exec bundled Rust binary per platform".
- Move harness execution, replay, verification, and receipt emission fully into
  the Rust runtime path.
- Publish the binary distribution pipeline.
- Run the canonical CLI matrix, native runtime suites, and distribution checks
  as the cutover gate.
- Run workspace parity and supply-chain gates as hard blockers, including
  `cargo test --workspace` and `cargo deny --manifest-path crates/Cargo.toml
  check bans licenses sources`. A red parity test or cargo-deny failure blocks
  approval.
- Prove no legacy shapes, v2 modes, aliases, or unscoped JS fallback remain in
  release artifacts.
- Document rollback and repair without creating long-lived compatibility code.

## Scope

In scope:
- Native CLI implementation for every canonical command in the cutover matrix.
- Removing the current candidate-only dispatch guard at the cutover point, while
  replacing it with a release launcher that execs the native binary directly
  and has no TypeScript fallback.
- Canonical command table, help, parser, JSON output, human output, exit codes,
  and release-note migration text for removed aliases.
- Rust runtime harness execution/receipts and CLI integration for `harness`.
- Rust integration for registry-backed commands through `runx-runtime::registry`.
- Rust integration for history/journal projections through `rust-journal-local`.
- Binary distribution pipeline, including checksums/signing, platform package
  resolution, and npm packaging.
- Package manifest pointer changes for `@runxhq/cli`: `bin.runx` must point to
  the native resolver/exec shim, the published package must declare or download
  pinned platform native artifacts, and TS workspace dependencies/files must no
  longer be required by the runtime launch path.
- Launcher logic to fetch/locate, verify, and exec the native binary while
  preserving raw argv, stdin, stdout, stderr, exit code, and signal behavior.
- Rollback and repair documentation.

Out of scope:
- Deleting the TS CLI package (deferred to a TS sunset spec).
- Keeping the TS CLI as a hidden compatibility backend for the new package.
- Adding v2 commands, v2 output modes, or compatibility schema projections.
- Adding alias forms for removed or renamed commands.
- Adding new user-facing commands not explicitly approved by the canonical
  command table.
- New platforms beyond what TS supports today.

## Dependencies

- `rust-cli-feature-parity-matrix` completed and consumed as the historical
  oracle plus migration inventory. The final cutover matrix must be
  canonical-only and must identify any removed legacy aliases.
- `rust-runtime-skeleton`, `rust-runtime-skill-execution`, and receipt/proof
  work complete enough that the Rust runtime is the execution and receipt
  authority.
- `rust-harness` complete, including native harness execution, fixture replay,
  canonical harness receipts, and proof-backed receipt comparison.
- The `runx-runtime::registry` module complete for search, inspect/read,
  acquire/install, local materialization, trust-tier validation, and install
  receipt metadata.
- `rust-journal-local` complete for receipt-store-backed history and any
  accepted journal projection surface.
- `rust-aster-runtime-cutover` complete for hosted aster execution against the
  Rust runtime before the public CLI package flips.
- All CLI-surface specs complete: skill execution, connect, config, scaffold,
  tool catalogs, doctor, dev, resume, replay, diff, export-receipts, history,
  knowledge show or retirement, policy inspect/lint, mcp serve, and evolve
  disposition.
- Current native command coverage is partial. `connect`, `config`, `new`,
  `init`, `history`, `list`, `policy inspect|lint`, supported `doctor`,
  supported `tool`, supported `mcp serve` without `--runner`, and harness replay
  have Rust candidate paths and focused tests. MCP runner selection still
  delegates to the TypeScript CLI until native runner support exists. The
  canonical matrix still includes `skill.run`, `skill.search`,
  `skill.add`, `skill.publish`, `skill.inspect`, `evolve`, `resume`, `replay`,
  `diff`, `export-receipts.trainable`, `knowledge.show`, `dev`, and
  runner-selected `mcp.serve`; those require Rust implementation, explicit
  removal, or migration notes before cutover.
- Binary distribution infrastructure: signing, CDN, version pinning.
- Release engineering can publish a previous known-good npm package as a
  rollback, and can revoke or quarantine a bad native binary artifact.

## Launcher And CLI Boundary

The npm launcher owns only native binary materialization:
- detect supported platform and architecture
- locate an already-installed platform package or download the pinned binary
- verify checksum/signature/notarization state required by release policy
- exec the Rust binary with raw argv and inherited stdio
- propagate process exit code or terminating signal

The npm launcher must not:
- parse subcommands or flags beyond a launcher-internal diagnostic flag that is
  not part of `runx --help`
- render command help or command errors
- rewrite aliases, expand shortcuts, or map old command spellings to new ones
- translate JSON or receipt payloads between old and new shapes
- dispatch to `npm exec`, `node`, `packages/cli`, `js-bin`, or any TypeScript
  command backend in release builds
- expose `--js-fallback`, `RUNX_JS_FALLBACK`, or equivalent public fallback

The Rust CLI owns:
- canonical command parser and help
- terminal and JSON presentation
- command dispatch and exit-code mapping
- policy/config/registry/journal/history/tool command orchestration
- all calls into `runx-runtime`

As of the 2026-05-20 inspection, this ownership is only partially true in code.
`crates/runx-cli/src/main.rs` invokes native Rust implementations for candidate
branches selected by `RUNX_RUST_CLI`/`RUNX_RUST_HARNESS`, then falls back to
`LauncherAction::Delegate` for unknown, unsupported, or unselected commands.
That fallback is valid before cutover only; it is a release blocker for this
spec.

The Rust runtime owns:
- skill and harness execution
- harness replay and verification
- receipt creation, signing, storage, path discovery, and safe projection
- registry install side effects when invoked by execution
- journal/history projections over receipt stores and ledgers

## Canonical CLI Surface

The accepted CLI surface is a table of simple canonical verbs maintained in the
cutover matrix before implementation. Each action has one public spelling and
one help entry. If a TypeScript-era command had an alias, grouped synonym, or
hidden admin form, this spec requires an explicit decision:
- keep one spelling as canonical
- remove the alias from help, parser fixtures, docs, and release artifacts
- add a migration note if external users may have depended on it

Examples of prohibited compatibility behavior:
- accepting both a top-level command and a nested `skill` synonym for the same
  action unless one is explicitly the canonical verb and the other is removed
- accepting old receipt-shaped `--json` output for a command that now emits the
  canonical post-spine shape
- accepting `v2` as a command, flag, environment switch, schema label, or
  package channel to hide the Rust CLI behind a second public mode

## Sequencing And Fallback

Command-specific Rust specs may keep temporary opt-in native routing or TS
fallback while they are in progress, but only inside the scope and acceptance
rules of those specs. Examples include pre-cutover native `harness` routing
behind an explicit selection signal.

This cutover is the point where those sequencing aids end:
- if any canonical command still needs TS fallback, this spec blocks
- if any fallback branch remains in release launcher code, this spec blocks
- if a fallback is retained only for tests, it must be behind a test-only
  compile feature or fixture harness and absent from published npm artifacts
- unsupported platforms fail closed with a typed launcher error; they do not
  fall back to TypeScript
- binary download fallback may retry approved native artifact mirrors only; it
  must still verify the same pinned digest/signature before exec

## Planned Phases

Phase 1: canonical surface freeze.
- Convert the completed parity matrix into a canonical cutover matrix.
- Mark every TypeScript-era alias as removed or select it as the only canonical
  spelling.
- Add migration notes for removed aliases and legacy JSON/receipt shapes.
- Add negative fixtures proving removed aliases fail with usage errors.

Phase 2: native CLI and runtime integration.
- Implement Rust parser/help/dispatch/presentation for every canonical command.
- Wire harness execution to the Rust runtime harness APIs.
- Wire registry commands through `runx-runtime::registry`.
- Wire history/journal commands through `rust-journal-local`.
- Ensure policy inspect/lint consumes the same operational policy validator and
  redacted readback as the rest of the runtime.
- Remove candidate-only native routing after the canonical matrix is complete:
  `RUNX_RUST_CLI` and `RUNX_RUST_HARNESS` may remain only as test or
  development controls outside the published release path, not as the mechanism
  that makes public commands native.

Phase 3: launcher replacement and distribution.
- Replace `packages/cli/bin/runx.js` with a native binary resolver/exec shim.
- Remove npm-exec, js-bin, and Node CLI delegation from release code.
- Replace the current `packages/cli/package.json` runtime shape. The cutover
  package must not require `@runxhq/adapters`, `@runxhq/authoring`,
  `@runxhq/contracts`, `@runxhq/core`, or `@runxhq/runtime-local` to execute
  `runx`; it must point at the native resolver and platform artifacts instead.
- Package per-platform binaries, checksums, signing metadata, and npm
  dependency or postinstall wiring.
- Add install, offline/cache, checksum mismatch, unsupported platform, and
  executable permission tests.

Phase 4: cutover verification.
- Run canonical CLI fixtures against the published-package layout.
- Run native runtime, harness, registry, journal/history, policy, and receipt
  suites.
- Run negative checks for legacy receipt fields, v2 modes, aliases, and JS
  fallback.
- Soak aster and other active dogfoods against the Rust runtime path.

Phase 5: release and rollback drill.
- Publish canary/pre-release native packages.
- Verify install and execution across supported platforms.
- Document the rollback command sequence and artifact quarantine process.
- Drill rollback by republishing the previous known-good npm package in a test
  registry or release dry run.

## Acceptance Criteria

- `packages/cli/bin/runx.js` is a launcher only: platform detection,
  binary location/download, verification, exec, and exit propagation.
- Published launcher artifacts contain no release path that invokes `npm exec`,
  `node`, `packages/cli/src`, `packages/cli/dist`, `js-bin`, or the TS command
  backend.
- `runx --help` and parser fixtures expose only canonical command spellings.
  Removed aliases fail with usage exit code `64` and a migration-safe message.
- The canonical command matrix has no alias entries, synonym groups, hidden
  admin spellings, or compatibility command variants.
- No public CLI, docs, fixtures, package metadata, or release channel exposes
  `runx v2`, `--v2`, `RUNX_V2`, `schema_version: "v2"`, or dual v1/v2 output
  behavior for this cutover.
- JSON outputs and receipts emitted by Rust contain only canonical
  post-contract-spine shapes. They do not emit retired `skill_execution`,
  `graph_execution`, pre-spine harness receipt fields, or compatibility
  projections.
- `runx harness <path>` executes through the Rust runtime harness runner and
  stores/verifies canonical harness receipts through Rust receipt APIs.
- Registry commands consume `runx-runtime::registry`; they do not duplicate
  hosted namespace/trust logic or call TS registry modules.
- History/journal commands consume `rust-journal-local` receipt-store
  projections and do not expose absolute local receipt paths.
- Exit codes match `docs/cli-exit-codes.md` for canonical commands.
- Unsupported platform, missing binary, checksum mismatch, signature failure,
  and exec permission failure produce typed launcher errors without JS fallback.
- Aster cutover fixtures pass against the Rust runtime before the native CLI
  package becomes the default public `runx`.
- Rollback has been drilled and documented as an npm release rollback plus bad
  artifact quarantine, not as a hidden runtime fallback.
- Cutover commits/PRs follow repo contribution rules: keep changes focused,
  use a conventional commit title such as `feat(cli): cut over npm launcher to
  rust`, include DCO sign-off when committing, and include the validation
  evidence below in the PR body.

## Validation Commands

```sh
git diff --check -- .scafld/specs/drafts/rust-cli-rust-cutover.md
pnpm exec tsx scripts/generate-cli-feature-parity.ts --check --canonical-only
pnpm exec tsx scripts/generate-cli-feature-parity.ts --check-help-coverage --canonical-only
pnpm exec tsx scripts/check-rust-cli-cutover.ts --candidate target/debug/runx --no-legacy-shapes --no-v2 --no-aliases --no-js-fallback
cargo fmt --manifest-path crates/Cargo.toml --all --check
cargo clippy --manifest-path crates/Cargo.toml -p runx-cli -p runx-runtime --all-targets -- -D warnings
cargo test --manifest-path crates/Cargo.toml -p runx-cli
cargo test --manifest-path crates/Cargo.toml -p runx-runtime
cargo test --manifest-path crates/Cargo.toml -p runx-runtime harness
cargo test --manifest-path crates/Cargo.toml -p runx-runtime --test registry_client
cargo test --manifest-path crates/Cargo.toml -p runx-receipts
pnpm exec tsx scripts/package-rust-cli.ts --check
pnpm exec tsx scripts/check-rust-cli-release-artifacts.ts --no-js-delegation --verify-signatures
! rg -n "npm exec|js-bin|--js-fallback|RUNX_JS_FALLBACK|packages/cli/src|packages/cli/dist" packages/cli/bin crates/runx-cli
! rg -n "runx v2|--v2|RUNX_V2|schema_version.*v2|\"v2\"" fixtures/cli-parity fixtures/harness fixtures/runtime crates/runx-cli packages/cli
! rg -n "skill_execution|graph_execution|pre_spine|legacy_receipt|compat_receipt" fixtures/cli-parity fixtures/harness fixtures/runtime crates/runx-cli crates/runx-runtime
```

Cutover validation script status from the 2026-05-20 inspection:
- Existing and runnable today:
  `pnpm exec tsx scripts/generate-cli-feature-parity.ts --check
  --canonical-only`,
  `pnpm exec tsx scripts/generate-cli-feature-parity.ts
  --check-help-coverage --canonical-only`,
  `cargo test --manifest-path crates/Cargo.toml -p runx-cli`,
  `cargo test --manifest-path crates/Cargo.toml -p runx-runtime --test
  harness_fixtures`, `cargo test --manifest-path crates/Cargo.toml -p
  runx-runtime --test registry_client`, `cargo test --manifest-path
  crates/Cargo.toml -p runx-runtime --test journal_history`, and the focused
  connect runtime tests.
- Missing and must be added before approval:
  `scripts/check-rust-cli-cutover.ts`, `scripts/package-rust-cli.ts`,
  `scripts/release-rust-cli.ts`, and
  `scripts/check-rust-cli-release-artifacts.ts`.
- `pnpm exec tsx scripts/check-cli-package-contract.mjs` is green for the
  current TypeScript package contract only. It does not prove the native
  package pointer, platform artifact, or no-JS-delegation contract required by
  this cutover.

## Rollback And Repair

- Primary rollback is republishing the previous known-good npm `@runxhq/cli`
  version and quarantining or revoking the bad native artifact. The new
  launcher must not keep a permanent JS fallback to implement rollback.
- The TypeScript CLI may remain publishable as a separate rollback artifact for
  one minor cycle, but it is not bundled as a hidden backend in the cutover
  package.
- If a platform binary is missing or mis-signed, repair the artifact manifest
  and republish the platform package; do not route that platform to TS.
- If a canonical command regresses, fix the Rust command or block the release.
  Do not restore removed aliases or old output shapes as a quick repair.
- If receipt validation fails, repair canonical receipt generation,
  canonicalization, path discovery, or proof verification. Do not emit legacy
  receipt compatibility fields.
- If harness execution regresses, rollback the npm package or fix the Rust
  runtime harness path. Do not delegate `runx harness` to
  `packages/runtime-local`.
- If registry or journal behavior is incomplete, block this cutover until
  `runx-runtime::registry` or `rust-journal-local` acceptance is complete.
- If an external dogfood such as aster fails its Rust-runtime fixture, block the
  CLI cutover even if standalone CLI tests pass.

## Open Questions

- Whether `knowledge show` ports or retires before this cutover. It must be
  resolved before Phase 1 canonical surface freeze.
- Binary signing scheme (Apple notarization, Authenticode, sigstore).
  Defer to Phase 1 ingest.

## Harden Notes

- 2026-05-19: Reframed this as a hard cutover rather than a TS-compatible
  launcher swap.
- 2026-05-19: Added launcher/Rust/runtime ownership boundaries, canonical
  simple-verb rules, fallback sequencing, no legacy shape/no v2/no alias gates,
  validation commands, dependencies on active Rust harness/registry/journal
  work, and rollback/repair rules.
- 2026-05-20: Reopened the current-state claims after code inspection. Native
  command dispatch is a candidate path behind explicit environment signals,
  `connect`/`list`/`harness`/`history` have focused Rust coverage, npm package
  pointers still target the TypeScript CLI, and cutover packaging/verifier
  scripts are still missing.
- 2026-05-20: Added native candidate coverage note for `policy inspect|lint`;
  release authority still remains with the TypeScript CLI until the hard
  cutover packaging and no-JS-fallback gates land.
- 2026-05-20: Narrowed MCP candidate coverage to supported `mcp serve` shapes
  without `--runner`; runner-selected MCP remains delegated to TypeScript until
  native runner support lands.
