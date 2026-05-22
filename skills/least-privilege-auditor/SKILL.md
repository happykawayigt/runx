---
name: least-privilege-auditor
description: Compare the scopes a subject was granted against the scopes its receipts show it actually used, and propose the narrowest grant that still works.
runx:
  category: security
---

# Least Privilege Auditor

Turn granted authority plus observed usage into a bounded attenuation proposal.

runx keeps a receipt of every scope a run actually exercised. This skill reads
that proof. It compares what a subject (a skill, a grant, or a principal) was
granted against what its receipts show it used, then proposes the narrowest
grant that still covers real usage. The output is a reviewable attenuation
proposal, not an automatic change.

## What this skill does

1. **Diff granted against used.** For each granted scope, decide whether the
   usage evidence shows it was exercised, partially exercised (a narrower verb
   would suffice), or never used.
2. **Propose the narrowest grant.** For unused scopes, propose removal. For
   over-broad scopes (write granted, only reads observed), propose the narrower
   verb. Leave exercised scopes untouched.
3. **State residual risk.** Name what the attenuated grant can still do, and any
   scope kept despite thin evidence (and why).

## When to use this skill

- Periodic least-privilege review of a skill or principal before publish or
  renewal.
- After an incident, to tighten a grant that turned out broader than needed.
- Before promoting a skill toward a higher maturity tier, to prove its grant is
  minimal.

## When not to use this skill

- To grant new authority. This skill only narrows; widening is a human decision.
- When no usage evidence exists. Without receipts there is nothing to diff;
  return `needs_more_evidence` rather than guess a grant down to nothing.
- For secret material handling. Use `secret-leak-triage` for credential
  exposure, not scope review.

## Audit philosophy

Narrow, never widen. Remove only what the evidence says is unused, and downgrade
a verb only when every observed use fits the narrower one. A scope used once is
used. When evidence is thin but the scope is plausibly load-bearing, keep it and
say so. The proposal must be safe to apply blind by a reviewer who trusts the
diff.

## Quality Profile

- Purpose: produce one bounded, reviewable attenuation that preserves real usage.
- Audience: the maintainer or security reviewer deciding a grant's next shape.
- Artifact contract: a scope-by-scope diff, an attenuation proposal (remove or
  narrow), and a residual-risk statement.
- Evidence bar: tie every removal or downgrade to the usage summary. Never
  propose narrowing a scope the evidence shows was exercised.
- Voice bar: direct security review. No "tighten permissions generally" without
  a named scope and the evidence behind the call.
- Strategic bar: the narrowest grant that still covers observed usage, nothing
  narrower.
- Stop conditions: return `no_change` when the grant already matches usage, and
  `needs_more_evidence` when the usage summary is empty or unattributable.

## Inputs

- `subject` (optional): what is being audited (a skill id, grant id, or
  principal). Used only to label the report.
- `granted_scopes` (required): the scopes currently granted to the subject.
- `usage_summary` (required): observed usage derived from receipts. For each
  scope or resource, what verbs were actually exercised.
- `objective` (optional): operator intent that focuses the review.
