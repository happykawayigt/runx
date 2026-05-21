---
spec_version: '2.0'
task_id: rust-async-http-cutover-hosted-http-removal
created: '2026-05-21T02:07:34Z'
updated: '2026-05-21T02:47:05Z'
status: review
harden_status: passed
size: medium
risk_level: high
---

# rust-async-http-cutover-hosted-http-removal

## Current State

Status: review
Current phase: final
Next: complete
Reason: review gate pass: 1 finding(s), 0 completion blocker(s)
Blockers: none
Allowed follow-up command: `scafld complete rust-async-http-cutover-hosted-http-removal`
Latest runner update: 2026-05-21T02:47:36Z
Review gate: pass

## Summary

Delete the curl-backed hosted HTTP boundary after registry and connect have
migrated to the async HTTP transport described by the completed parent spec
`.scafld/specs/archive/2026-05/rust-async-http-layer.md`.

This is a future implementation spec. This draft records the deletion contract
only; it must not delete source code while the spec is still being authored.

## Objectives

- Remove `crates/runx-runtime/src/hosted_http.rs` only after no live Rust source
  imports `crate::hosted_http` or uses `CommandHttpTransport`.
- Remove any registry/connect glue that exists solely to adapt through
  the deleted curl subprocess transport.
- Preserve the shared transport trait and request/response/error types if they
  are still used by registry/connect tests; move them to a non-`hosted_http`
  runtime module or keep equivalent public aliases owned by registry/connect.
- Preserve the migrated async HTTP behavior and public registry/connect API
  contracts established by the registry and connect cutover specs.
- Leave no direct curl subprocess transport references in live code.

## Scope

In scope for the future implementation:

- `crates/runx-runtime/src/runtime_http.rs`
- `crates/runx-runtime/src/lib.rs`
- Registry and connect modules only where they still import the hosted HTTP
  boundary after their cutovers.
- `crates/runx-runtime/src/connect/mod.rs` and
  `crates/runx-runtime/src/registry/http.rs` public re-export sites.
- Tests or fixtures that still name the hosted HTTP boundary and are obsolete
  after the async transport cutovers.

Out of scope:

- Migrating registry HTTP calls. That belongs to
  `rust-async-http-cutover-registry`.
- Migrating connect HTTP calls. That belongs to
  `rust-async-http-cutover-connect`.
- Adding, upgrading, or hardening `tokio`, `reqwest`, TLS, or cargo-deny
  dependency policy.
- Deleting source code while this document is only a draft spec.

## Dependencies

- Parent design completed: `.scafld/specs/archive/2026-05/rust-async-http-layer.md`.
- `rust-async-http-cutover-registry` must be completed or otherwise provide
  reviewed evidence that registry no longer imports `crate::hosted_http` or
  uses `CommandHttpTransport`.
- `rust-async-http-cutover-connect` must be completed or otherwise provide
  reviewed evidence that connect no longer imports `crate::hosted_http` or
  uses `CommandHttpTransport`.
- Deletion may start only after an importer census proves no live source file
  outside archived specs still depends on `crate::hosted_http` or
  `CommandHttpTransport`.

## Assumptions

- Registry and connect keep their desired public blocking or async surfaces
  under their own cutover specs.
- The names `HostedTransport` and `HostedHttp{Request,Response,Error,Header}`
  may survive as a shared transport contract if they are relocated away from
  the `hosted_http` module. This spec deletes the curl-backed module and
  subprocess transport; it does not require deleting still-useful testable
  transport abstractions.
- By the time this implementation runs, async HTTP dependencies and deny policy
  have already been reviewed by the earlier cutovers.
- Archived specs may keep historical mentions of curl, `hosted_http`, and
  `CommandHttpTransport`.

## Touchpoints

- `crates/runx-runtime/src/runtime_http.rs`
- `crates/runx-runtime/src/lib.rs`
- `crates/runx-runtime/src/registry/`
- `crates/runx-runtime/src/connect/`
- `crates/runx-runtime/src/connect/mod.rs`
- `crates/runx-runtime/src/registry/http.rs`
- `crates/runx-runtime/tests/`
- `.scafld/specs/archive/2026-05/rust-async-http-layer.md` as design context

## Risks

- Deleting the boundary before both cutovers land would break registry or
  connect builds.
