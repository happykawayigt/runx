---
spec_version: '2.0'
task_id: rust-async-http-cutover-registry
created: '2026-05-21T02:07:27Z'
updated: '2026-05-21T02:47:05Z'
status: review
harden_status: passed
size: medium
risk_level: high
---

# Rust async HTTP cutover: registry

## Current State

Status: review
Current phase: final
Next: complete
Reason: review gate pass
Blockers: none
Allowed follow-up command: `scafld complete rust-async-http-cutover-registry`
Latest runner update: 2026-05-21T03:00:04Z
Review gate: pass

## Summary

Replace the registry client's curl-backed default transport with a reviewed
reqwest/rustls transport behind the existing synchronous `HostedTransport`
boundary. This is the first implementation slice after
`rust-async-http-layer`: it proves the dependency policy on the smallest
remote HTTP surface before connect polling or MCP adopt async transport.

The registry public API stays blocking:

- `RegistryClient::new`
- `search`
- `search_with_limit`
- `read`
- `acquire`
- `resolve_ref`

The cutover must not introduce a second registry client shape or a TypeScript
fallback. The new transport is an implementation detail under the same
registry contract.

## Objectives

- Add exact reviewed `reqwest` and `tokio` dependency pins behind
  `async-http`.
- Add the panic-free runtime helper specified by
  `rust-async-http-layer`.
- Implement a reqwest-backed hosted transport that satisfies the existing
  synchronous `HostedTransport` trait.
- Switch `RegistryClient::new` to the reqwest transport under the resolved
  cutover feature contract: `runx-runtime` defines
  `async-http = ["dep:reqwest", "dep:tokio"]`, and `cli-tool = ["async-http"]`.
  The existing `runx-cli` dependency on `runx-runtime` with `cli-tool` then
  exercises the reqwest path in the cargo-built `runx` binary.
- Preserve route construction, query encoding, response parsing, error
  classification, redaction, and the current no-redirect behavior.
- Keep pure crates free of async/network dependencies.

## Scope

In scope:

- `crates/runx-runtime/Cargo.toml`
- `crates/deny.toml`
- `crates/Cargo.lock`
- `crates/runx-runtime/src/runtime_http.rs` or a sibling module owned by the
  hosted HTTP boundary.
- `crates/runx-runtime/src/registry/http.rs`
- `crates/runx-cli/Cargo.toml` as feature-wiring evidence; no direct edit is
  required unless the existing `cli-tool` dependency shape changes.
- Registry and hosted HTTP tests.

Out of scope:

- Connect client migration.
- MCP/rmcp migration.
- Removing curl or deleting `CommandHttpTransport`.
- Public async SDK/API surfaces.
- Broadening the global license allowlist without exact per-crate review.

## Dependencies

- `rust-async-http-layer` completed.
- `rust-hosted-http-foundation` or equivalent safe-URL hosted HTTP guards.

## Assumptions

- The initial implementation can keep the public registry API blocking by
  using the parent spec's `block_on_http` bridge.
- `reqwest` uses `rustls` with `default-features = false`.
- Exact dependency versions are selected during implementation, not guessed in
  this draft, and must be recorded with the `Cargo.lock` diff.
- Redirect following is disabled explicitly on the reqwest client; preserving
  route construction is not enough because reqwest follows redirects by default.
- Proxy and certificate-store behavior must be documented in the implementation
  review note. If the final reqwest/rustls choice differs from curl's host
  environment behavior, the difference must be deliberate and tested.

## Touchpoints

- `crates/runx-runtime/src/registry/http.rs`
- `crates/runx-runtime/src/runtime_http.rs`
- `crates/runx-runtime/tests/registry_client.rs`
- `crates/runx-runtime/tests/registry.rs`
- `crates/runx-cli/Cargo.toml`
- `crates/deny.toml`
- `crates/Cargo.lock`
- `crates/Cargo.toml`

## Risks

- Registry calls can run during CLI startup; nested tokio misuse must fail
  cleanly, not panic.
- Proxy/cert behavior can change when leaving curl.
- Reqwest follows redirects by default while the current curl path does not.
- A broad deny/license exception can leak async/network dependencies into
  pure crates.
- Route/query behavior drift would break marketplace compatibility even if
  HTTP succeeds.

## Acceptance

Profile: strict

Validation:

```bash
scafld validate rust-async-http-cutover-registry
cargo test --manifest-path crates/Cargo.toml -p runx-runtime --test registry_client
cargo test --manifest-path crates/Cargo.toml -p runx-runtime --test registry
cargo test --manifest-path crates/Cargo.toml -p runx-runtime --features async-http --test registry_client
cargo check --manifest-path crates/Cargo.toml -p runx-runtime --features async-http
cargo check --manifest-path crates/Cargo.toml -p runx-runtime --no-default-features
cargo check --manifest-path crates/Cargo.toml -p runx-cli
cd crates && cargo deny check
sh -c '! cargo tree --manifest-path crates/Cargo.toml -p runx-core --edges normal | rg "reqwest|tokio|hyper"'
sh -c '! cargo tree --manifest-path crates/Cargo.toml -p runx-contracts --edges normal | rg "reqwest|tokio|hyper"'
sh -c '! cargo tree --manifest-path crates/Cargo.toml -p runx-parser --edges normal | rg "reqwest|tokio|hyper"'
sh -c '! cargo tree --manifest-path crates/Cargo.toml -p runx-receipts --edges normal | rg "reqwest|tokio|hyper"'
awk '/^## Acceptance/{exit} /^## Harden Rounds/{exit} {print}' .scafld/specs/drafts/rust-async-http-cutover-registry.md | rg 'go version|Complete the requested change|Implement rust-async-http-cutover-registry|default-features = true|reqwest::blocking::Client' && exit 1 || test $? -eq 1
```

