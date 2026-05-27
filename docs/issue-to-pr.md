# Issue To PR Flow

`issue-to-pr` is the generic runx lane for turning one bounded source thread
into one governed draft pull request. It is not a Slack bot, Sentry handler, or
repo-specific triage policy.

The lane exists to make the engineering story reviewable:

1. Intake source: the original issue, chat thread, alert, or local harness_context.
2. Triage decision: why a PR is justified, or why the lane should stop.
3. scafld spec: the code-change contract and declared validation.
4. Build evidence: what changed and which checks ran.
5. Review result: adversarial findings and remaining risk.
6. Draft PR: linked, refreshed, and ready for a human reviewer.
7. Human merge gate: the generated PR is never auto-merged by this lane.
8. Final outcome: merged, closed, or superseded state posted back to the source
   thread when the provider can be observed.

## Ownership Boundary

runx owns reusable machinery:

- source-command normalization through `@runxhq/core/source`, including
  provider-neutral source locators, thread locators, dedupe keys, target repo
  hints, and chat-safe command responses
- normalized source threads
- provider-neutral evidence bundles
- lifecycle story helpers
- outbox entries and publication metadata
- receipt evidence
- scafld command boundaries
- provider adapters such as GitHub issue comments and pull requests
- idempotent update behavior for retries
- the `runx.operational_policy.v1` schema for sources, target repos, runners,
  owner routes, dedupe, closure/proof publication, source-thread publishing, and automation
  permissions
- sealed receipts for post-merge observation, verification, final
  source-thread update, and source-issue closure policy

Consuming repos own product policy:

- which Slack channels, Sentry alerts, GitHub issues, or support tools can start
  a lane
- how source messages are filtered to avoid non-issues
- which repo receives the work
- who is assigned for human review
- whether GitHub Projects, labels, or milestones are used
- deployment and live bot credentials

That split keeps `issue-to-pr` reusable. A service repo can normalize Slack or
Sentry into a `runx.thread.v1` source and a redacted
artifact refs and verification evidence, but runx core should not know that Nitrosend uses a
particular channel, label, Sentry project, or owner map.

For Slack/Sentry/GitHub command entrypoints, adapters should first call
`normalizeRunxSourceCommand` from `@runxhq/core/source`. The resulting
`source_event` feeds `issue-intake`, while `operationalPolicyRequest` feeds the
policy gate before target-runner or provider mutation work begins. A blocked or
unsupported response from this layer should be posted back to the originating
thread by the adapter; it is never permission to post a new root message or to
create a PR.

Before PR packaging, callers may pass a `runx.operational_policy.v1` packet plus
`source_id`, `target_repo`, `runner_id`, and source-thread locator. The
`outbox.build_pull_request` boundary calls `admitOperationalPolicyRequest`
before it emits a draft PR packet. Denied policy admission stops packaging and
reports stable finding codes such as `unknown_target_repo`, `unknown_runner`,
`runner_unavailable`, `source_action_not_allowed`, or
`source_thread_locator_required`.

Allowed admission is projected into the draft PR packet and pull-request outbox
metadata as a redacted summary: policy id, source id, target repo, runner id,
owner route id, owner count, dedupe strategy, outcome close mode, and human
merge gate requirement. Raw source locators stay out of public PR and admin
readback surfaces.

## Reviewer Context

The source issue and PR should be comprehensive without becoming an event log.
Use durable gate summaries, not every internal transition. The canonical lane
publishes the draft PR, then updates the source thread with a merge-gate story
and, when observed later, a stable outcome story so reviewers do not have to
reconstruct state from receipts.

Good public story sections include:

- source summary and relevant evidence
- hydration status when adapter context was needed
- triage decision and why build is justified
- scoped files or surfaces
- validation commands and results
- review verdict and actionable findings
- PR link and human merge instruction
- final merged or closed outcome

Do not publish:

- raw local absolute paths
- secret values or provider tokens
- full command dumps when a concise result is enough
- duplicate status comments for retry attempts
- provider-specific policy that belongs in the consuming repo

