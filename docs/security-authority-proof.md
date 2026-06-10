# Security Authority Proof

Runx receipts must explain the authority boundary without becoming a secret
side channel. The compact proof lives in receipt metadata under
`authority_proof` and validates against `runx.authority-proof.v1`.

Allowed public fields:

- `run_id`, `skill_name`, and `source_type`
- requested connected-auth scopes and whether the skill declared mutating work
- scope admission status, granted scopes, grant id, and decision summary
- provider, connection id, grant reference, and `material_ref` hash
- sandbox profile, declared enforcement, runtime enforcer, and approval result
- redaction policy status

Banned fields:

- raw access tokens, refresh tokens, API keys, passwords, client secrets, and
  provider credential bodies
- full private stdout or stderr bodies in public projections
- ambient environment dumps or unbounded local command logs
- unchecked provider output bodies in comments, public evidence, or ledgers

Credential material is represented by hashed opaque handles such as
`material_ref_hash`. Receipt writers still hash stdout and stderr, and metadata is
passed through the receipt redactor before signing. Hosted workers and local
runners use the same `authority_proof` schema name; consuming repos add policy
for source channels, assignees, and target repositories outside the core proof.
Runtime secret handoff is owned by `credential-broker-delivery-contract-v1`:
secret values may cross only the trusted broker/supervisor delivery channel, not
authority proofs, receipts, invocation metadata, adapter observations, or public
provider evidence.

## Ownership Boundary

The Rust `AuthorityProof` wire structs are policy-owned in `runx-core`, not
promoted into `runx-contracts`. The proof is produced only by the policy kernel,
shares admission support types such as `ScopeAdmission`, `AuthorityKind`, and
`CredentialGrantReference`, and is validated as a contract through generated
schema checks in `runx-contracts`. Future contract-spine work should treat this
as an explicit exception unless it can move the full boundary without changing
the `runx.authority-proof.v1` JSON shape.

The local kernel resolves authority in this order:

1. Structural policy admission runs before connected auth resolution.
2. Grant resolution returns only grant descriptors.
3. Sandbox approval gates run before execution.
4. Credential resolution returns an opaque credential envelope only after
   admission.
5. The signed receipt records the proof, hashes outputs, and omits raw secrets.

## Provider-Permission Grants

`provider_permission` graph policy may declare required scopes, an expected
grant id, and the authority verb. It must not declare `granted_scopes`; granted
scopes come only from operator-carried runtime grant evidence.

Provider-permission steps fail closed unless the operator supplies both:

- `RUNX_PROVIDER_PERMISSION_GRANT_ID`
- `RUNX_PROVIDER_PERMISSION_GRANTED_SCOPES`

This is intentional. Older local runs that relied on an implicit grant id must
set `RUNX_PROVIDER_PERMISSION_GRANT_ID` explicitly before executing
provider-permission steps.

When a provider-permission effect is admitted, the sealed step receipt records
the operator grant as a typed `runx:grant:*` reference under
`receipt.authority.grant_refs`. The grant id is evidence of the authority that
admitted the effect; it is not a credential body and does not carry provider
token material.

## Payment Aggregate Spend Caps

Spend-class payment authority must carry an aggregate cap (`max_per_run_units`
or `max_per_period_units`) in addition to any per-call cap. Both aggregate caps
are enforced by the runtime spend ledger.

Per-run: each run's reserved spend is bounded by the smaller of the two
declared caps, because a run never spans more than one period.

Per-period: when the authority also declares a `period` of `daily`, `weekly`,
or `monthly`, every spend is additionally reserved against a durable
calendar-window ledger in the effect state file (`RUNX_EFFECT_STATE_PATH` or
`<receipt dir>/effect-state.json`), so the cap holds across runs inside one
UTC window. An unrecognized `period` value fails closed at admission instead
of becoming an unenforced annotation. A period cap declared without a `period`
is enforced only as the run-level clamp, and the period ledger only exists
when an effect state path is configured — operators who want cross-run spend
bounds must configure a stable state path.

Period ledgers are bounded during the same locked state transaction that
records the reservation. For each family/authority/currency/period tuple, the
runtime retains at least the active reservation window and the immediately
previous window, keeps any newer windows already present, and prunes older
period-ledger rows. Idempotency entries, finality records, finality events,
consumed capabilities, and run-spend ledgers are not pruned by this retention
pass. Out-of-order reservations remain safe because retention is computed
relative to the reservation being recorded, not relative to the newest window
currently present in the file.

When a payment effect is admitted, the sealed step receipt records authority
evidence under `receipt.authority.grant_refs`: the admitted payment authority
reference and the spend-capability reference. Replay receipts preserve those
same authority references so a replayed sealed effect remains verifiable
against the same admitted authority boundary.

Payment supervisor proofs bind the original settlement evidence through
`evidence_digest`. Rebinding a stored proof to a re-sealed receipt first
re-verifies that the stored evidence still hashes to the sealed digest, so
evidence altered after issuance is rejected instead of silently re-blessed.

## Offline Receipt Verification

`runx verify [receipt-id] [--receipt-dir dir] [--json]` re-checks sealed
receipts from disk with no runtime or network dependency: canonical body
digests, content-addressed ids, linked-tree parent/child integrity, scope
adherence for privileged effects, and — when `RUNX_RECEIPT_VERIFY_KID` and
`RUNX_RECEIPT_VERIFY_ED25519_PUBLIC_KEY_BASE64` are set — production Ed25519
signatures against the operator-trusted key. Receipts are grouped into trees
by lineage; a chain that points at a receipt missing from the store is
reported as incomplete and fails verification. The command exits non-zero on
any finding, so it can gate automation.

Scope adherence is intentionally pure and offline. Any act carrying typed
`EffectEvidence` without corresponding `receipt.authority.grant_refs` produces
`EffectGrantEvidenceMissing`, fails verification, and exits non-zero. This is
the boundary between a signed activity log and a governance proof: the receipt
must show both the privileged effect and the operator-granted authority that
admitted it.

## Operator Authority Diagnostics

`runx doctor authority [--json]` gives operators a redacted authority readiness
view before exercising privileged effects. It reports:

- receipt signer readiness, naming `RUNX_RECEIPT_SIGN_KID`,
  `RUNX_RECEIPT_SIGN_ED25519_SEED_BASE64`, and
  `RUNX_RECEIPT_SIGN_ISSUER_TYPE`
- receipt verification readiness, naming `RUNX_RECEIPT_VERIFY_KID` and
  `RUNX_RECEIPT_VERIFY_ED25519_PUBLIC_KEY_BASE64`
- the resolved effect-state path when configured
- the consequence when `RUNX_EFFECT_STATE_PATH` is unset: cross-run spend caps,
  payment idempotency, and effect replay recovery are not durable without a
  configured state path
- provider-permission grant readiness, naming
  `RUNX_PROVIDER_PERMISSION_GRANT_ID` and
  `RUNX_PROVIDER_PERMISSION_GRANTED_SCOPES`

The diagnostic may show key ids and resolved filesystem paths. It must not show
signing seeds, public key material, provider scope values, grant ids, or
credential bodies.
