---
spec_version: '2.0'
task_id: rust-cli-rust-cutover
created: '2026-05-18T00:00:00Z'
updated: '2026-05-20T13:16:20Z'
status: review
harden_status: not_run
size: extra_large
risk_level: very_high
---

# Rust CLI hard cutover

## Current State

Status: review
Current phase: final
Next: repair
Reason: review gate fail: 4 finding(s), 3 completion blocker(s)
Blockers: none
Allowed follow-up command: `scafld handoff rust-cli-rust-cutover`
Latest runner update: 2026-05-20T13:19:50Z
Review gate: fail

## Summary

Flip the npm `@runxhq/cli` package from a Node CLI/runtime package to a
platform-aware Rust binary release package. Native Rust command foundations,
including canonical `runx skill <path>`, have landed behind candidate signals;
this draft is the next executable package/release cutover slice. Today
`packages/cli/bin/runx.js` still imports the TypeScript-built CLI or falls back
to the source CLI through Node, and `crates/runx-cli/src/launcher.rs` still has
pre-cutover delegate/candidate machinery. This spec removes release-path
delegation and makes the Rust binary the authoritative `runx` invocation.

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
- `crates/runx-cli/src/main.rs` and `crates/runx-cli/src/launcher.rs`
  (remove release-visible delegate, candidate-signal, and shim paths from the
  candidate binary)
- `packages/cli/bin/runx.js` (replace with native resolver/exec shim or remove
  from the release package if `bin.runx` points directly at the native binary)
- `packages/cli/package.json` (native package/bin/files/dependency shape)
- `scripts/check-rust-cli-cutover.ts`, `scripts/package-rust-cli.ts`, and
  `scripts/check-rust-cli-release-artifacts.ts` (existing gates for the
  candidate binary and package artifacts)
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
- The native command foundation is not reopened by this package/release slice.
  If the canonical matrix or candidate-binary gates reveal a command gap,
  alias, legacy shape, or TS fallback dependency, the package cutover blocks
  until the owning command spec fixes or explicitly removes that surface.
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

- Preserve the completed native Rust command foundation, including native skill
  run, and do not reintroduce aliases, shims, or compatibility command forms.
- Replace the published npm package/bin/runtime shape with a Rust binary
  artifact shape.
- Remove release-path Node, TypeScript, `npm exec`, `RUNX_JS_BIN`,
  candidate-signal, and shim delegation from the candidate binary and package
  artifacts.
- Package the Rust binary per supported platform with checksums/signatures and
  executable package metadata.
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
- Package/bin/release artifact cutover for the already landed native command
  foundation.
- Removing the current candidate-only dispatch guard and JS delegation from the
  release path, while replacing it with a package layout that execs the native
  binary directly and has no TypeScript fallback.
- Canonical command table, help, parser, JSON output, human output, exit codes,
  and release-note migration text for removed aliases.
- Rust runtime harness execution/receipts and CLI integration for `harness`.
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
- Reopening the completed native skill run foundation.
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
- Native CLI command foundations consumed by this slice, including native skill
  run, must remain canonical-only. This package/release cutover must not add
  aliases, shims, JS fallback, or compatibility output projections to paper over
  any command-surface gap.
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

Phase 1: package surface freeze.
- Treat the completed native command foundations as the Rust-owned command
  surface for this slice.
- Confirm the canonical matrix has no aliases and no compatibility command
  variants.
- Confirm no shim flags or candidate-selection environment variables are part
  of the release package contract.

Phase 2: candidate binary release hardening.
- Remove `RUNX_RUST_CLI`/`RUNX_RUST_HARNESS` as release-selection mechanisms.
- Remove `RUNX_JS_BIN`, `RUNX_NPM_PACKAGE`, `npm exec`, Node backend, and
  packaged JS path delegation from the Rust candidate binary.
- Remove release-visible shim flags such as `--shim-help` and `--shim-version`.
- Keep no-alias and no-legacy-shape negative gates blocking.

Phase 3: launcher replacement and distribution.
- Replace `packages/cli/bin/runx.js` with a native binary resolver/exec shim,
  or make `bin.runx` point directly at the packaged native binary.
- Remove npm-exec, js-bin, Node CLI, and TypeScript delegation from release
  code.
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
- Run native runtime, harness, registry, journal/history, policy, skill, and
  receipt suites needed by the package release.
