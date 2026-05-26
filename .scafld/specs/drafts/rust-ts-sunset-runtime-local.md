---
spec_version: '2.0'
task_id: rust-ts-sunset-runtime-local
created: '2026-05-18T00:00:00Z'
updated: '2026-05-22T00:09:50+10:00'
status: draft
harden_status: not_run
size: large
risk_level: very_high
---

# TS sunset: runtime-local

## Current State

Status: draft
Current phase: none
Next: smaller importer specs, not package deletion
Reason: refreshed after the CLI importer completion and a small non-CLI
contract fixture generator cleanup. This remains a deletion/cutover spec, not
a compatibility-bridge spec. A 2026-05-21 exact-package census finds 92
active files outside `.scafld/specs/**` and `dist/**` with
runtime-local/adapters package references, imports, direct source paths, docs,
fixtures, or package-resolution entries. Of those, 68 are outside the two
packages being deleted and 46 are actual package import files outside those
packages. `packages/cli/src/**` has zero exact runtime-local/adapters package
references in this tree. Blockers: not currently executable. `rust-harness`,
`rust-runtime-skill-execution`, `rust-runtime-adapters-agent`,
`rust-runtime-adapters-a2a`, `rust-runtime-adapters-catalog`,
`rust-runtime-adapters-mcp`, and `rust-mcp-server-receipt-seal` are
completed or archived completed. The remaining blockers are importer/routing
blockers: IDE core, langchain, package manifests, path aliases, vitest aliases,
oracle scripts, active docs/fixtures, Rust parity/doctor references, and many
tests still reference `@runxhq/runtime-local`, `@runxhq/adapters`, or their
package paths. Host-adapters and CLI source no longer import
runtime-local/adapters in the current tree. All surviving local callers must be
Rust-routed, moved to the correct language-neutral protocol lane under Rust
supervision, or explicitly sunset before deletion starts. Cloud `agent-runner`
binding is not settled by this draft while its target boundary remains open.
Allowed follow-up command: `none`
Latest runner update: 2026-05-22T00:09:50+10:00 promoted the extension-surface
boundary to priority-zero cutover law: `external-adapter-plugin-protocol-v1`
must be treated as the external execution-adapter protocol, not as the umbrella
for every extension, integration, source-ingress, catalog, hosted-runtime, or
outbox queue. Runtime-local deletion remains blocked by live non-CLI importers,
tests, scripts, package-resolution entries, open cloud binding disposition,
custom execution-adapter protocol gaps, and any unclassified surviving extension
surface that would otherwise be forced into Rust or hidden behind a TypeScript
runtime fallback.
Review gate: not_started

## Summary

Delete `packages/runtime-local/` and `packages/adapters/` in one coordinated
end-state sunset. This is not currently executable: the workspace still has
active package dependencies, TS path aliases, source imports, and test imports
for both packages. The package dependency direction is mostly
`@runxhq/adapters` -> `@runxhq/runtime-local`: the adapters package imports
runtime-local SDK, sandbox, MCP, harness, and tool-catalog helpers. The
remaining tests and TS package importers consume both packages together by
constructing adapters and passing them into runtime-local execution. Both
retire only after built-in trusted adapter behavior lands and is routed through
`runx-runtime::adapters::{cli_tool, agent, catalog, a2a, mcp}`, custom
execution-adapter authoring has a language-neutral disposition through
`external-adapter-plugin-protocol-v1` where needed, and every other surviving
integration surface is classified under its own stable Rust/contract/CLI/cloud
or protocol boundary.

The replacement execution model is Rust runtime plus the ratified harness
spine. Skill execution is expressed as a harness run: decisions and acts are
contained in the harness node, receipts are sealed harness nodes, and graph
execution is represented by parent/child receipt references. This spec
does not preserve a TypeScript compatibility bridge, a `runtime-local` shim, or
legacy receipt/object vocabularies.

