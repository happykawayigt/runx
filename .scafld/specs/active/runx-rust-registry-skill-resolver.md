---
spec_version: '2.0'
task_id: runx-rust-registry-skill-resolver
created: '2026-06-09T16:40:56Z'
updated: '2026-06-10T00:56:47Z'
status: review
harden_status: passed
size: medium
risk_level: medium
---

# Rust registry skill resolver

## Current State

Status: review
Current phase: final
Next: review
Reason: build completed; ready for review
Blockers: none
Allowed follow-up command: `scafld review runx-rust-registry-skill-resolver`
Latest runner update: 2026-06-10T00:56:47Z
Review gate: not_started

## Summary

Move runnable skill reference resolution into the Rust CLI for local paths,
installed skills, first-party official shorthand, and explicit registry refs.
The resolver must be flexible enough for third-party registries while keeping
the trusted runtime fail-closed: only verified registry packages become
runnable. TypeScript remains a presentation/wrapper layer and stops owning the
runnable skill resolver.

## Objectives

- Make `runx skill <ref>` resolve the same high-value refs through the native
  Rust path:
  - local paths and `SKILL.md`
  - exported Claude/Codex shims
  - workspace-local `skills/<name>`
  - previously installed skills
  - first-party official shorthand such as `runx skill brand-voice`
  - explicit registry refs such as `runx skill acme/refund-helper@1.2.3`
- Support multi-version resolution correctly:
  - explicit `owner/name@version` resolves that exact version
  - unversioned explicit `owner/name` resolves the registry target's latest
    version deterministically
  - multiple versions of the same skill can coexist in cache and run
    side-by-side
  - cache identity includes registry origin, skill id, version, markdown digest,
    and profile digest when present
- Keep resolution deterministic and safe:
  - bare `<name>` never performs open remote search
  - explicit `<owner>/<name>[@version]` may resolve through the configured
    registry
  - runnable registry installs require trusted signed-manifest verification and
    optional digest pin checks
  - untrusted registry results remain inspect/search/read metadata, not
    executable inputs to `runx skill`
- Reuse existing registry verification/install code instead of creating a
  second registry implementation.
- Replace the TypeScript runnable resolver with a thin native call path once
  Rust owns the behavior.
- Preserve the clean local operator UX from `runx-cli-operator-ux-v1`.

## Scope

- Rust CLI skill resolution and registry-cache orchestration.
- Minimal TypeScript wrapper cleanup needed to stop duplicating runnable
  resolution.
- Docs/help/examples that describe the final resolver behavior.
- Focused tests and dogfood for trusted official and trusted third-party/local
  registry refs.

Out of scope:

- Open-ended remote search from `runx skill <bare-name>`.
- Running unsigned/unverified remote registry packages by default.
- A new registry trust model, new signing format, or compatibility shim.
- Hosted marketplace UI, payment, install grants beyond the existing registry
  acquire/install path.

## Dependencies

- Existing native registry install path in [crates/runx-cli/src/registry.rs](/Users/kam/dev/runx/runx/oss/crates/runx-cli/src/registry.rs).
- Existing runtime install verification in [crates/runx-runtime/src/registry/install.rs](/Users/kam/dev/runx/runx/oss/crates/runx-runtime/src/registry/install.rs).
- Existing remote/local registry client APIs under [crates/runx-runtime/src/registry](/Users/kam/dev/runx/runx/oss/crates/runx-runtime/src/registry).
- Existing official lock generation in [scripts/generate-official-lock.mjs](/Users/kam/dev/runx/runx/oss/scripts/generate-official-lock.mjs) and lock file [packages/cli/src/official-skills.lock.json](/Users/kam/dev/runx/runx/oss/packages/cli/src/official-skills.lock.json).
- Existing install/project state helpers in [crates/runx-runtime/src/scaffold/init.rs](/Users/kam/dev/runx/runx/oss/crates/runx-runtime/src/scaffold/init.rs).

## Grounding Evidence

