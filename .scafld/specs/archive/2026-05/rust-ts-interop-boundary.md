---
spec_version: '2.0'
task_id: rust-ts-interop-boundary
created: '2026-05-18T00:00:00Z'
updated: '2026-05-19T01:26:57Z'
status: completed
harden_status: not_run
size: medium
risk_level: high
---

# Rust / TS interop boundary

## Current State

Status: completed
Current phase: final
Next: done
Reason: task completed
Blockers: none
Allowed follow-up command: `none`
Latest runner update: 2026-05-19T01:26:57Z
Review gate: pass

## Summary

Document the disposition of every TS package the runx workspace ships
today, the stable boundary it crosses to reach the Rust runtime, and the
ownership of any contract surface required for that crossing.

The goal is zero drift: when a downstream spec asks "what happens to
`@runxhq/X`?", the answer is here, not invented per-spec. If a TS package
isn't listed below, it doesn't exist in the workspace.

## Context

CWD: `.`

Packages enumerated and dispositioned:

**OSS workspace (`oss/packages/`):**
- `@runxhq/adapters`: folded into `rust-ts-sunset-runtime-local`.
  Adapter logic moves to `runx-runtime::adapters::{cli_tool, agent,
  catalog, a2a, mcp}` via the four adapter port specs. Sunset coordinates
  with runtime-local.
- `@runxhq/authoring`: open question. Two options: (a) port to Rust as
  `rust-authoring` (new follow-up spec) when the surface stabilizes, or
  (b) migrate into scafld. Decision lives in `plans/runx-authoring-dx.md`,
  not this spec. Until decided, `@runxhq/authoring` stays unchanged.
- `@runxhq/cli`: survives indefinitely as a platform-aware npm launcher.
  `rust-cli-rust-cutover` flips its behavior from "npm exec the TS CLI"
  to "download and exec the bundled Rust binary". No TS deletion.
- `@runxhq/contracts`: stays. After Rust takeover, contracts ship as a
  manually maintained TS package cross-validated against `runx-contracts`
  fixtures (current model, formalized). A future `contracts-codegen-from-rust`
  spec can replace manual maintenance with codegen if motivated; not in
  scope here.
- `@runxhq/core`: fully sunset across eight `rust-ts-sunset-*` specs
  (state-machine, policy, parser, executor, receipts, registry,
  marketplaces, plus the surviving shell deleted at the end of the
  runtime-local sunset if empty).
- `@runxhq/create-skill`: stays as a thin npm bootstrapper that wraps
  `runx new` against the bundled Rust binary. No TS reimplementation
  needed; the package becomes a one-file shell that defers to the CLI.
- `@runxhq/host-adapters`: stays. Published npm package, "thin host
  response adapters over the runx host protocol". Consumed by external
  users of the host protocol. Its dependency on TS host-protocol types
  retargets to `@runxhq/contracts` (which already mirrors
  `runx-contracts::host_protocol`).
- `@runxhq/langchain`: stays. Published npm package, "optional LangChain
  bridge for runx tool catalogs and governed workflow tools". Bridge
  shells the `runx` CLI for skill/tool invocation; works through cutover
  unchanged.
- `@runxhq/runtime-local`: fully sunset via
  `rust-ts-sunset-runtime-local`. Adapter logic, runner-local,
  process-sandbox, harness, MCP server, SDK caller / host-protocol all
  move to `runx-runtime` and sibling crates.
- `@runxhq/sdk-python` (`runx-py` on PyPI): stays. Thin Python client
  over the `runx` CLI JSON output. CLI JSON contract preservation
  (enforced by `rust-cli-feature-parity-matrix`) keeps it working through
  cutover.

**Cloud workspace (`cloud/packages/`):**
- All cloud packages remain TS. The Rust runtime consumes their HTTP
  surfaces via the contract owned by
  `cloud-http-contract-stabilization.yaml` (this spec's program-level
  partner). No cloud package is sunset by this program; cloud cutovers
  are their own future specs when motivated.

Files impacted (this spec is documentation; minimal repo touches):
- `oss/docs/ts-interop-boundary.md` (new; canonical version of section
  "Current Boundary" below)
- `plans/rust-takeover.md` (cross-reference to this spec)
- Per-package README.md updates where the disposition is non-obvious
  (`@runxhq/contracts`, `@runxhq/create-skill`, `@runxhq/host-adapters`,
  `@runxhq/langchain`, `runx-py`)

## Current Boundary (after the takeover)

Three crossings exist between the surviving TS surface and the Rust
runtime, each with a single contract surface that owns the wire shape:

1. **CLI JSON.** Anything that shells `runx` (langchain bridge, runx-py,
   user scripts, CI workflows). Contract: every command's JSON output
   shape, exit codes, and human-output stability. Owned by
   `rust-cli-feature-parity-matrix`.

2. **Published TS contracts.** Anything that imports from
   `@runxhq/contracts` (host-adapters, cloud packages, external TS
   consumers). Contract: TS shapes that mirror `runx-contracts` Rust
   types. Owned by `rust-contracts-parity` (Rust side) and
   `@runxhq/contracts` package maintainers (TS side, with fixture cross-
   validation).