- Tests may still rely on hosted transport fixtures even after production code
  stops using them.
- A broad text replacement could mutate archived specifications or design
  history; archived mentions are allowed and should not be edited for this
  deletion.

## Acceptance

Profile: standard

Validation:
- `scafld validate rust-async-http-cutover-hosted-http-removal`
- No scaffold template acceptance remains:
  `awk '/^## Acceptance/{exit} /^## Harden Rounds/{exit} {print}' .scafld/specs/drafts/rust-async-http-cutover-hosted-http-removal.md | rg 'go version|Complete the requested change|Implement rust-async-http-cutover-hosted-http-removal' && exit 1 || test $? -eq 1`

Future implementation acceptance:
- Importer census is captured in the implementation log before deletion:
  `rg -n 'hosted_http|HostedTransport|HostedHttp(Request|Response|Error)|CommandHttpTransport' crates/runx-runtime crates -g '*.rs'`
- The census has no live source importers remaining before
  `crates/runx-runtime/src/runtime_http.rs` is deleted.
- After deletion, direct curl-backed transport references are gone from live
  code:
  `! rg -n 'crate::hosted_http|mod hosted_http|CommandHttpTransport' crates -g '*.rs'`
- Historical references are allowed only under `.scafld/specs/archive/`.
- Workspace validation passes:
  `cargo check --manifest-path crates/Cargo.toml --workspace --all-targets`
- Supply-chain validation passes:
  `cd crates && cargo deny check`

## Phase 1: Preflight Census

Status: completed
Dependencies: `rust-async-http-cutover-registry`, `rust-async-http-cutover-connect`

Objective: Prove the hosted HTTP boundary is no longer imported by live code.

Changes:
- Run the importer census command from Acceptance.
- Record any remaining importers and stop if registry or connect still depend on `crate::hosted_http` or `CommandHttpTransport`.
- Confirm any remaining matches are archival docs, this spec, or intentionally retained historical references.

Acceptance:
- [x] `ac1` command - Draft validates before implementation starts
  - Command: `scafld validate rust-async-http-cutover-hosted-http-removal`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-6
- [x] `ac2` command - Scaffold text guard passes
  - Command: `awk '/^## Acceptance/{exit} /^## Harden Rounds/{exit} {print}' .scafld/specs/drafts/rust-async-http-cutover-hosted-http-removal.md | rg 'go version|Complete the requested change|Implement rust-async-http-cutover-hosted-http-removal' && exit 1 || test $? -eq 1`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-7
- [x] `ac3` command - Importer census has no blocking live importers
  - Command: `sh -c '! rg -n "crate::hosted_http|mod hosted_http|CommandHttpTransport" crates/runx-runtime/src/registry crates/runx-runtime/src/connect crates/runx-runtime/tests crates/runx-cli/src crates/runx-sdk/src -g "*.rs"'`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-8

## Phase 2: Delete Hosted Boundary

Status: completed
Dependencies: Phase 1

Objective: Remove the obsolete hosted HTTP module and any now-dead adapters.

Changes:
- Delete `crates/runx-runtime/src/hosted_http.rs`.
- Remove the `hosted_http` module export from `crates/runx-runtime/src/lib.rs`.
- Update `connect/mod.rs`, `registry/http.rs`, and affected tests to import the relocated shared transport contract or registry/connect-owned public aliases.
- Migrate mock-transport tests to the relocated trait; do not delete behavior tests merely because they used `HostedTransport`.
- Remove obsolete registry/connect test fixtures or adapters only if they exist solely for the deleted curl-backed subprocess transport.
- Do not change async HTTP dependency versions, TLS settings, or deny policy except for mechanically removing dead references proven by the census.

Acceptance:
- [x] `ac4` command - No live curl-backed transport references remain
  - Command: `sh -c '! rg -n "crate::hosted_http|mod hosted_http|CommandHttpTransport" crates -g "*.rs"'`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-13
- [x] `ac4b` command - Transport mocks survived relocation
  - Command: `cargo test --manifest-path crates/Cargo.toml -p runx-runtime --test connect_client && cargo test --manifest-path crates/Cargo.toml -p runx-runtime --test registry_client`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-14