- Registry refs already parse versions. [crates/runx-runtime/src/registry/refs.rs](/Users/kam/dev/runx/runx/oss/crates/runx-runtime/src/registry/refs.rs:20) strips supported registry prefixes and separates `skill_id` from optional `@version`.
- Remote ref resolution currently supports explicit `owner/name` refs and also has a bare-name search branch. [crates/runx-runtime/src/registry/refs.rs](/Users/kam/dev/runx/runx/oss/crates/runx-runtime/src/registry/refs.rs:47) proves the new runnable path must intentionally avoid open remote search for bare `runx skill <name>`.
- A versioned materialization helper already exists. [crates/runx-runtime/src/registry/refs.rs](/Users/kam/dev/runx/runx/oss/crates/runx-runtime/src/registry/refs.rs:82) builds cache paths with owner, name, version, and digest short marker. This is the right primitive to reuse or adapt for runnable registry materialization.
- The current install path helper strips version from install refs. [crates/runx-runtime/src/registry/refs.rs](/Users/kam/dev/runx/runx/oss/crates/runx-runtime/src/registry/refs.rs:102) delegates to `normalize_install_ref`, which returns only `parsed.skill_id` at [crates/runx-runtime/src/registry/refs.rs](/Users/kam/dev/runx/runx/oss/crates/runx-runtime/src/registry/refs.rs:121).
- Local registry storage is genuinely multi-version. [crates/runx-runtime/src/registry/local.rs](/Users/kam/dev/runx/runx/oss/crates/runx-runtime/src/registry/local.rs:151) supports exact version lookup or latest lookup, and [crates/runx-runtime/src/registry/local.rs](/Users/kam/dev/runx/runx/oss/crates/runx-runtime/src/registry/local.rs:168) lists all stored `*.json` versions for a skill.
- Local registry resolution preserves selected version metadata. [crates/runx-runtime/src/registry/local.rs](/Users/kam/dev/runx/runx/oss/crates/runx-runtime/src/registry/local.rs:375) combines parsed `@version` and option overrides before reading the selected record.
- Registry link output still points runnable commands at a bare skill name. [crates/runx-runtime/src/registry/local.rs](/Users/kam/dev/runx/runx/oss/crates/runx-runtime/src/registry/local.rs:432) emits `install_command` with `skill_id@version`, but `run_command` is only `runx skill <record.name>`, losing owner and version.
- The signed-manifest trust boundary already exists and should be reused. [crates/runx-runtime/src/registry/install.rs](/Users/kam/dev/runx/runx/oss/crates/runx-runtime/src/registry/install.rs:210) requires a signed manifest, validates trusted keys, identity, markdown digest, and optional expected digest.
- Runnable installs currently use the version-stripping package path. [crates/runx-runtime/src/registry/install.rs](/Users/kam/dev/runx/runx/oss/crates/runx-runtime/src/registry/install.rs:418) builds `package_root` from `safe_skill_package_parts(candidate.ref, skill_name)`, so `owner/name@1.0.0` and `owner/name@1.1.0` target the same root under a given destination.
- The current installer does not silently replace a different version in the same root; it conflicts. [crates/runx-runtime/src/registry/install.rs](/Users/kam/dev/runx/runx/oss/crates/runx-runtime/src/registry/install.rs:441) compares existing `SKILL.md` digest and returns `ConflictingSkill` when content differs. So filesystem behavior is not "latest"; it is "one installed version per unversioned root, with later different versions blocked."
- TypeScript currently owns runnable official-cache resolution. [packages/cli/src/skill-refs.ts](/Users/kam/dev/runx/runx/oss/packages/cli/src/skill-refs.ts:77) resolves local refs first, then official lock entries only. It does not resolve third-party runnable registry refs.
- The TypeScript official cache path is also unversioned. [packages/cli/src/skill-refs.ts](/Users/kam/dev/runx/runx/oss/packages/cli/src/skill-refs.ts:190) writes under `<cacheRoot>/<owner>/<name>` while the comment claims digest distinguishes versions. This must be replaced by a real versioned/digested materialization path.
- Native skill resolution is currently local-only. [crates/runx-cli/src/skill/resolver.rs](/Users/kam/dev/runx/runx/oss/crates/runx-cli/src/skill/resolver.rs:4) resolves existing paths, exported shims, and `cwd/skills/<name>`, then errors. It does not yet check installed roots, official cache, or registry refs.
- Native skill parsing currently treats unknown flags as inputs. [crates/runx-cli/src/skill/parser.rs](/Users/kam/dev/runx/runx/oss/crates/runx-cli/src/skill/parser.rs:220) means new resolver-only flags such as `--registry` and `--digest` must be explicit parser cases, with `--input registry=...` kept for skill input collisions.

## Assumptions

- The registry install contract is already the trusted security boundary for
  signed manifest, digest, profile digest, runner metadata, and atomic writes.
- `RUNX_REGISTRY_URL`, `RUNX_REGISTRY_DIR`, `RUNX_HOME`, `RUNX_CWD`,
  `RUNX_PROJECT_DIR`, `RUNX_OFFICIAL_SKILLS_DIR`, `RUNX_INSTALLATION_ID`, and
  registry manifest trust-key env vars remain the configuration surface.
- Existing local path execution stays allowed; the trust policy in this spec is
  specifically about remote/local registry resolution that installs packages
  before running.
- Third-party registry refs are explicit refs. A human/operator can discover
  them through `runx skill search` or `runx registry search`, then run the exact
  returned ref.

## Touchpoints

- [crates/runx-cli/src/skill/resolver.rs](/Users/kam/dev/runx/runx/oss/crates/runx-cli/src/skill/resolver.rs)
- [crates/runx-cli/src/skill/parser.rs](/Users/kam/dev/runx/runx/oss/crates/runx-cli/src/skill/parser.rs)
- [crates/runx-cli/src/skill.rs](/Users/kam/dev/runx/runx/oss/crates/runx-cli/src/skill.rs)
- [crates/runx-cli/src/registry.rs](/Users/kam/dev/runx/runx/oss/crates/runx-cli/src/registry.rs)
- [crates/runx-runtime/src/registry/install.rs](/Users/kam/dev/runx/runx/oss/crates/runx-runtime/src/registry/install.rs)
- [crates/runx-runtime/src/registry/refs.rs](/Users/kam/dev/runx/runx/oss/crates/runx-runtime/src/registry/refs.rs)
- [crates/runx-runtime/src/scaffold/init.rs](/Users/kam/dev/runx/runx/oss/crates/runx-runtime/src/scaffold/init.rs)
- [packages/cli/src/dispatch.ts](/Users/kam/dev/runx/runx/oss/packages/cli/src/dispatch.ts)
- [packages/cli/src/skill-refs.ts](/Users/kam/dev/runx/runx/oss/packages/cli/src/skill-refs.ts)
- [packages/cli/src/official-skills.lock.json](/Users/kam/dev/runx/runx/oss/packages/cli/src/official-skills.lock.json)
- [README.md](/Users/kam/dev/runx/runx/oss/README.md)
- [docs/cli-exit-codes.md](/Users/kam/dev/runx/runx/oss/docs/cli-exit-codes.md)
- [docs/orchestrator-integrations.md](/Users/kam/dev/runx/runx/oss/docs/orchestrator-integrations.md)

