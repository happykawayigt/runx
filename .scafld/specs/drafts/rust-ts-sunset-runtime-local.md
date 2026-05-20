---
spec_version: '2.0'
task_id: rust-ts-sunset-runtime-local
created: '2026-05-18T00:00:00Z'
updated: '2026-05-20T05:28:35Z'
status: draft
harden_status: not_run
size: large
risk_level: very_high
---

# TS sunset: runtime-local

## Current State

Status: draft
Current phase: none
Next: approve
Reason: refreshed against the current spec archive. This is a deletion/cutover
spec, not a compatibility-bridge spec. A 2026-05-20 importer census still finds
127 files with runtime-local/adapters package references, imports, direct
source imports, or docs/API-surface entries.
Blockers: not currently executable. `rust-harness`,
`rust-runtime-skill-execution`, `rust-runtime-adapters-agent`,
`rust-runtime-adapters-a2a`, `rust-runtime-adapters-catalog`, and
`rust-runtime-adapters-mcp` are completed or archived completed, but the MCP
adapter review left a focused receipt-proof gap now tracked by
`rust-mcp-server-harness-receipt-seal`: single-skill `runx mcp serve` calls
must prove they emit sealed `runx.harness_receipt.v1` nodes. Remaining TS
importers still include CLI, IDE core, host-adapters, langchain, package
manifests, path aliases, and many tests. MCP server receipt proof, dev,
journal-local, connect, scaffold, tool-catalogs, doctor, registry, receipt
path, package boundary retargeting, and all surviving local callers must be
Rust-routed or explicitly sunset before deletion starts.
Allowed follow-up command: `none`
Latest runner update: 2026-05-20T22:55:00+10:00 importer census refreshed; the
deletion remains blocked, not merely unhardened.
Review gate: not_started

## Summary

Delete `packages/runtime-local/` and `packages/adapters/` in one coordinated
end-state sunset. This is not currently executable: the workspace still has
active package dependencies, TS path aliases, source imports, and test imports
for both packages. The package dependency direction is mostly
`@runxhq/adapters` -> `@runxhq/runtime-local`: the adapters package imports
runtime-local SDK, sandbox, MCP, harness, and tool-catalog helpers. The CLI and
test suites consume both packages together by constructing adapters and passing
them into runtime-local execution. Both retire only after adapter behavior lands
and is routed through `runx-runtime::adapters::{cli_tool, agent, catalog, a2a,
mcp}` and surviving callers use stable Rust/contract/CLI boundaries.

The replacement execution model is Rust runtime plus the ratified harness
spine. Skill execution is expressed as a harness run: decisions and acts are
contained in the harness node, receipts are sealed harness nodes, and graph
execution is represented by parent/child harness receipt references. This spec
does not preserve a TypeScript compatibility bridge, a `runtime-local` shim, or
legacy receipt/object vocabularies.

This is the last big rip. After it lands, the Rust takeover is complete
for OSS purposes; cloud-side hosted surfaces (`agent-runner`, `worker`,
`api`, `auth`) remain TS unless and until separate cloud cutover specs
target them. The disposition of every remaining TS package is documented
in `rust-ts-interop-boundary`.

Current reality as of this refresh:
- Completed prerequisites: `rust-harness`, `rust-runtime-skill-execution`,
  `rust-runtime-adapters-agent`, `rust-runtime-adapters-a2a`, and archived
  `rust-runtime-adapters-catalog` and `rust-runtime-adapters-mcp`.
- Hard blocker: the archived `rust-runtime-adapters-mcp` review recorded
  `mcp-server-skill-may-skip-harness-receipt-seal`. The deletion cannot start
  until `rust-mcp-server-harness-receipt-seal` proves MCP server single-skill
  calls emit sealed harness receipts without TS runtime-local dispatch.
- Rust adapter files exist behind feature gates under
  `crates/runx-runtime/src/adapters/{cli_tool,agent,a2a,catalog,mcp}.rs`, but
  feature-gated files alone are not deletion evidence; routing and importer
  removal are required.
- Root, CLI, IDE core, host-adapters, langchain, and many tests still import or
  depend on `@runxhq/runtime-local`, `@runxhq/adapters`, or their subpaths.
- The 2026-05-20 importer census finds 127 files with runtime-local/adapters
  references across manifests, tsconfig paths, docs/API surface, CLI,
  runtime-local/adapters source, scripts, plugins, and tests.