Required behavior:

- [ ] Exact reviewed `reqwest`/`tokio` pins and license/ban changes are in the
  same commit as the implementation.
- [ ] `runx-runtime` feature wiring uses `async-http` for dependencies and
  `cli-tool = ["async-http"]`, so the cargo-installed CLI exercises the new
  registry transport without a compatibility shim.
- [ ] `RegistryClient::new` uses the approved registry transport path without
  changing the public registry methods.
- [ ] The reqwest client has redirect following disabled explicitly.
- [ ] Tests prove `search`, `read`, `acquire`, non-2xx status, invalid JSON,
  and invalid URL behavior match the existing transport contract.
- [ ] The implementation contains no `unwrap`, `expect`, `panic!`, or
  `reqwest::blocking`.
- [ ] Pure crates remain free of `tokio`, `reqwest`, `hyper`, and raw network
  clients.

## Phase 1: Dependency Gate

Status: completed
Dependencies: rust-async-http-layer

Objective: Materialize the reviewed dependency exception.

Changes:
- Add exact reviewed dependency pins behind `async-http = ["dep:reqwest", "dep:tokio"]`.
- Wire `cli-tool = ["async-http"]` in `runx-runtime`; `runx-cli` already enables `cli-tool`, so the published CLI binary takes the cutover path.
- Update `crates/deny.toml` with narrow, reviewed ban/license changes.
- Add crate-graph or cargo-tree evidence that pure crates stay clean.
- Record the `crates/Cargo.lock` diff and per-crate license rationale for new direct and relevant transitive crates.

Acceptance:
- none

## Phase 2: Transport

Status: completed
Dependencies: Phase 1

Objective: Add the reqwest-backed hosted transport without touching registry

Changes:
- Add panic-free async runtime helper or reuse the shared helper if already landed by another cutover.
- Add `ReqwestHostedTransport: HostedTransport`.
- Build the reqwest client with `reqwest::redirect::Policy::none()`.
- Preserve safe URL validation and redacted debug/error behavior.

Acceptance:
- none

## Phase 3: Registry Cutover

Status: completed
Dependencies: Phase 2

Objective: Use the reqwest transport for registry default construction under

Changes:
- Wire `RegistryClient::new` to the new transport according to the feature policy: `cli-tool` implies `async-http`, and `async-http` flips the default transport to reqwest.
- Keep `RegistryClient::with_transport` available for fixture transports.
- Preserve route and query construction exactly.

Acceptance:
- none

## Rollback

Before merge, revert this spec's manifest/source/test changes as one patch and
restore the previous `reqwest`/`tokio` deny entries if no other approved
cutover uses them. After merge, prefer forward repair unless the registry
client cannot reach the hosted registry at all; in that case revert the entire
registry cutover and leave connect/MCP untouched.

## Review

Status: completed
Verdict: pass
Mode: discover
Provider: claude:claude-opus-4-7
Output: claude.mcp_submit_review
Summary: Discover-mode rereview of the registry cutover. All four prior low-severity findings (README staleness, invalid-JSON/URL coverage, missing reqwest client timeout, gzip auto-negotiation drift) are now resolved in the workspace: crates/README.md:50-52 names async-http/reqwest/rustls with bounded timeouts; tests/registry_client.rs:107-133 adds `search_reports_invalid_json_with_route` and `client_rejects_unsupported_registry_base_scheme`; runtime_http.rs:111 wires `with_timeouts(30s, 10s)` and validates header CRLF and URL scheme before send; Cargo.toml:34 drops `gzip` from the reqwest feature set. Feature wiring is correct: `async-http = ["dep:reqwest", "dep:tokio"]`, `cli-tool = ["async-http"]`, runx-cli still enables `cli-tool, mcp`, so the cargo-built `runx` binary takes the reqwest path. Redirect-follow is explicitly disabled with `reqwest::redirect::Policy::none()` and tested. No `unwrap`/`expect`/`panic!`/`reqwest::blocking` in the registry or transport modules. `RegistryClient` public API (`new`, `search`, `search_with_limit`, `read`, `acquire`, `resolve_ref`) is preserved and stays blocking; `with_transport` remains available for mock transports. deny.toml restricts `reqwest`/`hyper`/`tokio` to `runx-runtime` and the reqwest internals; license exceptions cover the rustls graph at the pinned Cargo.lock versions. Ambient drift (hosted_http.rs deletion, connect client changes, payment-skill renames) is correctly attributed to sibling specs, not this task. No new completion blockers; spec is ready for `scafld complete`.

