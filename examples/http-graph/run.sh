#!/usr/bin/env sh
# HTTP front demo: a governed first-class http source against a local fixture.
#
# Starts the fixture pets server, runs the graph (whose step maps inputs to a
# governed GET and seals the response), and shows the real response in the
# receipt. The http source opts in to the loopback fixture via
# allow_private_network; the default transport blocks private networks.
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
# Operator grant for this demo's loopback-only fixture endpoint. The runtime
# still blocks private-network HTTP by default outside this explicit demo grant.
export RUNX_HTTP_ALLOW_PRIVATE_NETWORK="${RUNX_HTTP_ALLOW_PRIVATE_NETWORK:-1}"

node "$HERE/server.mjs" &
SERVER=$!
trap 'kill $SERVER 2>/dev/null || true' EXIT
sleep 1

RDIR="$(mktemp -d 2>/dev/null || echo /tmp/runx-http-demo)"
"$RUNX" harness "$OSS/examples/http-graph" --receipt-dir "$RDIR" --json

echo "------------------------------------------------------------"
echo "the governed HTTP call executed against the fixture endpoint:"
grep -rhoE '"http_status": *"200"|pet-p-42' "$RDIR" 2>/dev/null | sort -u