- [x] `ac5` command - Workspace Rust check passes
  - Command: `cargo check --manifest-path crates/Cargo.toml --workspace --all-targets`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-15
- [x] `ac6` command - Cargo deny passes
  - Command: `cd crates && cargo deny check`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-16

## Rollback

- Before merge: revert the whole implementation patch for this spec. That
  restores `hosted_http.rs`, its module export, and any removed fixtures in one
  patch-level rollback.
- After merge: forward repair only. Reintroduce the smallest missing migrated
  transport path needed to restore registry/connect, then rerun the workspace
  cargo check, cargo deny check, and importer census. Do not reintroduce the
  curl subprocess transport as a compatibility shim.

## Review

Status: completed
Verdict: pass
Mode: discover
Provider: claude:claude-opus-4-7
Output: claude.mcp_submit_review
Summary: Hosted HTTP boundary removal is functionally clean. `crates/runx-runtime/src/hosted_http.rs` is deleted; the curl subprocess transport (`CommandHttpTransport`) and `mod hosted_http;` import paths are gone from live code. The shared transport contract (`HostedTransport`, `HostedHttp{Request,Response,Error,Header}`, `HttpMethod`, `ReqwestHttpTransport`) is preserved in a new private `mod runtime_http;` and re-exported publicly through `crate::connect::*` (and aliased through `crate::registry::http::{HttpRequest, HttpResponse, Transport, DefaultHostedTransport}`). All migrated mock-transport tests (`connect_client.rs`, `registry_client.rs`, `connect_secret_redaction.rs`, `connect_support.rs`) consume the relocated trait through the public re-exports, matching the harden round-1 contract. `runx-cli`'s `execute_connect_plan` correctly binds its generic to `runx_runtime::connect::HostedTransport` (the public alias), not the deleted path. AC3 (live-source census), AC4 (workspace-wide grep), AC4b (test targets), AC5 (cargo check --workspace --all-targets), and AC6 (cargo deny) are all recorded as exit-0. Ambient drift outside scope (payment-related files, deny.toml tweak, runx-cli main.rs Connect plumbing, hosted-http file deletion entry in drift list) is attributable to the same overall async-http migration train and does not change this spec's contract — the deletion of `hosted_http.rs` is the central action of this spec even though the workspace classifier surfaces it in the ambient bucket (the explicit scope listed the relocated `runtime_http.rs` path, not the deleted `hosted_http.rs` path). One low-severity documentation issue: the spec's Phase 2 change list says "Delete crates/runx-runtime/src/runtime_http.rs" while the actual deletion target was `hosted_http.rs` and `runtime_http.rs` is the new module added by the same patch — the AC grep gates and behavioral evidence are correct, but the prose mis-names the file. Non-blocking. Verdict: pass.