- Existing TS oracle generators remain useful before deletion. They are not
  a post-sunset execution path.

## Context

CWD: `.`

Packages:
- `@runxhq/runtime-local`
- `@runxhq/adapters`
- `crates/runx-runtime`
- Every TS importer of `@runxhq/runtime-local` or `@runxhq/adapters`

Current TypeScript sources:
- `packages/runtime-local/**` (to be deleted)
- `packages/adapters/**` (to be deleted)

Files impacted:
- `packages/runtime-local/` (deleted)
- `packages/adapters/` (deleted)
- `pnpm-workspace.yaml` (remove workspace members)
- Every TS file currently importing from `@runxhq/runtime-local` or
  `@runxhq/adapters`
- Fixture-generation scripts retired by this sunset:
  `scripts/generate-rust-fanout-fixtures.ts`,
  `scripts/generate-rust-skill-fixtures.ts`, adapter oracle generators whose
  Rust specs no longer need the TS oracle, and any runtime-local-only
  approval/harness oracle scripts that have no surviving Rust consumer

Current importer and blocker inventory:
- Package manifests and path aliases:
  - Root `package.json` devDependencies on `@runxhq/adapters` and
    `@runxhq/runtime-local`.
  - `packages/cli/package.json` depends on both packages.
  - `plugins/ide-core/package.json` depends on `@runxhq/adapters`.
  - `packages/host-adapters/package.json` and `packages/langchain/package.json`
    depend on `@runxhq/runtime-local`.
  - `packages/adapters/package.json` depends on `@runxhq/runtime-local`.
  - `tsconfig.base.json` aliases `@runxhq/adapters`, subpaths,
    `@runxhq/runtime-local`, and subpaths.
- Surviving package source imports:
  - `packages/cli/src/**` imports runtime-local execution, SDK, harness,
    tool-catalogs, history, and adapters.
  - `plugins/ide-core/src/actions.ts` imports adapters and runtime-local
    harness/SDK surfaces.
  - `packages/host-adapters/src/**` imports runtime-local SDK types and must
    retarget to `@runxhq/contracts`.
  - `packages/langchain/src/**` imports runtime-local SDK, tool-catalogs, and
    result types and must move to CLI JSON or another boundary ratified by
    `rust-ts-interop-boundary`.
  - `packages/adapters/src/**` imports runtime-local MCP, sandbox, harness,
    SDK, and tool-catalog helpers; these imports confirm adapters depend on
    runtime-local rather than runtime-local depending on adapters.
- Test importers:
  - Runtime-local execution tests include graph runner, graph fanout, graph
    governance, graph registry refs, local skill runner, sourcey, scafld,
    issue-to-pr, issue-intake, MCP, A2A, approval, auth, history, receipt
    inspection, package resolution/profile, and CLI tool policy/sandbox tests.
  - SDK/host tests include host protocol, imported tools, framework bridge,
    caller approval boundary, and IDE action coverage.
  - Adapter tests under `packages/adapters/**` remain with the package until
    deleted or converted to durable Rust fixture coverage.
- Fixture/oracle scripts currently present:
  - `scripts/generate-rust-harness-fixtures.ts` has a surviving Rust harness
    consumer and is not automatically deleted by this sunset.
  - `scripts/generate-rust-skill-fixtures.ts` is tied to completed product
    skill execution evidence and is retired only when fixtures no longer need
    TS oracle regeneration.
  - `scripts/generate-a2a-adapter-fixtures.ts`,
    `scripts/generate-agent-adapter-fixtures.ts`, and
    `scripts/generate-runtime-catalog-adapter-oracles.ts` are retained until
    their completed adapter specs declare the checked-in Rust fixtures durable.
  - Any future MCP oracle generator remains blocked on the MCP adapter spec and
    must complete before runtime-local deletion.

Invariants:
- Every importer either re-routed to Rust (via CLI subprocess, in-process
  binding, or `runx-runtime-service` daemon) or is itself sunset.
- The cloud `agent-runner` package has a stable boundary against the Rust
  runtime (settled in `rust-aster-runtime-cutover`).
- No compatibility package, shim, alias, or v2 surface remains for
  `@runxhq/runtime-local` or `@runxhq/adapters` inside this workspace.
- No active runtime-local replacement code, fixture, receipt assertion, schema
  projection, or docs page introduced by this spec uses retired peer
  execution-object keys.
