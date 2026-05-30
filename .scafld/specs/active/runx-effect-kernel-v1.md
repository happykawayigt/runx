---
spec_version: '2.0'
task_id: runx-effect-kernel-v1
created: '2026-05-31T00:00:00Z'
updated: '2026-05-30T22:38:00Z'
status: active
harden_status: not_run
size: large
risk_level: high
---

# Clean-core effect kernel: domain-free runtime + payments as the first effect family

## Current State

Status: active
Current phase: phase0
Next: build
Reason: phase phase0 opened
Blockers: none
Allowed follow-up command: `scafld handoff runx-effect-kernel-v1`
Latest runner update: 2026-05-30T22:38:00Z
Review gate: not_started

## Summary

Refactor `runx-runtime` into a **domain-free execution kernel** plus pluggable
domains, so payment becomes the *first effect family* registered into the
kernel rather than a concept the kernel knows by name. The kernel knows three
verbs and one noun: **run / resolve / seal + receipt**; everything else
(approval, payment, deploy, messaging, credentials) is composed from generic
governance primitives and registered adapters. This both cleans the core and
makes the 11 payment skills harnessable (a registered test effect supervisor),
and it is validated against seven non-payment use cases so the seams are
generic, not payment-shaped.

Real rails stay in hosted TS. The native side governs and seals **evidence**;
it holds no secrets and makes no provider network calls.

## Objectives

1. Replace hardcoded step/source dispatch with **registries** (`StepType`,
   `SourceAdapter`, `Effect`); unregistered kinds fail closed.
2. Generalize the payment supervisor into a domain-free **`EffectSupervisor`**
   seam with a generic `Evidence`/`Proof`/`Request` and an extensible typed
   `ProofKind`; `RuntimeOptions` holds an `EffectRegistry`, not a payment field.
3. Lift `receipt_before_success` and the bound model into **generic governance**
   (`runx-core`): any resource family can require receipt-before-success and
   declare its own bounds; the kernel does subset/admission, the family
   validates its bounds.
4. Add the three non-payment-shaped primitives the pressure-test surfaced:
   **non-replay markers** for irreversible effects, a **deferred-evidence
   protocol** (`Provisional → InFlight → Sealed` via a follow-on settlement
   receipt), and a **credential delivery seam** (resolve + inject + redact).
5. Extract payment into a thin native **`runx-payments`** crate (governance +
   evidence + state + ledger + test supervisor; **no rails, no secrets, no
   network**) that registers into the kernel.
6. Make all 51 inline-harness skills pass (payment via a registered **test**
   supervisor), keep the nitrosend operational-intelligence layer and sourcey
   working, and keep every preserved contract byte-stable.

## Invariant (contracts that must not break)

These are consumed by the hosted platform and external users (e.g. nitrosend).
**No breaking change is made to any of them.** The only permitted evolution is
*additive*: new optional, `skip_serializing_if = none` fields and new enum
variants (e.g. `ProofKind::MessageSend`, an optional `criterion_status`), which
leave the canonical bytes of every existing/unchanged payload identical. Any
removal, rename, or retype is out of scope and must fail the gates.

- `runx.receipt.v1` shape + **receipt digests** (subject / seal / acts /
  criterion_bindings / evidence_refs / verification_refs). Digest of an
  unchanged skill's receipt must be byte-identical to the Phase 0 golden.
- Hosted worker boundary JSON: `HostedRuntimeServiceRequest` / `Outcome`
  (`cloud/packages/worker`). Existing fields frozen; additive optional fields
  (e.g. `criterion_status`) are reserved in Phase 0 and serialize identically
  when absent.
- `ResolutionRequest` / `ResolutionResponse`.
- The `runx harness <skill>` JSON summary the cloud publish-validator parses
  (`cloud/packages/api/native-publish-harness.ts`).
- The kernel never holds provider secrets and makes no provider network calls.

## Scope

**In:** `crates/runx-runtime`, `crates/runx-core`, `crates/runx-contracts`,
new `crates/runx-payments`, `crates/runx-cli` (composition root only),
`oss/skills/*` (authoring fixes), regression of `cloud/` consumers.