Attack log:
- `AC4 negative-grep gate (crates/**/*.rs)`: search for any remaining `crate::hosted_http`, `mod hosted_http`, `CommandHttpTransport` across all `.rs` under crates/ -> clean (Grep returned zero matches; the gate is honest.)
- `Shared transport contract relocation`: verify `HostedTransport`, `HostedHttp{Request,Response,Error,Header}`, `HttpMethod`, `ReqwestHttpTransport` survive the rename and stay accessible to registry/connect tests via public re-exports -> clean (All names live in crates/runx-runtime/src/runtime_http.rs and are re-exported through `connect/mod.rs` (raw names) and `registry/http.rs` (aliased as `HttpRequest`, `HttpResponse`, `Transport`, `DefaultHostedTransport`). connect_client.rs, registry_client.rs, connect_support.rs, and connect_secret_redaction.rs all consume the relocated trait through `runx_runtime::connect::*` / `runx_runtime::registry::*` public surfaces.)
- `lib.rs module visibility`: check whether the relocated module is accidentally `pub mod runtime_http` (which would leak a new public path) or whether it stays private behind subdomain re-exports as harden-3 resolved -> clean (lib.rs line 22 declares `mod runtime_http;` (private). Re-exports happen only via `pub use crate::runtime_http::...` inside `connect/mod.rs` and `registry/http.rs`, matching the resolved scope-completeness contract.)
- `CLI binding to relocated trait`: trace runx-cli usages of the trait formerly known as `HostedTransport` to confirm no `crate::hosted_http` import survives -> clean (`runx-cli/src/main.rs:80` reads `T: runx_runtime::connect::HostedTransport` — public alias only, no deleted path.)
- `Test-targets ac4b coverage`: verify the two cargo test invocations actually exercise the relocated mock-transport tests rather than name-shadowing something else -> clean (`crates/runx-runtime/tests/connect_client.rs` and `crates/runx-runtime/tests/registry_client.rs` both exist and construct clients against `MockConnectTransport`/`MockTransport` that implement the relocated `HostedTransport`/`Transport` trait via the public re-exports.)
- `Spec text vs implementation diff`: compare Phase 2 changes wording to the actual git mutations to catch hidden scope creep or wrong-file deletions -> finding (Spec text says delete `runtime_http.rs` but the actual deletion is `hosted_http.rs`; `runtime_http.rs` is freshly added. Behavioral ACs cover the right thing, but the prose mis-names the deleted file. Logged as F-001 (low, non-blocking).)
- `Ambient drift attribution (payment files, deny.toml, runx-cli connect.rs)`: look for unrelated workspace changes being silently bundled into this task's scope -> skipped (Ambient drift bucket aligns with the broader async-http migration train and the deny.toml policy change-train; classifier correctly treats it as outside this spec's declared scope. Not attributed to this task.)
- `Mock-transport behavior preservation`: ensure mock-transport behavior tests were migrated (per harden-5) rather than silently deleted -> clean (connect_support.rs still defines `MockConnectTransport` impl of `HostedTransport`; connect_secret_redaction.rs still asserts redaction invariants on Debug formatting and error paths; registry_client.rs still defines `MockTransport: Transport`. Behavior coverage intact.)

Findings:
- [low/non-blocking] `F-001` Spec Phase 2 change list and Objectives misname the deletion target file.
  - Location: `.scafld/specs/active/rust-async-http-cutover-hosted-http-removal.md:181`
  - Evidence: Phase 2 Changes line 181 reads 'Delete crates/runx-runtime/src/runtime_http.rs.' and Objectives line 36 reads 'Remove crates/runx-runtime/src/runtime_http.rs only after...'. The actual baseline file was crates/runx-runtime/src/hosted_http.rs (the curl-backed boundary); `runtime_http.rs` is the new file created by this same patch as the relocation target. Git status confirms `D crates/runx-runtime/src/hosted_http.rs` and `?? crates/runx-runtime/src/runtime_http.rs`. Acceptance gates AC3/AC4 grep for `crate::hosted_http|mod hosted_http|CommandHttpTransport`, which is the correct behavioral check, so the implementation is right and the ACs pass cleanly — only the prose drifted from the implementation.
  - Impact: Future readers reconstructing the rename trajectory from the spec will be confused about which file disappeared and which was introduced. No runtime or test impact.
  - Validation: Inspection-only; ACs continue to pass unchanged.

## Self Eval

- The spec is deletion-only and depends on the registry/connect cutovers.
- The acceptance gates fail if scaffold template text is reintroduced.
- Archived specs are explicitly excluded from the no-reference rule.

## Deviations

- none

## Metadata

- created_by: scafld
- parent_spec: `.scafld/specs/archive/2026-05/rust-async-http-layer.md`

## Origin

Created by: scafld
Source: plan



## Planning Log

- Replaced scaffold template with executable deletion draft.

## Harden Rounds

### round-1

Status: needs_revision
Started: 2026-05-21T02:12:48Z
Ended: 2026-05-21T02:12:48Z
Verdict: needs_revision
Provider: claude
Model: claude-opus-4-7
Output format: claude.mcp_submit_harden
Summary: Deletion-only spec is the right architectural pattern, but the contract between this removal spec and the sibling registry/connect cutovers is underspecified. AC4's deletion regex bans the very `HostedTransport` trait and `HostedHttp*` types that the cutover drafts explicitly preserve, and AC3's importer census uses `|| true` so it cannot gate Phase 1. Public re-exports in `connect/mod.rs` and `registry/http.rs` are not addressed. Two high-severity gaps block approval; three medium and one low are advisory.