## Naming

The graph may still use low-level runner contracts internally. Human-facing
docs, labels, and comments should describe those boundaries as agent-mediated
authoring, review, or decision steps. Public runner and schema identifiers must
cut over cleanly with every call site updated in the same change.

## Security Shape

The lane fails closed when source context, scafld state, provider auth, branch,
or review evidence is missing. The generated PR remains draft/reviewable, and a
human controls the merge. Post-merge behavior is observation and source-thread
update, not automatic merge.

Source-thread publishing is part of the security boundary. Public milestone
entries carry `metadata.source_thread.required=true`,
`publish_mode=reply`, and `missing_behavior=fail_closed`. A publisher that
cannot recover the original thread must stop rather than posting a new root
message in a busy issue channel.

Retries must reuse the same outbox entry, issue comment, branch, and PR when
possible. Duplicates are a correctness bug because the source thread is the
control surface.

Pull-request outbox entries carry `metadata.dedupe.strategy`,
`metadata.dedupe.key`, and `metadata.dedupe.result` so provider pushers and
admin surfaces can tell whether a retry created or reused the PR path.

## PR Review And Fix-Up Lanes

`issue-to-pr` creates or refreshes the draft PR and publishes the merge-gate
story. Adjacent PR work should stay as separate lanes over the same harness_context
instead of being hidden inside merge authority:

- `pr-review` reads the source thread, evidence bundle, scafld state, PR diff,
  checks, and human review comments, then publishes one concise review packet.
- `pr-fix-up` may apply bounded changes only when the review packet names
  actionable findings and the source harness_context remains inside the approved
  scope.
- `merge-assist` observes merge readiness, checks, deployment, and verification
  contracts, then posts a final recommendation or outcome. It does not click
  merge.

Those lanes share the same thread/outbox/harness_context contracts:

- input: `runx.thread.v1`, optional artifact refs and verification evidence, the draft PR
  outbox entry, and current provider observations
- output: updated story milestone, review or fix-up receipt, and an idempotent
  source-thread or PR comment
- gate: human merge stays outside runx mutation authority

If a hosted operator wants auto-merge someday, that is a new policy surface with
separate repo allowlists, branch protections, checks, audit events, and
rollback contracts. It is not part of this lane.

## Live Operations Preflight

Use the live preflight before running against a real GitHub issue:

```bash
pnpm live:issue-to-pr -- --allow-repo owner/repo --repo owner/repo --issue 123 --workspace /path/to/repo
```

The preflight is read-only. It verifies that the workspace is a scafld repo,
that the target repo is explicitly allowlisted for proving-ground mutation,
that the workspace is on the intended issue branch, that the selected scafld
binary can run in that workspace, that `--runx-bin`, `RUNX_BIN`, a local
`crates/target/{debug,release}/runx`, or `runx` on `PATH` points at an
executable native CLI, and that provider publication has explicit token env
available to the sandbox. It returns JSON with blocked checks and the exact
dogfood command to run next.

Live create/observe requires an explicit proving-ground repo allowlist. Pass
`--allow-repo owner/repo` or set
`RUNX_LIVE_ISSUE_TO_PR_ALLOWED_REPOS=owner/repo`. Multiple repos may be
comma-separated, but keep the list intentionally small; this harness is for
known proving-ground repos, not arbitrary customer or product repositories.

The provider-push tool does not receive ambient `gh` keychain state. Export an
explicit `RUNX_GITHUB_TOKEN`, `GH_TOKEN`, or `GITHUB_TOKEN` for create mode and
terminal observe mode. For local dogfood, `RUNX_GITHUB_TOKEN="$(gh auth token)"`
is sufficient when the active GitHub CLI account has repo access.

