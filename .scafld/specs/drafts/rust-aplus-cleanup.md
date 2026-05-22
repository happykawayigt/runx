---
spec_version: '2.0'
task_id: rust-aplus-cleanup
created: '2026-05-21T03:50:00Z'
updated: '2026-05-22T06:11:55Z'
status: draft
harden_status: needs_revision
size: large
risk_level: high
---

# Rust A+ cleanup before cutover

## Why this exists

A full dark-pattern / code-quality sweep of the Rust workspace (all 7 crates,
~62k LoC) found a ranked list of issues to clear before the launcher flip.
Several are already being executed by in-flight work (the payment→authority
generalization, `source_type`→`SourceKind`, the `runx:` URI/RunxRef work, the
TS↔Rust contract-spine reshape). This spec captures the **remaining** items so
they are not lost, with file:line and a one-line fix each. Two items were
already executed out-of-band and are recorded as done.

This is a tracking + execution spec. Each section is independently shippable.
Items touching files under active parallel work are marked **[blocked-until:
<workstream>]** and should be picked up once that workstream lands.

## Ultimate-shape correction (2026-05-22)

A design pass judged every item against runx's end-state shape (trusted Rust
core; boundary data parsed once into total typed models; one vocabulary and one
representation per concept; wire contracts are the cross-language source of
truth; determinism + single source of truth; extension lanes are explicit and
typed). Several spec items are *not* shape-aligned as written and are corrected
here. The blocked-until workstreams `source_type→SourceKind`, `RunxRef`, and the
`post_merge` reshape were never landed (verified: `enum SourceKind` count 0,
`RunxRef` count 0, `enum PostMergeCriterionKind` count 0); `payment→authority`
and the contract-spine harness/act/decision vocabulary did land.

Corrections:

- **E (RunxRef): do NOT introduce a parallel `RunxRef` type.** runx already has
  `Reference` + `ReferenceType` as the typed reference contract; a second
  abstraction for the same concept violates "one representation per concept".
  Fold the intent into the existing type: `ReferenceType::as_str()` (done), a
  `Reference` runx-URI constructor, and a parse helper. The duplicated
  `reference_type_name`/`reference()` collapse into that, not into RunxRef.
- **A (source_type→SourceKind): closed enum that includes the extension lane.**
  Verified `external_adapter.rs:97` matches a fixed `source_type ==
  "external-adapter"`, and custom-adapter identity lives in the external-adapter
  *manifest* (`adapter_id`), not on the `source.type` axis. So `SourceKind` is a
  fully closed 8-variant enum (`CliTool`, `Mcp`, `Catalog`, `A2a`, `AgentStep`,
  `HarnessHook`, `Graph`, `ExternalAdapter`) with serde kebab-rename and no
  open/`String` payload, the extension lane is honored by `ExternalAdapter`
  being a first-class variant. `input_mode` is closed (`args|stdin|none`).
- **K (`RuntimeError::Unsupported*`): keep as defensive boundary errors.**
  Because dispatch is not fully closed (the `External` lane), do not force-
  dissolve these; the parser rejects unknown built-in types, the runtime keeps
  fail-closed adapter guards.
- **A (Catalog / Sandbox): genuinely closed → typed enums.** `CatalogKind`,
  `CatalogAudience`, `CatalogVisibility`; reuse the existing
  `runx_core::policy::SandboxProfile` and `runx_core::policy::CwdPolicy` (both
  already defined and re-exported — reuse, do not redefine). Serde-rename
  preserves the wire form (no fixture change).
- **F (AutomationLevel): this is a WIRE CONTRACT, not a cleanup.** The principle
  (make the invalid `(true,true)` state unrepresentable) is right, but changing
  it Rust-only breaks TS parity. Deferred to a coordinated contract reshape.
- **J (id newtypes): shape-aligned but high-churn.** Do as one pass when the
  spine is stable; not in this slice.
- **Class L renames (L2/L4/L5/L7), G (CLI dispatch), I (clones): deferred.**
  Low-value / high-import-churn, and G conflicts with the launcher being the
  cutover parity oracle.

### Executable scope for this pass

1. **C dedup** — `ReferenceType::as_str()` + fold the 3 contracts-internal
   `sha256` copies into `fingerprint` (done); collapse the `reference()` builder
   into a `Reference` constructor.
2. **A: Catalog enums** — `CatalogKind`/`CatalogAudience`/`CatalogVisibility`.
3. **A: Sandbox enums** — reuse `SandboxProfile`, add `CwdPolicy`.
4. **A: source_type→SourceKind (incl. `ExternalAdapter`) and
   input_mode→InputMode**, parsed once in `validate_source`, with a fail-closed
   parse error for unknown types. `Unsupported*` runtime guards retained.

## Planned Phases

(Revised per harden round-1, claude provider, 2026-05-22.)

Phase 1 — Reference consolidation (C). Done: `ReferenceType::as_str()` + the 3
contracts-internal `sha256` folds. Remaining: collapse all **four**
byte-identical `fn reference(ReferenceType, id)` runx-URI builders in
`runx-runtime` (`receipts/seal.rs`, `execution/target_runner.rs`,
`credentials.rs`, `adapters/external_adapter.rs`) into one
`Reference::for_type(ReferenceType, &str)` constructor in
`runx-contracts::reference`, and replace the surviving `reference_type_name`
in `runx-receipts/src/tree.rs` with `ReferenceType::as_str()`. Acceptance:
`rg '^\s*fn reference\b' crates/runx-runtime/src` and `rg 'fn
reference_type_name' crates` both return 0; `cargo clippy --manifest-path
crates/Cargo.toml --workspace --all-targets --all-features -- -D warnings`
clean.