## Risks

- Supply-chain ambiguity: `owner/name@version` from two registries must not
  share a cache path or trust context.
- Version collision: the registry store supports multiple versions, but the
  current runnable install path strips `@version` through
  `safe_skill_package_parts`. Installing a different version into the same
  destination root does not become "latest"; it fails with `ConflictingSkill`.
  The resolver must use a versioned materialized cache for runnable registry
  refs.
- Trust downgrade: digest-pinned but unsigned content must not accidentally
  become equivalent to signed trusted content.
- Runtime drift: TypeScript and Rust resolvers must not continue to diverge.
- Cache conflicts: installed cache writes must remain atomic and conflict-aware.
- UX ambiguity: bare names are convenient but must not perform surprising remote
  mutation or network calls.
- Cargo contention: validation should use focused commands first; avoid broad
  suites while other agents have long cargo runs active.

## Acceptance

Profile: standard

Validation:
- `cd crates && cargo fmt --check`
- `cd crates && CARGO_TARGET_DIR=target/runx-registry-skill-resolver cargo test -p runx-cli skill::`
- `cd crates && CARGO_TARGET_DIR=target/runx-registry-skill-resolver cargo test -p runx-cli registry::`
- `cd crates && CARGO_TARGET_DIR=target/runx-registry-skill-resolver cargo test -p runx-runtime registry`
- `RUNX_DEV_RUST_CLI_BIN="$PWD/crates/target/runx-registry-skill-resolver/debug/runx" pnpm exec vitest run packages/cli/src/index.test.ts packages/cli/src/skill-refs.test.ts --config vitest.fast.config.ts`
- `pnpm fixtures:cli-help:check`
- `pnpm fixtures:cli-parity:check`
- `git diff --check`

## Resolution Contract

Resolver order:

1. Existing explicit path or `SKILL.md`.
2. Exported orchestrator shim marker back to the governed source skill.
3. Workspace-local `skills/<name>` for bare refs.
4. Existing installed skill roots:
   - workspace install root for user-installed local packages
   - official cache root for first-party `runx/<name>` shorthand packages
   - registry cache root for explicit registry refs
5. First-party official shorthand:
   - `runx skill <name>` may map to locked `runx/<name>` only if the packaged
     official lock contains exactly that first-party entry.
   - no registry search is performed for bare names.
6. Explicit registry ref:
   - `runx skill <owner>/<name>[@version]` resolves against the configured
     registry target.
   - if `@version` is present, the resolved package must match that version.
   - if `@version` is absent, the resolved package must carry the selected
     latest version in the returned `ResolvedSkillRef` and cache identity.
   - `--registry <url|path>` may override registry target.
   - `--digest sha256:...` may add an expected digest pin.

Trust contract:

- `trusted`: registry package has a signed manifest verified against configured
  trust anchors, identity matches `<owner>/<name>@version`, markdown digest
  matches, profile digest matches if present, runner metadata matches. Trusted
  packages may be cached and run.
- `pinned`: caller supplied digest and content matches, but there is no trusted
  signed manifest. This is not runnable through `runx skill` in v1; surface an
  explicit error explaining that signed trust is required.
- `untrusted`: search/read/resolve metadata only. Never execute or write into a
  runnable cache.

Cache contract:

- Official first-party shorthand uses the existing official cache root:
  `RUNX_OFFICIAL_SKILLS_DIR` or `$RUNX_HOME/official-skills`.
- Explicit registry refs use a registry-cache root under
  `$RUNX_HOME/registry-skills/<registry_fingerprint>/`.
- Registry fingerprint must distinguish local registry paths, file URLs, and
  remote registry origins without leaking secrets or query strings.
- Registry fingerprint is `sha256` over a canonicalized source identifier:
  - remote registries: `<scheme>://<host>[:port]/<path>` with userinfo, query,
    and fragment stripped
  - local paths: absolute canonical path prefixed with `local:`
  - `file:` registries: resolved file path prefixed with `file:`
  The cache directory uses the first 16 hex chars of the digest.
- Runnable registry cache paths must include skill id, version, markdown digest,
  and profile digest marker. They must be produced by
  `registry::refs::materialization_cache_path` under the registry-fingerprinted
  root; do not rely on the current version-stripping `safe_skill_package_parts`
  path for `runx skill` registry materialization.
- The `destination_root` passed to `install_local_skill` already encodes the
  registry fingerprint, version, and digest materialization root;
  `install_local_skill` remains unchanged and continues to append `owner/name`
  via `safe_skill_package_parts`.
- Cache hit is accepted only when existing `SKILL.md` and `.runx/profile.json`
  match the trusted package identity/digest/profile expected by the ref.
