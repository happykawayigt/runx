#!/usr/bin/env sh
set -eu

cd "$(dirname "$0")/.."
pnpm exec tsx scripts/generate-runtime-mcp-oracles.ts --check
