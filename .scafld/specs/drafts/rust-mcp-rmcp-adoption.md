---
spec_version: '2.0'
task_id: rust-mcp-rmcp-adoption
created: '2026-05-21T03:00:00Z'
updated: '2026-05-21T03:00:00Z'
status: draft
harden_status: not_run
size: large
risk_level: high
---

# rmcp adoption for the MCP adapter

## Current State

Status: draft
Current phase: planning
Next: design review
Reason: draft migration plan for replacing the hand-rolled MCP protocol layer
with `rmcp` after the async HTTP/runtime exception is approved.
Blockers: `rust-async-http-layer` must be approved first; no code changes until
a follow-up cutover spec is approved.
Allowed follow-up command: `scafld harden rust-mcp-rmcp-adoption --provider <provider>`
Latest runner update: none
Review gate: not_started

## Why this exists

The MCP adapter at [`crates/runx-runtime/src/adapters/mcp/`](../../crates/runx-runtime/src/adapters/mcp/)
implements a narrow MCP client and server over stdio, with hand-rolled JSON-RPC
framing, request/response correlation, server-state, and tool-call dispatch.
This is intentional per `crates/deny.toml`:

```
{ name = "rmcp", reason = "MCP currently uses the narrow local protocol layer;
                            rmcp needs a scoped adapter spec first." }
```

The hand-rolled layer was the right call when the protocol surface was small.
Now that:

- `adapters/mcp/` has ~2000 LoC across 9 files implementing transport,
  framing, JSON-RPC, server, server-skill execution, sandbox metadata,
  templating, types, and the adapter trait;
- the upstream `rmcp` crate has stabilized to v0.x with rustls + tokio
  transports;
- the rust-takeover plan §10 lists the MCP adapter as "the hardest port"
  (architecture doc §13);

it's time to spec the migration to the official `rmcp` crate.

## Scope

Design only. Migration lands in a follow-up `rust-mcp-rmcp-cutover` spec.

This spec answers:

1. Which parts of the hand-rolled MCP layer rmcp replaces.
2. Which parts stay (the runx-specific shapes that rmcp doesn't know
   about).
3. How the cutover stages so the existing `serve_mcp_json_rpc` byte-shape
   contract is preserved.
4. The async-runtime exception this spec consumes (depends on
   `rust-async-http-layer`).

## Replacement map

| Hand-rolled module | rmcp equivalent |
| --- | --- |
| `mcp/framing.rs` (Content-Length parsing) | `rmcp::transport::stdio` |
| `mcp/jsonrpc.rs` (request/response builders) | `rmcp::model::{Request, Response, JsonRpcMessage}` |
| `mcp/transport.rs::ProcessMcpTransport` (spawn child, stdio framing, response correlation, timeouts) | `rmcp::Client::serve(stdio)` |
| `mcp/transport.rs::FixtureMcpTransport` (in-process fixture) | stays — fixture support is runx-specific |
| `mcp/server.rs::serve_mcp_json_rpc` (stdio server loop) | `rmcp::Server::serve(stdio)` |
| `mcp/server.rs::McpServerState` (tool registry) | rmcp's `ServerHandler` trait |
| `mcp/server.rs::initialize_server_result` / `tools_list_result` / `mcp_tool_result_json` | rmcp model types |
| `mcp/server_skill.rs::execute_mcp_server_skill` | stays — runx skill execution under MCP |
| `mcp/templates.rs::map_mcp_arguments` and `stringify_*` | stays — runx template engine |
| `mcp/sandbox_metadata.rs` | stays — runx sandbox metadata is runx-specific |
| `mcp/adapter.rs::McpAdapter` (SkillAdapter impl) | stays — runx adapter trait remains |
| `mcp/types.rs::McpToolResult`, `McpHostRunResult` | partially: rmcp has `Content`, `CallToolResult`; runx `McpHostRunResult` stays |

Net result: ~1100 LoC removed (transport, jsonrpc, framing, server protocol),
~1000 LoC stays (skill execution, templates, sandbox metadata, runx-side
adapter glue).

## Cutover stages

Each stage compiles and tests independently. No "big bang" rewrite.

### Stage 1: pull rmcp behind a feature flag, no behavior change

Add to `runx-runtime/Cargo.toml`:

```toml
[features]
mcp = []
mcp-rmcp = ["mcp", "dep:rmcp", "async-http"]

[dependencies]
rmcp = { version = "0.x", default-features = false, features = [
    "transport-io",
    "macros",
], optional = true }
```

`mcp` (hand-rolled) and `mcp-rmcp` (rmcp-backed) are mutually exclusive in
the build but coexist in source.

### Stage 2: replace client transport