3. **Cloud HTTP contracts.** Anything where the Rust runtime calls
   cloud (approval routing, connect/auth, registry, receipts-store).
   Contract: versioned, documented HTTP endpoints. Owned by
   `cloud-http-contract-stabilization.yaml`.

Invariants:
- No fourth boundary is added without amending this spec.
- No published TS package is silently broken. Each disposition is named
  here.
- "Stays" means the package keeps existing and publishing; it does not
  mean "no edits ever". Stable-boundary edits to consume new
  `runx-contracts` versions are normal maintenance.
- "Sunset" means deletion via the named sunset spec; nothing else
  qualifies.
- Disposition changes require updating this spec first, then any
  affected spec second.

## Ultimate Shape (post-cutover, no TS oracle)

The end-state is a Rust monolith with thin TS bridges. The directory
ownership inverts during the cutover: today `packages/` is the home and
`crates/` is the satellite; at the end, `crates/` is the home and
`packages/` shrinks to bridges. Every choice during the dual-tree window
should accelerate that inversion.

```
oss/
  Cargo.toml                  workspace root
  crates/
    runx-contracts            wire + contract source of truth (published)
    runx-core                 pure decisions: state-machine, policy
    runx-parser               pure parsing
    runx-receipts             pure receipt model + verification
    runx-runtime              impure: runner, adapters, sandbox, MCP
    runx-cli                  native binary
    runx-sdk                  Rust SDK (published)
  fixtures/                   Tier 2 contracts, durable, language-agnostic
    contracts/  parser/  receipts/  runtime/  cli-parity/
  tests/                      Tier 3 black-box, language-agnostic
    cli/                      spawn binary + assert
    integration/              multi-component flows + cloud HTTP
  packages/                   surviving TS bridges (per dispositions above)
    cli                       npm launcher
    contracts                 TS view of runx-contracts (manual or codegen)
    create-skill              npm bootstrap
    host-adapters             published bridge
    langchain                 published bridge
    sdk-python                published Python bridge (PyPI: runx-py)
  docs/
cloud/                        unchanged by this program
```

Three test tiers, each with one ownership rule:

1. **Tier 1 unit tests.** Language-owned. TS tests TS, Rust tests Rust.
   When a TS domain sunsets, its unit tests go with it.
2. **Tier 2 contract / parity tests.** Language-agnostic. JSON fixtures
   under `oss/fixtures/`, consumed by fixture-runners in either
   language. Generators in TS today (oracle); generators retire when TS
   sunsets, fixtures remain as the regression contract.
3. **Tier 3 end-to-end / acceptance.** Black-box. Spawn the `runx`
   binary, hit cloud HTTP, assert on stdout / exit / JSON. Lives at
   `oss/tests/cli/` and `oss/tests/integration/`. Survives the launcher
   flip unchanged because it never imports anything.

The TS oracle is a wasting asset. Once a TS domain sunsets, no new
fixtures can be derived from TS for that domain. Maximize fixture
coverage while both implementations exist. After sunset, new behavior
validates against schema + proptest only.

## Waste Avoidance

Five rules that prevent rework. Spec authors operating under
`plans/rust-takeover.md` must follow them:

1. **Author Tier 3 tests in language-agnostic shape from day one.**
   Spawn the `runx` binary, assert on stdout / exit / JSON. Never
   import TS internals into a CLI integration test. The test survives
   the launcher flip unchanged. `rust-cli-feature-parity-matrix` is
   structured for this; new spec authors match the pattern.

2. **Fixtures live at the top level.** `oss/fixtures/parser/` not
   `oss/packages/core/src/parser/__fixtures__/`. Sunsetting the TS
   package does not delete the durable artifact.

3. **No new features in TS domains queued for sunset.** Add to
   `runx-runtime` directly once that crate exists. If a feature must
   ship pre-runtime, write it in TS *and* fixture-capture it in the
   same PR. Never land TS-only behavior with no fixture.

4. **Schema-first contract definition.** New JSON shapes start as
   `runx-contracts` typed structs, then mirror to `@runxhq/contracts`
   (or codegen later). Never start in TS and mirror to Rust; that
   direction creates drift bait.

5. **Do not double-author tests.** If a TS test verifies behavior,
   derive a fixture from it. Writing the same assertion in two
   languages doubles maintenance and creates two places for the
   assertion to drift apart.

The compass each spec author carries before writing code or tests:

> Does this artifact exist in the end-state shape? If yes, put it in
> its end-state home now, even during the dual-tree window. If no,
> minimize it ruthlessly and plan its deletion.

## Scaffolding Inventory

Every dual-tree-only artifact with its retirement trigger. Anything
not on this list is durable; anything on this list is scaffolding to
keep minimal and delete on schedule.

