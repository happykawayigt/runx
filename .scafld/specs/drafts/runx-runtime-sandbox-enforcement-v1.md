---
spec_version: '2.0'
task_id: runx-runtime-sandbox-enforcement-v1
created: '2026-05-22T12:00:00+10:00'
updated: '2026-05-22T12:00:00+10:00'
status: draft
harden_status: not_run
size: small
risk_level: high
---

# runx runtime sandbox enforcement v1

## Current State

Status: draft
Current phase: planning
Next: harden
Reason: Ratify the R1 posture before any code or documentation claims stronger
runtime sandbox confinement than the OSS runtime currently provides.
Blockers: none for ratification; actual OS sandbox enforcement remains a
separate implementation problem.
Allowed follow-up command: `scafld harden runx-runtime-sandbox-enforcement-v1`
Latest runner update: 2026-05-22T12:00:00+10:00 drafted as a docs/spec-only
R1 non-enforcement ratification.
Review gate: not_started

## Summary

R1 is ratified as a current non-enforcement finding: the OSS `runx-runtime`
sandbox is a declaration, admission, cwd/env shaping, and receipt-metadata
surface. It is not an OS confinement boundary for filesystem writes, network
egress, process trees, resource limits, or private temp/state.

The active hardening spec already records this as
`R1 [Critical] - sandbox is advisory, not enforced`. This draft exists to make
the ratification durable and to prevent optimistic docs or future specs from
treating sandbox profiles as enforced before an implementation lands with tests.

## Context

Current evidence in this checkout:

- `.scafld/specs/active/runx-security-hardening-v1.md` says R1 is advisory, not
  enforced, and requires real OS enforcement before any untrusted-skill story.
- `crates/runx-runtime/src/sandbox.rs` rejects `require_enforcement: true` with
  a sandbox violation because platform isolation helpers are not available.
- Runtime sandbox metadata reports filesystem and network enforcement as
  `not-enforced-local` and the runtime enforcer as `declared-policy-only`.
- The runtime still performs useful policy-adjacent checks: sandbox profile
  validation, unrestricted approval gating, cwd boundary checks, env allowlist
  shaping, input env collision rejection, and receipt metadata emission. These
  checks must not be described as filesystem or network confinement.

Terminology:

- **Admission** means deciding whether a declared sandbox shape is allowed to
  run at all.
- **Shaping** means selecting cwd, environment variables, input delivery, and
  receipt metadata.
- **Enforcement** means the runtime uses OS or equivalent primitives to confine
  filesystem access, network access, subprocesses, resource usage, and temp
  state even if the child process is malicious.

Only admission and shaping are currently present.

## Objectives

- Ratify that R1 is currently non-enforcing in OSS.
- Keep `sandbox.require_enforcement: true` fail-closed until real OS confinement
  exists for the requested platform/profile.
- Prevent docs/specs from implying bubblewrap, Landlock, seccomp, macOS
  sandbox-exec, namespaces, cgroups, setrlimit, or process-tree confinement is
  active before implementation evidence exists.
- Preserve current admission and shaping behavior while naming it accurately as
  `declared-policy-only`.

## Scope

In scope:

- Docs/spec ratification of the current sandbox posture.
- Future doc cleanup that replaces "sandbox enforcement" shorthand with
  "sandbox ownership" or "declared-policy-only sandbox metadata" unless the text
  is explicitly describing a future implementation.
- A later implementation plan may use this draft as the handoff point for real
  enforcement, but implementation is not part of this ratification.

Out of scope:

- Rust runtime code changes.
- Adding bubblewrap, Landlock, seccomp, sandbox-exec, cgroups, setrlimit,
  chroot, containers, or process-group semantics.
- Changing skill ABI, receipt schema, authority proof schema, or parser policy
  vocabulary.
- Treating TypeScript `runtime-local` sandbox behavior as a fallback or as proof
  of Rust runtime enforcement.

## Dependencies

- `runx-security-hardening-v1` owns the broader security backlog and records R1
  as critical.
- `skill-author-runtime-contract-v1` owns the author-visible subprocess ABI, not
  OS confinement.
- `rust-ts-sunset-runtime-local` owns TypeScript runtime-local deletion and must
  not reintroduce a TypeScript enforcement fallback.

## Assumptions

- The current OSS runtime may continue to run trusted local development skills
  with declared-policy-only sandbox metadata.
