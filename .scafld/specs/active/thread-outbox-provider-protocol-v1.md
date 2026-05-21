---
spec_version: '2.0'
task_id: thread-outbox-provider-protocol-v1
created: '2026-05-22T01:16:00+10:00'
updated: '2026-05-22T01:16:00+10:00'
status: active
harden_status: not_run
size: large
risk_level: high
---

# Thread outbox provider protocol v1

## Current State

Status: active
Current phase: planning boundary ratified; implementation blocked
Next: design provider push/fetch frames and Rust-supervised credential delivery
before adding any GitHub, Slack, or support-channel outbox adapter
Reason: thread/outbox provider writes are a distinct extension lane. They must
not be smuggled through `external-adapter-plugin-protocol-v1`, revived through
`@runxhq/runtime-local`, or implemented as hidden `@runxhq/core` provider
mutations. The only implemented outbox pusher today is the local file-thread
adapter, which is credential-free.
Blockers: provider adapter protocol frames, idempotency/readback receipts, and
credential profile mapping are not implemented.
Allowed follow-up command: `scafld harden thread-outbox-provider-protocol-v1`
Latest runner update: 2026-05-22T01:16:00+10:00 created the missing owning spec
and updated the local file-thread skip path to fail closed for provider
adapters until this protocol consumes Rust-supervised `CredentialDelivery`.
Review gate: not_started

## Summary

Define the language-neutral provider protocol for pushing thread outbox entries
to systems such as GitHub issues/PRs, Slack threads, or support-channel
surfaces. This lane owns provider-side publication and readback for outbox
entries; it is not an execution-adapter protocol and not a TypeScript runtime
fallback.

Provider adapters that require tokens must consume
`credential-broker-delivery-contract-v1` through Rust-supervised
`CredentialDelivery`. Public frames may carry credential refs, profile ids,
provider, purpose, delivery mode, and material-ref hashes only. Raw secret
material must not appear in outbox entries, thread state, receipts, logs,
provider observations, or adapter responses.

## Context

- `docs/ts-interop-boundary.md` already lists thread/outbox provider adapters as
  a separate language-neutral extension lane.
- `packages/core/src/knowledge/file-thread.ts` implements only the local file
  thread adapter. It is credential-free and should stay a helper/read-model
  path, not become a provider mutation runtime.
- `external-adapter-plugin-protocol-v1` explicitly excludes thread/outbox
  provider queues.
- `credential-broker-delivery-contract-v1` needs outbox/provider specs either
  to consume its primitive or to name provider credentials as a blocker.

## Objectives

- Specify provider outbox adapter manifest and invocation frames for push,
  fetch/readback, dedupe, and idempotent retry.
- Specify credential needs using `credential-broker-delivery-contract-v1`; no
  provider adapter may define a private secret channel.
- Specify receipt/readback observations for published messages/comments/PR
  updates without leaking raw provider payloads or secrets.
- Preserve source-thread routing: missing recoverable thread targets must fail
  closed before provider mutation.
- Keep the local file-thread adapter credential-free and explicit.

## Scope

In scope:
- Provider push/fetch frames for thread outbox entries.
- Credential delivery profile references and Rust-supervised process/env
  delivery for provider adapters.
- Idempotency keys, provider locator readback, and receipt-safe metadata.
- Negative tests proving `@runxhq/core` and package helpers do not mutate real
  providers without this protocol.

Out of scope:
- General execution adapters, owned by `external-adapter-plugin-protocol-v1`.
- Source-event ingress and webhook admission.
- OAuth/BYO storage lifecycle, owned by credential/connect specs.
- Merging pull requests or deciding policy admission.
- Local file-thread persistence beyond the current helper behavior.

## Dependencies

- `credential-broker-delivery-contract-v1`
- `ts-extension-survivorship-boundary`
- `rust-ts-sunset-runtime-local`
- `runx-target-repo-runners`
- `runx-post-merge-closure-observer`

## Touchpoints

- `packages/core/src/knowledge/file-thread.ts`
- `packages/core/src/knowledge/thread.ts`
- `docs/ts-interop-boundary.md`
- `docs/thread-story-contract.md`
- future `crates/runx-contracts` outbox provider contract module
- future Rust runtime provider adapter supervisor or CLI command surface