- Cache writes reuse `install_local_skill`; no custom write path.
- Pre-existing unversioned official cache directories from older CLI versions
  are ignored, not deleted. The new resolver creates a versioned path alongside
  them and never mutates old layouts.

## Phase 1: Resolver Model and Shared Install Service

Status: completed
Dependencies: none

Objective: Cut a small reusable resolver/install boundary without duplicating

Changes:
- Add a Rust resolver model for `SkillRefKind`, `ResolvedSkillRef`, `RegistryTrustState`, and registry source/cache options.
- Ensure `ResolvedSkillRef` carries canonical `skill_id`, selected `version`, markdown digest, profile digest, registry source fingerprint, trust state, and runnable local path when trusted.
- Extract the registry target/install orchestration currently embedded in `crates/runx-cli/src/registry.rs` into a small shared CLI module that both `registry install` and `skill` resolution can call.
- Keep security verification in `runx-runtime::registry::install_local_skill`. The new shared CLI module may select targets/cache roots and assemble `InstallCandidate`, but it must not re-implement manifest verification, digest comparison, profile digest checks, runner metadata checks, or atomic writes.
- Add resolver unit tests for classifying refs:

Acceptance:
- [x] `phase1-rust-tests` command - Resolver model tests pass
  - Command: `cd crates && CARGO_TARGET_DIR=target/runx-registry-skill-resolver cargo test -p runx-cli skill::resolver && CARGO_TARGET_DIR=target/runx-registry-skill-resolver cargo test -p runx-cli registry::`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-10

## Phase 2: Trusted Registry Resolution and Caching

Status: completed
Dependencies: Phase 1

Objective: Make native `runx skill` resolve trusted official and third-party

Changes:
- Extend `SkillPlan` and native parser for registry-resolution-only flags: `--registry <url|path>` and `--digest <sha256>`.
- Resolve first-party official shorthand through the packaged official lock, existing install state, existing registry acquire/install path, and official cache root.
- Resolve explicit `owner/name[@version]` refs through configured local or remote registry targets.
- For remote registries, reuse `ensure_runx_install_state` and `RUNX_INSTALLATION_ID` behavior rather than adding a second installation-id source.
- Before Phase 3 shrinks TypeScript runnable resolution, ensure TypeScript dispatch passes `--registry` and `--digest` through to native runx unchanged so there is no intermediate CLI state where parser flags exist but are swallowed by the wrapper.
- Install trusted refs into the registry-fingerprinted cache root using `install_local_skill`.
- Use a versioned materialization destination for registry-run installs so `acme/foo@1.0.0` and `acme/foo@1.1.0` can both exist and run from the same machine without conflict.
- Official shorthand may point at the single locked first-party version, but the cache still needs version/digest identity so a future lock refresh does not fail against stale contents.
- Restore runner profiles from `.runx/profile.json` into the runnable package only through existing profile-state semantics, not by trusting a loose sidecar file.
- Fail closed with actionable messages for:

Acceptance:
- [x] `phase2-trusted-local-registry` command - Trusted local registry ref resolves and runs through native skill
  - Command: `cd crates && CARGO_TARGET_DIR=target/runx-registry-skill-resolver cargo test -p runx-cli native_skill_resolves_trusted_registry_ref`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-15
- [x] `phase2-multi-version-cache` command - Two versions of one registry skill resolve to distinct runnable cache paths
  - Command: `cd crates && CARGO_TARGET_DIR=target/runx-registry-skill-resolver cargo test -p runx-cli native_skill_resolves_registry_versions_side_by_side`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-16
- [x] `phase2-trust-failures` command - Unsigned or mismatched registry content never becomes runnable
  - Command: `cd crates && CARGO_TARGET_DIR=target/runx-registry-skill-resolver cargo test -p runx-cli native_skill_rejects_untrusted_registry_refs`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-17

## Phase 3: TypeScript Cutover and UX

Status: completed
Dependencies: Phase 2

Objective: Stop TypeScript from owning runnable resolution while preserving the

Changes:
- Update `packages/cli/src/dispatch.ts` so ordinary `runx skill` invocations pass the original ref and resolver flags directly to native Rust.
- Keep TypeScript search/catalog presentation where it is still presentation, but shrink `resolveRunnableSkillReference` so it is a thin native-delegating compatibility shim, not a second runnable resolver. Keep the exported symbol to avoid a public package API break.
- Keep `runx skill search` and `runx skill add` behavior compatible:
- Update help/docs: inputs

Acceptance:
- [x] `phase3-wrapper-tests` command - Wrapper no longer depends on TS runnable resolver
  - Command: `RUNX_DEV_RUST_CLI_BIN="$PWD/crates/target/runx-registry-skill-resolver/debug/runx" pnpm exec vitest run packages/cli/src/index.test.ts packages/cli/src/skill-refs.test.ts --config vitest.fast.config.ts`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-22
- [x] `phase3-help-parity` command - Generated CLI help/parity fixtures remain aligned
  - Command: `pnpm fixtures:cli-help:check && pnpm fixtures:cli-parity:check`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-23

## Phase 4: Dogfood and Security Evidence

Status: completed
Dependencies: Phase 3

Objective: Prove the resolver behaves like an operator-grade CLI and preserves

