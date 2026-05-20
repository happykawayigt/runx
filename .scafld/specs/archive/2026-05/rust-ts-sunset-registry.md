---
spec_version: '2.0'
task_id: rust-ts-sunset-registry
created: '2026-05-18T00:00:00Z'
updated: '2026-05-20T05:23:56Z'
status: failed
harden_status: passed
size: medium
risk_level: high
---

# TS sunset: registry (core domain)

## Current State

Status: failed
Current phase: final
Next: inspect failure
Reason: fail
Blockers: fail
Allowed follow-up command: `scafld handoff rust-ts-sunset-registry`
Latest runner update: 2026-05-20T05:15:15Z
Review gate: not_started

## Summary

Delete the TypeScript registry core domain and its public subpath export:
`packages/core/src/registry/**` and `@runxhq/core/registry`. Registry IO becomes
Rust-owned through `crates/runx-runtime/src/registry/`; TS does not retain compat
shims, legacy emitted registry shapes, or a second-version registry surface.

Product registry names stay product names. Keep registry skill names, package
path names, owner/name refs, install refs, `skill_id`, `trust_tier`, and hosted
registry route fields where they are the registry product surface. Do not carry
old receipt or harness wording forward just to keep TS fixtures compiling.

This spec only sunsets OSS-side TS registry code. Hosted cloud registry routes,
namespace policy, and publisher authorization stay in the cloud package.

## Context

CWD: `.`

Packages:
- `@runxhq/core`
- `@runxhq/cli`
- `@runxhq/runtime-local`
- `crates/runx-runtime` (`registry` module owns registry IO and install logic)
- `cloud/packages/api` (registry HTTP routes; not touched)

Current TypeScript sources:
- `packages/core/src/registry/**` (to be deleted)
- `packages/runtime-local/src/runner-local/registry-resolver.ts`
- `packages/runtime-local/src/runner-local/skill-install.ts`
- `packages/runtime-local/src/runner-local/official-cache.ts`
- CLI dispatch, skill refs, and tests that still import `@runxhq/core/registry`

Files impacted:
- `packages/core/src/registry/` (deleted)
- `packages/core/package.json` (`"./registry"` export removed)
- TS importers only as needed to stop using `@runxhq/core/registry`
- Registry fixture/test references only as needed to point at
  `runx-runtime::registry`

Invariants:
- Rust source of truth: all registry search, read, acquire, resolve, and local
  install semantics are owned by `crates/runx-runtime/src/registry/`.
- Hosted registry HTTP behavior is unchanged. This sunset consumes the surface
  already implemented by `runx-runtime::registry`; it does not add endpoints,
  negotiate a new version, or invent fallback payloads.
- There is no `@runxhq/core/registry` re-export, proxy module, TS wrapper,
  cross-language adapter, compat shim, legacy shape emitter, or `/v2` registry
  path.
- Trust tiers (`first_party`, `verified`, `community`) remain exact
  server-provided values. TS deletion must not reintroduce owner-derived trust
  logic.
- Registry install package paths keep the `runx-runtime::registry`
  normalization: namespaced refs derive owner/name path components; bare refs
  derive from the skill name.
- Direct `runx skill add` / `runx skill add` remains a local install action. When
  registry install happens inside execution, evidence belongs to the enclosing
  sealed harness receipt or runtime ledger metadata. It does not emit retired
  `skill_execution` or `graph_execution` receipt shapes.
- Harness assertions use harness-spine terms: harness receipt refs, sealed
  receipt state, contained decisions, contained acts, artifact refs, signal
  refs, proof status, and verification checks.

## Objectives

- Prove every live registry IO caller has moved to
  `crates/runx-runtime/src/registry/` or a Rust-owned launcher/runtime boundary
  before deleting TS.
- Delete `packages/core/src/registry/**` and remove the `@runxhq/core/registry`
  package export.
- Remove or port TS tests whose only purpose was to validate the deleted TS
  registry implementation.
- Preserve product-facing registry behavior: search, inspect/read, acquire,
  bare-ref resolution, idempotent local install, profile binding validation,
  trust tier round-trip, and package path derivation.
- Keep registry install evidence in harness-spine vocabulary when it is part of
  an execution.

## Scope

In scope:
- TS registry core deletion and subpath export removal.
- Importer cleanup for OSS packages that still reference `@runxhq/core/registry`.
- Test and fixture cleanup required to assert the Rust registry client is the
  only live registry IO implementation.

Out of scope:
- Cloud-side registry routes / logic.
- Hosted namespace ownership and publisher authorization policy.
- Registry signing / attestation hierarchy beyond pass-through validation that
  already belongs to `runx-runtime::registry`.
- Adding a TS-to-Rust compatibility layer.
- Adding a second registry API version.
- Changing product registry skill names, owner/name refs, or install package
  names solely to satisfy the sunset.

