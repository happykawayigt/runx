---
name: docs-demo
description: docs-demo runx skill. Replace this with what the skill does and returns.
source:
  type: cli-tool
  command: node
  args:
    - run.mjs
  timeout_seconds: 30
  sandbox:
    profile: readonly
    cwd_policy: skill-directory
inputs:
  message:
    type: string
    required: true
    description: Input the skill acts on. Replace with the real inputs.
runx:
  category: ops
  input_resolution:
    required:
      - message
---

# docs-demo

Describe what this skill does, when an agent should reach for it, and what it
returns. Replace the echo in `run.mjs` with the real work, and add cases to
`X.yaml` so the behaviour is locked by the harness.
