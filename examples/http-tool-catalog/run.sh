#!/usr/bin/env sh
# HTTP tool demo: a graph step invokes a governed http tool through the catalog.
#
# Starts the fixture pets server, points RUNX_TOOL_ROOTS at the local tool, runs
# the graph (whose `tool:` step resolves demo.pet_get, sees its http source, and
# routes it through the governed HTTP adapter), and shows the real response
# sealed into the receipt. The http tool opts in to the loopback fixture via
# allow_private_network and uses a {id} path placeholder.
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

# Resolve the http tool from this example's local tool root.
export RUNX_TOOL_ROOTS="$HERE/tools"

node "$HERE/server.mjs" &
SERVER=$!
trap 'kill $SERVER 2>/dev/null || true' EXIT
sleep 1

RDIR="$(mktemp -d 2>/dev/null || echo /tmp/runx-http-tool-demo)"
"$RUNX" harness "$OSS/examples/http-tool-catalog" --receipt-dir "$RDIR" --json

echo "------------------------------------------------------------"
echo "the governed HTTP tool executed against the fixture endpoint:"
grep -rhoE '"http_status": *"200"|pet-p-42' "$RDIR" 2>/dev/null | sort -u
