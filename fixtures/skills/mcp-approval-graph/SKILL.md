---
name: mcp-approval-graph
description: Exercise a Rust-served MCP graph approval round trip.
source:
  type: graph
  graph:
    name: mcp-approval-graph
    steps:
      - id: approve
        run:
          type: approval
        inputs:
          gate_id: mcp-approval
          reason: Approve the MCP graph run.
        artifacts:
          wrap_as: approval
---

Pause for approval, then complete after `runx_resume`.