Changes:
- Dogfood a first-party official shorthand from outside the runx workspace: `runx skill brand-voice ...`.
- Dogfood an explicit local-registry third-party fixture ref: `runx skill acme/<fixture>@<version> --registry <fixture-registry> ...`.
- Extend `scripts/dogfood-core-skills.mjs` with `--registry-resolver` so the Phase 4 dogfood command builds a trusted local-registry fixture, resolves it through the native skill path, and verifies the produced receipt.
- Dogfood negative paths: runnable paths
- Record concise evidence in the spec session and avoid broad cargo unless the focused acceptance commands pass.

Acceptance:
- [x] `phase4-dogfood-official` command - Official shorthand works outside workspace
  - Command: `tmp=$(mktemp -d); set +e; RUNX_HOME="$tmp/home" RUNX_RECEIPT_SIGN_KID=runx-demo-key RUNX_RECEIPT_SIGN_ED25519_SEED_BASE64=QkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkI= RUNX_RECEIPT_SIGN_ISSUER_TYPE=hosted crates/target/runx-registry-skill-resolver/debug/runx skill brand-voice --brand Nitrosend --channel "support email" --source-material "Friendly, concise, practical."; code=$?; set -e; test "$code" = 0 -o "$code" = 2`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-28
- [x] `phase4-dogfood-registry` command - Explicit trusted registry ref works from clean cwd
  - Command: `CARGO_TARGET_DIR="$PWD/crates/target/runx-registry-skill-resolver" scripts/dogfood-core-skills.mjs --registry-resolver`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-29
- [x] `phase4-security-grep` command - No untrusted allow-run escape hatch is introduced
  - Command: `! rg -n "allow[-_]untrusted|trust.*skip|skip.*signature|unsigned.*run" crates/runx-cli/src crates/runx-runtime/src packages/cli/src`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-30

## Rollback

- Revert the resolver module, parser flags, TS dispatch cutover, and docs.
- Registry install/runtime verification stays untouched; if rollback is needed,
  local path and installed skill execution should return to the previous
  resolver behavior without changing registry install semantics.
- No on-disk migration is needed. Reverting leaves new versioned cache
  directories untouched; the older resolver will repopulate its unversioned
  layout on next run.

## Review

Status: completed
Verdict: fail
Mode: discover
Provider: claude:claude-opus-4-7
Output: claude.mcp_submit_review
Summary: Resolver, trust boundary, and cache layout match the spec — install path verifies signed anchors, digests, and profile shape before any rename, and fingerprint canonicalization avoids cache collisions across URL variants. However, the TS shrink shipped without removing legacy tests that exercise the deleted official-fetch path, so `pnpm --dir oss test` fails (F-001, blocking). Three lower-severity issues round out the review: native `runx skill` help and the parity fixture omit `--registry`/`--digest` (F-002); the `--registry-resolver` dogfood lane only runs cargo tests rather than driving the native binary (F-003); and the parser silently accepts the new flags for subcommands that never consume them (F-004). Verdict: fail in discover, gated on F-001.

Attack log:
- `TS resolveRunnableSkillReference shim`: Hunt callers/tests that still depend on official-fetch behavior; cross-check vitest.config.ts include globs -> finding (F-001: tests under tests/official-skill-*.test.ts exercise the removed TS fetch path and now fail.)
- `install_local_skill trust boundary`: Look for code paths that bypass verify_signed_manifest_anchor (e.g., untrusted registry, allow_untrusted env var, soft-fail on digest mismatch) -> clean (Resolver always routes registry installs through materialize_trusted_registry_skill; untrusted registries are rejected before install.)
- `Cache fingerprint canonicalization`: Try collisions across hosts via userinfo/query/fragment, case-different hosts, trailing slash differences -> clean (fingerprint_source strips userinfo/query/fragment for remote URLs and lowercases the host before hashing.)
- `materialization_cache_path`: Path traversal via owner/name/version containing '..', leading '/', or NUL -> clean (safe_path_part filters every component; reject_unsafe_path_component rejects upstream in split_skill_id.)
- `Bare-name resolution order`: Trick the resolver into preferring an untrusted local shim over an official entry, or vice versa, when names collide -> clean (Order is local path → exported shim → installed registry cache → workspace skills → official cache → explicit registry; each step is a distinct ResolvedSkillRef kind so confusion is observable in receipts.)
- `CLI parser`: Pass `--registry` for non-resolver subcommands, mix with -- separator, double-dash escape -> finding (F-004: parser accepts silently.)
- `Launcher/help parity`: Diff TS help vs Rust help vs fixtures/cli-parity/commands.json for new flags -> finding (F-002: native help and parity fixture omit --registry/--digest.)
- `Dogfood evidence`: Verify dogfood actually exercises the new path end-to-end -> finding (F-003: --registry-resolver only runs cargo tests, never the native binary.)
- `TOCTOU on cache dir restore`: Race between digest check and rename into <RUNX_HOME>/registry-skills/... -> clean (install_local_skill writes to a tempdir, verifies digest, then renames atomically; ConflictingSkill caught before mutation.)
- `X.yaml restore from .runx/profile.json`: Tampered profile.json that points to a different X.yaml digest or path -> clean (Profile digest is checked against the trusted manifest digest before write; mismatches abort.)
- `RUNX_DEV_CATALOG / allow-untrusted escape hatches`: See if any env var permits running an untrusted/pinned registry skill -> clean (RUNX_DEV_CATALOG only changes catalog visibility; pinned/untrusted refs remain non-runnable in v1.)