## Dependencies

- `rust-ts-sunset-receipts`.
- `runx-runtime::registry` completed and handed off, accepted as the source for
  registry search, read, acquire, resolve, and local install.
- `rust-harness` or equivalent harness-spine receipt support completed before
  any registry install evidence is used as cutover proof.

## Sequencing

1. Finish `runx-runtime::registry` first. The runtime module must expose and
   test the surfaces currently represented by `RegistryClient`, `RegistryStore`,
   `resolveRegistrySkill`, `acquireRegistrySkill`, `materializeRegistrySkill`,
   and local skill install helpers.
2. Confirm CLI/runtime registry callers use the Rust client path. This includes
   search/add/inspect/publish/list dispatch, graph registry refs, official skill
   cache acquisition, and runtime materialization.
3. Run an importer census. Every live `@runxhq/core/registry` import must be
   removed, replaced by a Rust-owned call path, or deleted with the TS test it
   only supported.
4. Delete `packages/core/src/registry/**`, remove the `./registry` export from
   `packages/core/package.json`, and remove stale build references.
5. Refresh tests and fixtures to assert Rust behavior. Registry HTTP payload
   fixtures may still contain product fields such as `owner`, `skill_id`,
   `source_type`, `trust_tier`, and `install_command`; receipt fixtures must use
   harness-spine vocabulary only.
6. Run the full acceptance command set before approval. If a command needs new
   Rust integration that is not yet present, stop and return to
   `runx-runtime::registry` rather than adding a TS shim here.

## 2026-05-20 Rust Local Registry Parity Slice

Implemented in `crates/runx-runtime/src/registry/`:
- `FileRegistryStore` with `put_version`, `get_version`, `list_versions`, and
  `list_skills`.
- Local publish/ingest helpers for skill markdown and optional profile
  documents.
- Local search/read/resolve/link helpers using the same product surface fields
  as the TS registry.
- Shared registry types for versions, search results, details, links,
  publishers, source metadata, attestations, trust signals, and trust tiers.
- Bare digest acceptance in local install validation for TS-style local
  registry records as well as `sha256:` digests.
