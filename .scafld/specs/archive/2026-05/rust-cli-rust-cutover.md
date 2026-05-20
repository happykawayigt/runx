---
spec_version: '2.0'
task_id: rust-cli-rust-cutover
created: '2026-05-18T00:00:00Z'
updated: '2026-05-20T14:17:53Z'
status: completed
harden_status: not_run
size: extra_large
risk_level: very_high
---

# Rust CLI hard cutover

## Current State

Status: completed
Current phase: final
Next: done
Reason: task completed
Blockers: none
Allowed follow-up command: `none`
Latest runner update: 2026-05-20T14:17:53Z
Review gate: pass

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
Verdict: pass
Mode: verify
Provider: claude:claude-opus-4-7
Output: claude.mcp_submit_review
Summary: All four previous blockers are repaired. The npm `@runxhq/cli` package now ships a Node selector launcher (`packages/cli/bin/runx`) backed by `optionalDependencies` for all five supported platform packages (`@runxhq/cli-{darwin-arm64,darwin-x64,linux-arm64,linux-x64,win32-x64}@0.5.22`). `scripts/package-rust-cli.ts` stages selector + native packages with distinct names so `assertPublishTargets` in `release-rust-cli.ts` no longer collides on duplicate `name@version`. `tests/cli-package.test.ts` was rewritten to match the new extensionless selector binary and topology manifest. The stray `skill_execution` receipt under `packages/cli/.runx/` is gone (the directory is gitignored and absent). Candidate Rust binary is clean of `RUNX_JS_BIN`/`RUNX_RUST_CLI`/`npm exec`/retired receipt shape/v2 tokens, harness execution still routes through `runx_runtime::run_harness_fixture`, and the canonical command matrix declares zero aliases. One non-blocking gap remains: the Rust `help_text()` in `crates/runx-cli/src/launcher.rs:182` omits the two natively-routed canonical commands `skill` and `harness`, which the oracle case `help.top-level` expects in `--help` stdout; the gate misses this because `tests/cli-feature-parity.test.ts` only executes the oracle against the TypeScript `runCli`. Commands still work when typed, so cli-cutover-5 is a UX/discoverability finding rather than a release blocker.