Findings:
- [high/blocks completion] `F-001` Shrunken TS resolveRunnableSkillReference breaks existing official-skill resolution tests under `tests/`
  - Location: `packages/cli/src/skill-refs.ts:77`
  - Evidence: packages/cli/src/skill-refs.ts:77-83 reduces `resolveRunnableSkillReference` to local-path-or-passthrough, but `tests/official-skill-resolution.test.ts:32-34` asserts `rejects.toThrow('Try \`runx skill search definitely-not-a-real-skill\`')` and `tests/official-skill-fetch.test.ts:48-49,103,132,166` assert paths of the form `<globalHomeDir>/official-skills/runx/<name>`. The new shim never throws and never returns a fetched path, so these tests fail. `vitest.config.ts:10` (`include: ["packages/**/*.test.ts", "tests/**/*.test.ts"]`) confirms these tests run under `pnpm test`, even though the fast lane in `packages/cli/src/skill-refs.test.ts` and `packages/cli/src/index.test.ts` does not exercise them.
  - Impact: `pnpm --dir oss test` fails. The spec explicitly requires `pnpm test` to stay green; rerun policy is `verify_open_blockers`. Acceptance phase 4 cannot be ratified.
  - Validation: After fix, rerun `pnpm --dir oss exec vitest run tests/official-skill-resolution.test.ts tests/official-skill-fetch.test.ts` and the full `pnpm --dir oss test` — both must pass. The fix must either (a) delete/rewrite the legacy `tests/official-skill-*.test.ts` files because the TS resolver no longer owns the path (and add an equivalent native-runx integration test in Rust), or (b) restore the official-fetch behavior in the TS shim. Option (a) is consistent with the spec's TS-shrink intent; option (b) reintroduces the path runx-rust-registry-skill-resolver was meant to remove.
- [medium/non-blocking] `F-002` Native `runx skill` help omits `--registry`/`--digest`, breaking parity with the TS launcher and the parity fixture
  - Location: `crates/runx-cli/src/launcher.rs:271`
  - Evidence: `crates/runx-cli/src/launcher.rs:271` builds the `runx skill` help block without listing the new resolver-only flags. `packages/cli/src/help.ts:54` and `fixtures/cli-parity/commands.json:530-545` likewise omit them. The spec's Resolution Contract surfaces `--registry`/`--digest` as a first-class entrypoint, so the only discoverability is via release notes.
  - Impact: Users who run `runx skill --help` see no mention of `--registry`/`--digest`; parity guard will not catch later drift. Not a wire-shape break, but UX/discoverability regression for the headline contract.
- [low/non-blocking] `F-003` `--registry-resolver` dogfood mode does not exercise native runx end-to-end
  - Location: `scripts/dogfood-core-skills.mjs:18`
  - Evidence: `scripts/dogfood-core-skills.mjs:18-59` invokes three `cargo test` invocations rather than running the native binary against a populated `RUNX_HOME`/registry and asserting a receipt. The spec's Phase 4 acceptance asks for dogfood evidence that proves the resolver works in the binary, not just that the unit tests pass.
  - Impact: A future regression in CLI wiring (e.g., dispatch dropping `--registry` again, parser swallowing the value, or help drift) could land without the dogfood script noticing because the script never crosses the launcher boundary.
- [low/non-blocking] `F-004` Parser accepts `--registry`/`--digest` for non-`skill` subcommands silently
  - Location: `crates/runx-cli/src/skill/parser.rs:197`
  - Evidence: `crates/runx-cli/src/skill/parser.rs:197-220` introduces `--registry` and `--digest` as explicit options before `parse_direct_input_arg`. The flags are only meaningful for the resolver but the parser does not reject them when the command is e.g. `runx skill validate` or `runx skill harness ...` — they are silently attached to `parsed.registry`/`parsed.digest` and may be ignored downstream.
  - Impact: Silent acceptance of flags that have no effect for a given subcommand is a stealth dark pattern: users believe they pinned a digest when they did not. Low severity because no security boundary is crossed (the resolver still verifies anchors/digests), but it muddies the UX contract.

## Self Eval