- Path traversal rejection for unsafe owner/name components (`.`, `..`, `/`,
  and `\`) before local registry path resolution.

Validation:
- `cargo check --manifest-path crates/Cargo.toml -p runx-runtime` passed.
- `cargo test --manifest-path crates/Cargo.toml -p runx-runtime --test registry`
  passed: 5 tests.
- `cargo test --manifest-path crates/Cargo.toml -p runx-runtime --test registry_client`
  passed: 11 tests.
- `cargo test --manifest-path crates/Cargo.toml -p runx-runtime registry`
  passed.
- `cargo clippy --manifest-path crates/Cargo.toml -p runx-runtime --all-targets -- -D warnings`
  passed.
- `cargo fmt --manifest-path crates/Cargo.toml --all --check` passed.

Current deletion blocker: importer migration, not Rust local registry parity.

## 2026-05-20 Importer Census

Blocking runtime/CLI boundaries:
- `packages/cli/src/dispatch.ts` creates local/HTTP cached registry stores,
  publishes, and passes registry stores into run/harness/graph flows.
- `packages/cli/src/skill-refs.ts` owns CLI registry search helpers.
- `packages/runtime-local/src/runner-local/registry-resolver.ts` materializes
  graph/runtime registry refs through TS `resolveRegistrySkill`.
- `packages/runtime-local/src/runner-local/skill-install.ts` owns TS
  acquire/resolve/install behavior for `runx skill add`.
- `packages/runtime-local/src/runner-local/official-cache.ts` acquires remote
  registry skills and synthesizes first-party cache records.
- `packages/runtime-local/src/sdk/index.ts` exposes registry store/search,
  install, and publish API surface.
- `packages/runtime-local/src/runner-local/index.ts`,
  `execution-targets.ts`, and `harness/runner.ts` carry `RegistryStore` through
  public runtime options.
- `packages/cli/src/commands/dev.ts` and `packages/cli/src/commands/mcp.ts`
  still accept registry store plumbing.

Lower-risk cleanup after the Rust boundary exists:
- `tests/util-split-skill-id.test.ts` can be deleted or replaced by Rust
  path-safety coverage.
- `tests/http-cached-registry-store.test.ts` and `tests/registry-ce.test.ts`
  can move to Rust `registry_client`/local registry tests.
- `scripts/generate-official-lock.mjs` and
  `packages/cli/src/commands/doctor-structure.ts` need a Rust registry record
  builder/validator surface.
- `apps/registry/src/skill-page.ts` should consume hosted/cloud API or
  Rust-generated product types.

Next executable registry work is not deletion. It is importer migration from
`@runxhq/core/registry` to the native `runx registry` JSON boundary for
search/add/publish/materialization/harness, followed by mechanical test/script
cleanup and only then export deletion.

## 2026-05-20 Rust CLI Registry Boundary Slice

Implemented in `crates/runx-cli`:
- Native launcher route for `runx registry` when `RUNX_RUST_CLI` is set.
- `runx registry search <query> [--registry <url-or-path>|--registry-dir <path>] [--limit n] --json`.
- `runx registry read <skill-ref> [--version v] [--registry ...] --json`.
- `runx registry resolve <skill-ref> [--version v] [--registry ...] --json`.
- `runx registry install <skill-ref> --to <dir> [--version v] [--registry ...] [--installation-id id] --json`.
- `runx registry publish <SKILL.md> [--registry-dir <path>] [--owner owner] [--version v] [--profile X.yaml] [--upsert] --json`.

Boundary behavior:
- Local registry operations call `FileRegistryStore`,
  `search_registry_with_options`, `read_registry_skill`,
  `resolve_registry_skill`, `publish_skill_markdown`, and
  `install_local_skill`.
- Remote operations call `RegistryClient` for search/read/resolve/acquire and
  use `install_local_skill` for local materialization after acquisition.
- JSON responses use the native registry command envelope:
  `{ "status": "success", "registry": { "action": "...", ... } }`.
- Existing TS registry importers were intentionally left in place; this slice is
  an executable migration target, not the TS deletion.

Validation:
- `cargo check --manifest-path crates/Cargo.toml -p runx-cli` passed.
- `cargo test --manifest-path crates/Cargo.toml -p runx-cli registry` passed.
- `cargo test --manifest-path crates/Cargo.toml -p runx-cli --test launcher registry` passed.
- `cargo fmt --manifest-path crates/Cargo.toml --all --check` passed.
- `cargo clippy --manifest-path crates/Cargo.toml -p runx-cli --all-targets -- -D warnings` passed.

## 2026-05-20 Cargo Package-Surface Audit

The previous `rust-hosted-http-foundation` and `rust-registry-client` archive
records describe standalone crates that are not present in the current
workspace. The active Rust package surface is `crates/Cargo.toml` only:
`runx-cli`, `runx-contracts`, `runx-core`, `runx-parser`, `runx-receipts`,
`runx-runtime`, and `runx-sdk`.

Current audit evidence:
- `find crates -maxdepth 2 -name Cargo.toml -print | sort` lists no
  `crates/runx-hosted-http/Cargo.toml` or
  `crates/runx-registry-client/Cargo.toml`.
- `cargo metadata --manifest-path crates/Cargo.toml --no-deps --format-version 1`
  reports the same seven workspace members and no hosted HTTP or registry
  client packages.
- `cargo package --manifest-path crates/Cargo.toml -p runx-hosted-http
  --allow-dirty` fails with `package ID specification 'runx-hosted-http' did
  not match any packages`.
- `cargo package --manifest-path crates/Cargo.toml -p runx-registry-client
  --allow-dirty` fails with `package ID specification 'runx-registry-client'
  did not match any packages`.
- `cargo tree --manifest-path crates/Cargo.toml -e normal,build,dev --prefix
  depth` shows no `reqwest`, `hyper`, or `serde_yml`; the YAML backend present
  in the lockfile is `serde_norway`, with `unsafe-libyaml-norway` transitive.
- `cargo deny --manifest-path crates/Cargo.toml check licenses bans sources`
  passes. The current `crates/deny.toml` still bans `reqwest`, `hyper`, `tokio`,
  `rmcp`, and adjacent heavy framework crates at workspace scope, so a future
  hosted HTTP or adapter-tier dependency must first update the active spec and
  deny rationale.

This clears the package-surface blocker for this sunset slice: there is no safe
obsolete-crate removal left in the working tree. The remaining blocker is still
importer migration from TS registry callers into a Rust-owned CLI/runtime
boundary. The safe next code slice is to add that boundary inside the existing
`runx-runtime::registry` / `runx-cli` surface without reintroducing
`runx-hosted-http`, `runx-registry-client`, `reqwest`, or `hyper`. If native
hosted HTTP is needed later, it requires a fresh adapter-tier exception spec
that updates `crates/deny.toml`, records the security rationale, and includes
focused cargo-deny validation.

## Acceptance Criteria

- `packages/core/src/registry/` is gone.
- `packages/core/package.json` no longer exports `./registry`.
- No live source, test, script, tool, or first-party skill imports
  `@runxhq/core/registry` or reaches into `packages/core/src/registry`.
- Registry search, read, acquire, bare-name resolution, idempotent install,
  digest/profile digest validation, runner manifest validation, and safe package
  path derivation are covered by `runx-runtime::registry` tests.
- CLI/runtime registry flows use the Rust registry client as the source for
  registry IO. TS may invoke a Rust binary or launcher boundary, but it must not
  duplicate registry semantics in TS.
- Hosted registry payload shape remains the stabilized registry product shape;
  no legacy emitted TS registry shape, compatibility adapter, or `/v2` endpoint
  is introduced.
- Product surface names remain stable: `skill_id`, owner/name refs, product
  skill names, package path names, install commands, run commands, trust tiers,
  publisher metadata, source metadata, and attestations round-trip where the
  registry surface owns them.
- Execution evidence uses harness-spine terms. No active registry cutover
  fixture asserts retired receipt fields such as `skill_execution`,
  `graph_execution`, `skill_name`, or `graph_name`.
- Rollback instructions below are documented in the implementation PR.

## Validation Commands

```sh
test ! -d packages/core/src/registry
node -e 'const pkg = require("./packages/core/package.json"); if (pkg.exports && pkg.exports["./registry"]) process.exit(1)'
! rg -n '@runxhq/core/registry|packages/core/src/registry|from "\.?\.?/.*registry/store|from "\.?\.?/.*registry/resolve' packages tests scripts tools skills --glob '!**/dist/**' --glob '!**/node_modules/**'
cargo test --manifest-path crates/Cargo.toml -p runx-runtime --test registry_client
cargo test --manifest-path crates/Cargo.toml -p runx-runtime registry
find crates -maxdepth 2 -name Cargo.toml -print | sort
cargo metadata --manifest-path crates/Cargo.toml --no-deps --format-version 1
! cargo package --manifest-path crates/Cargo.toml -p runx-hosted-http --allow-dirty
! cargo package --manifest-path crates/Cargo.toml -p runx-registry-client --allow-dirty
! cargo tree --manifest-path crates/Cargo.toml -e normal,build,dev --prefix depth | rg '^(.*)(reqwest|hyper|serde_yml) v'
cargo deny --manifest-path crates/Cargo.toml check licenses bans sources
pnpm test -- tests/graph-registry-refs.test.ts tests/graph-registry-refs.integration.test.ts tests/skill-add.test.ts tests/skill-search.test.ts tests/skill-publish.test.ts tests/registry-ce.test.ts packages/cli/src/index.test.ts
! rg -n 'skill_execution|graph_execution|skill_name|graph_name' fixtures/registry crates/runx-runtime
cargo clippy --manifest-path crates/Cargo.toml -p runx-runtime --all-targets -- -D warnings
cargo fmt --manifest-path crates/Cargo.toml --all --check
pnpm typecheck
```

## Rollback And Repair

- Pre-merge rollback is to back out the whole sunset implementation patch and
  return to the `runx-runtime::registry` blocker. Do not add an interim
  `@runxhq/core/registry` proxy to keep partial deletion alive.
- Post-merge repair is forward through `crates/runx-runtime/src/registry/`,
  `crates/runx-runtime`, or the launcher boundary. Do not resurrect TS registry
  modules, legacy emitted shapes, or a second registry version.
- If a CLI/runtime caller still needs a registry capability, add it to
  `runx-runtime::registry` with tests, then wire the caller to that Rust-owned
  capability.
- If hosted payload validation is too strict for the live registry, repair the
  Rust payload parser and fixture against the hosted surface; do not tolerate
  missing required fields in a TS fallback.
- If harness evidence is wrong, repair the harness-spine fixture/projection and
  proof checks. Do not restore retired receipt expectation fields.
- If cloud registry behavior is the issue, stop this OSS sunset and fix the
  cloud route or fixture in its owning repo/spec. This spec must not patch
  cloud-side behavior.

## Open Questions

- None at draft time.

## Harden Rounds

### round-1

Status: passed
Started: 2026-05-19T06:13:24Z
Ended: 2026-05-19T06:13:24Z
Verdict: passed
Provider: manual
Summary: Reframed the draft around `runx-runtime::registry` ownership,
explicit TS deletion, harness-spine receipt vocabulary, acceptance commands,
sequencing, and repair rules.

Checks:
- registry client alignment
  - Result: passed
  - Evidence: The spec names `runx-runtime::registry` as the source for
    registry search, read, acquire, resolve, and local install.
- sunset target
  - Result: passed
  - Evidence: The spec forbids TS compatibility shims, legacy emitted shapes,
    and a second registry version.
- vocabulary audit
  - Result: passed
  - Evidence: Registry product fields are preserved, while execution evidence
    is constrained to harness-spine receipt terms.
- acceptance coverage
  - Result: passed
  - Evidence: Acceptance criteria and validation commands cover deletion,
    import cleanup, Rust tests, TS flow tests, and retired receipt field scans.
- rollback audit
  - Result: passed
  - Evidence: Rollback/repair requires whole-patch rollback before merge or
    forward repair through Rust after merge.

Issues:
- none
