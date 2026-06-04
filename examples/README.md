# Examples

Runnable reference skills that demonstrate each runx front. These are examples,
not catalog entries: `runx list skills|graphs|tools` scans `skills/`, `graphs/`,
and `tools/`, so the examples here are intentionally absent from that catalog.
Run them directly instead.

Most need a receipt-signing identity (runx mandates signed receipts). A demo-only
identity:

```sh
export RUNX_RECEIPT_SIGN_KID=runx-demo-key
export RUNX_RECEIPT_SIGN_ED25519_SEED_BASE64=QkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkI=
export RUNX_RECEIPT_SIGN_ISSUER_TYPE=hosted
```

## The fronts

| Example | Front | Run |
| --- | --- | --- |
| `hello-world` | cli-tool (top-level runner) | `runx harness examples/hello-world` |
| `managed-agent` | agent (host-drives default; yields `needs_agent` to the calling agent) | `runx harness examples/managed-agent` |
| `external-adapter-graph` + `external-adapter-tool` | external-adapter (graph-step source; a governed subprocess adapter) | `runx harness examples/external-adapter-graph` |
| `openapi-graph` + `openapi-tool` | OpenAPI via external-adapter (an OpenAPI operation executed and sealed) | `sh examples/openapi-graph/run.sh` (real call against a local fixture) |

`external-adapter` is a graph-step source, not a top-level runner, so its examples
are driven by a one-step graph. Graph input values reach a step with the
`$input.<name>` form (for example `message: "$input.message"`).