- Skill names remain canonical. Do not introduce aliases such as
  `issue-control`, `runtime-local-v2`, or any v2 package name to mask the
  sunset. `issue-to-pr` and `issue-intake` are legitimate product skill names
  and must not be flagged or renamed by vocabulary checks. Existing product
  packet names such as `runx.issue_to_pr_outcome.v1` are not runtime-local
  compatibility aliases.
- The only accepted runtime-local replacement for skill execution is a Rust
  harness run that emits and verifies canonical sealed harness receipts with
  contained decision payloads, contained act payloads, child harness receipt
  refs, proof status, and verification checks.
- Historical archived specs may mention old terms, but active specs, fixtures,
  package manifests, and runtime-local replacement docs must not depend on
  them.

## Objectives

- Enumerate every importer and classify it as Rust-routed, sunset, or surviving
  stable boundary.
- Verify migration for each importer without falling back to runtime-local.
- Prove real product skill execution through Rust harness runs, not a TS oracle
  or compatibility object.
- Delete `packages/adapters/`.
- Delete `packages/runtime-local/`.
- Retire runtime-local-only fixture generators and TS unit tests after their
  durable fixture coverage is owned by Rust.
- Update workspace config.
- Update package manifests, internal docs, and public package disposition notes
  so no workspace import points at `@runxhq/runtime-local` or
  `@runxhq/adapters`.

## Scope

In scope:
- TS runtime-local deletion.
- TS adapters deletion bundled with runtime-local.
- Removing workspace package members, package references, tsconfig paths,
  exports, tests, scripts, and internal docs that keep runtime-local alive.
- Re-routing surviving local callers through Rust runtime, Rust CLI JSON, Rust
  harness execution, or another stable boundary already ratified in
  `rust-ts-interop-boundary`.
- Retiring runtime-local-owned TS fixture generators after their fixture output
  has either moved to durable `fixtures/**` coverage or become obsolete.
- Enforcing the ratified harness spine as the skill-execution contract.

Out of scope:
- Any runtime feature change.
- Cloud-side TS package deletions (their own specs).
- Creating a new `@runxhq/runtime-local` compatibility package, v2 package,
  import alias, adapter facade, or work-item/object bridge.
- Preserving legacy pre-cutover execution objects for skill runs, graph runs,
  or retired peer terminal artifacts.
- Adding new skills or changing product skill `SKILL.md`/`X.yaml` content to
  make the sunset pass.

## Dependencies

- `runx-contract-spine-hard-cutover` complete; it is the canonical source for
  harness, act, decision, signal, authority, and harness receipt shapes.
- `rust-harness` complete and default-ready for every harness mode needed by
  surviving local callers. It must reject retired fixture receipt fields rather
  than translate them.
- `rust-runtime-skill-execution` complete; checked-in product skill harnesses
  execute in Rust and verify sealed harness receipt trees.
- `rust-ts-sunset-marketplaces` complete.
- Every runtime adapter path complete and routed:
  `rust-runtime-adapters-agent`, `rust-runtime-adapters-a2a`, and archived
  `rust-runtime-adapters-catalog` and `rust-runtime-adapters-mcp` are
  completed; `rust-mcp-server-harness-receipt-seal` remains a hard blocker
  until MCP server single-skill receipt sealing is proven and no longer depends
  on TS runtime-local dispatch. The `cli_tool` runtime path is already consumed
  by skill execution but does not cover MCP, agent, A2A, or catalog by
  implication.
- MCP server, dev, journal-local, connect, scaffold, tool-catalogs, doctor,
  registry, receipt path, and every CLI surface that imports runtime-local
  consumed by Rust or explicitly sunset.
- `rust-ts-interop-boundary` remains the package disposition source of truth.

## Sequencing

1. `runx-contract-spine-hard-cutover` ratifies the harness spine and receipt
   vocabulary. This spec must not create new schema aliases, v2 shapes, or
   transitional runtime-local object models.
2. `rust-harness` ports replay execution to Rust and upgrades active harness
   fixtures to canonical harness receipt assertions. This sunset must wait for
   those fixtures to reject retired fields such as `skill_execution`,
   `graph_execution`, `skill_name`, `source_type`, `graph_name`, and `owner`
   under `expect.receipt`.