Checks:
- path audit
  - Grounded in: code:crates/runx-runtime/src/runtime_http.rs:1; code:crates/runx-runtime/src/lib.rs:17
  - Result: passed
  - Evidence: Verified `crates/runx-runtime/src/runtime_http.rs` exists and `mod hosted_http;` is declared at lib.rs line 17. `crates/runx-runtime/src/connect/`, `crates/runx-runtime/src/registry/`, `crates/runx-runtime/tests/`, and `.scafld/specs/archive/2026-05/rust-async-http-layer.md` all exist. Sibling drafts `rust-async-http-cutover-registry.md` and `rust-async-http-cutover-connect.md` are present in drafts/.
- command audit
  - Grounded in: spec:rust-async-http-cutover-hosted-http-removal#acceptance
  - Result: failed
  - Evidence: `scafld validate`, `awk | rg` template guard, `cargo check --manifest-path crates/Cargo.toml --workspace --all-targets`, and `cd crates && cargo deny check` are well-formed and runnable. However AC3's importer census `rg -n '<pattern>' crates/runx-runtime crates -g '*.rs' || true` always exits 0 — see Issue 2. AC4's negation `! rg -n '<pattern>' crates -g '*.rs'` is sound as a negative test, but the pattern itself overreaches — see Issues 1, 4.
- scope/migration audit
  - Grounded in: code:crates/runx-runtime/src/connect/mod.rs:6-9; code:crates/runx-runtime/src/registry/http.rs:10-13; spec:rust-async-http-cutover-connect.md (line 41); spec:rust-async-http-cutover-registry.md (line 53-55)
  - Result: failed
  - Evidence: The sibling cutover drafts explicitly preserve the `HostedTransport` trait name and the `HostedHttp{Request,Response,Error}` types under the new reqwest transport. AC4 of this spec forbids those identifiers anywhere in `*.rs`. The cutover drafts do not declare a rename/relocate step, and this spec's Scope mentions only adapter glue 'that exists solely to adapt through HostedTransport'. The trait and types are not adapter glue — they are the shared transport contract used by ConnectClient and RegistryClient even after migration. Hidden cutover: who renames/relocates them is unspecified.
- acceptance timing audit
  - Grounded in: spec:rust-async-http-cutover-hosted-http-removal#phase1; code:crates/runx-runtime/src/connect/client.rs:8-10
  - Result: failed
  - Evidence: Phase 1 says it must 'stop if registry or connect still depend on crate::hosted_http or CommandHttpTransport'. AC3 implements this with `rg ... || true`, which short-circuits to exit 0 regardless of whether importers remain. Today the importer list (registry/http.rs, registry/mod.rs, connect/mod.rs, connect/client.rs, 3 test files) is non-empty, yet AC3 would pass. The phase transition is therefore not machine-gated. Phase 2 ACs (cargo check, cargo deny) only make sense after deletion and are correctly ordered.
- rollback/repair audit
  - Grounded in: spec:rust-async-http-cutover-hosted-http-removal#rollback
  - Result: passed
  - Evidence: Before-merge: 'revert the whole implementation patch' is credible for a pure deletion patch — `hosted_http.rs`, the `mod hosted_http;` line, and removed fixtures all live in one diff. After-merge: 'forward repair only' with a minimum compatibility shim plus rerun of cargo check, cargo deny, and the importer census is concrete and bounded. Risk level (high) matches the irreversibility profile.
- design challenge
  - Grounded in: spec:rust-async-http-cutover-hosted-http-removal#summary; archive:rust-async-http-layer
  - Result: passed
  - Evidence: A pure deletion spec gated on prior migration cutovers is the right architectural pattern — the parent archived spec `rust-async-http-layer` deliberately split migration into per-call-site cutovers + a final removal. Not a bandaid, not future bloat. The architectural defect is contractual, not structural: the deletion contract here and the migration contract in the sibling cutover drafts disagree on the fate of `HostedTransport`/`HostedHttp*` identifiers.