- Any untrusted-skill, production-payment, or secret-bearing story must either
  require a real enforcement profile or explicitly document why sandbox
  non-enforcement is acceptable for that flow.

## Touchpoints

- `.scafld/specs/active/runx-security-hardening-v1.md`
- `README.md`
- `docs/rust-kernel-architecture.md`
- `docs/ts-interop-boundary.md`
- `docs/skill-author-runtime-contract.md`
- `crates/runx-runtime/src/sandbox.rs` (read-only evidence for this ratification)

## Risks

- High: Documentation that says "sandbox enforcement" can be read as a security
  guarantee even when the runtime only performs admission and shaping.
- High: `readonly`, `network: false`, and `writable_paths` names can imply
  confinement. Mitigation: public docs must qualify them as declarations until
  an OS enforcer is active and attested.
- Medium: Future Linux-only enforcement could leave macOS or Windows in
  non-enforcing mode. Mitigation: acceptance must be platform/profile-specific.

## Acceptance

Profile: strict

Definition of done:

- [ ] `dod1` R1 remains documented as non-enforcing until OS confinement exists.
- [ ] `dod2` Docs/specs do not claim current bubblewrap, Landlock, seccomp,
  sandbox-exec, namespace, cgroup, setrlimit, or process-tree enforcement unless
  backed by runtime implementation and tests.
- [ ] `dod3` `sandbox.require_enforcement: true` remains fail-closed when no
  matching platform enforcer is available.
- [ ] `dod4` No Rust runtime source is changed by this ratification.

Validation:

- [ ] `v1` command - This spec validates.
  - Command: `scafld validate runx-runtime-sandbox-enforcement-v1 --json`
  - Expected kind: `exit_code_zero`
  - Status: pending
- [ ] `v2` command - R1 non-enforcement evidence remains present.
  - Command: `rg -n "sandbox is advisory|not-enforced-local|declared-policy-only|require_enforcement: true" .scafld/specs/active/runx-security-hardening-v1.md crates/runx-runtime/src/sandbox.rs`
  - Expected kind: `exit_code_zero`
  - Status: pending
- [ ] `v3` command - Ratification changes only this draft spec.
  - Command: `git diff --name-only -- .scafld/specs/drafts/runx-runtime-sandbox-enforcement-v1.md`
  - Expected kind: `exit_code_zero`
  - Status: pending

## Phase 1: Ratify Current Posture

Status: pending
Dependencies: none

Objective: Record the current R1 posture as non-enforcing without changing
runtime code.

Changes:

- `.scafld/specs/drafts/runx-runtime-sandbox-enforcement-v1.md` - Add this
  ratification draft.

Acceptance:

- [ ] `ac1_1` command - The draft spec exists at the allowed path.
  - Command: `test -f .scafld/specs/drafts/runx-runtime-sandbox-enforcement-v1.md`
  - Expected kind: `exit_code_zero`
  - Status: pending

## Phase 2: Documentation Cleanup

Status: pending
Dependencies: Phase 1

Objective: Align public docs to the ratified wording.

Changes:

- Replace current-tense "sandbox enforcement" claims with ownership or
  declared-policy-only wording unless they are explicitly future-tense.
- Preserve the distinction between admission/shaping and actual OS confinement.

Acceptance:

- [ ] `ac2_1` command - Current-tense docs do not imply active OS sandboxing.
  - Command: `rg -n "bubblewrap|bwrap|sandbox enforcement|OS-level sandbox enforcement" README.md docs .scafld/specs --glob '*.md'`
  - Expected kind: `exit_code_zero`
  - Status: pending

## Phase 3: Future Enforcement Implementation

Status: pending
Dependencies: Phase 2

Objective: Implement real profile-specific sandbox enforcement in a separate
code change if the product chooses to support untrusted or secret-bearing local
execution.

Changes:

- None in this ratification.

Acceptance:

- [ ] `ac3_1` Real enforcement is platform/profile-specific, tested, and
  reflected in receipt metadata before docs describe it as available.

## Rollback

Delete this draft spec. No runtime behavior changes are coupled to it.

## Review

Status: not_started
Verdict: none

Findings:

- none

## Self Eval

- none

## Deviations

- none

## Origin

Created from the 2026-05-22 request to inspect and, if needed, minimally update
docs/spec for R1 sandbox non-enforcement ratification without touching Rust
runtime code.