Priority-zero architecture rule: this cutover must not collapse all extension
work into one external adapter protocol. `external-adapter-plugin-protocol-v1`
is the no-Rust-required external execution-adapter lane for replacing custom
`SkillAdapter` behavior under Rust supervision. It is not the skill author
subprocess ABI, not source-event ingress, not hosted embedded runtime binding,
not the public tool-catalog read model, and not the thread/outbox provider
adapter protocol. If any surviving runtime-local/adapters consumer belongs to
one of those other lanes, package deletion stays blocked until that lane is
classified by a named spec or explicitly ruled out of scope.

This is the last big rip. After it lands, the Rust takeover is complete
for OSS trusted local execution; cloud-side hosted surfaces (`agent-runner`,
`worker`, `api`, `auth`) remain TS unless and until separate cloud cutover specs
target them. Their Rust binding mode is still open unless a later inspected
cloud-tree pass records exact paths and a stable boundary. The disposition of
every remaining TS package is documented in `rust-ts-interop-boundary` and
refined by `ts-extension-survivorship-boundary`.

Current reality as of this refresh:
- Completed prerequisites: `rust-harness`, `rust-runtime-skill-execution`,
  `rust-runtime-adapters-agent`, `rust-runtime-adapters-a2a`, archived
  `rust-runtime-adapters-catalog`, archived `rust-runtime-adapters-mcp`, and
  archived `rust-mcp-server-receipt-seal`.
- The former MCP blocker is closed: `rust-mcp-server-receipt-seal`
  is archived completed with review gate pass and acceptance evidence for
  `cargo test --manifest-path crates/Cargo.toml -p runx-runtime mcp_server
  --features mcp -- --nocapture`, harness tests, receipt proof tests, and MCP
  clippy. Runtime-local deletion is now blocked by importer/routing work, not
  by the MCP server single-skill receipt proof.
- Rust adapter files exist behind feature gates under
  `crates/runx-runtime/src/adapters/{cli_tool,agent,a2a,catalog,mcp}.rs`, but
  feature-gated files alone are not deletion evidence; routing and importer
  removal are required.
- Those Rust adapter files cover built-in trusted adapters. They do not make
  custom execution-adapter authoring Rust-only; any surviving custom execution
  authoring path must use `external-adapter-plugin-protocol-v1` or remain an
  explicit blocker. Non-execution extension queues must not be stuffed into that
  protocol to make deletion appear ready.
- Cloud `agent-runner` is not settled by `rust-aster-runtime-cutover`; that
  draft defers cloud binding until a checkout with `cloud/**` is inspected.
- Root package metadata, IDE core, langchain, scripts, Rust parity/doctor
  references, docs/API surface, active fixtures, package path aliases, vitest
  aliases, the runtime-local/adapters packages themselves, and many tests still
  import or refer to `@runxhq/runtime-local`, `@runxhq/adapters`, or their
  package paths.
- Update: the CLI source package-import blocker is closed in the current tree.
  `packages/cli/src/**` has zero exact `@runxhq/runtime-local`,
  `@runxhq/adapters`, `packages/runtime-local`, or `packages/adapters`
  references. Do not use this spec to take the parent-owned CLI dead-command
  cleanup.
- Update: `scripts/generate-rust-contract-fixtures.ts` no longer imports the
  runtime-local act-assignment SDK helper; it now uses `@runxhq/contracts` for
  validation and `@runxhq/core/util` for stable hashing.
- `packages/host-adapters/**` no longer has a runtime-local/adapters
  dependency or import in the current tree.
- The refreshed 2026-05-21 exact-package census finds 92 active files outside
  `.scafld/specs/**` and `dist/**` with runtime-local/adapters references; 68
  of those are outside `packages/runtime-local/**` and `packages/adapters/**`.
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
- Census command used for this refresh, from repo root:

  ```sh
  rg -l "@runxhq/(runtime-local|adapters)|@runxhq/runtime-local|@runxhq/adapters|packages/(runtime-local|adapters)" --glob '!node_modules/**' --glob '!dist/**' --glob '!.scafld/specs/**' | sort
  ```

- 2026-05-21 refreshed exact-package active reference totals:
  - 92 active files outside `.scafld/specs/**` and `dist/**`.
  - 68 active files outside the two packages being deleted.
  - 61 files with actual package import statements under
    `packages/**`, `plugins/**`, `tests/**`, and `scripts/**`.
  - 46 actual package import files outside `packages/runtime-local/**` and
    `packages/adapters/**`.
