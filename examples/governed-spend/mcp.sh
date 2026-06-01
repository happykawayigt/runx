#!/usr/bin/env bash
#
# Serve the governed-spend payment skills as MCP tools, so an agent
# (Claude, ChatGPT, any MCP client) governs its spend through runx with no
# custody. tools/list returns x402-pay, mpp-pay, stripe-pay, overspend-refused;
# tools/call runs the governed graph and returns the sealed receipt or refusal.
#
set -uo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
OSS="$(cd "$HERE/../.." && pwd)"

RUNX="${RUNX_BIN:-}"
if [ -z "$RUNX" ]; then
  for cand in "$OSS/crates/target/debug/runx" "$OSS/crates/target/release/runx"; do
    [ -x "$cand" ] && RUNX="$cand" && break
  done
fi
[ -z "$RUNX" ] && command -v runx >/dev/null 2>&1 && RUNX="runx"
[ -z "$RUNX" ] && { echo "runx binary not found; set RUNX_BIN." >&2; exit 1; }

export RUNX_RECEIPT_SIGN_KID="${RUNX_RECEIPT_SIGN_KID:-runx-demo-key}"
export RUNX_RECEIPT_SIGN_ED25519_SEED_BASE64="${RUNX_RECEIPT_SIGN_ED25519_SEED_BASE64:-QkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkI=}"
export RUNX_RECEIPT_SIGN_ISSUER_TYPE="${RUNX_RECEIPT_SIGN_ISSUER_TYPE:-hosted}"

cd "$OSS"
exec "$RUNX" mcp serve \
  skills/x402-pay skills/mpp-pay skills/stripe-pay \
  examples/governed-spend/skills/overspend-refused \
  --receipt-dir "${1:-/tmp/runx-mcp}"