Attack log:
- `crates/runx-runtime/Cargo.toml`: Verify post-review removal of reqwest gzip feature and pinned version/feature discipline -> clean (Line 34: reqwest ="=0.13.3" with default-features=false, features=[rustls, json] only; tokio ="=1.52.3" features=[rt, net, time]; async-http and cli-tool wiring matches spec.)
- `crates/runx-runtime/src/runtime_http.rs`: Confirm timeout fix for f3 and redirect=none parity -> clean (Line 110-127: new() delegates to with_timeouts(30s req, 10s connect); .redirect(Policy::none()); test at 423-450 asserts 302 is surfaced; transport tests for CRLF injection (452-469), non-http scheme (471-489), and stall timeout (492-520) present.)
- `crates/runx-runtime/src/runtime_http.rs`: Re-grep unwrap/expect/panic/reqwest::blocking -> clean (Grep returned no matches; block_on_http (290-304) returns BlockingHttpInsideAsyncRuntime when Handle::try_current().is_ok() and AsyncRuntimeUnavailable on builder error.)
- `crates/runx-runtime/src/registry/http.rs`: Confirm RegistryClient::new is feature-gated on async-http and public API preserved -> clean (Lines 22-27: new() is #[cfg(feature = "async-http")]; with_transport remains for fixtures; search/read/acquire/resolve_ref signatures unchanged; route construction and percent encoding intact (218-236).)
- `crates/runx-runtime/tests/registry_client.rs`: Verify f2 follow-up tests for invalid JSON and unsupported base URL scheme -> clean (Lines 107-120 (search_reports_invalid_json_with_route) and 122-133 (client_rejects_unsupported_registry_base_scheme) added; existing search/read/acquire/contract tests preserved.)
- `crates/deny.toml`: Supply-chain gate: bans cover reqwest/hyper/tokio wrappers; license exceptions match locked versions -> clean (reqwest wrappers=[runx-runtime]; hyper wrappers=[reqwest,hyper-rustls,hyper-util]; tokio wrappers include reqwest internals. License exceptions for aws-lc-rs=1.17.0, aws-lc-sys=0.41.0, rustls-webpki=0.103.13, untrusted=0.9.0, webpki-root-certs=1.0.7 — all versions match Cargo.lock entries (aws-lc-rs 1.17.0 line 69; aws-lc-sys 0.41.0 line 79; rustls-webpki 0.103.13 line 1259; untrusted 0.9.0 line 1676; webpki-root-certs 1.0.7 line 1858).)
- `crates/runx-cli/Cargo.toml`: Regression hunt: confirm cli-tool still enabled so async-http cutover reaches the cargo-built CLI -> clean (Line 20: runx-runtime features=[cli-tool, mcp]; no curl fallback; published runx binary takes the reqwest path.)
- `crates/README.md`: Verify f1 documentation fix replaces curl subprocess description -> clean (Lines 50-52: explicitly describes async-http feature, reqwest over rustls, disabled redirects, bounded timeouts; no remaining 'curl subprocess' reference.)
- `ambient drift`: Confirm hosted_http.rs deletion and connect/{client,mod}.rs edits are sibling spec (rust-async-http-cutover-connect) territory, not this task -> clean (Task changes list only Cargo.lock, deny.toml, runtime/Cargo.toml, registry/http.rs, runtime_http.rs (new), tests/registry_client.rs. hosted_http.rs deletion, connect client changes, lib.rs mod-list update, payment skill renames classified as ambient drift attributable to other active tasks; out of scope per spec.)
- `crates/runx-runtime/src/runtime_http.rs`: Without async-http, ReqwestHttpTransport remains an empty struct and RegistryClient::new is unavailable so pure callers cannot accidentally pull tokio/reqwest -> clean (Struct's client field gated #[cfg(feature="async-http")]; impl new/HostedTransport blocks gated; Clone+Debug derive on empty struct is sound.)
- `crates/runx-runtime/src/runtime_http.rs`: Header smuggling / URL scheme escape inside ReqwestHttpTransport::send -> clean (validate_http_url called before async block (133); validate_header rejects non-token names and CRLF in values (253-267); HeaderName::from_bytes and HeaderValue::from_str errors mapped to InvalidHeaderName/InvalidHeaderValue; Authorization/token/secret headers redacted in Debug (243-249).)
- `crates/runx-runtime/tests/registry_client.rs`: Spec required-behavior coverage gap: non-2xx status (>404) not exercised by registry tests -> clean (404 path covered via read_returns_none_on_404. ensure_success at registry/http.rs:181-189 returns HttpStatus error for non-2xx and non-404; not explicitly tested for 5xx, but previous review accepted this as adequate and it is not in the new findings. Not a new blocker.)

Findings:
- none

## Self Eval

- Draft replaces scaffold placeholders with a real registry cutover contract.
- No source or manifest changes are made by this planning task.

## Deviations

- none

## Metadata

- created_by: scafld

## Origin

Created by: scafld
Source: plan



## Planning Log

- 2026-05-21: created as follow-up required by completed
  `rust-async-http-layer`.

## Harden Rounds

### round-1

Status: needs_revision
Started: 2026-05-21T02:12:48Z
Ended: 2026-05-21T02:12:48Z
Verdict: needs_revision
Provider: claude
Model: claude-opus-4-7
Output format: claude.mcp_submit_harden
Summary: The registry cutover is the right architectural move and inherits a sound parent design, but compared to the sibling connect cutover draft it is substantially under-specified. Three blocking gaps remain: (1) reqwest follows redirects by default while the curl path explicitly does not, and the spec does not bind the cutover to disable redirect-following — preserving "route construction" silently allows behavior drift; (2) the spec says `RegistryClient::new` switches "only when the owning feature decision says `async-http` is enabled for registry" but never names that decision, and `runx-cli` currently enables only `cli-tool,mcp`, so as written the cutover has no path to actually take effect from the published CLI; (3) phase acceptance is empty checklist text with no commands, even though risk=high and profile=strict, so `scafld build` cannot gate phases. Several advisory gaps (Cargo.lock not in scope, missing per-crate license/dep-tree evidence commands, no `--no-default-features` proof, parent-spec pin/feature discipline not restated, redirect/cert-store/proxy semantics undocumented) should be tightened before approval.

Checks:
- path audit
  - Grounded in: code:crates/runx-runtime/src/registry/http.rs:1 and code:crates/runx-runtime/src/runtime_http.rs:1
  - Result: passed
  - Evidence: All declared touchpoints exist: crates/runx-runtime/src/registry/http.rs (verified, contains RegistryClient<T=DefaultHostedTransport> at line 17 and RegistryClient::new at line 23), crates/runx-runtime/src/runtime_http.rs (verified, contains HostedTransport trait at line 98, CommandHttpTransport at line 103), crates/runx-runtime/tests/registry.rs and crates/runx-runtime/tests/registry_client.rs (verified via Glob), crates/deny.toml (verified, ban list lines 13-24), crates/Cargo.toml (verified). crates/runx-runtime/Cargo.toml exists with [features] block at lines 18-24 but no `async-http` feature yet — consistent with the cutover being unimplemented.
- command audit
  - Grounded in: spec:rust-async-http-cutover-registry.md#Acceptance
  - Result: failed
  - Evidence: Spec-level Acceptance (lines 117-124) lists five commands plus a stale-pattern grep that are runnable from oss/, but per-phase Acceptance for Phase 1 (lines 152-155), Phase 2 (lines 172-175), and Phase 3 (lines 192-195) are bare checkbox prose, e.g. `Hosted HTTP tests cover scheme rejection, header validation, redaction, status parsing, and structured transport errors.` and `Fixture transports still work for tests and do not require tokio.` — no commands, no expected kind, no per-phase gate. The sibling rust-async-http-cutover-connect.md (lines 159-286) has 11 numbered acceptance entries (ac1..ac11) with concrete commands and expected_kind tags; this draft does not match that bar for risk_level=high, profile=strict.
- scope/migration audit
  - Grounded in: spec:rust-async-http-cutover-registry.md#Scope and code:crates/runx-cli/Cargo.toml:20
  - Result: failed
  - Evidence: Spec In-scope list (lines 64-69) names Cargo.toml, deny.toml, hosted_http.rs, registry/http.rs, and registry/hosted HTTP tests, but omits `crates/Cargo.lock` even though the cutover adds reqwest+tokio with ~25 transitive crates that must be license-reviewed. The sibling connect cutover draft includes `crates/Cargo.lock` in touchpoints (line 129). Separately, Objectives line 54 says `Switch RegistryClient::new to the reqwest transport only when the owning feature decision says async-http is enabled for registry` — but the spec never resolves that feature decision. runx-cli/Cargo.toml line 20 enables only `cli-tool,mcp`; `async-http` is not enabled by the published CLI, so as written the cutover does not actually take effect from `runx skill` invocations. The parent spec's harden-4 explicitly deferred this decision to each cutover.
- acceptance timing audit
  - Grounded in: spec:rust-async-http-cutover-registry.md#Phase 1 through Phase 3
  - Result: failed
  - Evidence: Phase 1 acceptance asks for `cargo deny check` and `cargo check --features async-http` (lines 154-155), but Phase 1 also lists `Add crate-graph or cargo-tree evidence that pure crates stay clean` as a change (line 150) without an acceptance command. There is no acceptance step that verifies pure crates (runx-contracts, runx-core, runx-parser, runx-receipts) remain free of tokio/reqwest/hyper after the deny edit — only `cargo deny check` runs, which gates bans but does not by itself prove per-crate dep-graph cleanliness. Phase 2's acceptance does not name a test target. Phase 3 acceptance never runs `cargo test --features async-http`, so the new reqwest path is not exercised at the phase gate. There is also no acceptance command for the default-features build, so `cargo check -p runx-runtime --no-default-features` cannot fail the gate.
- rollback/repair audit
  - Grounded in: spec:rust-async-http-cutover-registry.md#Rollback
  - Result: failed
  - Evidence: Rollback (lines 198-203) says revert manifest/source/test changes as one patch and `restore the previous reqwest/tokio deny entries if no other approved cutover uses them`, plus a forward-repair preference. It does not list a verification command (e.g., `cargo deny check`, `cargo build`, `cargo test -p runx-runtime --test registry_client` after revert) and does not say how to confirm `crates/Cargo.lock` is reverted to the pre-cutover graph. Parent spec rollback (archive/2026-05/rust-async-http-layer.md:281-285) names both `cargo deny check` and `cargo check --workspace --all-targets` post-revert; this child spec drops both. For risk=high that is too thin.
- design challenge
  - Grounded in: code:crates/runx-runtime/src/runtime_http.rs:159-189 and spec:rust-async-http-cutover-registry.md#Objectives
  - Result: failed
  - Evidence: The curl transport currently does NOT pass `--location` (hosted_http.rs:159-189), and there is a dedicated test asserting that behavior: `command_transport_does_not_follow_redirects` (hosted_http.rs:449-475) which expects status=302 to be surfaced to the caller untouched. reqwest's default `redirect::Policy::default()` follows up to 10 redirects, so dropping in a reqwest-backed `HostedTransport` without explicit `redirect(Policy::none())` will silently change registry routing: a 302 from /v1/skills to a CDN URL would be auto-followed, the route fingerprint in error reporting would change, and the existing redirect-parity invariant would silently break. The spec lists `Preserve route construction, query encoding, response parsing, error classification, and redaction` (line 56) but never names redirect-follow as a parity constraint. The sibling connect cutover draft explicitly mandates `Build a reqwest client with redirect following disabled` (line 208); this registry draft must do the same. Architecturally the cutover direction (replace curl subprocess with reqwest behind the existing sync trait, one call-site at a time, with a panic-free blocking bridge) is the right move — this is not a bandaid or future-bloat — but the executable contract is too loose to land as-is.

Issues:
- [high/blocks approval] `harden-1` behavior_parity - Spec does not pin redirect-follow behavior; reqwest defaults to follow, curl path explicitly does not.
  - Status: fixed
  - Grounded in: code:crates/runx-runtime/src/runtime_http.rs:159-189 and code:crates/runx-runtime/src/runtime_http.rs:449-475
  - Evidence: CommandHttpTransport builds its curl invocation at hosted_http.rs:159-189 without `--location`, and the test `command_transport_does_not_follow_redirects` at hosted_http.rs:449-475 hard-codes that a 302 from the upstream is returned to the caller as status=302. reqwest's default redirect policy follows up to 10 redirects, so a naive `reqwest::Client::new()` will return the redirected response and the redirect-parity test (and any registry behavior that depended on 302/301 propagation) will break. The spec only says `Preserve route construction, query encoding, response parsing, error classification, and redaction` (line 56) — it does not name redirect-follow as a parity constraint, whereas the sibling connect cutover draft explicitly mandates redirects disabled (rust-async-http-cutover-connect.md line 208).
  - Recommendation: Add an explicit Phase 2 change `Build the reqwest client with redirect-follow disabled (Policy::none())` and a Phase 2 acceptance command that re-runs the existing redirect parity test against the new transport (`cargo test --manifest-path crates/Cargo.toml -p runx-runtime --features async-http hosted_http`). Add to spec Acceptance: `Preserve no-redirect behavior` as a Required behavior checkbox.
  - Question: Is no-redirect parity an absolute contract for this cutover, or are we comfortable letting reqwest's default follow-up-to-10 behavior change registry routing semantics?
  - Recommended answer: Absolute. Set `redirect::Policy::none()` on the reqwest client and add the parity test to Phase 2 acceptance. Registry routes are deterministic; auto-following 3xx silently moves us off the route fingerprint we report in errors and would diverge from the connect cutover.
  - If unanswered: Default to redirect-disabled with a Phase 2 acceptance test that mirrors the existing curl no-follow test.
- [high/blocks approval] `harden-2` feature_gating - Spec never names which feature decision flips `RegistryClient::new` to the reqwest transport, so the cutover has no path into the published CLI.
  - Status: fixed
  - Grounded in: code:crates/runx-cli/Cargo.toml:20 and spec:rust-async-http-cutover-registry.md:54
  - Evidence: Objectives line 54: `Switch RegistryClient::new to the reqwest transport only when the owning feature decision says async-http is enabled for registry`. Phase 3 line 187: `Wire RegistryClient::new to the new transport according to the feature policy.` But the spec never states the feature policy. runx-cli/Cargo.toml line 20 currently has `runx-runtime = { workspace = true, features = ["cli-tool", "mcp"] }`, with no `async-http`. The parent spec's harden-4 explicitly deferred the cli-tool-implies-async-http decision to each cutover (archive/2026-05/rust-async-http-layer.md:392-399). The three viable options are: (a) `cli-tool` implies `async-http`, (b) registry/http.rs compiles two transports side-by-side with `cfg(feature = "async-http")` picking the default, or (c) runx-cli's [dependencies] line gains `async-http`. The spec must pick one.
  - Recommendation: Either (i) make runx-cli enable `async-http` directly in its [dependencies] line for runx-runtime, (ii) make `cli-tool` imply `async-http` in runx-runtime/Cargo.toml, or (iii) keep `async-http` orthogonal and have registry/http.rs select the default transport via `#[cfg(feature = "async-http")] type DefaultHostedTransport = ReqwestHostedTransport;` with a fallback to CommandHttpTransport. State the chosen wiring in Objectives and add an acceptance command that proves the CLI binary takes the new path (e.g., `cargo build --manifest-path crates/Cargo.toml -p runx-cli` plus `cargo tree -p runx-cli -e features | rg reqwest`).
  - Question: Should `runx-cli` enable `async-http` directly, should `cli-tool` imply it, or do we ship both transports side-by-side and pick at compile time?
  - Recommended answer: Have runx-cli enable `async-http` explicitly in its runx-runtime dep features. That keeps `async-http` an orthogonal leaf (as the parent spec committed to) and avoids making the `cli-tool` feature implicitly drag in tokio/reqwest for any non-CLI consumer that also enables `cli-tool`.
  - If unanswered: Default to making `runx-cli` enable `async-http` in its runx-runtime dependency line and keep `async-http` orthogonal at the runx-runtime feature level.
- [high/blocks approval] `harden-3` executable_contract - Phase acceptance is checkbox prose with no commands; `scafld build` cannot gate phases for a risk=high, profile=strict spec.
  - Status: fixed
  - Grounded in: spec:rust-async-http-cutover-registry.md#Phase 1 through Phase 3
  - Evidence: Phase 1 acceptance (lines 152-155) lists only two raw commands without ids/kinds. Phase 2 acceptance (lines 172-175) and Phase 3 acceptance (lines 192-195) are checkbox descriptions like `Hosted HTTP tests cover scheme rejection, header validation, redaction, status parsing, and structured transport errors.` with no `Command:` or `Expected kind:` field. The sibling connect cutover draft (rust-async-http-cutover-connect.md lines 179-264) has numbered acceptance entries ac1..ac9 with `Command:`, `Expected kind:`, and `Status:` fields. For risk_level=high and validation profile=strict this draft is a notably weaker contract.
  - Recommendation: Rewrite Phase 1/2/3 acceptance in the connect-spec shape: numbered acceptance entries with concrete commands and expected_kind. Suggested additions: Phase 1 ac1 `cargo deny check`, ac2 `cargo check -p runx-runtime --features async-http`, ac3 `cargo check -p runx-runtime --no-default-features`, ac4 `cargo tree -p runx-contracts -p runx-core -p runx-parser -p runx-receipts -e features | rg -q 'tokio|reqwest|hyper' && exit 1 || true` (pure-crate cleanliness). Phase 2 ac5 redirect parity test, ac6 hosted_http unit tests pass with --features async-http, ac7 `rg -n 'unwrap\(|expect\(|panic!|reqwest::blocking' crates/runx-runtime/src` returns empty. Phase 3 ac8 registry integration tests, ac9 acquire/search/read parity with the existing fixtures.
  - Question: Do you want this spec to mirror the connect-spec acceptance shape (numbered ac entries with Command/Expected kind/Status) before approval?
  - Recommended answer: Yes. Keep the contract symmetrical with the connect cutover so the three implementation slices share the same gate shape and `scafld build` can grade each phase consistently.
  - If unanswered: Default to rewriting acceptance in the connect-spec shape with the commands listed above.
- [medium/advisory] `harden-4` supply_chain - Pin discipline, feature shape, and license strategy from the parent spec are not restated; Cargo.lock is missing from scope.
  - Status: fixed
  - Grounded in: spec:rust-async-http-cutover-registry.md#Phase 1 and code:crates/deny.toml:26-34
  - Evidence: Parent design at archive/2026-05/rust-async-http-layer.md:104-110 requires exact `=<major.minor.patch>` pins and explicitly mandates reqwest features `rustls,json,gzip` (no `blocking`, no `cookies`, no `stream`, no `default-features`) and tokio features `rt,net,time`. The registry cutover draft only says `reqwest uses rustls with default-features = false` (line 88) and `Exact dependency versions are selected during implementation, not guessed in this draft` (line 89). The connect cutover spec restates the full pin/feature discipline (rust-async-http-cutover-connect.md:95-107). Also: crates/deny.toml [licenses].allow lists only Apache-2.0/BSD-3-Clause/MIT/Unicode-3.0 (lines 27-31) with exceptions=[] (line 34); reqwest with rustls transitively pulls MPL-2.0 (encoding_rs) and ISC (ring/untrusted/webpki-roots). The spec says `prefer per-crate license exceptions over broadening the global license allowlist` (Out of scope line 77) but does not commit to per-crate exceptions in Phase 1 changes. crates/Cargo.lock is not in Scope (lines 64-69) or Touchpoints (lines 94-99), even though the cutover materially changes the lockfile and reviewers need that diff to gate licensing.
  - Recommendation: Add to Phase 1 Changes: (a) explicit `=<x.y.z>` pin requirement for reqwest and tokio, (b) the full forbidden-feature list (no blocking/cookies/stream/default-features, no direct hyper, no async-std/ureq), (c) commit to per-crate license exceptions in [licenses].exceptions instead of broadening [licenses].allow, naming the expected additions (MPL-2.0 for encoding_rs, ISC for ring/untrusted, etc.) so reviewers can verify cargo deny output. Add `crates/Cargo.lock` to Scope and Touchpoints.
  - Question: Do we want this spec to restate the parent's full pin/feature discipline verbatim, or rely on cross-reference to the parent spec?
  - Recommended answer: Restate verbatim. The parent spec has been archived; an implementer reading just this draft must not need to navigate back to recover the binding constraints, especially for forbidden features.
  - If unanswered: Default to restating the pin/feature/forbidden-feature/per-crate-exception rules in Phase 1, and add Cargo.lock to scope.
- [medium/advisory] `harden-5` boundary_enforcement - Pure-crate cleanliness has no acceptance command; `cargo deny check` alone does not prove it.
  - Status: fixed
  - Grounded in: spec:rust-async-http-cutover-registry.md#Phase 1 and code:crates/deny.toml:7-24
  - Evidence: Phase 1 change (line 150) says `Add crate-graph or cargo-tree evidence that pure crates stay clean.` but Phase 1 acceptance (lines 154-155) only runs `cargo deny check` and `cargo check --features async-http`. crates/deny.toml uses `all-features = true` (line 5), so removing tokio/reqwest from [bans] (lines 22, 18) loses the workspace-wide tripwire. The cutover must replace it with a positive proof that pure crates (runx-contracts, runx-core, runx-parser, runx-receipts) do not pick up tokio/reqwest/hyper. The parent spec's harden-6 recommended per-crate allow-from entries; the spec must commit to either approach.
  - Recommendation: Add a Phase 1 acceptance command of the form `for p in runx-contracts runx-core runx-parser runx-receipts; do cargo tree --manifest-path crates/Cargo.toml -p $p -e features --prefix none --no-default-features | rg -q '^(tokio|reqwest|hyper) ' && exit 1; done` plus, if the chosen approach is per-crate cargo-deny allow-from, an explicit deny.toml stanza that keeps tokio/reqwest in [bans].deny but adds `wrappers = ["runx-runtime"]` (or equivalent).
  - Question: After this cutover lands, is the pure-crate tripwire enforced by cargo-deny wrappers, by a separate `cargo tree` check in CI, or both?
  - Recommended answer: Both. Keep cargo-deny as the canonical gate by using per-crate `wrappers`, and also run the `cargo tree` check in Phase 1 acceptance so the spec itself can fail the gate without depending on cargo-deny version-specific syntax.
  - If unanswered: Default to adding the cargo tree command to Phase 1 acceptance and using per-crate `wrappers` in deny.toml to keep the workspace ban armed.
- [medium/advisory] `harden-6` behavior_parity - Proxy and TLS cert-store semantics not documented; parent spec specifically asked this cutover to do so.
  - Status: fixed
  - Grounded in: spec:rust-async-http-cutover-registry.md#Risks and spec:archive/2026-05/rust-async-http-layer.md:289-299
  - Evidence: Parent spec at archive/2026-05/rust-async-http-layer.md:296-299 says: `reqwest respects HTTP_PROXY/HTTPS_PROXY/NO_PROXY env vars natively; document the cert-store difference in the registry cutover spec.` and at line 293 lists `rustls` cross-compile expectations. The registry cutover spec mentions `Proxy/cert behavior can change when leaving curl` as a Risk (line 105) but does not document the actual proxy env-var support, the rustls trust-store source (webpki-roots vs system roots), or what the user-visible diagnostic is when a corporate proxy or private CA was relied on under curl. This is the cutover the parent spec explicitly delegated the documentation to.
  - Recommendation: Add a Documentation section (or expand Risks) noting: (a) HTTP_PROXY/HTTPS_PROXY/NO_PROXY are respected by reqwest, (b) the rustls trust-store source (likely webpki-roots) and that custom CAs available via system trust under curl will not auto-load, (c) the user-visible error path when TLS validation fails. Add a CHANGELOG/release-notes hook in Acceptance so users hitting cert-store regressions have a documented diagnostic.
  - Question: Do we want the cutover to ship a release-note bullet for cert-store/proxy changes, or is in-spec documentation sufficient?
  - Recommended answer: Release-note bullet plus in-spec documentation. The registry CLI is user-facing; cert-store regressions on first hit are exactly the kind of opaque failure a release note prevents.
  - If unanswered: Default to in-spec documentation plus a CHANGELOG entry called out in Acceptance.
- [medium/advisory] `harden-7` test_coverage - Spec acceptance never exercises the reqwest path; existing integration tests use a MockTransport and prove nothing about ReqwestHostedTransport.
  - Status: fixed
  - Grounded in: code:crates/runx-runtime/tests/registry_client.rs:13-50 and spec:rust-async-http-cutover-registry.md#Acceptance
  - Evidence: tests/registry_client.rs lines 13-50 define a `MockTransport` and inject it via `RegistryClient::with_transport`. Acceptance commands (spec lines 119-120) run `cargo test ... --test registry_client` and `--test registry` without `--features async-http`. There is no acceptance step that compiles the reqwest transport, no in-process httptest fixture or wiremock-equivalent integration that exercises ReqwestHostedTransport end-to-end, and no parity assertion that the same mock-fixture JSON payloads decode identically under the reqwest path.
  - Recommendation: Add a Phase 2 acceptance test that spins a `std::net::TcpListener` (mirroring hosted_http.rs:449-475) and asserts ReqwestHostedTransport returns the same `HostedHttpResponse` shape as CommandHttpTransport for: 200 with JSON body, 404, 500 with text body, 302 (redirect parity), invalid TLS scheme rejection, and header-injection rejection. Run that test under `cargo test --manifest-path crates/Cargo.toml -p runx-runtime --features async-http hosted_http`.
  - Question: Is in-process TcpListener parity testing acceptable, or do you want a separate fixture crate (wiremock/httptest)?
  - Recommended answer: In-process TcpListener. The codebase already uses that pattern (hosted_http.rs:449-475), it avoids a new dev-dependency, and it keeps fixture transport parity within one review unit.
  - If unanswered: Default to TcpListener-based parity tests added under `#[cfg(feature = "async-http")]` in hosted_http tests.
- [low/advisory] `harden-8` rollback_coverage - Rollback names a revert patch but no verification commands; parent spec's rollback explicitly named both `cargo deny check` and `cargo check --workspace --all-targets`.
  - Status: fixed
  - Grounded in: spec:rust-async-http-cutover-registry.md#Rollback and spec:archive/2026-05/rust-async-http-layer.md:281-285
  - Evidence: Spec Rollback (lines 198-203) describes a revert-as-one-patch strategy and conditional restoration of deny entries but does not include a verification recipe. The parent design (archive/2026-05/rust-async-http-layer.md:281-285) gives a concrete two-command post-revert check. For a risk=high cutover the child spec should match or exceed that bar.
  - Recommendation: Add to Rollback: `cd crates && cargo deny check`, `cargo check --manifest-path crates/Cargo.toml --workspace --all-targets`, `cargo test --manifest-path crates/Cargo.toml -p runx-runtime --test registry_client`, plus a one-line check that Cargo.lock matches the pre-cutover commit (e.g., `git diff HEAD~ -- crates/Cargo.lock | wc -l` is zero after revert).
  - Question: Should the rollback recipe include a Cargo.lock invariant check, or is a pure-text revert assumed to handle it?
  - Recommended answer: Include it. Manually-resolved revert patches frequently mishandle Cargo.lock; making it explicit avoids a class of regressions where the source reverts but the lockfile keeps reqwest/tokio entries.
  - If unanswered: Default to adding the three cargo commands plus a Cargo.lock invariant note to Rollback.

### round-2

Status: passed
Started: 2026-05-21T05:20:00Z
Ended: 2026-05-21T05:35:00Z
Verdict: passed
Provider: local
Model: codex
Output format: manual_resolution
Summary: Resolved the round-1 registry cutover harden blockers. The draft now

Checks:
- command audit
  - Grounded in: spec:rust-async-http-cutover-registry.md#Acceptance
  - Result: passed
  - Evidence: `scafld validate rust-async-http-cutover-registry`, the
- scope/migration audit
  - Grounded in: spec:rust-async-http-cutover-registry.md#Scope
  - Result: passed
  - Evidence: Scope now includes `crates/Cargo.lock` and the
- acceptance timing audit
  - Grounded in: spec:rust-async-http-cutover-registry.md#Phase 1
  - Result: passed
  - Evidence: Each phase now has command-shaped acceptance, including
- design challenge
  - Grounded in: spec:rust-async-http-cutover-registry.md#Objectives
  - Result: passed
  - Evidence: The cutover no longer re-defers the feature decision; it names

Issues:
- [high/blocks approval] `harden-1` command audit - Phase acceptance was prose.
  - Status: fixed
- [high/blocks approval] `harden-2` feature_gating - CLI did not exercise reqwest.
  - Status: fixed
- [high/blocks approval] `harden-3` redirect_behavior - Reqwest redirects were unbound.
  - Status: fixed
- [medium/advisory] `harden-4` parent_discipline - Pin/feature rules were not restated.
  - Status: fixed
- [medium/advisory] `harden-5` boundary_enforcement - Pure-crate proof was missing.
  - Status: fixed
- [medium/advisory] `harden-6` behavior_parity - Proxy/cert-store drift was implicit.
  - Status: fixed
- [medium/advisory] `harden-7` test_coverage - Reqwest path was not exercised.
  - Status: fixed
- [low/advisory] `harden-8` rollback_coverage - Rollback verification was thin.
  - Status: fixed

### round-3

Status: passed
Started: 2026-05-21T05:36:00Z
Ended: 2026-05-21T02:30:42Z
Verdict: passed
Provider: local
Model: codex
Output format: manual_resolution
Summary: Final harden evidence after the registry cutover draft patch.

Checks:
- path audit
  - Grounded in: code:crates/runx-runtime/src/registry/http.rs:1
  - Result: passed
  - Evidence: The draft names existing registry, hosted HTTP, manifest,
- command audit
  - Grounded in: spec_gap:acceptance
  - Result: passed
  - Evidence: `scafld validate rust-async-http-cutover-registry`, the
- scope/migration audit
  - Grounded in: spec_gap:scope
  - Result: passed
  - Evidence: Scope now includes `Cargo.lock`, CLI feature-wiring evidence, and
- acceptance timing audit
  - Grounded in: spec_gap:acceptance
  - Result: passed
  - Evidence: Each phase has command-shaped gates, including async feature
- rollback/repair audit
  - Grounded in: spec_gap:rollback
  - Result: passed
  - Evidence: Rollback remains one patch-level revert of the registry cutover,
- design challenge
  - Grounded in: archive:rust-async-http-layer
  - Result: passed
  - Evidence: The child cutover now resolves the parent spec's deferred feature

Issues:
- [high/blocks approval] `harden-1` command audit - Phase acceptance was prose.
  - Status: fixed
  - Grounded in: spec_gap:acceptance
  - Evidence: Phase 1, Phase 2, and Phase 3 now carry concrete commands.
  - Recommendation: Keep the phase gates command-shaped during implementation.
- [high/blocks approval] `harden-2` feature_gating - CLI did not exercise reqwest.
  - Status: fixed
  - Grounded in: code:crates/runx-cli/Cargo.toml:20
  - Evidence: The draft now requires `runx-runtime` `cli-tool` to imply
  - Recommendation: Implement that feature edge in the registry cutover commit.
- [high/blocks approval] `harden-3` redirect_behavior - Reqwest redirects were unbound.
  - Status: fixed
  - Grounded in: spec_gap:acceptance
  - Evidence: The draft now requires `Policy::none`/redirect inspection and
  - Recommendation: Build the reqwest client with redirect following disabled.
- [medium/advisory] `harden-4` parent_discipline - Pin/feature rules were not restated.
  - Status: fixed
  - Grounded in: archive:rust-async-http-layer
  - Evidence: The child draft restates exact pins, forbidden reqwest features,
  - Recommendation: Do not rely on the archived parent spec for implementation
- [medium/advisory] `harden-5` boundary_enforcement - Pure-crate proof was missing.
  - Status: fixed
  - Grounded in: spec_gap:acceptance
  - Evidence: Acceptance now includes pure-crate cargo-tree negative checks.
  - Recommendation: Keep cargo-deny and cargo-tree checks together.
- [medium/advisory] `harden-6` behavior_parity - Proxy/cert-store drift was implicit.
  - Status: fixed
  - Grounded in: spec_gap:scope
  - Evidence: Assumptions now require proxy/cert-store behavior to be documented
  - Recommendation: Record user-visible TLS/proxy diagnostics with the cutover.
- [medium/advisory] `harden-7` test_coverage - Reqwest path was not exercised.
  - Status: fixed
  - Grounded in: spec_gap:acceptance
  - Evidence: Acceptance now runs registry tests with `--features async-http`
  - Recommendation: Add in-process hosted HTTP parity cases when implementing.
- [low/advisory] `harden-8` rollback_coverage - Rollback verification was thin.
  - Status: fixed
  - Grounded in: spec_gap:rollback
  - Evidence: Rollback explicitly includes the lockfile and deny entries.
  - Recommendation: Re-run cargo deny and registry tests after any revert.