- Run negative checks for legacy receipt fields, v2 modes, aliases, shims, and
  JS fallback/delegation.
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
  `scripts/check-rust-cli-cutover.ts`, `scripts/package-rust-cli.ts`, and
  `scripts/check-rust-cli-release-artifacts.ts`, plus the canonical CLI parity
  and Rust runtime commands listed above.
- Current blockers expected before this slice lands:
  `pnpm exec tsx scripts/check-rust-cli-cutover.ts --candidate
  target/debug/runx --no-legacy-shapes --no-v2 --no-aliases
  --no-js-fallback` blocks because the candidate binary still contains
  JS-fallback/candidate-selection tokens and legacy shape tokens.
- Current package artifact blockers expected before this slice lands:
  `pnpm exec tsx scripts/check-rust-cli-release-artifacts.ts --artifact-dir
  packages/cli --no-js-delegation --verify-signatures` blocks because the
  package still points `bin.runx` at JavaScript, ships JS/TS runtime paths,
  lacks native checksum/signature manifests, and keeps TS workspace runtime
  dependencies.
- `pnpm exec tsx scripts/package-rust-cli.ts --check --binary
  target/debug/runx` proves the packaging helper can stage a native package
  from an executable candidate, but it does not by itself prove the candidate
  binary is cutover-clean.
- `pnpm exec tsx scripts/check-cli-package-contract.mjs`, if present, is a
  TypeScript package contract check only. It does not prove the native package
  pointer, platform artifact, or no-JS-delegation contract required by this
  cutover.

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

- Binary signing scheme (Apple notarization, Authenticode, sigstore).
  Defer to package/release implementation before approval.

## Harden Notes

- 2026-05-19: Reframed this as a hard cutover rather than a TS-compatible
  launcher swap.
- 2026-05-19: Added launcher/Rust/runtime ownership boundaries, canonical
  simple-verb rules, fallback sequencing, no legacy shape/no v2/no alias gates,
  validation commands, dependencies on active Rust harness/registry/journal
  work, and rollback/repair rules.
- 2026-05-20: Reopened the current-state claims after code inspection. Native
  command dispatch is a candidate path behind explicit environment signals,
  `connect`/`list`/`harness`/`history` have focused Rust coverage, and npm
  package pointers still target the TypeScript CLI. This older note was
  superseded later the same day for packaging/verifier script status.
- 2026-05-20: Added native candidate coverage note for `policy inspect|lint`;
  release authority still remains with the TypeScript CLI until the hard
  cutover packaging and no-JS-fallback gates land.
- 2026-05-20: Narrowed MCP candidate coverage to supported `mcp serve` shapes
  without `--runner`; runner-selected MCP remains delegated to TypeScript until
  native runner support lands.
- 2026-05-20: Updated this draft for the package/release cutover slice after
  native skill run foundation landed. The cutover/package/release verifier
  scripts now exist; the remaining blockers are the npm package/bin/artifact
  shape and candidate-binary fallback/legacy tokens, not missing verifier
  scripts.

## Review

Status: completed
Verdict: fail
Mode: discover
Provider: claude:claude-opus-4-7
Output: claude.mcp_submit_review
Summary: The cutover ships a single-platform (darwin-arm64) binary inside `packages/cli/` with no platform detection, no `optionalDependencies`, no `postinstall`, no `os`/`cpu` filter, and no launcher shim. The spec invariant explicitly requires the published `@runxhq/cli` to install `runx` on macOS x86_64/arm64, Linux x86_64/arm64, and Windows x86_64. Today, installing on any non-darwin-arm64 platform either fails (exec-format error on Linux, missing `.exe` on Windows) or runs a binary for the wrong architecture, with no typed launcher error path because there is no launcher script. The release pipeline cannot work around this: `scripts/release-rust-cli.ts` rejects duplicate `name@version` publish targets while `scripts/package-rust-cli.ts` stages every platform under `@runxhq/cli@0.5.22`, so a multi-platform release crashes before it can publish anything. The cutover also deletes `packages/cli/bin/runx.js` but leaves `tests/cli-package.test.ts` asserting that exact file plus `dist/index.js`, and `scripts/test-workspace.mjs` forces that test to run on every `pnpm --dir oss test` — turning the standard test gate into a regression. A stray legacy `skill_execution` receipt is also committed under `packages/cli/.runx/receipts/` (not packaged, but contradicts the no-legacy-shape stance for the cutover package). Together these block completion.