3. `rust-runtime-skill-execution` proves checked-in product skills execute as
   Rust harness runs with contained decisions, contained acts, child harness
   receipt refs, proof validation, and unsupported-source fail-closed evidence.
4. Adapter specs move live adapter behavior into `runx-runtime::adapters::*`.
   This sunset may not start deleting packages until importer audit shows no
   runtime-local caller still needs TS adapters for production execution.
5. Runtime-local deletion removes the TS package only after the above evidence
   is checked in. The deletion PR is not a place to repair Rust parity except
   for narrow import cleanup caused by removing the TS package.

## Build Decisions

- `@runxhq/runtime-local` and `@runxhq/adapters` are removed from the workspace
  rather than replaced with empty packages.
- Public external-consumer migration is handled by release notes, deprecation
  metadata, and the surviving CLI/contracts boundaries. It is not handled by a
  local compatibility bridge.
- Product skill execution evidence is a Rust harness run. A passing assertion
  must cite canonical harness receipt schema/id, harness id, seal status,
  contained decisions, contained acts, child receipt refs, proof status, and
  verification checks.
- The Rust CLI launcher or SDK may spawn Rust runtime or call Rust libraries,
  but it must not import a TS runtime-local facade after this sunset.
- TS tests that only exercised runtime-local internals are deleted with the
  package. Durable end-to-end coverage lives under `fixtures/**`,
  `tests/cli/**`, or Rust crate tests.
- Any surviving TypeScript package may import stable TS contracts, shell the
  Rust CLI, or speak cloud HTTP. It may not import deleted runtime-local or
  adapters package paths.

## Planned Phases

Phase 1: importer and fixture inventory.
- Enumerate all imports, package deps, tsconfig paths, workspace scripts, docs,
  and fixture generators that reference `@runxhq/runtime-local`,
  `@runxhq/adapters`, `packages/runtime-local`, or `packages/adapters`.
- Start from the current inventory in this draft and refresh it immediately
  before execution; other workers may have added or removed importers.
- Classify each importer as Rust-routed, sunset with runtime-local, or
  surviving stable-boundary package.
- Enumerate runtime-local-only fixture generators and identify the Rust spec or
  durable fixture set that now owns each behavior.

Phase 2: evidence gate.
- Verify `rust-harness` acceptance evidence is checked in and active fixtures
  use canonical harness receipt assertions.
- Verify `rust-runtime-skill-execution` acceptance evidence is checked in for
  `issue-intake` and `issue-to-pr` without modifying product skill files.
- Verify adapter specs cover every source type reachable from surviving
  callers, and unsupported production source types fail closed with receipt
  evidence.
- Verify MCP adapter/client and MCP server routing are complete, then verify
  `rust-mcp-server-harness-receipt-seal` has closed the single-skill sealed
  harness receipt proof gap. This is a hard blocker, not a nice-to-have
  coverage item.

Phase 3: route surviving callers.
- Remove runtime-local/adapters package dependencies from surviving packages.
- Route local execution through Rust CLI JSON, Rust runtime APIs, or the stable
  TS contracts boundary.
- Keep cloud-side TS packages on their existing cloud HTTP boundary; do not
  delete cloud packages here.

Phase 4: delete packages and scaffolding.
- Remove `packages/runtime-local/` and `packages/adapters/`.
- Remove workspace members, exports, package references, test targets, TS path
  aliases, and scripts that only existed for those packages.
- Remove or archive runtime-local-only oracle generators once their durable
  fixtures are owned by Rust.

Phase 5: validation and review.
- Run the negative import/object-vocabulary checks and positive Rust harness,
  runtime, receipt, workspace, and CLI validation commands.
- Record any intentionally surviving mention as archived documentation only.
- Hand off with explicit evidence that no compatibility bridge remains.

## Acceptance Criteria

- No active workspace file outside archived specs imports, depends on, exports,
  or path-aliases `@runxhq/runtime-local`, `@runxhq/adapters`,
  `packages/runtime-local`, or `packages/adapters`.
- `packages/runtime-local/` and `packages/adapters/` are gone and are not
  replaced by empty packages, shim packages, aliases, or v2 package names.
- All surviving skill execution acceptance evidence is expressed as Rust
  harness runs. Receipts are sealed harness nodes with contained decisions,
  contained acts, child harness receipt refs where graph execution is involved,
  proof status, and verification checks.
- No active replacement code, fixture, docs page, package manifest, or spec
  added by this sunset uses retired peer terminal artifact keys as execution
  objects.
