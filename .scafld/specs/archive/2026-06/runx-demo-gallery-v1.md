---
spec_version: '2.0'
task_id: runx-demo-gallery-v1
created: '2026-06-04T06:20:35Z'
updated: '2026-06-04T21:49:36Z'
status: completed
harden_status: not_run
size: medium
risk_level: medium
---

# runx-demo-gallery-v1

## Current State

Status: completed
Current phase: final
Next: done
Reason: task completed
Blockers: none
Allowed follow-up command: `none`
Latest runner update: 2026-06-04T21:49:36Z
Review gate: pass

## Summary

A demos/showcase surface that links each runnable example to its one-line proof and
its sealed, offline-verifiable receipt, turning the shipped + new heroes into a
coherent product story: the governed GitHub refusal, offline verify, the payment
launch demo, OpenAPI multi-spec, and the per-lane fronts. Distinct from frantic.md
(the separate capture-venue brand); this is a runx `/demos` surface (and/or a
recorded asciinema set). It also promotes the offline verifier
(`examples/governed-spend/verify.mjs`) into a standalone reusable verifier with a
JWKS-style issuer-pubkey discovery endpoint — the objection-killer carried across
every hero and launch-blocking for the real-rail demo.

## Objectives

- A curated demo gallery: each demo = a runnable example + a one-line proof + a
  sealed receipt the viewer verifies offline.
- Feature the GitHub hero trio, governed-spend, the offline-verify demo, OpenAPI,
  and the per-lane fronts; add the payment HN artifact when it lands.
- Promote `verify.mjs` to a standalone reusable verifier + publish the issuer pubkey
  via a JWKS-style discovery endpoint.

## Scope

In scope:
- The `/demos` gallery surface; wiring each runnable example to its proof + receipt;
  the standalone verifier + pubkey discovery; a harness case per featured demo so a
  demo cannot silently rot.

Out of scope:
- frantic.md (separate brand/venue); marketing polish; demos for unbuilt lanes
  (link them as they land).

## Dependencies

- The heroes/examples (GitHub hero — Wave 0; payment — Wave 1; OpenAPI — Wave 2);
  `verify.mjs` (shipped, to promote); the issuer signing key (for pubkey discovery).

## Assumptions

- Each featured demo already (or will) ship as a runnable example with a sealed
  receipt; the gallery curates + links, it does not re-implement.

## Touchpoints

- A site `/demos` surface (or asciinema set); `oss/examples/**/run.sh`; the promoted
  standalone verifier; the JWKS-style pubkey endpoint; harness cases per demo.

## Risks

- **Demo rot.** A linked demo that silently breaks is worse than none. Mitigation:
  every gallery demo has a harness case gated in CI.

## Acceptance

Profile: standard

Validation:
- The gallery lists each runnable example with its one-line proof and a receipt the
  viewer verifies offline with the standalone verifier + pubkey discovery.
- Each featured demo has a CI-gated harness case.

## Phase 1: Gallery surface + the shipped demos + verifier promotion

Status: completed
Dependencies: the shipped examples, verify.mjs

Objective: a curated gallery of the demos that ship today, each offline-verifiable.

Changes:
- Build the `/demos` surface; wire the shipped examples + proofs + receipts; promote the standalone verifier + JWKS pubkey discovery; harness case per demo.

Acceptance:
- [x] `ac1` command - the standalone verifier confirms a featured demo receipt offline
  - Command: `bash -lc 'set -e; cargo build --manifest-path crates/Cargo.toml -p runx-cli --bin runx --all-features >/dev/null; OUT="$(mktemp -d)"; RDIR="$OUT/receipts"; mkdir -p "$RDIR"; node examples/http-graph/server.mjs >"$OUT/server.log" 2>&1 & SERVER=$!; trap "kill $SERVER 2>/dev/null || true" EXIT; sleep 1; RUNX_RECEIPT_SIGN_KID=runx-demo-key RUNX_RECEIPT_SIGN_ED25519_SEED_BASE64=QkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkI= RUNX_RECEIPT_SIGN_ISSUER_TYPE=hosted crates/target/debug/runx harness examples/http-graph --receipt-dir "$RDIR" --json >"$OUT/harness.json"; ROOT_ID="$(node -e "const fs=require(\"fs\");const j=JSON.parse(fs.readFileSync(process.argv[1],\"utf8\"));console.log(j.receipt_ids[0] ?? \"\")" "$OUT/harness.json")"; test -n "$ROOT_ID"; node tools/verify/verify.mjs "$RDIR/$ROOT_ID.json" --jwks tools/verify/runx-demo-jwks.json --walk-ancestry --receipt-dir "$RDIR"'`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-6

## Phase 2: Add the heroes as they land (payment, OpenAPI, per-lane)

Status: completed
Dependencies: the respective wave demos

Objective: the gallery grows with each wave.

Changes:
- Add the payment HN artifact, the OpenAPI multi-spec demo, and per-lane demos as they ship.

Acceptance:
- [x] `ac2` command - each newly-featured demo has a gated harness case
  - Command: `pnpm fixtures:harness:check`
  - Expected kind: `exit_code_zero`
  - Status: pass
  - Evidence: exit code was 0
  - Source event: entry-11

## Rollback

- Additive surface; remove the gallery + the standalone verifier export (the
  in-example verify.mjs remains).

## Review

Status: completed
Verdict: pass
Mode: discover
Provider: command
Output: command.stdout
Summary: Reviewed demo gallery and standalone verifier promotion. The verifier is single-sourced in tools/verify, the governed-spend path is a wrapper, JWKS key discovery is tested, and the shipped demo gallery lists only runnable examples with harness status.

Attack log:
- `verifier single-source`: Old examples/governed-spend/verify.mjs path must not fork verifier logic after promotion. -> clean
- `JWKS key discovery`: Verifier must select Ed25519 key by issuer kid and public_key_sha256 rather than trusting arbitrary JWKS entry. -> clean
- `demo rot guard`: Gallery must distinguish harness-gated demos from runnable-only examples and fixtures:harness:check must remain green. -> clean

Findings:
- none

## Self Eval

- none

## Deviations

- none

## Metadata

- created_by: scafld

## Origin

Created by: scafld
Source: plan

## Harden Rounds

- none

## Planning Log

- none