- 2026-05-21 refreshed exact-package prefix census:
  - `README.md`: 1 file.
  - `docs`: 3 files.
  - `fixtures/cli-parity`: 1 file.
  - `fixtures/rust-cli-cutover-negative`: 1 file.
  - root `package.json`: 1 file.
  - `packages/adapters`: 12 files.
  - `packages/runtime-local`: 12 files.
  - `pnpm-lock.yaml`: 1 file.
  - `scripts`: 8 files.
  - `skills/write-harness`: 1 file.
  - `tests`: 49 files.
  - `tsconfig.base.json`: 1 file.
  - `vitest.workspace-aliases.ts`: 1 file.
- Package manifests and path aliases:
  - Root `package.json` devDependencies on `@runxhq/adapters` and
    `@runxhq/runtime-local`.
  - `packages/cli/package.json` no longer declares either dependency, and
    `packages/cli/src/**` has zero exact runtime-local/adapters references in
    the current tree.
  - `plugins/ide-core/package.json` no longer references either package in
    this census.
  - `packages/langchain/package.json` no longer references either package in
    this census.
  - `packages/host-adapters/package.json` no longer references either package.
  - `packages/adapters/package.json` depends on `@runxhq/runtime-local`.
  - `tsconfig.base.json` aliases `@runxhq/adapters`, subpaths,
    `@runxhq/runtime-local`, and subpaths.
  - `vitest.workspace-aliases.ts` mirrors the same aliases for tests.
  - `pnpm-lock.yaml` still records the root, adapters, langchain, runtime-local,
    and ide-core links.
  - `pnpm-workspace.yaml` does not name the packages directly, but
    `packages/*` still includes both package directories until deletion.
- Surviving package source imports:
  - `packages/cli/src/**` is no longer a package-import blocker; the CLI
    importer routing slice is archived completed. Leave parent-owned CLI
    dead-command cleanup out of this spec.
  - `plugins/ide-core/src/**` has zero exact runtime-local/adapters package
    references in the current tree.
  - `packages/langchain/src/**` has zero exact runtime-local/adapters package
    references in the current tree.
  - `packages/adapters/src/**` imports runtime-local MCP, sandbox, harness,
    SDK, and tool-catalog helpers; these imports confirm adapters depend on
    runtime-local rather than runtime-local depending on adapters.
  - `packages/runtime-local/src/**` still has self-referential public package
    exports and subpath imports; these are deleted with the package, not routed.
- Rust-side direct package-path references:
  - The current exact-package census finds zero `crates/runx-runtime/**`
    references to runtime-local/adapters package paths. Keep this at zero.
- Test importers:
  - 49 files under `tests/**` still import package APIs or direct package
    source paths. They cover graph runner/fanout/governance/registry refs,
    local skill runner, sourcey, scafld, issue-to-pr, issue-intake, MCP, A2A,
    approval, auth, history, receipt inspection, package resolution/profile,
    CLI tool policy/sandbox, host protocol, SDK imported tools, and framework
    bridge coverage.
  - SDK/host tests include host protocol, imported tools, framework bridge,
    caller approval boundary, and IDE action coverage.
  - Adapter tests under `packages/adapters/**` remain with the package until
    deleted or converted to durable Rust fixture coverage.
- Active docs, fixtures, and skills references:
  - `README.md`, `docs/api-surface.md`, `docs/rust-kernel-architecture.md`, and
    `docs/ts-interop-boundary.md` still describe or expose runtime-local or
    adapters surfaces.
  - `fixtures/cli-parity/runtime-surfaces.json`,
    `fixtures/rust-cli-cutover-negative/legacy-v2-package-candidate/package.json`,
    and `skills/write-harness/SKILL.md` still name package paths or package
    dependencies.
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
  - `scripts/generate-runtime-mcp-oracles.ts` exists and imports the TS MCP
    adapter; after the MCP receipt-seal completion it remains an oracle
    ownership decision, not a receipt-proof blocker.
  - `scripts/generate-rust-contract-fixtures.ts` has moved off the
    runtime-local act-assignment SDK helper and is no longer a blocker for this
    sunset.
  - `scripts/dogfood-github-issue-to-pr.mjs` still imports adapters and
    runtime-local directly and must route through the Rust CLI or be sunset.
  - `scripts/generate-cli-feature-parity.ts`, `scripts/check-boundaries.mjs`,
    `scripts/check-rust-cli-cutover-negative.mjs`, and
    `scripts/check-rust-cli-release-artifacts.ts` still encode package names or
    direct package path expectations and must be updated as part of the final
    package-boundary cleanup.