**Out:** real rails / customer billing (stays in `cloud/packages/billing` + the
nitrosend repo); renaming `runx-runtime` (keep the name; it *is* the kernel);
a separate `runx-governance` crate (generic governance lands in `runx-core`).

## Resulting crate shape

- **`runx-runtime`** (unchanged name) = the kernel: run/resolve/seal/receipt +
  the `StepType`/`SourceAdapter`/`Effect` registries + the `EffectSupervisor`,
  non-replay, deferred-evidence, and credential-delivery seams. No domain names.
- **`runx-core`** = generic governance: authority algebra, generic bound model,
  gate (approval generalized), receipt-before-success, act/decision/signal
  vocab, idempotency, maturity.
- **`runx-payments`** (new, feature `payment`) = payment governance/evidence:
  payment authority schema + bounds, evidence/proof domain payload,
  idempotency/recovery state, ledger, x402 sequence logic, and the deterministic
  **test** supervisor. Registers into the kernel. No rails, secrets, or network.
- **`runx-contracts`** = generic schemas only (payment-specific contract types
  move to `runx-payments`); gains extensible typed `ProofKind` + the
  deferred-evidence settlement-receipt schema.
- **`runx-cli`** = composition root: assembles the registries (registers
  cli-tool/agent/catalog/mcp/a2a adapters + the real or test effect families)
  and hands them to the kernel. No rename.
- **`runx-parser` / `runx-receipts` / `runx-sdk` / `runx-contracts-derive`** =
  essentially unchanged.
- Dependency arrows point only toward the foundations; the kernel never
  references `runx-payments`. `--no-default-features` builds a payment-free
  kernel.

## Registry & adapter shape (RESOLVED — implement exactly this)

This is the main implementation trap and is decided here so build does not
improvise:

- **`Runtime<A: SkillAdapter>` keeps its generic adapter.** Do NOT erase it to
  `Box<dyn SkillAdapter>` — `SkillAdapter` is not trivially object-safe
  (`clone_for_fanout`) and erasure would churn every `where A: SkillAdapter`
  site for no benefit. The single per-execution adapter stays statically typed.
- **`StepTypeRegistry<A>` = a map of stateless function pointers**, not
  `Box<dyn StepHandler>`. Concretely:
  `type StepHandlerFn<A> = fn(StepHandlerCtx<'_, A>) -> Result<StepRun, RuntimeError>;`
  and `StepTypeRegistry<A> { handlers: HashMap<&'static str, StepHandlerFn<A>> }`.
  Built-ins (`approval`, `agent-task`, `cli-tool`, subskill) register their
  existing `run_*_step::<A>` fns; `StepHandlerCtx` bundles
  `(&Runtime<A>, graph_dir, graph_name, &GraphStep, attempt, JsonObject,
  &mut dyn Host)`. fn-pointers monomorphize per `A`, so generics are preserved
  with zero dynamic-dispatch-of-`Runtime` cost. Lookup misses fail closed
  (`UnsupportedRunStep`). Domains register additional `run_type`s here.
- **`SourceAdapterRegistry<A>`** uses the same fn-pointer shape for the
  `SkillRunGraphAdapter`/source-kind dispatch (`source_kind -> SourceHandlerFn`).
- **`EffectRegistry` = trait objects** (`BTreeMap<EffectFamily,
  Arc<dyn EffectSupervisor>>`) on `RuntimeOptions`, because supervisors hold
  state (rails config / test fixtures). `EffectSupervisor` is kept object-safe
  (methods take `&self` + refs, return owned `Evidence`/`Proof`); no
  `Self`-returning methods. This registry does not touch the `A` generic.
- Registries are built once at `Runtime`/composition-root construction
  (`runx-cli`), never per-step; feature flags gate which handlers/supervisors
  are registered.

## No-dual-path cutover gates (hard, CI-enforced)

Old and new paths must NEVER coexist. Extend `scripts/check-runtime-cutover-legacy.mjs`
(`pnpm cutover:legacy-check`) with grep gates that FAIL the build if, after the
phase that retires each symbol, it appears in kernel orchestration:

- After **Phase 2**: `PaymentRailSupervisor`, `RuntimePaymentSupervisor`,
  `payment_supervisor`, `attach_payment_supervisor_evidence_before_gate`,
  `record_payment_supervisor_proof_metadata`, `persist_payment_step_state`, and
  any `payment::` import must NOT appear under
  `crates/runx-runtime/src/execution/runner/**` or `runner.rs`. They are allowed
  ONLY in `crates/runx-payments/**` and payment tests.
- After **Phase 4**: zero matches for `payment`, `spend`, `settlement`, `x402`,
  `rail` (case-insensitive, identifier boundaries) anywhere in `runx-runtime` or
  `runx-core` non-test sources; `cargo tree -p runx-runtime` shows no
  `runx-payments` edge.
- The cutover is a hard replacement: there is no compatibility shim, no
  feature-flagged in-tree fallback payment path, and no shadow seam. The only
  way back is `git revert`.

## Dependencies

- The hosted TS `EffectSupervisor` over the worker boundary (production rails)
  is the production implementation behind the seam; this spec defines the
  contract but does not implement TS rails.
- Aligns with `runx-operational-intelligence-action-layer-v1` (nitrosend ops
  layer): that layer's skills must keep passing unchanged.

## Assumptions

- Greenfield/0-users permits additive optional schema fields without a
  migration; no breaking canonical change is made (see Invariant).
- `Reference.proof_kind: Option<ProofKind>` and typed criterion bindings already
  exist, so new proof families are additive enum variants, not schema breaks.
- The authority *algebra* (terms/verbs/subset/chaining) is already domain-free
  and is kept; only the payment-specific *bounds/structure* move.
- `node`/`git` and similar local tools exist for cli-tool graph steps in the
  harness env; effects needing absent external binaries register a test adapter.

## Risks

- **Receipt-digest drift** during dispatch reordering. Mitigation: Phase 0
  golden corpus + run the x402 idempotency replay fixtures before/after each
  phase; any digest delta is a hard stop.
- **Runtime dispatch cost** vs today's static dispatch. Mitigation: fn-pointer
  registries (see Registry shape) monomorphize per `A` and are pre-populated
  once at construction; capture a criterion bench + harness-sweep wall-clock
  before and after Phases 1 and 2 (perf gate) and treat a >5% regression as a
  hard stop; feature flags still gate *crate inclusion*.
- **New proof families / `criterion_status` must stay additive.** Mitigation:
  every new field is optional + `skip_serializing_if = none`; the
  `fixtures:kernel:check` digest parity + the Phase 0 goldens fail on any
  non-additive drift.
- **Effect-state collision** when multiple families run in one graph.
  Mitigation: per-family-namespaced `EffectStateStore`.
- **Deferred-evidence vs immutable receipts.** Mitigation: never mutate a sealed
  receipt; async completion emits a *follow-on settlement receipt* that
  references the original act/criterion.
- **Cloud lock-step.** The worker `Outcome` JSON is frozen (Phase 0 snapshot);
  Phase 7 re-verifies the worker + publish-harness.

## Acceptance

Criteria:

- Kernel (`runx-runtime` + `runx-core`) contains zero payment references; builds
  `--no-default-features` payment-free.
- All registries fail closed on unregistered kinds; `approval`/`agent-task`/
  `cli-tool` are registered handlers, not match arms.
- One generic `EffectSupervisor` seam serves **two** unrelated proof families in
  tests (payment + a mock non-payment effect), proving genericity.
- `receipt_before_success` is enforceable for a non-payment resource family
  (deploy-style unit test); a delete-style test proves non-replay on resume.
- Inline harness: **51/51** skills pass (payment via the registered test
  supervisor; `issue-to-pr` via a registered mock for its external tool), or
  50/51 with `issue-to-pr` explicitly flagged environmental in the sweep output.
- nitrosend operational-intelligence skills + sourcey pass unchanged; receipts
  byte-identical to the Phase 0 goldens.
- No-dual-path gates green; perf within 5% of Phase 0 baseline.