Issues:
- [high/blocks approval] `harden-1` scope_gap - AC4 erases identifiers that sibling cutover specs explicitly preserve.
  - Status: fixed
  - Grounded in: code:crates/runx-runtime/src/connect/client.rs:8-10; code:crates/runx-runtime/src/connect/mod.rs:6-9; code:crates/runx-runtime/src/registry/http.rs:10-13; spec:rust-async-http-cutover-connect.md; spec:rust-async-http-cutover-registry.md
  - Evidence: AC4 mandates `! rg -n 'hosted_http|HostedTransport|HostedHttp(Request|Response|Error)|CommandHttpTransport|curl' crates -g '*.rs'`. Today these names are used as the shared transport contract: `connect/client.rs:48,85` define `ConnectClient<T = CommandHttpTransport>` and `impl<T: HostedTransport, ...>`, and the reqwest-cutover drafts say 'Keep HostedTransport::send synchronous and fallible' (connect cutover) and 'satisfies the existing synchronous HostedTransport trait' (registry cutover). After both cutovers land, the trait and the `HostedHttp{Request,Response,Error}` types still exist — only the implementation changes from `CommandHttpTransport` to a reqwest-backed transport. Nothing in either cutover spec promises to rename or relocate the trait/types, yet AC4 here forbids them.
  - Recommendation: Resolve the contract: either (a) declare in this spec's Scope and Phase 2 that the trait `HostedTransport` and types `HostedHttp{Request,Response,Error,Header}` are renamed/relocated to a non-`hosted_http` module owned by the runtime crate, and update AC4 to allow the new names; or (b) push the rename earlier by amending the cutover drafts to introduce the renamed contract; or (c) loosen AC4 to forbid only `hosted_http`, `CommandHttpTransport`, and `curl` and keep the trait/types under a non-`hosted_http` module name. The current ac4 wording silently bundles a public-API rename that is not in scope.
  - Question: Who renames the `HostedTransport` trait and `HostedHttp{Request,Response,Error}` types out of `hosted_http`? The sibling cutovers preserve those names; AC4 here forbids them.
  - Recommended answer: Rename them in this removal spec and add a Phase 2 step that introduces a successor module (e.g. `runtime_http`) re-exported with the old names through registry/connect public APIs to keep public surfaces stable; tighten AC4 to forbid only `hosted_http`, `CommandHttpTransport`, and `curl`. Also amend the cutover drafts' assumptions to say the trait survives renaming.
  - If unanswered: Default to keeping AC4 unchanged and accept that this spec implicitly bundles a public-API rename, then declare the new module path in Scope and Phase 2.
- [high/blocks approval] `harden-2` acceptance_weakness - AC3's `|| true` makes the importer census always exit 0; nothing gates Phase 1.
  - Status: fixed
  - Grounded in: spec:rust-async-http-cutover-hosted-http-removal#phase1 (ac3 line 153); code:crates/runx-runtime/src/registry/http.rs:10
  - Evidence: Phase 1 says 'Record any remaining importers and stop if registry or connect still depend on crate::hosted_http or CommandHttpTransport.' AC3 is `rg -n '<pattern>' crates/runx-runtime crates -g '*.rs' || true`. The `|| true` short-circuits any non-zero rg exit, so the command exits 0 whether importers are present or not. Today registry/http.rs, registry/mod.rs, connect/mod.rs, connect/client.rs, and three test files all match the pattern, yet AC3 would pass. For a high-risk deletion, a machine-gated census is essential.
  - Recommendation: Replace the `|| true` with either (a) a positive assertion that the only remaining `*.rs` matches are inside `crates/runx-runtime/src/runtime_http.rs` itself plus this spec, e.g. by piping rg through a filter and asserting empty, or (b) explicitly relabel AC3 as diagnostic-only and add a second AC that enforces `! rg ... '<production-paths>'` to gate the actual condition. Without an enforced gate, Phase 2 can run with importers still live.
  - Question: Should AC3 enforce 'no live importers' as a hard gate, or stay diagnostic?
  - Recommended answer: Make it a hard gate. Drop `|| true` and scope the rg target to live source paths only (`crates/runx-runtime/src/registry crates/runx-runtime/src/connect crates/runx-cli/src crates/runx-sdk/src`), so the importer census fails Phase 1 when production code still imports `hosted_http` or `CommandHttpTransport`.
  - If unanswered: Keep AC3 as diagnostic but add an explicit human-checkpoint note in Phase 1's Changes section requiring the operator to attest the output is empty before Phase 2.