Attack log:
- `packages/cli/package.json + bin/runx + native/*.json`: Verify multi-platform shape: optionalDependencies, postinstall, os/cpu filter, launcher shim -> finding (Single darwin-arm64 binary committed; no platform resolution exists. Spec invariant L73-77 + acceptance L344 violated.)
- `scripts/release-rust-cli.ts + scripts/package-rust-cli.ts`: Trace whether release pipeline can publish multiple platforms end-to-end -> finding (release-rust-cli.ts:152-164 throws on duplicate name@version, but package-rust-cli.ts uses fixed manifest.name across platforms.)
- `tests/cli-package.test.ts + scripts/test-workspace.mjs`: Find callers of deleted packages/cli/bin/runx.js that the cutover missed -> finding (Test references deleted bin/runx.js + dist/index.js; test-workspace.mjs forces it to run on every pnpm test.)
- `packages/cli/bin/runx (Mach-O arm64) + crates/runx-cli/src/main.rs`: Check for residual RUNX_RUST_CLI / RUNX_JS_BIN / npm exec / packages/cli/src delegation in candidate binary and crates -> clean (No matches via grep across crates/runx-cli for forbidden tokens; main.rs invokes only native runners.)
- `crates/runx-cli/src/launcher.rs + tests/launcher.rs`: Confirm no alias/v2/legacy-shape spellings in canonical command table or test expectations -> clean (help_text + plan_launcher route canonical verbs only; launcher tests assert shim flags fail closed and package.json must not contain workspace:/runtime-local strings.)
- `packages/cli/.runx/receipts/`: Hunt for retired skill_execution/graph_execution receipts in the cli package tree -> finding (Stray rx_50447baa…json with kind: skill_execution committed; not packaged but stale and contradicts negative gate.)
- `scripts/check-rust-cli-release-artifacts.ts + check-rust-cli-cutover.ts + check-rust-cli-cutover-negative.mjs`: Validate the cutover gate scripts actually enforce no-JS / no-v2 / no-aliases / no-legacy-shape -> clean (Scripts implement the documented checks (token scans, manifest pack inspection, canonical matrix alias check).)
- `crates/runx-cli/tests/launcher.rs::package_manifest_is_native_binary_shaped`: Check whether the launcher test gate would catch single-platform package shape -> finding (Test only asserts bin.runx=./bin/runx and files=['LICENSE','bin','native']; says nothing about optionalDependencies / postinstall / os filter — single-platform package passes today.)

Findings:
- [critical/blocks completion] `cli-cutover-1` Published @runxhq/cli only supports darwin-arm64; spec invariant requires 5 platforms
  - Location: `packages/cli/package.json:20`
  - Evidence: packages/cli/package.json declares `bin.runx: "./bin/runx"` and `files: ["LICENSE", "bin", "native"]` with no `os`, `cpu`, `optionalDependencies`, `postinstall`, or launcher shim. `packages/cli/bin/runx` is a Mach-O 64-bit arm64 executable; `packages/cli/native/checksums.json:5` and `signatures.json:5` both declare `"platform": "darwin-arm64"`. There is no platform-resolution code anywhere — `bin/runx` is the binary itself, not a launcher script.
  - Impact: Installing `@runxhq/cli` on Linux x86_64, Linux arm64, Windows x86_64, or macOS x86_64 either silently runs a binary for the wrong CPU/OS (exec format error on Linux, no .exe on Windows) or fails without the typed launcher error the spec promises. The spec's `Invariants` section (lines 73-77) requires support for darwin x86_64/arm64, linux x86_64/arm64, and win32 x86_64; the `Launcher And CLI Boundary` section (lines 199-205) mandates platform detection, artifact resolution, checksum/signature verification, and exec; Acceptance Criteria (line 344) requires "Unsupported platform, missing binary, checksum mismatch, signature failure, and exec permission failure produce typed launcher errors without JS fallback." None of that exists.
  - Validation: Inspect `packages/cli/package.json` for `optionalDependencies`/`postinstall`/`os`/`cpu` — absent. Run `file packages/cli/bin/runx` — single-architecture Mach-O arm64. Attempt `npm install @runxhq/cli` on linux/x64 in a sandbox — installation succeeds but `runx` invocation fails with `exec format error` and no typed message.