Verification commands (all must pass; run from `oss/` unless noted):

```
# kernel + integration + perf
cargo test -p runx-runtime --all-features
cargo build -p runx-runtime --no-default-features        # payment-free kernel
cargo bench -p runx-runtime -- --save-baseline current && \
  node scripts/perf-compare.mjs baseline current          # <=5% regression (script added Phase 0)
# digest / contract parity (no canonical drift)
pnpm fixtures:kernel:check
# payment + graph parity
pnpm test:heavy:graph                                     # tests/payment-graph-harness.test.ts
# inline-harness sweep (script added Phase 0): prints "N/51" and exits non-zero unless target met
node scripts/harness-sweep.mjs --require 51               # or --require 50 --allow issue-to-pr
# no-dual-path + boundary gates
pnpm cutover:legacy-check                                 # extended with the Phase 2/4 grep gates
pnpm boundary:check
# hosted consumers
cd ../cloud && pnpm typecheck:server && \
  npx vitest run packages/receipts-store packages/billing packages/api
```

Review gate:

```
scafld review runx-effect-kernel-v1 --provider claude --model claude-opus-4-8 --review-depth deep
```

Claude 4.8 review is mandatory at the lifecycle review gate for the spec and
again after each phase commit before moving to the next phase. A phase is not
complete if Claude finds a blocker or if the no-dual-path/perf gates fail.

## Phase 0: Confirm the golden net (no breaking change needed)

Refinement after grounding: `Reference.proof_kind: Option<ProofKind>` ALREADY
exists (reference.rs:124, `skip_serializing_if = none`), and `CriterionBinding`
already holds typed references. So extending `ProofKind`
(`PaymentRail` -> + `MessageSend`/`DeploymentMutation`/`CredentialResolution`/…)
is **purely additive** — existing receipts (`PaymentRail`/none) are unaffected,
and `runx.receipt.c14n` does not change. The earlier "one breaking canonical
change" is unnecessary; drop it.

- Confirm the existing golden net is the baseline: oracle receipts
  (`fixtures/harness/oracle/{echo-skill,sequential-graph,payment-approval-graph}.receipt.json`)
  asserted by `tests/harness_fixtures.rs` + `tests/receipt_signing.rs` +
  `tests/parity.rs`, the 386-test integration suite, and the 40/51 harness sweep.
- Add the regression tools that don't yet exist: `scripts/harness-sweep.mjs`
  (runs every `skills/*` inline harness, prints `N/51`, `--require`/`--allow`
  gating), `scripts/perf-compare.mjs` (criterion baseline delta, fails >5%), and
  a worker-`Outcome` + `runx harness` summary JSON snapshot test.
- **Reserve the additive contract fields now** (additive only; serialize
  identically when absent): an optional `criterion_status`
  (`sealed`/`pending`/`failed`) on the worker `Outcome` criterion projection,
  and the deferred-evidence settlement-receipt schema type (unused until
  Phase 3). This makes the Phase 3 async path an intentional *additive* contract
  extension, not a later break.
- Capture the criterion bench baseline (`cargo bench -p runx-runtime -- --save-baseline phase0`)
  and the harness-sweep wall-clock as the perf reference.
- **Acceptance:** baseline green (`cargo test -p runx-runtime --all-features` +
  `node scripts/harness-sweep.mjs` reports 40/51 + `cloud` suites); snapshots +
  perf baseline captured; `pnpm fixtures:kernel:check` clean (no digest drift);
  the reserved fields serialize byte-identically to pre-Phase-0 when absent.

## Phase 1: Registries (dispatch decoupling, behavior-preserving)

- Introduce `StepTypeRegistry<A>` and `SourceAdapterRegistry<A>` using the
  **fn-pointer shape decided in "Registry & adapter shape"** (NOT
  `Box<dyn StepHandler>`/`Box<dyn SkillAdapter>`; `Runtime<A>` keeps its
  generic). Replace the `run_native_step` match, the `SkillRunGraphAdapter.invoke`
  match, and the `step.run/tool/skill` branches with registry lookups that
  **fail closed** (`UnsupportedRunStep`/`UnsupportedSource`) on unregistered kinds.