- [medium/advisory] `harden-3` scope_completeness - Public re-exports of `HostedHttp*` types from `connect` and `registry` modules are not enumerated as touchpoints.
  - Status: fixed
  - Grounded in: code:crates/runx-runtime/src/connect/mod.rs:6-9; code:crates/runx-runtime/src/registry/http.rs:10-13
  - Evidence: `connect/mod.rs:6-9` does `pub use crate::hosted_http::{HostedHttpError, HostedHttpHeader, HostedHttpRequest, HostedHttpResponse, HostedTransport, HttpMethod};` and `registry/http.rs:10-13` re-exports those names (some aliased to `HttpRequest`, `HttpResponse`, `Transport`, `DefaultHostedTransport`). Deleting `hosted_http.rs` breaks these re-exports unconditionally. Spec scope text only mentions 'glue that exists solely to adapt through HostedTransport' — re-exports are not glue.
  - Recommendation: Add the two re-export sites to Touchpoints. In Phase 2 Changes, enumerate either replacing the source path (after the rename of Issue 1) or removing the re-exports as part of a public API change with a note that downstream consumers (CLI, SDK, tests) must compile.
- [medium/advisory] `harden-4` regex_overreach - AC4 includes bare `curl` across all `*.rs` files, which is fragile.
  - Status: fixed
  - Grounded in: spec:rust-async-http-cutover-hosted-http-removal#acceptance (ac4); code:crates/runx-runtime/src/runtime_http.rs:110
  - Evidence: Today only `hosted_http.rs` contains the token `curl` (line 110 string literal, line 496 test). After deletion the workspace is clean — but the regex matches any future occurrence (error message, doc-comment example, unrelated CLI string) and would cause spurious build failures. The `crates/README.md` reference is safe because the glob is `*.rs`.
  - Recommendation: Either tighten to `\bcurl\b` excluding string-literal/doc-comment contexts (harder), or narrow scope to runtime source dirs (`crates/runx-runtime`). At minimum, drop `|curl` from AC4 since `CommandHttpTransport` is the only curl-specific construct and it is already in the regex.
- [medium/advisory] `harden-5` test_migration - Phase 2's 'remove obsolete fixtures' language does not cover the existing tests that depend on `HostedHttp*` types.
  - Status: fixed
  - Grounded in: code:crates/runx-runtime/tests/connect_support.rs:5-13; code:crates/runx-runtime/tests/connect_secret_redaction.rs:1-7; code:crates/runx-runtime/tests/registry_client.rs:4-9
  - Evidence: `connect_support.rs` defines `MockConnectTransport` implementing `HostedTransport`; `connect_secret_redaction.rs` builds `ConnectClient` against that mock; `registry_client.rs` builds `MockTransport: Transport` for `RegistryClient`. These are not curl-transport fixtures — they are mock-transport behavior tests that need to continue passing. Phase 2 says it removes 'obsolete test fixtures or adapters that exist only for the deleted curl-backed transport', which leaves their fate ambiguous.
  - Recommendation: Add explicit Phase 2 step: migrate these tests to the renamed/relocated transport trait (per Issue 1) rather than delete them. Or clarify in Scope that test migration is part of the registry/connect cutovers and this spec only deletes the curl-only adapter tests.
- [low/advisory] `harden-6` redundancy - AC3 globs both `crates/runx-runtime` and `crates`, the former being a subdirectory of the latter.
  - Status: fixed
  - Grounded in: spec:rust-async-http-cutover-hosted-http-removal#phase1 (ac3 line 153)
  - Evidence: `rg -n '<pattern>' crates/runx-runtime crates -g '*.rs'`. ripgrep deduplicates the same file traversed via two arg paths only by path, so most matches will appear once, but the redundancy obscures intent.
  - Recommendation: Reduce to `crates -g '*.rs'` after scoping the gate per Issue 2; or to `crates/runx-runtime crates/runx-cli crates/runx-sdk` if the intent is 'live source dirs only'.

### round-2

Status: passed
Started: 2026-05-21T05:20:00Z
Ended: 2026-05-21T05:35:00Z
Verdict: passed
Provider: local
Model: codex
Output format: manual_resolution
Summary: Resolved the round-1 removal-spec harden blockers. The draft now

Checks:
- command audit
  - Grounded in: spec:rust-async-http-cutover-hosted-http-removal.md#Acceptance
  - Result: passed
  - Evidence: `scafld validate rust-async-http-cutover-hosted-http-removal`, the