Phase 2 — Catalog enums (A). `CatalogKind` (`skill|graph`), `CatalogAudience`
(`public|builder|operator`), `CatalogVisibility` (`public|private`) in
`runx-parser`, serde-rename, parsed once. Acceptance: `cargo test -p
runx-parser`; catalog fixtures deserialize unchanged.

Phase 3 — Sandbox enums (A). `SkillSandbox.profile` reuses the existing
`runx_core::policy::SandboxProfile`; `cwd_policy` reuses the existing
`runx_core::policy::CwdPolicy` (`skill-directory|workspace|custom`) — both
already defined/re-exported, no new enum. Acceptance: `cargo test -p
runx-parser -p runx-runtime`; sandbox fixtures unchanged.

Phase 4a — Parser SourceKind/InputMode (A). Closed 8-variant `SourceKind`
(incl. `ExternalAdapter`) + `InputMode` (`args|stdin|none`) in `runx-parser`,
serde kebab-rename; `validate_source` plus its helpers (`validate_mcp_server`,
`validate_catalog_ref`, `validate_mcp_tool`, `validate_a2a_url`,
`validate_agent`, `validate_task`, `validate_hook`,
`validate_agent_command_boundary`, `validate_agent_step_outputs`) thread the
typed kind; expose `SkillSource.kind`; fail-closed parse error for unknown
type. Decide `HarnessReceiptExpectation.source_type` (skill.rs:269) in/out
here. Acceptance: per-variant round-trip + unknown-type fail-closed; `cargo
test -p runx-parser`.

Phase 4b — Runtime call-site sweep (A). Re-point the ~22 runtime files that
read `source_type` (`skill_run`, `journal`, `registry/local/build`,
`tool_catalogs/*`, `adapters/*`, `external_adapter`, `agent_invocation`, …)
onto the typed kind; `Unsupported*` guards retained. Acceptance: full
workspace suite green.

## Acceptance

- [ ] No stringly source/catalog/sandbox discriminant is re-matched after parse;
  each is a typed enum parsed once.
- [ ] Every discriminant enum serde-renames to the existing wire strings;
  skill/catalog/sandbox fixtures deserialize unchanged (no fixture edits).
- [ ] `SourceKind` includes `ExternalAdapter`; the extension lane is preserved.
- [ ] No new `RunxRef` type; reference construction/parsing lives on
  `Reference`/`ReferenceType`; all four runtime `reference()` builders and the
  `runx-receipts` `reference_type_name` collapsed.
- [ ] `cargo fmt --check`, workspace `cargo clippy --all-features -- -D
  warnings`, `node scripts/check-rust-core-style.mjs` (no new findings), and
  `cargo test --workspace --all-features` all green.

## Validation Commands

```sh
cargo build --manifest-path crates/Cargo.toml --workspace --all-features
cargo clippy --manifest-path crates/Cargo.toml --workspace --all-targets --all-features -- -D warnings
cargo fmt --manifest-path crates/Cargo.toml --all --check
cargo test --manifest-path crates/Cargo.toml --workspace --all-features
node scripts/check-rust-core-style.mjs
```

## Rollback

Each phase is independent and additive-internal. The enums serde-rename to the
existing wire strings, so rollback is reverting that phase's edits with no
fixture or wire change. If a downstream consumer needs the raw string, expose it
through the enum's `as_str()`/serde, do not revert to `String`. No migration
shims (greenfield).

## Done (executed)