Invariants:
- Every importer either re-routed to Rust (via CLI subprocess, in-process
  binding, or `runx-runtime-service` daemon), moved to a language-neutral
  protocol under Rust supervision where the behavior is execution, moved to a
  separate stable protocol where the behavior is non-execution integration, or
  is itself sunset. TypeScript helper code over ratified protocols is allowed;
  trusted TypeScript runtime execution is not.
- The extension taxonomy is part of the cutover contract, not future cleanup:
  skill author subprocess ABI (`run.js`/`run.mjs`/CLI-tool input/output),
  external execution adapter protocol (custom source execution replacing
  runtime-local `SkillAdapter` behavior), source-event ingress (Slack, Sentry,
  GitHub, file, API, and webhook signals admitted into receipts),
  hosted or embedded runtime binding (cloud worker, agent-runner, SDK, host
  bridge, continuation, auth resolver, and resume semantics), tool catalog/read
  model access (public search, inspect, and registry/catalog views), and
  thread/outbox provider adapters (source-thread comments, PR updates, outbox
  pushes, and rendered story consumers).
- A surviving extension surface is deletion-ready only when it is classified into
  one taxonomy lane and has either a Rust route, a ratified language-neutral
  protocol, or an explicit blocker. The deletion PR must not make classification
  decisions implicitly.
- The cloud `agent-runner` package does not have a settled Rust runtime boundary
  in this draft and is not settled by `rust-aster-runtime-cutover`. Before
  deletion can depend on it, a cloud-tree binding pass must classify the
  boundary as runtime HTTP, CLI JSON, service/FFI, external execution-adapter
  protocol, another reviewed stable boundary, or out of scope for this OSS
  sunset.
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
  harness run that emits and verifies canonical sealed receipts with
  contained decision payloads, contained act payloads, child receipt
  refs, proof status, and verification checks.
- Historical archived specs may mention old terms, but active specs, fixtures,
  package manifests, and runtime-local replacement docs must not depend on
  them.

## Objectives

- Enumerate every importer and classify it as Rust-routed, sunset, or surviving
  stable boundary.
- Classify every remaining runtime-local/adapters behavior by extension lane:
  skill subprocess ABI, external execution adapter, source-event ingress,
  hosted/embedded runtime binding, tool catalog/read model, thread/outbox
  provider adapter, or explicit sunset.
- Verify migration for each importer without falling back to runtime-local.
- Distinguish built-in trusted adapters that move into Rust from custom
  execution-adapter authoring that survives only through language-neutral
  protocols.
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
  harness execution, the language-neutral external execution-adapter protocol
  under Rust supervision, or another stable boundary already ratified in
  `rust-ts-interop-boundary` and `ts-extension-survivorship-boundary`.
- Classifying non-execution integration queues before deletion so source-event
  ingress, hosted runtime binding, catalog/read-model access, and thread/outbox
  provider writes do not get mis-modeled as execution adapters.
- Retiring runtime-local-owned TS fixture generators after their fixture output
  has either moved to durable `fixtures/**` coverage or become obsolete.
- Enforcing the ratified harness spine as the skill-execution contract.

Out of scope:
- Any runtime feature change.
- Cloud-side TS package deletions (their own specs).
- Implementing the external execution-adapter process protocol; owned by
  `external-adapter-plugin-protocol-v1`.
- Implementing source-event ingress, hosted runtime binding, catalog/read-model,
  or thread/outbox provider protocols. This spec may require those lanes to be
  classified or blocked before deletion, but it does not implement them.
- Creating a new `@runxhq/runtime-local` compatibility package, v2 package,
  import alias, adapter facade, or work-item/object bridge.
