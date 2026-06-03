# Runx OSS Conventions

## Scope

This file applies to the OSS workspace under `oss/`. It complements
`AGENTS.md`, `CLAUDE.md`, `docs/rust-kernel-architecture.md`, and
`docs/trusted-kernel-package-truth.md`.

## Contract Vocabulary

Governed runtime artifacts use the harness spine:

- `harness`: governed execution boundary with attenuated authority.
- `act`: contained payload with `intent`, `form`, and `closure`.
- `receipt`: sealed proof of a harness node.
- `decision`: accountable harness lifecycle choice.
- `signal`: world-before-action input.

Do not introduce compatibility aliases, `.v2` contract ids, or retired central
object names at governed boundaries. Product-facing skill names may remain
recognizable; wire contracts must use the spine vocabulary.

## Package Boundaries

Package names carry trust claims:

- contracts define portable schemas and generated validators.
- Rust `runx-core` owns pure state-machine and policy decisions.
- TypeScript `@runxhq/core` is private legacy build-only surface during sunset.
  Do not add new imports to it, and do not restore local execution, policy,
  receipt sealing, or runtime fallback behavior there.
- `runx-runtime` coordinates local execution, adapters, sandbox planning,
  caller interaction, and receipts.
- host adapters and protocol adapters touch external processes and protocols.
- `runx-cli` is the native command shell over the runtime.

OSS packages must not import cloud code. Core must not import runtime, adapter,
CLI, host-adapter, filesystem, network, or subprocess concerns.

## Rust Bar

Rust code must keep the workspace green under:

```sh
cargo fmt --manifest-path crates/Cargo.toml --all --check
cargo clippy --manifest-path crates/Cargo.toml --workspace --all-targets -- -D warnings
cargo test --manifest-path crates/Cargo.toml --workspace
cargo deny --manifest-path crates/Cargo.toml check bans licenses sources
```

Workspace lints deny unsafe code and common escape hatches such as unwrap,
expect, panic, todo, unimplemented, dbg, and print macros. Do not work around
these with broad allows.

## Specs

Scafld specs are execution contracts, not notes. A spec that is stale against
the current harness spine or package truth must be repaired before approval or
build. Completed specs with failed, blocked, or not-run hardening need an
explicit follow-up or a recorded deviation before another spec treats them as
authoritative evidence.

## Fixtures

Fixtures are parity evidence. Do not regenerate fixtures merely to make a new
implementation pass. Preserve semantic meaning, review diffs, and add negative
fixtures when a contract rejects retired vocabulary or unsafe payloads.