- Move `approval`/`agent-task`/`cli-tool`/subskill handlers into registered
  `fn`s; the kernel keeps the orchestration order (replay → recovery → authority
  admission → run → attach evidence → seal).
- Build registries once at `Runtime` construction; feature flags gate which
  handlers are registered.
- **Acceptance:** `node scripts/harness-sweep.mjs` still 40/51;
  `cargo test -p runx-runtime --all-features` (386+) green; `pnpm
  fixtures:kernel:check` clean (no digest drift); unregistered `run_type`/source
  fails closed with a clear error; **perf:** `cargo bench -p runx-runtime` +
  `node scripts/perf-compare.mjs phase0 current` within 5% of the Phase 0
  baseline.

## Phase 2: Generic EffectSupervisor + EffectRegistry + generic Evidence/Proof

- Rename `PaymentRailSupervisor` → `EffectSupervisor`; move payment-specific
  fields out of `Evidence`/`Proof`/`Request` into an opaque per-family domain
  payload. Generic `Evidence { verifier_id, proof_ref, provider_event_ref?,
  status?, idempotency_key?, payload }`.
- Replace `RuntimeOptions.payment_supervisor` with
  `effects: EffectRegistry` (`BTreeMap<EffectFamily, Arc<dyn EffectSupervisor>>`).
  The seal-path call sites dispatch by family/`proof_kind` via the registry.
- Generalize `PaymentStateStore`/`FileBackedPaymentStateStore` →
  `EffectStateStore` with per-family namespacing (`state[family][...]`).
- Payment remains the only registered family (behavior-preserving) — but it is
  reached ONLY through the registry; the old direct payment-supervisor calls are
  deleted from runner orchestration in this same phase (no dual path).
- **Acceptance:** `pnpm test:heavy:graph` (payment + x402 replay parity) green;
  `cargo test -p runx-runtime --all-features` green; `pnpm fixtures:kernel:check`
  clean; a mock non-payment `EffectSupervisor` registered in a unit test seals
  through the same path; **no-dual-path gate:** the extended `pnpm
  cutover:legacy-check` reports zero `PaymentRailSupervisor`/`RuntimePaymentSupervisor`/
  `payment_supervisor`/`attach_payment_supervisor_evidence_before_gate`/
  `record_payment_supervisor_proof_metadata`/`persist_payment_step_state` under
  `crates/runx-runtime/src/execution/runner/**`; **perf** within 5% of Phase 0.

## Phase 3: Generalize governance (receipt-before-success, bounds, non-replay, deferred)

- Lift `receipt_before_success` out of `PaymentAuthorityBounds` to a
  per-resource-family authority bound in `runx-core`; the seal gate enforces it
  for any family that requires it.
- Introduce a generic `Bound` algebra: `Capability { resource_family,
  bounds: Vec<Bound> }`; the kernel does subset/admission, the family validates
  its bounds; scope validation dispatches per family.
- Add a **non-replay marker** (e.g. step/effect `form: Irreversible`): on
  resume, such a step is never re-invoked; its proof is verified in place.
- Add the **deferred-evidence protocol**: `EffectSettlementPhase { Provisional,
  InFlight, Sealed }`. `settle()` may return `Provisional` (proof_ref reserved);
  async completion emits a **follow-on settlement receipt** referencing the
  original act/criterion (the sealed receipt is NEVER mutated). Provisional
  proofs surface via the optional `criterion_status` field **reserved in
  Phase 0** (additive; absent for sync-sealed proofs, so the worker `Outcome`
  for every existing skill is byte-identical to the Phase 0 snapshot).
- **Acceptance:** a deploy-style unit test enforces receipt-before-success on a
  non-payment family; a delete-style test proves non-replay on resume; a
  deferred-evidence test produces a valid follow-on settlement receipt; goldens
  unchanged for existing skills.

## Phase 4: Extract `runx-payments` (governance/evidence; no rails/secrets)