- Preserving legacy pre-cutover execution objects for skill runs, graph runs,
  or retired peer terminal artifacts.
- Adding new skills or changing product skill `SKILL.md`/`X.yaml` content to
  make the sunset pass.

## Dependencies

- `runx-contract-spine-hard-cutover` complete; it is the canonical source for
  harness, act, decision, signal, authority, and receipt shapes.
- `rust-harness` complete and default-ready for every harness mode needed by
  surviving local callers. It must reject retired fixture receipt fields rather
  than translate them.
- `rust-runtime-skill-execution` complete; checked-in product skill harnesses
  execute in Rust and verify sealed receipt trees.
- `rust-ts-sunset-marketplaces` complete.
- Every runtime adapter path complete and routed:
  `rust-runtime-adapters-agent`, `rust-runtime-adapters-a2a`, and archived
  `rust-runtime-adapters-catalog` and `rust-runtime-adapters-mcp` are
  completed; `rust-mcp-server-receipt-seal` is also archived completed
  and closes the former MCP server single-skill receipt proof gap. The
  `cli_tool` runtime path is already consumed by skill execution but does not
  cover MCP, agent, A2A, catalog, CLI package source routing, or TS package
  import removal by implication.
- `ts-extension-survivorship-boundary` remains the TypeScript survivorship
  guardrail: TypeScript may survive as contracts, clients, cloud/product code,
  scaffolding, host adapters, and helper SDKs over stable protocols, not as a
  trusted local runtime.
- `external-adapter-plugin-protocol-v1` must provide or explicitly block the
  language-neutral custom execution-adapter authoring path before this sunset can
  claim custom execution adapters are migrated. It must not be cited as the
  migration answer for source-event ingress, hosted embedded runtime binding,
  catalog/read-model access, or thread/outbox provider writes unless those
  separate behaviors are explicitly modeled by their own accepted protocol
  sections or sibling specs.
- `credential-broker-delivery-contract-v1` must provide or explicitly block the
  shared credential-delivery primitive for any surviving execution adapter,
  subprocess skill, hosted runtime, or outbox/provider path that needs provider
  secret material. Credential delivery must not be reinvented per protocol.
- Missing non-execution extension lanes are deletion blockers, not reasons to
  broaden the execution-adapter protocol until it recreates runtime-local out of
  process.
- `embedded-sdk-migration-story` must classify embedded cloud/runtime-local SDK
  consumers without assuming a settled cloud `agent-runner` binding.
- MCP server, dev, journal-local, connect, scaffold, tool-catalogs, doctor,
  registry, receipt path, and every CLI surface that imports runtime-local
  consumed by Rust or explicitly sunset.
- `rust-ts-interop-boundary` remains the package disposition source of truth.

## Sequencing

1. `runx-contract-spine-hard-cutover` ratifies the harness spine and receipt
   vocabulary. This spec must not create new schema aliases, v2 shapes, or
   transitional runtime-local object models.
2. `rust-harness` ports replay execution to Rust and upgrades active harness
   fixtures to canonical receipt assertions. This sunset must wait for
   those fixtures to reject retired fields such as `skill_execution`,
   `graph_execution`, `skill_name`, `source_type`, `graph_name`, and `owner`
   under `expect.receipt`.
3. `rust-runtime-skill-execution` proves checked-in product skills execute as
   Rust harness runs with contained decisions, contained acts, child harness
   receipt refs, proof validation, and unsupported-source fail-closed evidence.
4. Adapter specs move built-in trusted adapter behavior into
   `runx-runtime::adapters::*`. Custom execution-adapter authoring is not forced
   into Rust; it must either be covered by
   `external-adapter-plugin-protocol-v1` or remain a named deletion blocker.
   Source-event, hosted-runtime, catalog/read-model, and thread/outbox queues
   must be classified separately and must not be hidden behind the execution
   adapter protocol. This sunset may not start deleting packages until importer
   audit shows no runtime-local caller still needs TS adapters for production
   execution.
5. Runtime-local deletion removes the TS package only after the above evidence
   is checked in. The deletion PR is not a place to repair Rust parity except
   for narrow import cleanup caused by removing the TS package.