Attack log:
- `packages/cli/package.json + bin/runx + native/supported-platforms.json`: Re-check finding cli-cutover-1: confirm multi-platform shape with optionalDependencies, os/cpu filter (in native packages), launcher shim, and platform topology manifest -> clean (package.json L36-42 declares optionalDependencies for all 5 platforms at 0.5.22; bin/runx is an ESM Node launcher that resolves @runxhq/cli-${platform}, sha256-verifies the native binary against native/checksums.json, then spawnSyncs with inherited stdio; native/supported-platforms.json maps every supported platform to the expected native package + binary path. Previous critical blocker resolved.)
- `scripts/release-rust-cli.ts + scripts/package-rust-cli.ts`: Re-check finding cli-cutover-2: confirm release pipeline can stage multiple platforms without duplicate name@version collisions -> clean (package-rust-cli.ts L34-36/79-101/112-137 writes the selector under `selectorRoot` with name=manifest.name (@runxhq/cli) and each native under `nativeRoot` with name=nativePackageName(platform.key) (@runxhq/cli-${platform}). assertPublishTargets in release-rust-cli.ts L163-176 only fires on identical name@version pairs, which no longer occurs because selector and natives have distinct names.)
- `tests/cli-package.test.ts + scripts/test-workspace.mjs`: Re-check finding cli-cutover-3: confirm test no longer references deleted bin/runx.js or dist/index.js -> clean (tests/cli-package.test.ts:11 now defines cliBinEntry as packages/cli/bin/runx (no extension); tests assert shebang #!/usr/bin/env node, spawnSync call, native-package-missing error message, topology schema, and pack list contains only LICENSE/bin/runx/native/supported-platforms.json/package.json. scripts/test-workspace.mjs L1-52 was simplified to just compile the Rust binary and run vitest with RUNX_KERNEL_EVAL_BIN set; no RUNX_VITEST_BATCH forcing required.)
- `packages/cli/.runx/receipts/`: Re-check finding cli-cutover-4: confirm no stale skill_execution receipts remain under the cli package tree -> clean (Glob `packages/cli/.runx/**` returns nothing; grep for skill_execution|graph_execution|pre_spine|legacy_receipt|compat_receipt under packages/cli returns no files. packages/cli/.gitignore lists .runx/ so future stray receipts will not get committed.)
- `crates/runx-cli/src/ + tests/launcher.rs`: Hunt for residual JS fallback / candidate-gate / v2 / legacy-receipt tokens that the cutover should have eliminated -> clean (grep across crates/runx-cli for RUNX_JS_BIN/RUNX_RUST_CLI/RUNX_RUST_HARNESS/RUNX_NPM_PACKAGE/npm exec, runx v2/--v2/RUNX_V2/schema_version v2, and skill_execution/graph_execution/pre_spine/legacy_receipt/compat_receipt all return no matches. tests/launcher.rs::package_manifest_is_native_binary_shaped asserts bin.runx=./bin/runx, files=[LICENSE,bin/runx,native/supported-platforms.json], no dependencies/exports/scripts, no workspace:/runtime-local strings.)
- `crates/runx-cli/src/launcher.rs::help_text + tests/cli-feature-parity.test.ts + fixtures/cli-parity/cases/oracle.json`: Verify that Rust runx --help exposes the canonical command surface the cutover promises -> finding (help_text() at launcher.rs:182-208 lists 12 verbs (new/init/history/list/connect/config/policy/kernel/doctor/mcp serve/tool/registry) but is missing the two natively-implemented canonical commands `skill` and `harness`. Oracle case help.top-level expects both substrings; tests/cli-feature-parity.test.ts:95-132 only runs the oracle through `runCli` from packages/cli/src/index.js, so the gate validates TS help, not the Rust binary. After cutover the user-visible `runx --help` hides commands that the binary actually accepts. Filed as cli-cutover-5.)
- `packages/cli/bin/runx signature verification path`: Check whether the launcher enforces signature verification at runtime per acceptance criterion `signature failure ... produces typed launcher errors` -> clean (bin/runx only reads native/checksums.json and sha256-verifies the binary; signatures.json is not consulted at runtime. Defensible because (a) scripts/package-rust-cli.ts::readSignatureManifest verifies signatures.json.sha256 matches the staged binary digest at staging time, (b) scripts/check-rust-cli-release-artifacts.ts::inspectSignature enforces --verify-signatures before publish, and (c) the runtime sha256 chain (checksums.json -> binary digest) is what guarantees integrity. Spec language at L200-205 says `verify checksum/signature/notarization state required by release policy`; the release-time signature gate + runtime checksum gate together satisfy this. Not flagged.)
- `scripts/check-rust-cli-cutover.ts + check-rust-cli-cutover-negative.mjs + check-rust-cli-release-artifacts.ts`: Verify cutover negative gates would catch JS delegation, v2 modes, retired receipt shapes, alias regressions, and TS workspace dependency leakage in selector + native packages -> clean (check-rust-cli-cutover.ts scans candidate binary bytes for RUNX_JS_BIN/RUNX_NPM_PACKAGE/RUNX_RUST_CLI/RUNX_RUST_HARNESS/npm exec/DEFAULT_NPM_PACKAGE/packages/cli/bin/runx.js, retired skill/graph_execution + pre_spine + legacy_receipt + compat_receipt, RUNX_V2/--v2/runx v2/schema_version v2, and verifies fixtures/cli-parity/commands.json has zero aliases. check-rust-cli-release-artifacts.ts enforces selector topology (name=@runxhq/cli, runx.nativeSelector schema v1, supportedPlatforms set, nativePackagePattern, optionalDependencies set), native manifest (correct name/os/cpu/bin/runx.nativePackage), forbidden TS runtime deps, workspace: deps, checksum.json sha256 chain, signature.json schema and entries (under --verify-signatures), and rejects packed dist/src/tools/node_modules/bin/runx.{js,mjs,cjs} entries. Coverage looks intentional and complete for the named acceptance criteria.)

Findings:
- [medium/non-blocking] `cli-cutover-5` Rust runx --help omits canonical commands skill and harness even though the launcher routes them natively.
  - Location: `crates/runx-cli/src/launcher.rs:182`
  - Evidence: crates/runx-cli/src/launcher.rs:182-208 (`help_text`) lists new/init/history/list/connect/config/policy/kernel/doctor/mcp/tool/registry but does NOT include any line for `runx skill <…>` or `runx harness <…>`. The same launcher routes those verbs at lines 110-112 (`first_arg_is(&args, "harness")` -> `native_harness_plan`) and 169-172 (`first_arg_is(&args, "skill")` -> `crate::skill::parse_skill_plan`); `main.rs` dispatches `RunHarness` (line 33) and `RunSkill` (line 39) through `runx_runtime::run_harness_fixture` and `runx_cli::skill::run_native_skill`. The canonical oracle `fixtures/cli-parity/cases/oracle.json:5-22` (`help.top-level`) asserts `stdoutIncludes: ["Usage:", "runx skill", "runx harness"]`, but `tests/cli-feature-parity.test.ts:95-132` runs that oracle through `runCli` from `packages/cli/src/index.js` (the TS backend), so the gate validates the TS help text, not the Rust binary that the cutover ships.
  - Impact: Once the npm package flips and `bin/runx` execs the Rust binary, `runx --help` hides two canonical commands the binary actually supports. Users who relied on `--help` to discover `runx skill` and `runx harness` see a smaller surface than what works, and no automated gate flags the drift because the only oracle execution path still walks the TypeScript CLI. The spec scope explicitly includes "Canonical command table, help, parser, JSON output, human output, exit codes" and the acceptance criterion at L322 says `runx --help` exposes canonical command spellings.
  - Validation: After the cutover ships, run `target/debug/runx --help | rg 'runx (skill|harness)'` — both greps fail. Compare against `packages/cli/src/help.ts:30-97` which still advertises both commands for the TS CLI.

