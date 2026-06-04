#!/usr/bin/env sh
# OpenAPI front demo: a sealed OpenAPI call against a local fixture endpoint.
#
# Starts the fixture pets server, runs the graph (whose step resolves the getPet
# operation and calls it), and shows the real response sealed into the receipt.
# No external network; override the binary with RUNX_BIN=/path/to/runx ./run.sh
set -e

HERE="$(cd "$(dirname "$0")" && pwd)"
OSS="$(cd "$HERE/../.." && pwd)"
RUNX="${RUNX_BIN:-$OSS/crates/target/debug/runx}"
[ -x "$RUNX" ] || RUNX="$(command -v runx || true)"
[ -n "$RUNX" ] || { echo "runx binary not found; set RUNX_BIN." >&2; exit 1; }

# A demo-only receipt-signing identity (runx mandates signed receipts).
export RUNX_RECEIPT_SIGN_KID="${RUNX_RECEIPT_SIGN_KID:-runx-demo-key}"
export RUNX_RECEIPT_SIGN_ED25519_SEED_BASE64="${RUNX_RECEIPT_SIGN_ED25519_SEED_BASE64:-QkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkI=}"
export RUNX_RECEIPT_SIGN_ISSUER_TYPE="${RUNX_RECEIPT_SIGN_ISSUER_TYPE:-hosted}"

node "$HERE/server.mjs" &
SERVER=$!
trap 'kill $SERVER 2>/dev/null || true' EXIT
sleep 1
kill -0 "$SERVER" 2>/dev/null || { echo "OpenAPI fixture server did not start." >&2; exit 1; }

RDIR="$(mktemp -d 2>/dev/null || echo /tmp/runx-openapi-demo)"
"$RUNX" harness "$OSS/examples/openapi-graph" --receipt-dir "$RDIR" --json

node - "$RDIR" <<'NODE'
const fs = require("node:fs");
const path = require("node:path");

const root = process.argv[2];
const runs = path.join(root, "runs");
const states = fs.existsSync(runs)
  ? fs.readdirSync(runs).filter((name) => name.endsWith(".graph-state.json"))
  : [];

for (const name of states) {
  const state = JSON.parse(fs.readFileSync(path.join(runs, name), "utf8"));
  const steps = state?.checkpoint?.steps ?? [];
  const call = steps.find((step) => step.step_id === "call");
  const output = call?.outputs;
  if (
    output?.executed === true &&
    output?.method === "GET" &&
    output?.status_code === 200 &&
    output?.response?.id === "p-42" &&
    output?.response?.name === "pet-p-42"
  ) {
    console.log(
      JSON.stringify(
        {
          executed: output.executed,
          method: output.method,
          status_code: output.status_code,
          response: output.response,
        },
        null,
        2,
      ),
    );
    process.exit(0);
  }
}

console.error("OpenAPI fixture GET was not executed successfully in graph state");
process.exit(1);
NODE

echo "------------------------------------------------------------"
echo "the OpenAPI call executed against the fixture endpoint and sealed:"
echo "receipts: $RDIR"