- **CLI FilterMode** — `ListPlan { ok_only: bool, invalid_only: bool }` had a
  representable-but-invalid `(true, true)` state guarded only by a runtime
  check. Replaced with `FilterMode { All, OkOnly, InvalidOnly }` so the invalid
  state is unrepresentable (parse-don't-validate). `runx-cli/src/launcher.rs`,
  `list.rs`, `tests/launcher.rs`. Tests green.
- **L3: `Caller` → `Host`** — the host-callback trait (`report`/`resolve`) now
  uses the core-model word. Renamed trait `Caller` → `Host`, `NoopCaller` →
  `NoopHost`, file `caller.rs` → `host.rs`, and the `caller` parameter/local
  vars → `host` across the 8 runtime files + 3 test files. The harness-fixture
  JSON field literally named `caller` was deliberately left untouched (it is a
  wire-contract field, not the trait). Zero external consumers. clippy + fmt +
  affected tests green.
- **SDK pass-through fields** — investigated `SkillSearchResult.source_type`,
  `ConnectionSummary.status`/`provider` being `String`. Verdict: NOT a defect.
  The SDK is a thin subprocess bridge that mirrors CLI JSON output and never
  matches these strings for control flow; typing them would duplicate
  vocabularies owned in other crates and make the bridge brittle to CLI
  additions. No change.
- **Local registry `split_skill_id` invariant** —
  `runx-runtime/src/registry/local.rs` no longer uses `unwrap_or_default()` to
  mask missing owner/name segments before validation. It now matches the two
  required non-empty segments explicitly and rejects extra segments before path
  safety checks.
- **Runtime time helpers** — removed the duplicate `now_iso8601`,
  `civil_from_unix_seconds`, and `civil_from_days` implementations from
  `runtime/scaffold/ids.rs` and `runtime/registry/local/util.rs`. Both now use
  one private `runtime/time.rs` helper.
- **L1: runtime module-file style** — converted the 10 legacy `mod.rs` files in
  `runx-runtime` (`adapters`, `adapters/mcp`, `connect`, `dev`, `execution`,
  `execution/harness`, `receipts`, `registry`, `scaffold`, `tool_catalogs`) to
  modern `foo.rs` siblings via `git mv` (100% rename, no content change). Zero
  `mod.rs` remain in the crate; it now matches every other crate. clippy green.
- **D: default-timestamp constant** — the MCP server path invented its own
  `"2026-05-20T00:00:00Z"` (`adapters/mcp/server_skill.rs`) that diverged from
  the runner default `"2026-05-18T00:00:00Z"` (`execution/runner.rs`,
  `execution/skill_run.rs`) — a latent determinism bug. All four sites now read
  one `pub(crate) const DEFAULT_CREATED_AT` in `runtime/time.rs`. clippy green.
- **C: sha256 helpers (core/receipts/runtime)** — added the canonical
  `hex_lower` / `sha256_hex` / `sha256_prefixed` (all `&[u8]`) to
  `runx-contracts::fingerprint` and re-exported them. Removed the duplicate
  private definitions from `core/policy/authority_proof.rs`,
  `receipts/canonical.rs`, and the runtime copies (`approval.rs`,
  `adapters/mcp/adapter.rs`, `adapters/a2a.rs`, `adapters/catalog.rs`,
  `doctor.rs`, `tool_catalogs/hash.rs`, `registry/install.rs`,
  `registry/local/util.rs`); callers now use the shared API (`&str` sites pass
  `.as_bytes()`). The 3 contracts-internal copies (`target_runner/plan.rs`,
  `post_merge_observer/plan.rs`, `act_assignment/hash.rs`) are deferred per the
  spec's "fold-in-after" sequencing — they sit in actively-reshaped files.
  contracts/core/receipts clippy green; runtime edits verified clean (the only
  runtime build errors are unrelated in-flight payment-ledger work).
- **External adapter + MCP staged proof style cleanup** — the 2026-05-22
  execution slices removed non-payment `check-rust-core-style` findings by
  making external-adapter frame dispatch typed, avoiding public
  `serde_json::Value` in the staged rmcp server bridge, and documenting the
  large external-adapter process-supervisor file. Focused external-adapter and
  MCP tests pass.

## Tier 1 — correctness / drift risk

### A. Stringly discriminants (parse-time validated, stored as `String`, re-matched)

- **[blocked-until: source_type→SourceKind]** `SkillSource.source_type`
  (`runx-parser/src/skill.rs:55`) and `.input_mode`
  (`skill.rs:64`, matched at `runx-runtime/src/adapters/cli_tool.rs:71`).
  → `SourceKind` / `InputMode` enums, `#[serde(rename_all = "kebab-case")]`,
  parsed once. Dissolves the `RuntimeError::Unsupported*` variants (Class K).
- `CatalogMetadata.kind` / `.audience` / `.visibility`
  (`runx-parser/src/skill.rs:531-549`) — validated against `skill|graph`,
  `public|builder|operator`, `public|private`, stored as `String`.
  → `CatalogKind` / `CatalogAudience` / `CatalogVisibility` enums.
  *Same-file risk with source_type work; sequence after it.*
- `SkillSandbox.profile` / `.cwd_policy` (`runx-parser/src/skill.rs:104,106`).
  → reuse `runx_core::policy::sandbox::SandboxProfile` (already exists; do not
  duplicate) and a new `CwdPolicy` enum.
- **[blocked-until: RunxRef]** `journal.rs:901` `event.kind == "run_started"`;
  string status assignments `journal.rs:768,923,932,946,952`
  (`"paused"`/`"valid"`/`"invalid"`). → `JournalEventKind` / `JournalStatus`.
  *journal.rs also builds `runx:` URIs, so the RunxRef workstream will be in
  this file; coordinate.*

### B. Magic-string / label semantics

- **[blocked-until: payment→authority]** payment scope `"payment:spend"` +
  label `"payment rail proof"`. Already being generalized to `AuthorityVerb`
  dispatch via `admit_step_authority`.
- **[blocked-until: post_merge reshape]** `post_merge_observer/plan.rs:191`
  `criterion.criterion_id == "post_merge.close_policy_authorized"` and the
  whole `post_merge.*` criterion-id family. → `PostMergeCriterionKind` enum
  with serde rename. *`post_merge_observer/plan.rs` is under active reshape.*

### D. Hardcoded magic constants (latent bug)

- **[done]** Two different fake "default" timestamps unified into one
  `pub(crate) const DEFAULT_CREATED_AT` in `runtime/time.rs`. The MCP server
  path's divergent `"2026-05-20T00:00:00Z"` (a latent determinism bug) is gone;
  `server_skill.rs`, `execution/runner.rs`, and `execution/skill_run.rs` all
  reference the shared const. (The MCP path constructs its own `RuntimeOptions`,
  so there is no upstream `created_at` to inject — one canonical const is the
  correct fix.)

### E. The `runx:` URI scheme is hand-built and hand-parsed

- **[blocked-until: RunxRef]** `format!("runx:act:{}")`,
  `format!("runx:decision:{}")`, `format!("runx:harness_receipt:{}")`,
  `strip_prefix("runx:harness_receipt:")`, `HARNESS_RECEIPT_REF_PREFIX` const —
  scattered across `journal.rs`, `receipts/seal.rs`, `receipts/tree.rs`,
  `post_merge_observer.rs`, `execution/target_runner.rs`.
  → one `RunxRef::{Act,Decision,HarnessReceipt,AgentAct}(id)` with `Display` +
  `FromStr`. *All these files are in the active reference/spine workstream.*

### H. `unwrap_or` masking invariants (not benign defaults)

- **[done]** `registry/local.rs:452` — `split_skill_id` uses explicit segment
  matching instead of `.unwrap_or_default()` plus `is_empty()`.
- **[blocked-until: RunxRef]** `journal.rs:988,995` —
  `value.as_str().unwrap_or("unknown")` hides a serialization failure behind
  the string `"unknown"`. → propagate the error.

## Tier 2 — mechanical, high-leverage, low-risk (pure deletion / dedup)

### C. Duplicated primitive helpers

- **[done for core/receipts/runtime]** `sha256_hex` / `sha256_prefixed` /
  `hex_lower` — canonical impls now live in `runx-contracts::fingerprint`
  (re-exported from the crate root). All core/receipts/runtime copies removed
  and repointed at the shared API. **Remaining (deferred):** the 3
  contracts-internal copies in `act_assignment/hash.rs`,
  `post_merge_observer/plan.rs`, `target_runner/plan.rs` — fold these in once
  their reshape settles (they are byte-equivalent `&str` variants; replace with
  `runx_contracts::sha256_*(x.as_bytes())`).
- `reference_type_name()` — **byte-identical** in `runtime/receipts/seal.rs:617`
  and `runtime/execution/target_runner.rs:901`. → one shared fn, or a
  `ReferenceType::as_str()` in contracts. *Both files active (RunxRef); after.*
- **[done]** `now_iso8601` + `civil_from_unix_seconds` + `civil_from_days` —
  consolidated into one private `runtime/time.rs` module.
- `reference()` URI builder duplicated `seal.rs:599` + `target_runner.rs:890`
  (pairs with `reference_type_name`).

### F. Boolean blindness (3+ adjacent bools)

- `OperationalPolicyAutomationPermissions { auto_merge, mutate_target_repo,
  require_human_merge_gate }` (`contracts/operational_policy.rs`). The validator
  already forces `auto_merge == false` and `require_human_merge_gate == true`,
  so two of three bools are effectively constants — the type is wrong.
  → `AutomationLevel` enum. *Contract spine active; coordinate.*
- `OperationalPolicyPostMergePolicy` bools — same file.
- **[done]** CLI `ListPlan` → `FilterMode` (above).
- CLI `InitPlan { global, prefetch_official, json }` — `runx-cli/launcher.rs`.
  → keep `json` separate, group the rest if a third mode appears. Low urgency.

### K. Domain-specific error variants

- **[blocked-until: source_type→SourceKind]** `RuntimeError::Unsupported{Adapter,
  RunStep,RunnerSelection} { ...: String }` (`error.rs`). Collapses once
  `SourceKind` parsing rejects unknown types at the boundary.

## Tier 3 — ergonomics / polish

### G. CLI dispatch + parse duplication

- 14× copy-pasted `if first_arg_is(&args, "...")` branches (`launcher.rs`).
  → static dispatch table. *Note: launcher.rs is the cutover parity oracle
  ("centralized for CLI routing tests"); keep all `tests/launcher.rs` +
  `fixtures/cli-parity` green. Defer until after the launcher flip soak to
  avoid perturbing the oracle.*
- `parse_*_plan` repeat the same flag-loop skeleton — shared helper.
- `Err(format!("unknown {cmd} flag {flag}"))` ×20 → `unknown_flag(cmd, flag)`.
- flag literals `"--json"` etc ×100 → `const FLAG_JSON`. The `truthy()` matcher
  (`launcher.rs:357`) mixes magic `"official"` into a boolean parser — document
  or split.

### I. Clone-driven design (runtime: 574 `.clone()`)

- `execution/target_runner.rs` `revision_details()` (~542-620) and
  `target_repo_runner_revision_receipt()` (~364-451) — 10+ clones each building
  static metadata in branch arms. → build references once / `Cow<Reference>`.
  *target_runner.rs active.*
- `registry/local/trust.rs` — highest clone density; audit trust-signal
  builders for clone-in-loop.

### J. Primitive obsession (35 `String` id fields)

- `step_id`, `run_id`, `receipt_id`, `act_id`, `policy_id`, `decision_id`,
  `term_id` threaded untyped across contracts + runtime; the compiler can't
  tell a `term_id` from a `step_id`. → newtypes for the 4-5 highest-traffic ids
  (`ReceiptId`, `StepId`, `ActId`, `RunId`). High churn; do last, as one pass,
  when the spine is stable.

## Class L — Naming & module structure (A+ infra naming vs the core model)

Audit of module names, filenames, and the layering they imply, judged against
the runx core vocabulary (act / authority / harness / decision / signal /
receipt / host / skill / graph / scope / sandbox). The crate layer itself is
clean (`contracts` → `core` → `parser`/`receipts` → `runtime` → `cli`/`sdk`);
findings are below that line.

- **L1. Module-file style is inconsistent across the workspace (mechanical,
  highest-leverage).** `runx-runtime` is the *only* crate using legacy
  `mod.rs` — 10 of them: `adapters/mod.rs`, `adapters/mcp/mod.rs`,
  `connect/mod.rs`, `dev/mod.rs`, `execution/mod.rs`,
  `execution/harness/mod.rs`, `receipts/mod.rs`, `registry/mod.rs`,
  `scaffold/mod.rs`, `tool_catalogs/mod.rs`. Every other crate uses the modern
  `foo.rs + foo/` style (`contracts/operational_policy.rs + operational_policy/`,
  `core/policy.rs + policy/`, etc.). → convert the runtime's 10 `mod.rs` to
  `foo.rs` siblings. Matches the workspace convention and the established
  preference (modern style was chosen when the contracts decomposition was
  corrected). Pure mechanical move; no logic change.

- **L2. `execution/orchestrator.rs` vs `execution/runner.rs` — synonym names
  hide a real layering.** `orchestrator.rs` is the canonical entrypoint facade
  (`LocalOrchestrator`, `RunRequest`/`RunResult`/`RunStatus`); `runner.rs` is
  the graph engine (`Runtime`, `GraphRun`, the step state machine). The names
  are near-synonyms, so a reader can't tell which is the front door. → rename
  for legibility, e.g. `runner.rs` → `engine.rs` (keep `orchestrator` as the
  entrypoint), or `orchestrator.rs` → `entry.rs`. Pick the pair that reads as
  facade-over-engine.

- **[done] L3. `host.rs::Host` now matches the core model.** The runtime host
  callback interface (`report(event)`, `resolve(request)`) now uses the same
  vocabulary as the contracts crate (`host_protocol`, `HostRunResult`,
  `HostRunState`, `ResolutionRequest`). The harness-fixture JSON field named
  `caller` remains a wire-contract field and is not part of this Rust API
  surface.

- **L4. `adapter.rs` (the `SkillAdapter` trait + invocation types) vs
  `adapters/` (the implementations) — distinguished only by singular/plural.**
  Fragile: a one-character difference carries the contract-vs-impl boundary.
  → after L1, fold the trait into `adapters.rs` (the modern parent file), or
  rename `adapter.rs` → `skill_adapter.rs` to make the trait's identity
  explicit and break the singular/plural collision.

- **L5. `post_merge_observer.rs` is an orphan at the runtime top level.** It is
  a runtime-side projector/observer sitting among unrelated peers (`adapter`,
  `approval`, `host`, `config`, `doctor`). → cluster it under a named home
  (`observers/` or `signals/`), or move it into the closure flow under
  `execution/` if that is where it belongs. Structural tidiness, not a defect.

- **L6. Generic `types.rs` proliferation (7 instances): `core/policy/types.rs`,
  `core/state_machine/types.rs`, `parser/graph/types.rs`,
  `runtime/connect/types.rs`, `runtime/dev/types.rs`,
  `runtime/registry/types.rs`, `runtime/adapters/mcp/types.rs`** (plus
  `parser/graph/helpers.rs`, `registry/local/util.rs`). Defensible inside a
  multi-file feature module (the parent is just re-exports, no single "primary"
  file), but inconsistent with the contracts decision to fold types into the
  parent. → document one rule: "types live in the parent file unless the module
  has ≥3 sibling concern-files, in which case a `types.rs` sibling is allowed."
  Then apply it. Low priority; consistency rather than correctness.

- **L7. Verb-prefix near-synonyms in the pure-crate public surface (minor).**
  Validation-class operations split across `validate_*` (3), `lint_*` (1),
  `evaluate_*` (4); construction split across `build_*` (3), `create_*` (2),
  `derive_*` (3). → pick one verb per operation class (e.g. `validate` for
  fail-closed checks, `lint` for finding-collection, `build` for construction)
  and document it. Low priority polish.

- **L8. `runtime_http.rs` — generic name for the transitional curl-subprocess
  HTTP transport.** Acceptable as-is; it is replaced wholesale by
  `rust-async-http-layer`. Note only — rename naturally when that spec lands
  (`http.rs` once it is a real client, or delete).

Sequencing note: L1 (mod.rs → modern) should run *before* L4 (adapter fold) and
is independent of the Tier 1-3 logic work. L2/L3 are small, isolated renames.
L5-L8 are tidiness.

## Explicitly NOT findings (examined, kept)

- `Option<Vec<T>>` in `contracts/execution.rs` (`surface_refs`, `evidence_refs`)
  and `act_assignment.rs` (`scope_set`): mirror TypeScript
  `Type.Optional(Type.Array(...))` wire contracts where absent ≠ empty.
  Collapsing breaks cross-language parity. Verified against the TS schema.
- `ReferenceType::` / `AuthorityVerb` comparisons: already typed + exhaustive —
  the *correct* pattern the stringly ones should converge to.
- `serde_json::Value` in public APIs outside the active payment recovery lane:
  zero instances after the external-adapter/MCP cleanup. The remaining style
  finding is `payment_state.rs` and belongs to the concurrent
  payment→authority workstream.

## Sequencing

0. Active payment→authority lane must clear the remaining Rust style findings:
   `execution/runner/authority.rs` file/function size,
   `execution/runner/steps.rs` replay helper size, and `payment_state.rs`
   public `serde_json::Value`.
1. After payment→authority lands: confirm Class B(payment), K resolved.
2. After source_type→SourceKind lands: A(source_type/input_mode), then A
   (CatalogMetadata, SkillSandbox), then K collapse.
3. After RunxRef lands: E, A(journal), H(journal), C(reference_type_name).
4. Independent of the above (do anytime files are clean): C(time helpers — needs
   lib.rs settled), C(sha256 in core/receipts/runtime), F(operational_policy),
   D(timestamps), H(registry split_skill_id).
5. Post-cutover soak: G(CLI dispatch table), I(clone hotspots), J(newtypes).

## References

- Full scan findings: this spec's predecessor analysis in conversation.
- [`plans/rust-takeover.md`](../../../plans/rust-takeover.md) §9 (sunset order),
  §11 (outreach gating — assumes A+ baseline).
- [`oss/docs/rust-kernel-architecture.md`](../../docs/rust-kernel-architecture.md)
  §18 (Rust implementation quality bar).

## Current State

Status: draft
Current phase: none
Next: harden
Reason: hardening found draft contract issues
Blockers: check needs revision: path audit; check needs revision: command audit; check needs revision: scope/migration audit; check needs revision: acceptance timing audit; check needs revision: scope/migration audit; 4 approval-blocking issue(s)
Allowed follow-up command: `edit the draft, then run scafld harden rust-aplus-cleanup --provider <provider>`
Latest runner update: none
Review gate: not_started

## Harden Rounds

### round-1

Status: in_progress
Started: 2026-05-22T06:01:30Z
Ended: none

Checks:
- none

Issues:
- none

### round-2

Status: error
Started: 2026-05-22T06:09:02Z
Ended: 2026-05-22T06:09:02Z
Summary: provider error: provider failed: invalid harden dossier: empty provider output: ... es. Challenge the draft before approval: verify declared paths and commands exist or are intentionally future files, question scope and migration claims, test whether acceptance commands can run at the right phase, verify rollback/repair is credible, and explicitly ask whether the plan is a short-sighted bandaid, future bloat, or the right architectural move. Preserve full detail, but separate gate decisions from useful advice: record harden issues with severity, status, and blocks_approval. Use blocks_approval only when approval would be unsafe, incoherent, non-executable, or architecturally harmful. Advisory issues must still include grounded evidence but must not block the verdict. Call `submit_harden` exactly once with the final HardenDossier; do not emit final prose or JSON text. Return verdict `pass` when all required checks pass and there are no open issues with blocks_approval=true.

Checks:
- none

Issues:
- none

### round-3

Status: in_progress
Started: 2026-05-22T06:09:31Z
Ended: none

Checks:
- none

Issues:
- none

### round-4

Status: needs_revision
Started: 2026-05-22T06:11:55Z
Ended: 2026-05-22T06:11:55Z
Verdict: needs_revision
Provider: claude
Model: claude-opus-4-7
Output format: claude.mcp_submit_harden
Summary: Architectural correction in the spec (ultimate-shape pass) is solid and the closed-enum direction is right. Three concrete blockers remain: (1) the spec contradicts itself about whether the 3 contracts-internal sha256 wrapper folds are done or deferred while the wrappers still exist in code; (2) Phase 1 names only 2 of the 4 `fn reference()` copies that currently exist in the runtime, so its own acceptance criterion ("no duplicate `fn reference(`") is incomplete; (3) Phase 4's acceptance ("no stringly source discriminant re-matched after parse") implies a parser/runtime restructure touching 22 runtime files (112 `source_type` references) plus a fan-out through `validate_source`'s helper signatures, which is larger than "medium" risk admits and should be explicitly scoped. Additionally the cargo `--features cli-tool` invocation form does not match repo CI conventions and likely will not run as written against a virtual workspace with multiple `-p` packages. `CwdPolicy` already exists in `runx_core::policy::types` — Phase 3 should say "reuse", not "add", and the canonical re-export path is `runx_core::policy::SandboxProfile`/`CwdPolicy`, not `runx_core::policy::sandbox::SandboxProfile`. Direction is correct; resolve these before approval so the phases are executable as written.

Checks:
- path audit
  - Grounded in: code:crates/runx-core/src/policy.rs:32-46
  - Result: failed
  - Evidence: Spec cites `runx_core::policy::sandbox::SandboxProfile` (lines 79, 99-101, 214-216) but `policy::sandbox` only re-exports admission functions (`admit_sandbox, normalize_sandbox_declaration, sandbox_requires_approval`). `SandboxProfile`/`CwdPolicy` live in `policy::types` and are re-exported as `runx_core::policy::SandboxProfile`/`CwdPolicy`. Same for Phase 3's `add CwdPolicy` — `CwdPolicy` already exists at `crates/runx-core/src/policy/types.rs:544` (kebab-renamed `SkillDirectory|Workspace|Custom`); it should be reused, not added. SkillSource/SkillSandbox/CatalogMetadata paths in the parser (`runx-parser/src/skill.rs:55,64,104,106,531-549`) verified. `scripts/check-rust-core-style.mjs` verified.
- command audit
  - Grounded in: code:crates/runx-runtime/Cargo.toml:18-26 and .github/workflows/ci.yml:75-76
  - Result: failed
  - Evidence: `cli-tool` is a per-crate feature on `runx-runtime` only (Cargo.toml:21). Repo CI runs `cargo clippy --workspace --all-targets --all-features` and `cargo test --workspace --all-features` (ci.yml:75-76; crates/README.md:13; plans/rust-takeover.md:70). The spec's `cargo {build|clippy|test} --manifest-path crates/Cargo.toml --workspace --features cli-tool` deviates from the repo convention and Phase 1's `cargo clippy -p runx-contracts -p runx-runtime --all-targets --features cli-tool` selects two packages where only one has the feature, which cargo rejects in a virtual workspace. Either switch to `--all-features` (CI convention) or qualify as `--features runx-runtime/cli-tool` and drop it from `-p runx-contracts`.
- scope/migration audit
  - Grounded in: code:crates/runx-runtime/src — `source_type|source.kind|source.source_type` matches 112 across 22 files; crates/runx-parser/src/skill.rs:601-1097
  - Result: failed
  - Evidence: Phase 4 cites only `cli_tool.rs:71` as the runtime re-match site, but ripgrep shows 22 runtime files and 112 references to `source_type` (e.g. skill_run.rs:9, journal.rs:4, external_adapter.rs:8, registry/local/build.rs:3). `validate_source` itself dispatches `&str source_type` through 8+ helpers (`validate_mcp_server`, `validate_catalog_ref`, `validate_mcp_tool`, `validate_a2a_url`, `validate_agent`, `validate_task`, `validate_hook`, `validate_agent_command_boundary`, `validate_agent_step_outputs`). Acceptance criterion 'no stringly source/catalog/sandbox discriminant is re-matched after parse' demands changing these signatures and 22 downstream files. The medium-risk/medium-size classification under-describes this.
- acceptance timing audit
  - Grounded in: code:fn reference\b in runtime — 4 hits: credentials.rs:408, target_runner.rs:2485, external_adapter.rs:664, seal.rs:703
  - Result: failed
  - Evidence: Phase 1 names only 2 `reference()` sites (`receipts/seal.rs`, `execution/target_runner.rs`), but `fn reference(` exists in 4 runtime files. Phase 1 acceptance `cargo clippy ... -- -D warnings` will not catch the duplication (clippy doesn't flag it), and the textual criterion 'no duplicate `fn reference(`/`reference_type_name` in runtime' will fail if executed as a grep gate unless all 4 are collapsed (or the gate is explicitly scoped). Also `reference_type_name` no longer exists in `runx-runtime` (already removed via `ReferenceType::as_str()`), but a `reference_type_name` remains in `crates/runx-receipts/src/tree.rs:1116` — out of scope as written.
- rollback/repair audit
  - Grounded in: spec:rust-aplus-cleanup.md#rollback (lines 133-139)
  - Result: passed
  - Evidence: Rollback story is credible per phase: enums serde-rename to existing wire strings, so reverting a phase is a code-only revert with no fixture or wire change. Verified `SourceKind` variants `CliTool|Mcp|Catalog|A2a|AgentStep|HarnessHook|Graph|ExternalAdapter` with `rename_all = "kebab-case"` round-trip to today's wire literals (`crates/runx-parser/src/skill.rs:610-1085` and `crates/runx-runtime/src/adapters/external_adapter.rs:97`). Phase 4's larger blast radius (see scope/migration audit) makes 'just revert' practically heavier than the prose implies, but the rollback shape is sound.
- design challenge
  - Grounded in: spec:rust-aplus-cleanup.md#ultimate-shape-correction (lines 28-72)
  - Result: passed
  - Evidence: The 2026-05-22 ultimate-shape correction is the right architectural move: refusing a parallel `RunxRef` type because `Reference`/`ReferenceType` already owns the contract, closing `SourceKind` to 8 variants with `ExternalAdapter` as a first-class variant (extension lane honored without re-opening to `String`), keeping `RuntimeError::Unsupported*` as boundary guards because dispatch is not fully closed, and deferring `AutomationLevel` because the (true,true) invariant is a wire-contract reshape requiring TS parity. The split between this slice and `J`/`L2-L8`/`G`/`I` deferrals is well-reasoned. Not a bandaid; not premature bloat — it converges on the trusted-Rust-core end state.
- scope/migration audit
  - Grounded in: code:crates/runx-contracts/src/{act_assignment/hash.rs:85, target_runner/plan.rs:690, post_merge_observer/plan.rs:1070-1076}
  - Result: failed
  - Evidence: Spec is self-contradictory on the 3 contracts-internal sha256 wrappers. 'Executable scope §1' states '`ReferenceType::as_str()` + fold the 3 contracts-internal `sha256` copies into `fingerprint` (done)'. The Done(executed) section states they are 'deferred per the spec's fold-in-after sequencing'. Actual code: thin wrappers still exist at all 3 sites delegating to `crate::fingerprint::sha256_*(value.as_bytes())`. Phase 1's executable scope and its acceptance must agree on whether the inline fold happens in this slice or is explicitly deferred.

Issues:
- [high/blocks approval] `harden-1` spec_inconsistency - Phase 1 contradicts itself on the 3 contracts-internal sha256 wrappers
  - Status: open
  - Grounded in: code:crates/runx-contracts/src/act_assignment/hash.rs:85; target_runner/plan.rs:690; post_merge_observer/plan.rs:1070-1076 — and spec lines 75-77 vs 268-273
  - Evidence: Executable Scope §1 claims '(done)' for the 3 contracts-internal `sha256` folds; the Done(executed) section says they are deferred; the code still has the wrappers (thin `&str` shims around `crate::fingerprint::sha256_*`). Phase 1 acceptance and scope are therefore ambiguous about what ships.
  - Recommendation: Pick one: (a) move the 3 folds into Phase 1's scope, list the 3 file paths, and update acceptance to `rg -n 'fn sha256_(prefixed|hex)' crates/runx-contracts/src returns 0`; or (b) leave them deferred and remove the '(done)' clause from Executable Scope §1. Either is fine — the spec must not say both.
  - Question: Are the 3 contracts-internal sha256 wrapper folds in or out of this slice?
  - Recommended answer: Out — keep them deferred until the act_assignment / post_merge_observer / target_runner reshape settles (consistent with Sequencing §3 and the Done section). Remove the '(done)' parenthetical from Executable Scope §1.
  - If unanswered: Treat as deferred and strike the '(done)' from Executable Scope §1.
- [high/blocks approval] `harden-2` scope_undercount - Phase 1 names only 2 of 4 duplicate `fn reference()` builders in the runtime
  - Status: open
  - Grounded in: code:`rg -nP '^\s*fn reference\b' crates/runx-runtime/src` returns 4 hits: credentials.rs:408, target_runner.rs:2485, external_adapter.rs:664, seal.rs:703
  - Evidence: Spec lists `seal.rs:599 + target_runner.rs:890` as the duplicates; ripgrep shows the helper also exists in `credentials.rs:408` and `adapters/external_adapter.rs:664`. The Phase 1 acceptance text 'no duplicate `fn reference(`/`reference_type_name` in runtime' is satisfiable only by collapsing all 4 (line numbers in the spec are also stale; the seal copy is at 703, the target_runner copy at 2485).
  - Recommendation: Update Phase 1's scope to list all 4 sites and either (a) collapse all to a single `Reference::for_type(ReferenceType, id_or_uri)` constructor in `runx-contracts::reference`, or (b) carve `credentials.rs` and `external_adapter.rs` out explicitly with a stated reason. Refresh the cited line numbers, or drop them (the spec already drifts).
  - Question: Should Phase 1 collapse all 4 `fn reference()` copies into one `Reference` constructor in contracts, or leave credentials.rs and external_adapter.rs for a later pass?
  - Recommended answer: Collapse all 4 in this slice — they are byte-identical and the constructor belongs in contracts next to `ReferenceType::as_str()`. Replace `fn reference(` in runtime with `Reference::for_type(...)`. Drop the literal line-number citations to avoid drift.
  - If unanswered: Default to collapsing all 4; the slice is small and the gate is otherwise vacuous.
- [high/blocks approval] `harden-3` scope_undercount - Phase 4's acceptance demands a runtime/parser restructure far larger than 'medium'
  - Status: open
  - Grounded in: code:`rg 'source_type|source\.kind|source\.source_type' crates/runx-runtime/src` returns 112 matches across 22 files; crates/runx-parser/src/skill.rs:601-1097
  - Evidence: Phase 4 cites `cli_tool.rs:71` as the runtime re-match site. Actual: 22 runtime files reference `source_type` (skill_run.rs, journal.rs, external_adapter.rs, agent_invocation.rs, registry/local/build.rs, tool_catalogs/*, adapters/*, etc.). Inside `validate_source` the parser threads `&str source_type` through 8+ helpers (`validate_mcp_server`, `validate_catalog_ref`, `validate_mcp_tool`, `validate_a2a_url`, `validate_agent`, `validate_task`, `validate_hook`, `validate_agent_command_boundary`, `validate_agent_step_outputs`). Acceptance 'no stringly source/catalog/sandbox discriminant is re-matched after parse' requires changing all of these signatures plus the 22 runtime call sites in one slice.
  - Recommendation: Either (a) split Phase 4 into 4a (parser: `SourceKind`/`InputMode` types + `validate_source` and its helpers; expose typed `SkillSource.kind`) and 4b (runtime call-site sweep across 22 files, including journal/registry/tool_catalogs), with separate acceptance commands per phase; or (b) bump size/risk to large/high and document the file count up front. Also: `HarnessReceiptExpectation.source_type: Option<String>` at `crates/runx-parser/src/skill.rs:269` is a sibling stringly field — call it in or out explicitly.
  - Question: Split Phase 4 into parser-typing + runtime-sweep slices, or keep as one slice and re-label size=large/risk=high?
  - Recommended answer: Split into 4a (parser surface: `SourceKind`/`InputMode` + `validate_source` plumbed through helpers; expose `SkillSource.kind`) and 4b (runtime/journal/registry/tool_catalogs call-site sweep across the 22 files). Decide `HarnessReceiptExpectation.source_type` once 4a lands — natural follow-on.
  - If unanswered: Re-label size=large, risk=high, and add a 'files touched' enumeration to Phase 4.
- [medium/blocks approval] `harden-4` command_correctness - Validation commands use `--features cli-tool` against a virtual workspace; repo convention is `--all-features`
  - Status: open
  - Grounded in: code:crates/runx-runtime/Cargo.toml:18-26 and .github/workflows/ci.yml:75-76
  - Evidence: `cli-tool` is a runx-runtime-only feature. CI uses `cargo {clippy,test} --workspace --all-features` (ci.yml:75-76; crates/README.md:13; plans/rust-takeover.md:70). The spec's `cargo clippy --manifest-path crates/Cargo.toml --workspace --all-targets --features cli-tool -- -D warnings` and the Phase 1 acceptance `cargo clippy -p runx-contracts -p runx-runtime --all-targets --features cli-tool -- -D warnings` will fail in a virtual workspace context: cargo errors when `--features X` selects a package that doesn't declare X (here, runx-contracts).
  - Recommendation: Match the repo convention: replace `--features cli-tool` with `--all-features` in both the global Validation Commands and the Phase 1 acceptance. If a smaller surface is desired, scope to `-p runx-runtime --features cli-tool` instead. Acceptance for parser-only work can stay `cargo test -p runx-parser` (no `--features`).
  - Question: Switch to `--all-features` (matching CI) or qualify as `-p runx-runtime --features cli-tool`?
  - Recommended answer: `--all-features` for the global Validation Commands (parity with CI); keep `-p runx-parser` (no feature flag) for Phase 2/3 acceptance; drop the `--features cli-tool` clause from Phase 1's clippy line because `runx-contracts` doesn't have it.
  - If unanswered: Default to `--all-features` everywhere.
- [medium/advisory] `harden-5` stale_claim - Phase 3 says 'add `CwdPolicy`' but it already exists and is re-exported
  - Status: open
  - Grounded in: code:crates/runx-core/src/policy/types.rs:544 and crates/runx-core/src/policy.rs:33-46
  - Evidence: `CwdPolicy` (variants `SkillDirectory|Workspace|Custom`, kebab-renamed) exists at `crates/runx-core/src/policy/types.rs:544` and is re-exported at `crates/runx-core/src/policy.rs:33-46` as `runx_core::policy::CwdPolicy`. Similarly `SandboxProfile` is re-exported as `runx_core::policy::SandboxProfile` — the spec's cited path `runx_core::policy::sandbox::SandboxProfile` (lines 79, 99-101, 214-216) refers to a sibling module that only exposes admission functions.
  - Recommendation: Update Phase 3 / Tier-1A wording from 'add `CwdPolicy`' to 'reuse `runx_core::policy::CwdPolicy`', and change every `policy::sandbox::SandboxProfile` reference to `policy::SandboxProfile` (or `policy::types::SandboxProfile` if the source path is preferred). Low impact; tightens the spec.
  - If unanswered: Just fix in-place when applying the slice; not a gate.
- [low/advisory] `harden-6` missing_artifact - Phase 1 mentions `reference_type_name` but the runtime copies are already gone; one remains in `runx-receipts`
  - Status: open
  - Grounded in: code:`rg 'reference_type_name' crates` returns only crates/runx-receipts/src/tree.rs:1106,1116
  - Evidence: Phase 1 acceptance says 'no duplicate `fn reference(`/`reference_type_name` in runtime'. The runtime copies were already removed (the `ReferenceType::as_str()` work landed). A `reference_type_name` helper still exists at `crates/runx-receipts/src/tree.rs:1116` (used at line 1106). The spec doesn't say whether to touch it.
  - Recommendation: Either (a) include the receipts copy in Phase 1's scope (one-line edit: replace with `reference_type.as_str()`), or (b) drop the `reference_type_name` clause from the Phase 1 acceptance text since it is already satisfied in `runx-runtime`.
  - Question: Pull the `runx-receipts` `reference_type_name` into Phase 1 or leave it alone?
  - Recommended answer: Pull it in — it's a 2-line change and finishes the dedup; otherwise the receipts crate becomes the only surviving copy.
  - If unanswered: Strike the `reference_type_name` clause from Phase 1 acceptance to avoid a phantom acceptance item.