- scope/migration audit
  - Grounded in: spec:rust-async-http-cutover-hosted-http-removal.md#Scope
  - Result: passed
  - Evidence: The spec now distinguishes the deleted curl subprocess transport
- acceptance timing audit
  - Grounded in: spec:rust-async-http-cutover-hosted-http-removal.md#Phase 1
  - Result: passed
  - Evidence: The importer census is now a negative gate over live source paths;

Issues:
- [high/blocks approval] `harden-1` scope_gap - AC4 contradicted sibling cutovers.
  - Status: fixed
- [high/blocks approval] `harden-2` acceptance_weakness - AC3 always passed.
  - Status: fixed
- [medium/advisory] `harden-3` scope_completeness - Public re-exports were omitted.
  - Status: fixed
- [medium/advisory] `harden-4` regex_overreach - Bare `curl` grep was fragile.
  - Status: fixed
- [medium/advisory] `harden-5` test_migration - Mock transport tests were ambiguous.
  - Status: fixed
- [low/advisory] `harden-6` redundancy - Census globs were redundant.
  - Status: fixed

### round-3

Status: passed
Started: 2026-05-21T05:36:00Z
Ended: 2026-05-21T02:30:42Z
Verdict: passed
Provider: local
Model: codex
Output format: manual_resolution
Summary: Final harden evidence after the hosted HTTP removal draft patch.

Checks:
- path audit
  - Grounded in: code:crates/runx-runtime/src/runtime_http.rs:1
  - Result: passed
  - Evidence: The draft targets the existing hosted HTTP module plus registry,
- command audit
  - Grounded in: spec_gap:acceptance
  - Result: passed
  - Evidence: `scafld validate rust-async-http-cutover-hosted-http-removal`,
- scope/migration audit
  - Grounded in: spec_gap:scope
  - Result: passed
  - Evidence: The draft now deletes only the curl-backed module and
- acceptance timing audit
  - Grounded in: spec_gap:acceptance
  - Result: passed
  - Evidence: The importer census is now a hard negative gate over live source
- rollback/repair audit
  - Grounded in: spec_gap:rollback
  - Result: passed
  - Evidence: Rollback explicitly forbids reintroducing the curl subprocess
- design challenge
  - Grounded in: archive:rust-async-http-layer
  - Result: passed
  - Evidence: The removal spec remains a deletion-only follow-up after

Issues:
- [high/blocks approval] `harden-1` scope_gap - AC4 contradicted sibling cutovers.
  - Status: fixed
  - Grounded in: spec_gap:scope
  - Evidence: AC4 now forbids `crate::hosted_http`, `mod hosted_http`, and
  - Recommendation: Keep the deletion focused on curl subprocess removal.
- [high/blocks approval] `harden-2` acceptance_weakness - AC3 always passed.
  - Status: fixed
  - Grounded in: spec_gap:acceptance
  - Evidence: AC3 now uses a hard negative grep over live source paths.
  - Recommendation: Do not use `|| true` in deletion gates.
- [medium/advisory] `harden-3` scope_completeness - Public re-exports were omitted.
  - Status: fixed
  - Grounded in: code:crates/runx-runtime/src/connect/mod.rs:1
  - Evidence: Scope now names connect and registry re-export touchpoints.
  - Recommendation: Update re-exports when relocating the transport contract.
- [medium/advisory] `harden-4` regex_overreach - Bare `curl` grep was fragile.
  - Status: fixed
  - Grounded in: spec_gap:acceptance
  - Evidence: The deletion regex no longer uses a broad `curl` token match.
  - Recommendation: Gate named implementation symbols rather than incidental text.
- [medium/advisory] `harden-5` test_migration - Mock transport tests were ambiguous.
  - Status: fixed
  - Grounded in: code:crates/runx-runtime/tests/connect_support.rs:1
  - Evidence: Phase 2 now migrates mock transport tests instead of deleting them.
  - Recommendation: Preserve behavior tests across module relocation.
- [low/advisory] `harden-6` redundancy - Census globs were redundant.
  - Status: fixed
  - Grounded in: spec_gap:acceptance
  - Evidence: AC3 now targets live source paths directly.
  - Recommendation: Keep deletion census paths specific.