- No active replacement fixture or receipt expectation accepts retired
  `skill_execution` or `graph_execution` receipt objects.
- `issue-intake` and `issue-to-pr` run through Rust runtime skill execution and
  their emitted harness receipts validate through `runx-receipts`.
- `runx harness` and the runtime skill execution tests pass without invoking
  `packages/runtime-local` or `packages/adapters`.
- Runtime-local-only fixture generator scripts are either deleted or retained
  only when another active spec explicitly names them as still required before
  this sunset can complete.
- The deletion does not change product skill files (`skills/**/SKILL.md` or
  `skills/**/X.yaml`) except through separately approved skill specs.

## Validation Commands

```sh
test ! -d packages/runtime-local
test ! -d packages/adapters
! rg -n "@runxhq/(runtime-local|adapters)|packages/(runtime-local|adapters)" . --glob '!.scafld/specs/**' --glob '!**/archive/**'
! rg -n '"(work[_]item|engagement|matter|operation)"\\s*:' crates/runx-runtime crates/runx-contracts fixtures/runtime fixtures/harness tests --glob '!**/archive/**'
! rg -n '"(skill_execution|graph_execution)"\\s*:' fixtures/runtime fixtures/harness crates/runx-runtime tests --glob '!**/archive/**'
! rg -n "\\b(issue-control|runtime-local-v2|adapters-v2)\\b" packages crates fixtures tests docs --glob '!**/archive/**'
! rg -n "(^|[^A-Za-z0-9_])issue_to_pr([^A-Za-z0-9_]|$)" crates/runx-runtime fixtures/runtime tests --glob '!**/archive/**' --glob '!fixtures/runtime/skills/issue-to-pr/**'
pnpm install --frozen-lockfile
pnpm test
pnpm build
cargo test --manifest-path crates/Cargo.toml -p runx-runtime harness
cargo test --manifest-path crates/Cargo.toml -p runx-runtime --features cli-tool --test skill_issue_intake
cargo test --manifest-path crates/Cargo.toml -p runx-runtime --features cli-tool --test skill_issue_to_pr
cargo test --manifest-path crates/Cargo.toml -p runx-receipts
cargo clippy --manifest-path crates/Cargo.toml -p runx-runtime -p runx-receipts --all-targets --features cli-tool -- -D warnings
cargo fmt --manifest-path crates/Cargo.toml --all --check
node scripts/check-rust-core-style.mjs
```

## Explicit Blockers

- `rust-harness` not completed, not reviewed, or still accepting retired
  receipt expectation fields.
- `rust-runtime-skill-execution` not completed, or product skill execution
  still depending on TS runtime-local, product skill aliases, old issue-control
  names, or fixture-only success that skips receipt proof verification.
- `rust-mcp-server-harness-receipt-seal` not completed, or MCP server
  single-skill execution still failing to emit sealed `runx.harness_receipt.v1`
  nodes without TS runtime-local dispatch.
- Any surviving local caller still importing `@runxhq/runtime-local` or
  `@runxhq/adapters`.
- Any adapter source type reachable from surviving local execution still lacks
  a Rust adapter or explicit fail-closed receipt evidence.
- Any active replacement object model still centered on retired peer terminal
  artifacts, `skill_execution`, or `graph_execution`.
- Any proposal to keep a workspace shim, v2 package, path alias, or bridge
  package for runtime-local/adapters.
- Cloud-side hosted cutover ambiguity that would make local deletion break a
  published stable boundary already retained by `rust-ts-interop-boundary`.

## Rollback And Repair

- Before package deletion, rollback is to leave runtime-local/adapters in place
  and keep this spec blocked; do not add a bridge.
- After deletion, rollback must restore only the deleted package files from the
  deletion commit if the Rust runtime path is proven incomplete. Do not repair
  by adding aliases, v2 packages, or legacy object translation.
- If validation finds an importer, route that importer through Rust or sunset
  it. Do not reintroduce runtime-local as a dependency.
- If a fixture still needs old TS receipt fields, fix the upstream Rust harness
  or runtime skill execution evidence. Do not whitelist the old field here.

## Open Questions

- Whether npm deprecation metadata for the deleted packages is published in the
  same release train or a separate release-management spec. Default: publish
  deprecation metadata outside this deletion PR; do not keep local shim
  packages.