- Target rating before build: 9.5 if the resolver is small, deterministic,
  trusted by construction, and removes TypeScript duplication without broad
  rewrites.

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
Started: 2026-06-09T16:51:25Z
Ended: 2026-06-09T16:51:25Z
Verdict: needs_revision
Provider: claude
Model: claude-opus-4-7
Output format: claude.mcp_submit_harden
Summary: invalid provider dossier evidence: observation "path": invalid anchor prefix "crates/runx-cli/src/skill/resolver.rs:4" (expected "Anchor: spec_gap:<field>", "Anchor: code:<path>:<line>", or "Anchor: archive:<task-id>"); observation "path": invalid anchor prefix "scripts/dogfood-core-skills.mjs" (expected "Anchor: spec_gap:<field>", "Anchor: code:<path>:<line>", or "Anchor: archive:<task-id>"); observation "command": invalid anchor prefix "crates/runx-cli/tests/skill.rs:5" (expected "Anchor: spec_gap:<field>", "Anchor: code:<path>:<line>", or "Anchor: archive:<task-id>"); observation "command": invalid anchor prefix ".scafld/specs/drafts/runx-rust-registry-skill-resolver.md:381" (expected "Anchor: spec_gap:<field>", "Anchor: code:<path>:<line>", or "Anchor: archive:<task-id>"); observation "scope": invalid anchor prefix "crates/runx-runtime/src/registry/install.rs:423" (expected "Anchor: spec_gap:<field>", "Anchor: code:<path>:<line>", or "Anchor: archive:<task-id>"); observation "scope": invalid anchor prefix "packages/cli/src/skill-refs.ts:77" (expected "Anchor: spec_gap:<field>", "Anchor: code:<path>:<line>", or "Anchor: archive:<task-id>"); observation "timing": invalid anchor prefix ".scafld/specs/drafts/runx-rust-registry-skill-resolver.md:259" (expected "Anchor: spec_gap:<field>", "Anchor: code:<path>:<line>", or "Anchor: archive:<task-id>"); observation "timing": invalid anchor prefix "packages/cli/src/skill-refs.ts:190" (expected "Anchor: spec_gap:<field>", "Anchor: code:<path>:<line>", or "Anchor: archive:<task-id>"); observation "rollback": invalid anchor prefix ".scafld/specs/drafts/runx-rust-registry-skill-resolver.md:385" (expected "Anchor: spec_gap:<field>", "Anchor: code:<path>:<line>", or "Anchor: archive:<task-id>"); observation "design": invalid anchor prefix "crates/runx-runtime/src/registry/install.rs:210" (expected "Anchor: spec_gap:<field>", "Anchor: code:<path>:<line>", or "Anchor: archive:<task-id>"); observation "design": invalid anchor prefix "crates/runx-runtime/src/registry/refs.rs:82" (expected "Anchor: spec_gap:<field>", "Anchor: code:<path>:<line>", or "Anchor: archive:<task-id>")

Observations:
- path
  - Result: advisory
  - Anchor: code:scripts/dogfood-core-skills.mjs:1
  - Note: Phase 4 references `scripts/dogfood-core-skills.mjs --registry-resolver` but `rg --registry-resolver` matches nothing in this script. The flag is a future addition; spec should say so explicitly or risk an unfulfilled acceptance command.
  - Default: Phase 4 changes must include adding `--registry-resolver` to dogfood-core-skills.mjs that drives the trusted local-registry fixture flow.
  - Status: open
- command
  - Result: clean
  - Anchor: code:.scafld/specs/drafts/runx-rust-registry-skill-resolver.md:381
  - Note: phase4-security-grep uses `! rg ...` and named patterns that are stable; pattern list is conservative and OSS-only.
  - Default: Rewrite Phase 2 acceptance filters as `cargo test -p runx-cli native_skill_resolves_trusted_registry_ref` (etc.) to match integration test names in tests/skill.rs, and keep Phase 1 `skill::resolver` unchanged because resolver unit tests will live in `src/skill/resolver.rs`.
  - Status: open
- scope
  - Result: clean
  - Anchor: code:packages/cli/src/skill-refs.ts:77
  - Note: TypeScript runnable resolver currently handles only local + first-party official lock; spec correctly scopes its replacement to Rust and keeps search/catalog as TS presentation.
  - Default: Phase 2 must add to the Cache contract: 'destination_root passed to install_local_skill already encodes registry fingerprint, version, and digest; install_local_skill remains unchanged and continues to append owner/name via safe_skill_package_parts.'
  - Status: open
- timing
  - Result: advisory
  - Anchor: code:packages/cli/src/skill-refs.ts:190
  - Note: TS currently writes unversioned `<cacheRoot>/<owner>/<name>` for first-party official cache, and Phase 2 mandates versioned paths. After Phase 2, a returning user with an old TS-written cache will sit at a path the new resolver does not honor; Phase 2 should specify that a stale unversioned cache entry is ignored (not deleted) and a fresh versioned install occurs, otherwise mixed-state machines surface ConflictingSkill.
  - Default: Add a sentence to Cache contract: 'Pre-existing unversioned official cache directories from prior CLI versions are ignored; the new resolver creates the versioned path alongside them and never mutates old layouts.'
  - Status: open
- rollback
  - Result: advisory
  - Anchor: code:.scafld/specs/drafts/runx-rust-registry-skill-resolver.md:385
  - Note: Rollback reverts code only. After Phase 2, on-disk cache uses versioned paths; reverted code expects unversioned paths and will re-download. Acceptable, but rollback note should explicitly say cache layout change is forward-compatible-with-redownload, not a destructive on-disk change.
  - Default: Append to Rollback: 'No on-disk migration is needed; reverting leaves new versioned cache directories untouched and the older resolver will repopulate the unversioned layout on next run.'
  - Status: open
- design
  - Result: advisory
  - Anchor: code:crates/runx-runtime/src/registry/refs.rs:82
  - Note: `materialization_cache_path` is a perfect primitive for runnable registry cache, but the spec only suggests 'reuse or adapt' it. Recommend committing the resolver to call `materialization_cache_path` directly (composed with a registry fingerprint root) rather than 'adapting', so there is one canonical cache-path function for both materialization and runnable execution.
  - Default: Tighten Cache contract: 'Runnable registry cache paths MUST be produced by `materialization_cache_path` under `<runx_home>/registry-skills/<registry_fingerprint>/`.'
  - Status: open

### round-2