**Fixture generators (TS oracle scripts):**
- `scripts/generate-rust-contract-fixtures.ts`: retired by
  `contracts-codegen-from-rust` (future spec) or by complete sunset of
  `@runxhq/contracts` (not in this program).
- `scripts/generate-rust-parser-fixtures.ts`: retired by
  `rust-ts-sunset-parser`.
- `scripts/generate-rust-receipt-fixtures.ts`: retired by
  `rust-ts-sunset-receipts`.
- `scripts/generate-rust-fanout-fixtures.ts`: retired by
  `rust-ts-sunset-runtime-local`.
- `scripts/generate-rust-skill-fixtures.ts`: retired by
  `rust-ts-sunset-runtime-local`.
- `scripts/generate-rust-approval-fixtures.ts`: retired by
  `rust-ts-sunset-executor` (gate types) + `rust-ts-sunset-runtime-local`
  (runner side).
- `scripts/generate-rust-cli-fixtures.ts`: retired by
  `rust-cli-rust-cutover` (the launcher flip; after that the CLI
  oracle is the Rust binary itself).

**TS-side unit tests in domains queued for sunset:**
- `packages/core/src/state-machine/index.test.ts`: `rust-ts-sunset-state-machine`.
- `packages/core/src/policy/index.test.ts`: `rust-ts-sunset-policy`.
- `packages/core/src/parser/index.test.ts`: `rust-ts-sunset-parser`.
- `packages/core/src/executor/*.test.ts`: `rust-ts-sunset-executor`.
- `packages/core/src/receipts/*.test.ts`: `rust-ts-sunset-receipts`.
- `packages/core/src/registry/*.test.ts`: `rust-ts-sunset-registry`.
- `packages/core/src/marketplaces/*.test.ts`: `rust-ts-sunset-marketplaces`.
- `packages/runtime-local/**/*.test.ts`: `rust-ts-sunset-runtime-local`.
- `packages/adapters/**/*.test.ts`: `rust-ts-sunset-runtime-local`
  (bundled with adapters deletion).

**TS-side build / lint infrastructure that retires with TS:**
- `oss/scripts/check-boundaries.mjs`: retired when the last TS source
  domain it covers sunsets (after `rust-ts-sunset-marketplaces`).
- `vitest.config.ts` / `vitest.workspace-aliases.ts`: survive as long
  as any TS package keeps unit tests; final retirement only after every
  bridge package's tests move to Rust integration shape (open question,
  not blocking).
- The cross-language diff CI lane: retired when the last TS source
  domain sunsets.

**Things people sometimes mistake for scaffolding but are durable:**
- `scripts/check-rust-core-style.mjs`: checks Rust style. No TS
  dependency. Durable.
- `scripts/check-rust-crate-graph.mjs`: checks crate boundaries. Durable.
- `oss/fixtures/**`: durable. Generators retire; fixtures stay.
- `packages/cli/bin/runx.js`: durable as the npm launcher shim. Body
  changes when `rust-cli-rust-cutover` lands; the file itself stays.

## Objectives

- Land this disposition table as the canonical TS-interop reference.
- Land the ultimate shape and waste-avoidance section as the compass
  every spec author under `plans/rust-takeover.md` reads first.
- Add `oss/docs/ts-interop-boundary.md` matching the sections above.
- Update each affected package's README to point at this document for
  the disposition story.
- Verify every existing rust-* spec under
  `oss/.scafld/specs/drafts/` references this spec when its scope
  touches a TS package fate.

## Scope

In scope:
- Disposition documentation.
- README/docs updates for the affected packages.
- Cross-spec consistency check (the verify step above).

Out of scope:
- Any code change to a TS package beyond README updates.
- The actual sunset work (each sunset spec owns its own deletion).
- The contracts-codegen-from-rust decision (future spec when motivated).
- Authoring disposition (lives in `plans/runx-authoring-dx.md`).

## Dependencies

- None for landing the disposition table. Downstream specs cite this
  document; they do not gate on it.
- `cloud-http-contract-stabilization` scoped in parallel as the
  cloud-side contract owner. This spec names it as the owner of the
  cloud HTTP crossing; that spec produces the actual stabilization.

## Open Questions

- Authoring fate: port vs migrate to scafld. Lives in
  `plans/runx-authoring-dx.md`.
- Contracts codegen from Rust: future spec, not blocking this one.
- Whether `@runxhq/host-adapters` ever gets a Rust sibling crate. Today
  it has none; not blocking.

## Review

Status: completed
Verdict: pass
Mode: discover
Provider: command
Output: command.stdout
Summary: Documentation-only TS interop boundary is coherent: canonical docs page added, bridge package READMEs point to it, and no runtime code changed.

Attack log:
- `scope`: Checked changed paths against docs-only spec scope -> clean (Changes are docs/README/spec lifecycle files only for this task.)
- `package dispositions`: Compared documented package list with packages directory -> clean (All surviving package dispositions are named in docs/ts-interop-boundary.md.)
- `risk`: Checked for code/runtime/config changes -> clean (No package source code or public API behavior changed.)

Findings:
- none