## Build Decisions

- `@runxhq/runtime-local` and `@runxhq/adapters` are removed from the workspace
  rather than replaced with empty packages.
- Public external-consumer migration is handled by release notes, deprecation
  metadata, surviving CLI/contracts boundaries, the external execution-adapter
  protocol where custom execution adapter authoring is required, and separately
  named protocols for non-execution integration lanes. It is not handled by a
  local compatibility bridge.
- Product skill execution evidence is a Rust harness run. A passing assertion
  must cite canonical receipt schema/id, harness id, seal status,
  contained decisions, contained acts, child receipt refs, proof status, and
  verification checks.
- The Rust CLI launcher or SDK may spawn Rust runtime or call Rust libraries,
  but it must not import a TS runtime-local facade after this sunset.
- TS tests that only exercised runtime-local internals are deleted with the
  package. Durable end-to-end coverage lives under `fixtures/**`,
  `tests/cli/**`, or Rust crate tests.
- Any surviving TypeScript package may import stable TS contracts, shell the
  Rust CLI, speak cloud HTTP, or provide helpers over ratified protocol lanes.
  It may not import deleted runtime-local or adapters package paths or execute
  skills outside Rust supervision.

## Deletion Classification

Can be deleted or rerouted now:
- `scripts/generate-rust-contract-fixtures.ts` direct runtime-local SDK source
  import: completed in this slice by moving act-assignment fixture generation
  onto `@runxhq/contracts` validation plus `@runxhq/core/util` stable hashing.

Needs a fresh smaller spec before deletion:
- Custom execution-adapter authoring:
  `external-adapter-plugin-protocol-v1` must either provide the durable
  language-neutral replacement for custom execution adapters or leave
  runtime-local/adapters deletion blocked for those consumers. Built-in Rust
  adapters alone are not sufficient evidence for third-party/custom authoring.
- Non-execution extension queue taxonomy:
  source-event ingress, hosted/embedded runtime binding, tool catalog/read-model
  access, and thread/outbox provider writes must each have a named stable
  disposition before this sunset can claim there is no adoption-breaking
  regression. Do not drag integration code into Rust, and do not preserve
  runtime-local to avoid classifying the queue.
- Shared credential broker/delivery:
  `credential-broker-delivery-contract-v1` must define how admitted credentials
  become scoped secret delivery for subprocess skills, external execution
  adapters, hosted runtimes, and outbox/provider side effects. The sunset must
  not pass by routing secrets through ad hoc env fields, invocation metadata, or
  provider-specific blobs.
- Adapter oracle scripts:
  `scripts/generate-a2a-adapter-fixtures.ts`,
  `scripts/generate-agent-adapter-fixtures.ts`,
  `scripts/generate-runtime-catalog-adapter-oracles.ts`, and
  `scripts/generate-runtime-mcp-oracles.ts` need explicit durable-fixture
  ownership or deletion.
- Root `tests/**`: triage into Rust parity/CLI JSON coverage, package-internal
  runtime-local/adapters tests deleted with the packages, or obsolete tests.
  Do not touch payment tests in this sunset slice.
- Root package metadata, lockfile entries, TS path aliases, vitest aliases,
  generated API docs, active fixtures, and the package directories themselves
  are final package-boundary cleanup after importer gates are zero.

Do not take in this spec slice:
- `packages/cli/src/**`: exact runtime-local/adapters references are already
  zero; the parent owns the remaining small CLI dead-command cleanup.
- Payment/x402 work.

## Next Executable Slices

This draft is still not executable as a package deletion. The next executable
work must be narrower than this sunset and should land in separate specs:

1. Non-CLI package boundary routing:
   - Keep `plugins/ide-core/**`, `packages/langchain/**`, `packages/cli/src/**`,
     `packages/host-adapters/**`, and `crates/runx-runtime/**` at zero exact
     runtime-local/adapters references.
   - Acceptance gate: the zero-count areas above remain zero while the
     remaining package/source/test references are retired by narrower specs.