- Create `crates/runx-payments` (feature `payment`). Move: payment authority
  schema + bounds, evidence/proof domain payload, idempotency/recovery state,
  ledger projection, packets, x402 sequence logic, and the supervisor split into
  a **test** supervisor (deterministic) registered for harness/local.
- Payment **registers** its `EffectSupervisor`, its settlement step types into
  `StepTypeRegistry`, its authority schema/bounds, and its ledger hook.
- Kernel (`runx-runtime` + `runx-core`) compiles `--no-default-features`
  payment-free; `runx-cli` registers payment when the feature is on. No
  compatibility shim or in-tree fallback survives the move.
- **Acceptance:** **after-Phase-4 no-dual-path gate** (extended
  `pnpm cutover:legacy-check`): zero identifier-boundary matches for
  `payment`/`spend`/`settlement`/`x402`/`rail` in `runx-runtime` + `runx-core`
  non-test sources, and `cargo tree -p runx-runtime` shows no `runx-payments`
  edge; `cargo build -p runx-runtime --no-default-features` green; payment tests
  green via `runx-payments`; `pnpm fixtures:kernel:check` clean.

## Phase 5: Credential delivery seam

- Add a `CredentialSupervisor` seam: `resolve(request) -> Allow/Deny + encrypted
  ref` (by scopes/provider), then `deliver` (inject the secret into the step
  process env), with redaction so secrets never enter receipts/logs. Kernel
  holds no secret material. Add a `CredentialResolution` `ProofKind`.
- Fix the hosted adapter that strips `credential`
  (`cloud/packages/agent-runner/hosted-agent-adapter.ts` durable path) and
  thread credential requests through `HostedRuntimeServiceRequest`.
- **Acceptance:** a scoped-credential skill resolves + delivers locally (test
  resolver) with redaction proven; the hosted path threads credentials; a
  credential-resolution proof appears in the receipt.

## Phase 6: Test supervisors + payment harness green + multi-effect

- `runx-payments` ships a deterministic test `EffectSupervisor` + a test
  authority issuer; the inline + fixture harness register them. Fix skill
  authoring bugs surfaced (e.g. mock-refund's mis-keyed approval answer; declare
  the authority chain the issuer mints).
- Multi-effect graphs: per-family state isolation (Phase 2) + at minimum the
  InFlight/Escalated recovery path; record full saga compensation as a tracked
  follow-on if it exceeds this spec.
- **Acceptance:** the 11 payment skills **seal** in the inline harness; corpus
  sweep at 50/51 (issue-to-pr environmental) or 51/51 with its tool mocked.

## Phase 7: Skills + nitrosend + sourcey + cloud verification

- Confirm every skill passes; nitrosend operational-intelligence skills
  (issue-intake, issue-triage, run-history-analyst, receipt-auditor,
  least-privilege-auditor) and sourcey pass with byte-identical receipts vs
  Phase 0 goldens (edit skills only where authoring bugs require it).
- Re-verify the hosted consumers: `cloud` typecheck + receipts/billing tests,
  the native-publish-harness (now validates payment skills, restoring their
  maturity signals), and the worker `Outcome` boundary.
- Full regression: `cargo test -p runx-runtime`, the corpus harness sweep, the
  golden + worker-boundary assertions.
- **Acceptance:** all Acceptance criteria met end-to-end.

## Rollback

Rollback is **by `git revert` of the offending phase commit only**. There is no
runtime compatibility layer, no feature-flagged in-tree fallback payment path,
and no shadow seam to "switch back" to — the cutover is a hard replacement, so
the only supported recovery is reverting the commit (which restores the prior
phase's state wholesale). The tripwires that trigger a revert: `pnpm
fixtures:kernel:check` / the Phase 0 goldens show a receipt-digest or worker
`Outcome` change for an unchanged skill, `pnpm cutover:legacy-check` finds a
dual path, the harness sweep regresses, or perf drifts >5%. Phases are sequenced
so each is independently revertible without unwinding a later one.

## Phase 8: Confirm the golden net (no breaking change needed)

Status: active
Dependencies: none

Objective: Complete this phase.

Changes:
- none

Acceptance:
- none
