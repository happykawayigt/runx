---
name: operator-inbox
description: Maintain a provider-neutral local action queue from bounded provider observations and explicit human dispositions.
runx:
  category: ops
---

# Operator Inbox

Maintain a durable action queue without turning a connector into the owner of
operator state.

The caller fetches bounded, grant-authorized provider pages and passes their
normalized observations to this skill. The skill owns work-item identity,
status, dispositions, replay suppression, reopen rules, and scan coverage. Every
read and write is composed through `data-store`; the skill does not call Slack,
SQLite, Postgres, or another provider directly.

## State boundary

Use `local://runx/operator-inbox/default` unless the operator selects another
logical source. Unbound local refs resolve to SQLite under
`.runx/data/local-sources/`. A hosted database is opt-in through the same
`data_source_ref` binding. Runx Connect may still own OAuth, grants, and provider
execution; that does not move this queue into the hosted control plane.

Observations and resumable checkpoints live in `operator_inbox_scans`, partitioned
by query digest. Action snapshots live in `operator_inbox_actions`, with one
stream per stable thread digest. Queue reads use bounded `list_stream_heads`
pages; no command folds or transports the complete queue.

## Status rules

Items use `open`, `waiting`, `followed_up`, `resolved`, or `dismissed`.

- Provider observations never infer completion.
- A human disposition records actor, reason, time, the latest external
  occurrence it covers, and optional HTTPS evidence.
- Replaying old search history preserves the human status.
- An external message newer than the covered occurrence reopens the item to
  `open`, including unseen work that arrived before the disposition was saved.
- Scan coverage is explicit: `running`, `complete`, `truncated`, or `failed`.
- Direct mentions are actionable structural evidence. Author and keyword scans
  remain observation-only unless the operator explicitly marks the query
  actionable. The skill does not contain provider-specific keyword heuristics.

The provider-neutral thread locator is the item key. Stored previews are bounded;
credentials, tokens, and full provider response envelopes are forbidden.

## Host loop

1. Read the latest checkpoint for the bounded query digest.
2. Resume its provider cursor when the prior scan was interrupted or truncated.
3. Fetch one bounded provider page through the caller's authorized connector.
4. Record actionable messages against their per-thread streams and append the
   scan page with its next cursor.
5. On a version conflict, reload only the affected scan or action stream and
   retry the idempotent transition.
6. List queue state through bounded action-head pages and use
   `record_disposition` only for an explicit operator correction.

The loop is outside the kernel. Each page or disposition remains one governed,
receipt-backed Runx turn.

## Stop conditions

- `needs_input`: missing query identity, observation, disposition,
  actor, reason, or scan coverage.
- `conflict`: the projection version is stale; reload before retrying.
- `provider_unavailable`: the caller cannot prove provider read coverage.
- `too_broad`: a page exceeds the bounded message count or contains unnormalized
  provider data.
- `refused`: a caller asks this skill to send, reply, broaden a grant, store a
  token, or silently claim complete coverage.