- [high/blocks completion] `cli-cutover-2` release-rust-cli.ts rejects multi-platform publish via duplicate name@version guard
  - Location: `scripts/release-rust-cli.ts:152`
  - Evidence: `scripts/release-rust-cli.ts:152-164` (`assertPublishTargets`) throws `duplicate npm publish target` if it sees two staged directories with the same `name@version`. `scripts/package-rust-cli.ts:34-36,79-101` writes every platform's `package.json` with `name: manifest.name` (always `@runxhq/cli`) and `version: manifest.version` (always `0.5.22`) into `${outDir}/${platform}/`. So when the release pipeline materializes more than one platform under `.runx/rust-cli-artifacts/`, it crashes before npm publish runs.
  - Impact: There is no path to publish multi-platform Rust CLI artifacts through the documented release script — either every release is single-platform (already violating the invariant) or the second-platform publish blows up. The spec explicitly calls out `Phase 3: Package per-platform binaries, ... and npm dependency or postinstall wiring` (L298-301) and Phase 5 canary publication of native packages across supported platforms.
  - Validation: Stage two platforms via `pnpm exec tsx scripts/package-rust-cli.ts --binary <darwin-arm64-runx> --out-dir .runx/rust-cli-artifacts --signature-manifest packages/cli/native/signatures.json` and `... <linux-x64-runx> ...`, then run `pnpm exec tsx scripts/release-rust-cli.ts --artifact-dir .runx/rust-cli-artifacts --publish`. The script throws `duplicate npm publish target: @runxhq/cli@0.5.22`.
- [high/blocks completion] `cli-cutover-3` tests/cli-package.test.ts still asserts deleted packages/cli/bin/runx.js and forces pnpm test to fail
  - Location: `tests/cli-package.test.ts:12`
  - Evidence: Git status shows `D packages/cli/bin/runx.js`. `tests/cli-package.test.ts:11-12` declares `cliDistEntry = packages/cli/dist/index.js` and `cliBinEntry = packages/cli/bin/runx.js`. Tests at L44-83 `stat` and `execFile` `cliBinEntry` through `process.execPath`; L99-127 assert that `npm pack --dry-run` includes `bin/runx.js`, `dist/index.js`, `dist/src/index.js`, `dist/src/official-skills.lock.json`, etc. — but the new `packages/cli/package.json` `files` is `["LICENSE", "bin", "native"]` (no `dist`, no `src`), and `bin/runx.js` is gone. `scripts/test-workspace.mjs:34-35` unconditionally runs this test under `RUNX_VITEST_BATCH=cli-package` for every workspace test invocation.
  - Impact: `pnpm --dir oss test` (a CLAUDE.md-listed standard command and a workspace parity gate) will fail in `beforeAll`/`stat` once the previously-deleted bin is gone. The same test also embeds explicit assertions (`expect(files).toContain('bin/runx.js')`, `dist/index.js`, etc.) that contradict the cutover's `files` shape. Either the cutover is incomplete (forgot to remove or rewrite this test) or the workspace test gate is silently broken.
  - Validation: Run `pnpm --dir oss test` (or `RUNX_VITEST_BATCH=cli-package pnpm --dir oss exec vitest run tests/cli-package.test.ts`). The first `stat(cliDistEntry)`/`execFile(cliBinEntry, ...)` call ENOENTs, and the `expect(files).toContain('bin/runx.js')` assertion fails against the post-cutover pack output.
- [low/non-blocking] `cli-cutover-4` Stale skill_execution receipt committed under packages/cli/.runx/receipts/
  - Location: `packages/cli/.runx/receipts/rx_50447baa803d4cdc88e909983746a9ec.json:3`
  - Evidence: `packages/cli/.runx/receipts/rx_50447baa803d4cdc88e909983746a9ec.json` has `"kind": "skill_execution"` — the retired pre-spine receipt kind the cutover's negative gates forbid (`scripts/check-rust-cli-cutover.ts:34-38` lists `skill_execution`/`graph_execution`/`pre_spine`/`legacy_receipt`/`compat_receipt` as forbidden binary tokens; spec L107-108 explicitly forbids emitting these).
  - Impact: Not shipped (package.json `files` excludes `.runx/`), so no consumer-facing impact, but it contradicts the cutover's no-legacy-receipt-in-the-cli-package stance and may trip future repo-wide negative scans.
  - Validation: `grep -rn skill_execution packages/cli/` returns this receipt.