## Risks

- Reusing the external execution-adapter protocol would blur execution and
  publication semantics and recreate a catch-all plugin API.
- Letting TypeScript helpers perform provider mutations would keep a hidden
  trusted runtime alive after the Rust cutover.
- Provider retries without idempotency/readback can duplicate comments, PR
  updates, or source-thread replies.
- Credential material can leak if provider observations are recorded before
  redaction and receipt shaping.

## Acceptance

Profile: strict

Definition of done:
- [x] `dod1` The outbox/provider lane has an owning spec distinct from the
  external execution-adapter protocol.
- [x] `dod2` Existing local file-thread outbox push explicitly skips provider
  adapters and names this protocol plus Rust-supervised `CredentialDelivery` as
  the blocker.
- [ ] `dod3` Provider push/fetch/readback frames are defined in
  `runx-contracts` and generated TypeScript contracts.
- [ ] `dod4` Provider adapters consume `credential-broker-delivery-contract-v1`
  and reject private secret fields.
- [ ] `dod5` Idempotent provider mutation and readback receipt tests exist for
  at least one provider fixture.
- [ ] `dod6` Runtime-local/adapters sunset can point provider outbox work at
  this protocol without preserving a TypeScript mutation fallback.

Validation:
- [x] `v1` Local knowledge tests prove provider outbox push is skipped, not
  silently handled by TS core.
  - Command:
    `pnpm vitest run packages/core/src/knowledge/index.test.ts`
  - Expected kind: `exit_code_zero`
  - Status: passed
  - Evidence: 2026-05-22T01:17:00+10:00 passed 20 tests, including
    `skips push when no runtime adapter is registered`, whose reason now names
    `thread-outbox-provider-protocol-v1`/Rust-supervised `CredentialDelivery`
    as the provider mutation blocker.
- [x] `v2` Scafld validates this spec.
  - Command: `scafld validate thread-outbox-provider-protocol-v1 --json`
  - Expected kind: `exit_code_zero`
  - Status: passed
  - Evidence: 2026-05-22T01:17:00+10:00 returned
    `{"ok":true,"command":"validate","result":{"task_id":"thread-outbox-provider-protocol-v1","path":"/Users/kam/dev/runx/runx/oss/.scafld/specs/active/thread-outbox-provider-protocol-v1.md","valid":true,"errors":null}}`.

## Phase 1: Boundary

Status: active
Dependencies: `ts-extension-survivorship-boundary`

Changes:
- [x] Name thread/outbox provider adapters as a separate protocol lane.
- [x] Keep file-thread local and credential-free.
- [x] Fail closed for provider adapter types until this protocol exists.

## Phase 2: Contracts

Status: blocked
Dependencies: Phase 1, `credential-broker-delivery-contract-v1`

Changes:
- Define push/fetch/readback frames.
- Define credential profile references and delivery observations.
- Define idempotency/readback receipt metadata.

## Phase 3: Provider Fixture

Status: blocked
Dependencies: Phase 2

Changes:
- Add one provider fixture adapter under Rust supervision.
- Prove idempotent push and readback receipt shaping.
- Prove secrets are redacted from observations and receipts.

## Rollback

If provider outbox semantics cannot be made language-neutral, keep provider
mutation blocked. Do not route provider outbox writes through
`external-adapter-plugin-protocol-v1` and do not revive `@runxhq/runtime-local`
or `@runxhq/adapters` as a trusted provider mutation fallback.

## Review

Review must reject any implementation that:
- accepts raw provider tokens in public outbox frames;
- mutates real providers from `@runxhq/core` or another surviving TypeScript
  helper without Rust supervision;
- treats external execution adapters as the outbox provider protocol;
- publishes to a fallback root channel/thread when source-thread routing is
  missing.

## Origin

User architecture review on 2026-05-22: external execution adapters, source
ingress, hosted runtime binding, catalog/read-model access, and thread/outbox
provider queues are separate lanes. The outbox/provider credential blocker
needed an owning spec so the credential broker contract could stop treating it
as an unnamed future gap.