2. Extension-lane classification:
   - For every importer that is not plain Rust runtime execution, assign exactly
     one lane: skill author subprocess ABI, external execution adapter,
     source-event ingress, hosted/embedded runtime binding, tool catalog/read
     model, thread/outbox provider adapter, cloud-only out of scope, or sunset.
   - Create or reference sibling specs for any lane that lacks an accepted stable
     protocol. At minimum, source ingress, hosted runtime binding, catalog/read
     model, and thread/outbox provider adapters must not be assumed solved by
     `external-adapter-plugin-protocol-v1`.
   - For any lane that requires provider credentials, reference
     `credential-broker-delivery-contract-v1` or mark credential delivery as a
     blocker.
   - Acceptance gate: no remaining importer is justified by
     `external-adapter-plugin-protocol-v1` unless it is actually an execution
     adapter.
3. Oracle and fixture ownership cleanup:
   - For `generate-a2a-adapter-fixtures`, `generate-agent-adapter-fixtures`,
     `generate-runtime-catalog-adapter-oracles`, and
     `generate-runtime-mcp-oracles`, either declare the checked-in Rust
     fixtures durable and retire the TS generator, or keep a named pre-sunset
     owner that still runs before deletion.
   - `generate-rust-contract-fixtures.ts` is no longer in scope for this
     cleanup; it has moved off runtime-local SDK helpers.
   - Acceptance gate: every remaining oracle generator has a Rust owner or is
     deleted before package deletion.
4. Test-suite triage:
   - Classify the 49 `tests/**` reference files as Rust parity coverage, CLI JSON
     coverage, package-internal coverage deleted with runtime-local/adapters,
     or obsolete tests.
   - Exclude payment/x402 files from this sunset slice unless a separate
     payment owner explicitly scopes them.
   - Acceptance gate: no root `tests/**` file imports runtime-local/adapters or
     direct package source paths.
5. Package-boundary deletion cleanup:
   - Remove root devDependencies, TS path aliases, vitest aliases, pnpm lock
     links, docs/API-surface entries, active fixture references, and package
     directories only after the above importer gates are zero.
   - Acceptance gate: the negative import check in this spec passes outside
     archived scafld specs.
6. Cloud and embedded binding disposition:
   - Classify `cloud/packages/agent-runner/**`, worker, and embedded SDK callers
     through `embedded-sdk-migration-story` after inspecting the cloud tree.
   - Acceptance gate: no deletion claim depends on an unverified cloud binding or
     a hidden runtime-local fallback.

## Planned Phases

Phase 1: importer and fixture inventory.
- Enumerate all imports, package deps, tsconfig paths, workspace scripts, docs,
  and fixture generators that reference `@runxhq/runtime-local`,
  `@runxhq/adapters`, `packages/runtime-local`, or `packages/adapters`.
- Start from the current inventory in this draft and refresh it immediately
  before execution; other workers may have added or removed importers.
- Classify each importer as Rust-routed, sunset with runtime-local, or
  surviving stable-boundary package, including language-neutral protocol helpers
  that remain outside trusted local execution.
- For every surviving boundary package, classify the lane before deciding the
  migration path. Execution adapters may point at
  `external-adapter-plugin-protocol-v1`; source ingress, hosted runtime binding,
  catalog/read-model, and thread/outbox provider adapters require their own
  stable dispositions.
- Enumerate runtime-local-only fixture generators and identify the Rust spec or
  durable fixture set that now owns each behavior.

Phase 2: evidence gate.
- Verify `rust-harness` acceptance evidence is checked in and active fixtures
  use canonical receipt assertions.
- Verify `rust-runtime-skill-execution` acceptance evidence is checked in for
  `issue-intake` and `issue-to-pr` without modifying product skill files.
- Verify adapter specs cover every source type reachable from surviving
  callers, and unsupported production source types fail closed with receipt
  evidence.
- Verify custom execution-adapter authoring is covered by
  `external-adapter-plugin-protocol-v1` only when the behavior is execution
  adapter behavior; otherwise verify the appropriate sibling lane disposition or
  keep deletion blocked.
- Treat MCP adapter/client and MCP server receipt sealing as completed
  prerequisites, then verify no surviving TS caller still reaches the TS MCP
  adapter/runtime-local path in production execution.