Behind `#[cfg(feature = "mcp-rmcp")]`, swap `ProcessMcpTransport::call_tool`
to use `rmcp::Client`. Keep `FixtureMcpTransport` unchanged.

Validation: every existing `mcp_adapter` integration test passes against
the rmcp client.

### Stage 3: replace server transport

Behind `mcp-rmcp`, swap `serve_mcp_json_rpc`'s stdio loop for rmcp's server.
Keep the runx-specific `McpServerState` and tool-call dispatch — wrap them
in an rmcp `ServerHandler` impl.

Validation: every existing `mcp_server` integration test passes.

### Stage 4: byte-exact wire compatibility check

Fixture test that records the bytes emitted by the hand-rolled server on
the existing fixture corpus, then runs the same fixtures through the rmcp
server and diffs. Acceptable diffs are pre-declared (e.g., rmcp may set a
`jsonrpc` field that the hand-rolled didn't, or vice versa); any
unexpected diff is a regression.

### Stage 5: delete the hand-rolled layer

Once rmcp passes Stage 4 and external production users (nitrosend, aster)
have soaked on `mcp-rmcp`, delete `mcp/{framing,jsonrpc,transport,server}`
and remove the `mcp` feature. The `mcp-rmcp` feature becomes `mcp`.

## Deny.toml exception

After `rust-async-http-layer` lands (tokio/reqwest allowed), this spec
removes:

```toml
{ name = "rmcp", reason = "..." }
```

from `crates/deny.toml`.

Keep `cargo deny check licenses` clean — rmcp pulls in tokio (already
allowed by the async-http spec), schemars, and JSON-Schema codegen
helpers. Confirm all are Apache-2.0 / MIT.

## What rmcp does NOT replace

The cutover must not touch:

- **Sandbox metadata emission** (`sandbox_metadata.rs`) — runx receipt
  shape, not part of the MCP protocol.
- **Template engine** (`templates.rs`) — runx-specific argument templating
  for `{{ field }}` substitution. rmcp doesn't know about runx templates.
- **Server-side skill execution** (`server_skill.rs`) — runx skill model;
  rmcp doesn't know runx skills.
- **`McpHostRunResult` projection** — runx `runx:` content object that
  encodes runx run state (skillName, runId, receiptId, status); the
  `mcp_tool_result_from_host_result` function continues to exist.
- **runx receipt sealing** (`step_receipt`, `LocalReceiptStore` writes) —
  runx contract.

## Risks

- **rmcp churn**: the crate is pre-1.0. Stage 1 must include a CI pin to a
  specific minor and a `cargo update -p rmcp` review gate.
- **Tokio bloat**: rmcp pulls full-feature tokio if we're not careful. The
  feature spec above uses `transport-io` only.
- **Test fixture diff**: byte-exact diffs are hard to predict. Stage 4 may
  surface protocol-version drift between our hand-rolled `2025-06-18` and
  rmcp's default. The cutover spec must enumerate which JSON keys are
  allowed to differ.
- **Production soak**: aster and nitrosend depend on `runx mcp serve`.
  Stage 5 must not delete the hand-rolled path until both have run on
  `mcp-rmcp` for ≥ 30 days without incident.

## Acceptance gates for stages

| Stage | Acceptance |
| --- | --- |
| 1 (feature exists) | `cargo check --workspace --features mcp-rmcp` clean |
| 2 (client replaced) | all `mcp_adapter` tests pass with `--features mcp-rmcp` |
| 3 (server replaced) | all `mcp_server` tests pass with `--features mcp-rmcp` |
| 4 (wire parity) | fixture diff against hand-rolled bytes within pre-declared envelope |
| 5 (deletion) | aster + nitrosend production soak ≥ 30 days; hand-rolled files removed; `mcp-rmcp` renamed to `mcp` |

## Open questions

- Should rmcp's HTTP transport (SSE / streamable HTTP) be enabled too, or
  only stdio? Defer: scope this spec to stdio only; add a follow-up if a
  consumer needs HTTP.
- Should runx publish rmcp `ServerHandler` impls as a public reusable
  type? Probably yes, but defer to a `runx-mcp-public-server-trait` spec
  after the cutover.

## References

- [`crates/runx-runtime/src/adapters/mcp/`](../../crates/runx-runtime/src/adapters/mcp/)
- [`crates/deny.toml`](../../crates/deny.toml) — current rmcp ban
- [`oss/docs/rust-kernel-architecture.md`](../../docs/rust-kernel-architecture.md)
  §13 (MCP is "the hardest port")
- [`plans/rust-takeover.md`](../../../plans/rust-takeover.md) §9 step 7
  ("MCP is last")
- rmcp upstream: `https://github.com/modelcontextprotocol/rust-sdk`