`pnpm live:issue-to-pr` without a configured target is also read-only: it emits
`status: "skipped"` and names the missing `repo`, `issue`, and `workspace`
inputs. Configure those with flags or `RUNX_LIVE_ISSUE_TO_PR_REPO`,
`RUNX_LIVE_ISSUE_TO_PR_ISSUE`, `RUNX_LIVE_ISSUE_TO_PR_WORKSPACE`, and
`RUNX_LIVE_ISSUE_TO_PR_ALLOWED_REPOS`.

The harness modes are explicit:

- `preflight`: local validation only, no provider mutation.
- `create`: runs the governed lane and may create/update issue comments, branch,
  and PR.
- `observe`: reads the source issue and PR after a human merge/close; it does
  not mutate code, and when the PR is terminal it upserts one source-thread
  outcome comment.

If the workspace is clean and you want the live command to create or switch to
the issue branch before mutation, pass `--prepare-branch`:

```bash
pnpm live:issue-to-pr -- --prepare-branch --allow-repo owner/repo --repo owner/repo --issue 123 --workspace /path/to/repo
```

When the preflight is ready, run:

```bash
pnpm dogfood:github-issue-to-pr -- --mode create --prepare-branch --allow-repo owner/repo --repo owner/repo --issue 123 --workspace /path/to/repo
```

The dogfood command hydrates the GitHub issue thread, executes the governed
lane through native `runx skill skills/issue-to-pr`, and passes explicit
contracts for the hydrated thread, target workspace, branch, scafld binary,
allowlisted operational policy, repo snapshot, and receipt directory. If the
graph reaches an agent-mediated authoring boundary, create mode returns
`status: "needs_agent"` with a `run_id`, sanitized request payload, and a
continuation command. Resolve that request into an answers file, then resume
the same native run:

```bash
pnpm dogfood:github-issue-to-pr -- --mode create --run-id <run-id> --answers answers.json --allow-repo owner/repo --repo owner/repo --issue 123 --workspace /path/to/repo
```

When the graph seals, the provider push remains inside the existing
`thread.push_outbox` step. The command rehydrates the source thread and emits a
machine-readable dossier with source issue URL, PR URL, branch, run id, receipt
refs, source-thread publication refs, and the human merge gate without printing
absolute local paths.

After a human merges or closes the PR, observe the outcome:

```bash
pnpm dogfood:github-issue-to-pr -- --mode observe --allow-repo owner/repo --repo owner/repo --issue 123 --workspace /path/to/repo
```

Observe mode is intentionally narrow: it records `merged` or `closed` provider
state back to the source issue with the PR URL, branch, scafld task id, and the
human-gate statement. If the PR is still open, it returns
`dogfood_pr_open_human_gate_pending` and does not post another comment.
Terminal observe mode should seal provider state into a receipt before
publishing, so wrappers can validate the verification result, human gate, close
policy, and source-thread target from one proof-backed receipt projection.

## Fixtures

Use the checked-in thread fixtures when building repo-local wrappers:

- `fixtures/threads/issue-to-pr-file-thread.json` shows a local file-backed
  harness_context for deterministic tests.
- `fixtures/threads/issue-to-pr-github-thread.json` shows the normalized shape
  a GitHub issue adapter should produce.
- `fixtures/issue-to-pr/dogfood-answers.json` is an empty caller-answer file
  for dogfood commands that should fail closed before real provider context is
  supplied.

## Aster Live Handoff

Aster should consume this as a runx proving-ground lane, not as OSS policy.

Mapping:

- Aster `issue-triage` decides whether a public issue deserves reply, plan, or
  build.
- Aster `fix-pr` and `docs-pr` prepare repo-local policy: target repo, branch,
  authoring model, labels, and publication gate.
- The normalized source issue becomes the `thread` input for `issue-to-pr`.
- Hydrated provider context becomes artifact refs and verification evidence and should
  already be redacted by the adapter.
- `issue-to-pr` owns scafld lifecycle, draft PR packaging, receipts, and generic
  GitHub thread updates.
- Aster keeps the rolling work-issue status comment and generated-PR policy.

The live merge gate remains human. Aster may observe the merged PR and publish
the final source-thread outcome, but it should not merge generated changes.
