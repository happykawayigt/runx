---
spec_version: '2.0'
task_id: rust-async-http-layer
created: '2026-05-21T03:00:00Z'
updated: '2026-05-21T03:00:00Z'
status: draft
current_phase: planning
harden_status: not_run
size: medium
risk_level: high
---

# Rust async HTTP layer

## Current State

Status: draft
Current phase: planning
Next: design review
Reason: draft architecture spec for a scoped adapter-tier async HTTP exception.
Blockers: operator approval of the supply-chain exception; no code changes
until a follow-up cutover spec is approved.
Allowed follow-up command: `scafld harden rust-async-http-layer --provider <provider>`
Latest runner update: none
Review gate: not_started

## Why this exists

`runx-runtime`'s registry and connect surfaces talk to remote HTTP endpoints
through `hosted_http.rs`, which is a thin wrapper around a `curl` subprocess.
This is deliberate per the architecture doc: the parity workspace has banned
`tokio`, `reqwest`, `hyper`, `ureq`, and `async-std` in `crates/deny.toml`
because no spec yet justifies introducing an async runtime.

The blocking-curl approach is fine for low-frequency calls (registry search,
single grant fetch). It will not scale to:

- The launcher flip workload, where every `runx skill` invocation may resolve
  a registry skill, fetch a profile, fetch attestations, and post a receipt.
- Adapter-tier consumers that need to dispatch multiple parallel HTTP calls
  (e.g., MCP servers, A2A peers, hosted agents).
- Connect-flow polling loops that would otherwise spawn N curl processes per
  poll cycle.

This spec proposes the *single, scoped* introduction of an async runtime and
HTTP client to the runtime crate.

## Scope

This spec covers the design and the `deny.toml` exception. It does **not**
ship the migration; that lands behind feature gates and per-call-site
cutovers in a follow-up `rust-async-http-cutover-{registry,connect,...}`
spec series.

## Choices

### Async runtime: `tokio` (single-threaded current-thread by default)

Why `tokio`:

- Ecosystem default; reqwest pulls it transitively.
- `current_thread` flavor adds ~50 KB and no OS threads; the runtime crate
  remains predominantly synchronous code that hops into the runtime only at
  HTTP call sites.
- `tokio::runtime::Runtime::block_on` provides a clean blocking-to-async
  bridge so the existing blocking public surface in `runx-runtime` does not
  change.

Rejected:

- `async-std`: smaller ecosystem, ABI churn, reqwest incompatible.
- Pure futures executors (e.g., `futures::executor::block_on`): no I/O
  driver; can't drive reqwest.
- Roll-our-own `hyper` directly: TLS, retries, redirects, decompression are
  all reinventions the workspace will regret in 6 months.

### HTTP client: `reqwest` with `default-features = false`, opt-in features

Why `reqwest`:

- Drop-in replacement for the curl-subprocess surface. Synchronous-looking
  API exists via `reqwest::blocking::Client` if needed for migration
  staging.
- TLS via `rustls-tls` (no native dependency on macOS keychain or Windows
  CryptoAPI).
- Native gzip/brotli decompression, redirects, connection pooling.

Cargo dependency shape:

```toml
[dependencies]
reqwest = { version = "0.13", default-features = false, features = [
    "rustls-tls",
    "json",
    "gzip",
] }
tokio = { version = "1", default-features = false, features = [
    "rt",
    "net",
    "time",
    "macros",
] }
```

No `default-features = true`. No `blocking` feature. No `cookies`. No
`stream` until a specific consumer needs it.

Rejected:

- `ureq`: blocking only; no async. Adopting it would require yet another
  migration when an async consumer appears.
- `surf` / `isahc`: smaller ecosystems; pull libcurl back in transitively.

### Feature gating

```toml
[features]
default = []
async-http = ["dep:reqwest", "dep:tokio"]
```

Adapter-tier consumers (`cli-tool`, `mcp`, `a2a`, `agent`, `catalog`) do not
enable `async-http` by default. Each consumer that needs it must enable it
explicitly:

```toml
[features]
catalog = ["cli-tool", "async-http"]
```

This preserves the pure default build for kernel-parity testing.

## `deny.toml` exceptions

Remove these from `[bans]` once this spec lands:

- `reqwest` — replaced by allowlist (this spec)
- `tokio` — replaced by allowlist (this spec)

Keep banned:

- `hyper` — only allowed transitively via reqwest, not as a direct dep
- `async-std`, `ureq` — explicitly not allowed
- `axum` — server framework, separate spec needed if ever required

## Runtime lifecycle

The runtime crate exposes a private `async_runtime()` helper that lazily
constructs a `tokio::runtime::Runtime` on first use and returns
`Arc<Runtime>` for shared access:

```rust
fn async_runtime() -> Arc<tokio::runtime::Runtime> {
    static RUNTIME: OnceLock<Arc<tokio::runtime::Runtime>> = OnceLock::new();
    RUNTIME.get_or_init(|| {
        Arc::new(
            tokio::runtime::Builder::new_current_thread()
                .enable_io()
                .enable_time()
                .build()
                .expect("tokio runtime"),
        )
    }).clone()
}
```

Blocking call sites use `runtime.block_on(async move { ... })`. The public
API stays blocking. A future `runx-sdk` async surface (separate spec) can
expose the runtime through a feature-gated path.

## Migration plan

Each cutover lands as a separate, narrowly scoped spec:

1. `rust-async-http-cutover-registry` — replace `hosted_http`'s curl
   subprocess with reqwest for the registry crate's GET/PUT calls; keep
   the public registry API unchanged.
2. `rust-async-http-cutover-connect` — same for the connect client.
3. `rust-async-http-cutover-hosted-http-removal` — once all consumers are
   migrated, delete `hosted_http.rs` entirely.

Each cutover spec must include:

- Side-by-side fixture parity: existing curl-fixture tests pass with the
  reqwest implementation.
- A short performance comparison (latency, allocation count, parallelism
  ceiling). The point is to prove the migration is justified, not to chase
  microbenchmarks.

## Risks

- **Supply chain**: reqwest pulls ~25 transitive deps. The workspace `deny.toml`
  must add a license allowlist review. Run `cargo deny check licenses` after the
  dep is added.
- **Cross-compile**: `rustls-tls` requires `ring` (or `aws-lc-rs`); both build
  cleanly on macOS, Linux, Windows. Confirm in CI before merging.
- **Binary size**: adds ~3 MB to the `runx` binary. Acceptable for the launcher
  flip given the perf headroom this unlocks.
- **No more curl**: some users may be relying on curl behavior (cert store,
  proxy env vars). reqwest respects `HTTP_PROXY`/`HTTPS_PROXY`/`NO_PROXY` env
  vars natively; document the cert-store difference in the registry cutover
  spec.

## Open questions deferred

- Should `runx-sdk` v1 expose an async surface that shares the same tokio
  runtime? Defer to a `runx-sdk-async-path` spec after this lands.
- Connection pooling tuning (max idle connections, keep-alive timeout) —
  defer until real workload data exists.
- Per-call-site retry policy. Defer to `rust-http-retry-policy` follow-up.

## References

- [`crates/deny.toml`](../../crates/deny.toml) — current bans
- [`crates/runx-runtime/src/hosted_http.rs`](../../crates/runx-runtime/src/hosted_http.rs)
- [`crates/runx-runtime/src/registry/http.rs`](../../crates/runx-runtime/src/registry/http.rs)
- [`crates/runx-runtime/src/connect/client.rs`](../../crates/runx-runtime/src/connect/client.rs)
- [`oss/docs/rust-kernel-architecture.md`](../../docs/rust-kernel-architecture.md) §10 (boundary enforcement)
- [`plans/rust-takeover.md`](../../../plans/rust-takeover.md) §3 (commitment shift)