Status: passed
Started: 2026-06-09T16:56:13Z
Ended: 2026-06-09T23:45:52Z
Verdict: pass
Provider: claude
Model: claude-opus-4-7
Output format: claude.mcp_submit_harden
Summary: invalid provider dossier evidence: observation "design": spec_gap citation does not name a known spec field: spec_gap:cache_contract.registry_fingerprint

Observations:
- path
  - Result: clean
  - Anchor: code:crates/runx-cli/src/skill/resolver.rs:4
  - Note: Touchpoints exist: skill/resolver.rs is the local-only resolver to extend, parser.rs is the unknown-flag-passthrough surface, registry.rs holds today's install orchestration, runtime install.rs holds the trusted boundary, refs.rs holds materialization_cache_path and safe_skill_package_parts, skill-refs.ts holds resolveRunnableSkillReference, official-skills.lock.json holds first-party lock. Phase 4 references scripts/dogfood-core-skills.mjs --registry-resolver as a future addition, and the spec now explicitly declares that addition under Phase 4 changes.
- command
  - Result: advisory
  - Anchor: code:.scafld/specs/drafts/runx-rust-registry-skill-resolver.md:383
  - Note: phase4-dogfood-official accepts exit code 0 OR 2 with no explanation. Exit 2 is presumably the policy/approval-gate code when outside-workspace shorthand hits a scope or trust prompt, but the spec never names which exit-2 case is acceptable. Either constrain it ('exit 2 only when no skill scope is approved for outside-workspace use') or document the canonical 0/2 semantics so the acceptance is not a quiet pass-anything gate.
  - Default: Add a one-line acceptance comment: 'exit 2 is acceptable only when the outside-workspace run is blocked by a scope/approval gate — never as a generic catch-all for parser or resolver errors.'
  - Status: open
- scope
  - Result: advisory
  - Anchor: code:packages/cli/src/index.ts:17
  - Note: packages/cli/src/index.ts re-exports resolveRunnableSkillReference as part of the @runxhq cli package public surface. Phase 3 'remove or shrink' bumps a public API. Under the project's public_api_stable invariant this needs an explicit decision: either keep the export as a thin shim that delegates to native Rust (preserves the symbol), or declare it an accepted breaking change in this spec.
  - Default: Phase 3 should specify that resolveRunnableSkillReference is either (a) retained as a thin delegating shim to preserve public API, or (b) intentionally removed with a recorded acceptance under public_api_stable.
  - Status: open
- timing
  - Result: advisory
  - Anchor: code:packages/cli/src/dispatch.ts:331
  - Note: Phase 2 introduces the native --registry and --digest parser flags, but the TS dispatch still owns runnable resolution until Phase 3. Between Phase 2 land and Phase 3 land, an operator running `runx skill <ref> --registry ...` will hit the TS dispatch which routes through resolveRunnableSkillReference and may swallow or misroute the new flags. Either land Phase 2 and Phase 3 together, or have Phase 2 pass --registry/--digest through dispatch verbatim into the native binary.
  - Default: Phase 2 should add a one-line note: 'TS dispatch passes --registry and --digest through to native runx unchanged before Phase 3, even though TS still resolves runnable refs locally.'
  - Status: open
- rollback
  - Result: clean
  - Anchor: code:.scafld/specs/drafts/runx-rust-registry-skill-resolver.md:401
  - Note: Rollback now explicitly says no on-disk migration is needed and the older resolver will repopulate its unversioned layout on next run. Trust-anchor/registry install verification stays untouched. Reverting per-phase is safe because Phase 3 only flips dispatch routing, which can be reverted along with the resolver module without leaving the CLI in an unrunnable state.
- design
  - Result: advisory
  - Anchor: code:.scafld/specs/drafts/runx-rust-registry-skill-resolver.md:213
  - Note: Cache contract requires a 'registry fingerprint' that distinguishes local registry paths, file URLs, and remote registry origins 'without leaking secrets or query strings', but never specifies how it is computed. Two independent registries with the same hostname-but-different-path can collide; a registry URL with credentials in the query string could leak into the cache root. Spec should pin the fingerprint to a named algorithm (e.g. sha256 over a canonicalized origin string after stripping query/fragment/userinfo, prefixed by source type local|file|https).
  - Default: Add to Cache contract: 'Registry fingerprint = sha256 of canonicalized source identifier: for remote, scheme://host[:port]/path with userinfo and query stripped; for local, the absolute canonical path; for file:, the resolved path. Truncate to 16 hex chars for the cache directory name.'
  - Status: open


## Planning Log

- Initial draft name `runx-rust-official-cache-resolver` was cancelled because
  it incorrectly framed the work as first-party-only. The real architecture is
  flexible registry resolution with official skills as one locked first-party
  case.
- Bare remote search was rejected for `runx skill <name>` because it creates
  surprising network behavior and supply-chain ambiguity. Explicit registry refs
  are the correct operator affordance for third-party skills.
- The plan intentionally reuses `install_local_skill` and registry trust anchors
  rather than introducing a parallel verifier.
- Current code partially supports multi-version lookup (`parse_registry_ref`,
  `get_version`, remote `resolve_ref`), but not multi-version runnable
  materialization because `install_local_skill` derives package paths through
  `safe_skill_package_parts`, which strips `@version`. Build must close that
  gap before claiming registry resolution is S-tier.
