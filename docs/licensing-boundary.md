# Connect/Auth Licensing Boundary

Status: current boundary.

## Rule

OSS crates may consume local credential material, enforce declared authority,
redact credential references, and write receipt-safe observations. OSS crates
must not broker third-party OAuth, custody hosted secrets, issue verified hosted
grants, or expose hosted provider connection APIs.

The local credential path is intentionally small: a run may receive an API key
or personal token through a per-run local descriptor, resolve it through
`MaterialResolver`, and inject it only into the child process boundary for that
execution. The hosted/cloud layer owns OAuth brokerage and credential custody.

## Current OSS Surface

- `runx-contracts` keeps provider-neutral credential envelope and credential
  delivery contracts.
- `runx-core` keeps policy and authority admission. It does not call providers
  or issue grants.
- `runx-runtime` keeps local credential consumption, sandbox delivery, and
  redaction.
- `runx-cli` keeps the native OSS CLI shape and does not perform hosted connect
  brokerage.
- `runx-sdk` does not expose hosted connect-list APIs.

## Denied OSS Surface

The manifest at `docs/license-boundary.manifest.json` is the machine-readable
guard input. It blocks hosted connect client identifiers, legacy connection
keys, and private provider implementation names in MIT Rust crates, with the
single negative fixture allowlist required by the guard tests.

For TypeScript/JavaScript, `scripts/check-boundaries.mjs` additionally blocks
hosted OAuth credential contract shapes in active OSS code, fixtures, and
generated schemas.

## Private Home

Hosted OAuth, provider gateway routing, credential custody, grant issuance, and
grant revocation live in `../cloud/packages/auth` and the cloud/API wiring that
depends on it. Public OSS documentation should describe only the boundary and
the local credential-consumption contract, not private provider implementation
details.
