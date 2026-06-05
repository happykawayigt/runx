# Demo Gallery

These demos are runnable from this repository and produce signed receipts. Use the
standalone verifier at `tools/verify/verify.mjs` with the demo issuer key in
`tools/verify/runx-demo-jwks.json`.

```sh
export RUNX_RECEIPT_SIGN_KID=runx-demo-key
export RUNX_RECEIPT_SIGN_ED25519_SEED_BASE64=QkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkI=
export RUNX_RECEIPT_SIGN_ISSUER_TYPE=hosted
```

## Shipped Demos

| Demo | Proof | Run | Gate |
| --- | --- | --- | --- |
| `examples/github-mcp-hero` | GitHub MCP repo read succeeds, out-of-scope write is refused, and the denial receipt verifies offline. | `sh examples/github-mcp-hero/run.sh` | harness |
| `examples/http-graph` | A graph step uses the governed HTTP front against a local fixture and seals a receipt tree. | `sh examples/http-graph/run.sh` | harness |
| `examples/openapi-graph` | An OpenAPI-described operation is executed through the governed external-adapter lane and sealed. | `sh examples/openapi-graph/run.sh` | harness |
| `examples/governed-spend/skills/overspend-refused` | A spend request over authority is refused and sealed as a deterministic local receipt. | `runx harness examples/governed-spend/skills/overspend-refused` | harness |
| `examples/governed-spend/stripe-spt.sh` | Stripe SPT test-mode path, deterministic by default and live when operator test credentials are exported; settlement and refusal receipts verify offline. | `sh examples/governed-spend/stripe-spt.sh` | `pnpm demos:check` |

## Payment Demo Gate

```sh
pnpm demos:check
```

This runs the safe payment demo paths (`payments-demo.mjs`, x402 mock, and Stripe
SPT mock) and verifies every emitted receipt with the standalone verifier.

## Runnable Previews

These examples are useful local proofs but are not part of the featured,
harness-gated set yet.

| Demo | Proof | Run |
| --- | --- | --- |
| `examples/byo-http-graph` | A locally delivered credential reaches the governed HTTP front without entering the skill manifest. | `sh examples/byo-http-graph/run.sh` |

## Verify A Receipt

The verifier is independent of runx runtime code. It recomputes the canonical
receipt body hash, checks the content-addressed receipt id, verifies the Ed25519
signature, and can walk receipt ancestry from top-level receipt-store artifacts.

```sh
node tools/verify/verify.mjs /path/to/receipt.json \
  --jwks tools/verify/runx-demo-jwks.json

node tools/verify/verify.mjs /path/to/graph-root-receipt.json \
  --jwks tools/verify/runx-demo-jwks.json \
  --walk-ancestry \
  --receipt-dir /path/to/receipt-store
```

`examples/governed-spend/verify.mjs` remains as a compatibility wrapper for older
demo instructions.