Phase 3: route surviving callers.
- Remove runtime-local/adapters package dependencies from surviving packages.
- Route local execution through Rust CLI JSON, Rust runtime APIs, or the stable
  TS contracts/external execution-adapter protocol boundary.
- Route non-execution integration through its classified stable boundary rather
  than through runtime-local/adapters or a broadened execution-adapter protocol.
- Keep cloud-side TS packages on their existing cloud HTTP boundary; do not
  delete cloud packages here or claim `agent-runner` Rust binding without a
  cloud-tree pass.

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
  contained acts, child receipt refs where graph execution is involved,
  proof status, and verification checks.
- No active replacement code, fixture, docs page, package manifest, or spec
  added by this sunset uses retired peer terminal artifact keys as execution
  objects.
- No active replacement fixture or receipt expectation accepts retired
  `skill_execution` or `graph_execution` receipt objects.
- `issue-intake` and `issue-to-pr` run through Rust runtime skill execution and
  their emitted receipts validate through `runx-receipts`.
- `runx harness` and the runtime skill execution tests pass without invoking
  `packages/runtime-local` or `packages/adapters`.
- Runtime-local-only fixture generator scripts are either deleted or retained
  only when another active spec explicitly names them as still required before
  this sunset can complete.
- Built-in trusted adapters are Rust-routed, and language-neutral external
  execution-adapter authoring is either implemented through
  `external-adapter-plugin-protocol-v1` or recorded as a blocker.
- Source-event ingress, hosted/embedded runtime binding, tool catalog/read-model,
  and thread/outbox provider adapter surfaces are each classified by a named
  spec or explicitly out of scope. None is assumed solved by the external
  execution-adapter protocol.
- Provider credential delivery for any surviving lane is covered by
  `credential-broker-delivery-contract-v1` or recorded as a blocker. No surviving
  lane introduces its own secret material channel.
- Cloud `agent-runner` is either classified by an inspected cloud binding pass
  or explicitly out of scope for this OSS package deletion; it is not assumed
  settled by Aster runtime cutover.
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

- Regression in a completed prerequisite: `rust-harness`,
  `rust-runtime-skill-execution`, any Rust adapter spec, or
  `rust-mcp-server-receipt-seal` losing sealed receipt proof or
  reintroducing TS runtime-local dispatch.
- The 2026-05-21 importer census is nonzero: 92 active exact-package
  reference files remain outside `.scafld/specs/**` and `dist/**`, including
  68 outside the two packages to delete and 46 actual package import files
  outside those packages.
- Any surviving local caller still importing `@runxhq/runtime-local` or
  `@runxhq/adapters`.
- `packages/cli/src/**` must stay at zero exact runtime-local/adapters
  references while parent-owned CLI cleanup continues out of band.
- `plugins/ide-core/**` and `packages/langchain/**` must stay at zero exact
  runtime-local/adapters references.
- Root package metadata, pnpm lock entries, `tsconfig.base.json`, or
  `vitest.workspace-aliases.ts` still resolving runtime-local/adapters after
  callers are routed.
- Runtime-local/adapters oracle generators still importing deleted package
  sources without a named durable Rust fixture owner.
- Root `tests/**` still importing runtime-local/adapters package APIs or direct
  package source paths after their behavior has a Rust/CLI owner.
- Any adapter source type reachable from surviving local execution still lacks
  a Rust adapter or explicit fail-closed receipt evidence.
- Any custom execution-adapter authoring path still depends on
  `@runxhq/runtime-local` or `@runxhq/adapters` because the language-neutral
  external protocol is missing or too narrow.
- Any source-event, hosted-runtime, catalog/read-model, or thread/outbox provider
  queue still depends on `@runxhq/runtime-local` or `@runxhq/adapters`, is
  silently folded into the external execution-adapter protocol, or has no named
  stable disposition.
- Any surviving lane requiring provider secrets lacks
  `credential-broker-delivery-contract-v1` coverage or uses ad hoc env,
  metadata, receipt, or provider-specific JSON secret delivery.
- Cloud `agent-runner` binding remains uninspected but deletion depends on it,
  or any spec claims `rust-aster-runtime-cutover` settled that binding.
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
