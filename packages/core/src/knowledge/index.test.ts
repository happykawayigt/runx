import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";

import { describe, expect, it, vi } from "vitest";

import { canonicalJsonStringify, sha256Hex } from "@runxhq/contracts";

import {
  createFileKnowledgeStore,
  buildFeedStoryOutboxEntry,
  fetchThreadViaAdapter,
  findLatestControlOutboxEntry,
  findLatestOutboxEntry,
  findOutboxEntry,
  findActiveSuppressionRecord,
  handoffIsSuppressed,
  handoffStateAllowsOutboxPush,
  handoffStateAllowsSignalDisposition,
  latestDecisionForGate,
  latestHandoffSignal,
  materializeOutboxEntryFiles,
  pushOutboxEntryViaAdapter,
  reduceHandoffState,
  renderIssueToPrReviewerMarkdown,
  renderFeedStoryMarkdown,
  readOutboxEntryControl,
  sanitizePublicMarkdown,
  summarizePublicHandoffMarkdown,
  sortOutboxEntriesByRecency,
  threadAllowsGate,
  summarizeThread,
  validateHandoffSignal,
  validateHandoffState,
  validateSuppressionRecord,
  validateThread,
  validateFeedStory,
} from "./index.js";

describe("thread contract", () => {
  it("accepts provider-native thread without leaking provider nouns into core fields", () => {
    const state = validateThread({
      kind: "runx.thread.v1",
      adapter: {
        type: "github",
        provider: "github",
        surface: "issue_thread",
        cursor: "comment:4286817434",
      },
      thread_kind: "signal",
      thread_locator: "runxhq/aster#issue/110",
      title: "[skill] Add a collaboration issue distillation skill",
      canonical_uri: "https://github.com/runxhq/aster/issues/110",
      entries: [
        {
          entry_id: "comment-1",
          entry_kind: "message",
          recorded_at: "2026-04-21T07:25:06Z",
          actor: {
            actor_id: "auscaster",
            role: "maintainer",
          },
          body: "Opened draft PR for this run.",
        },
      ],
      decisions: [
        {
          decision_id: "publish-1",
          gate_id: "skill-lab.publish",
          decision: "allow",
          recorded_at: "2026-04-21T08:00:00Z",
          reason: "same subject approved one rolling draft PR",
        },
      ],
      outbox: [
        {
          entry_id: "pr-111",
          kind: "pull_request",
          locator: "https://github.com/runxhq/aster/pull/111",
          status: "draft",
        },
      ],
      source_refs: [
        {
          type: "provider_thread",
          uri: "https://github.com/runxhq/aster/issues/110",
        },
      ],
      generated_at: "2026-04-21T08:05:00Z",
    });

    expect(state.thread_kind).toBe("signal");
    expect(state.thread_locator).toBe("runxhq/aster#issue/110");
    expect(state.adapter.type).toBe("github");
    expect(threadAllowsGate(state, "skill-lab.publish")).toBe(true);
    expect(findOutboxEntry(state, "pull_request")?.status).toBe("draft");
  });

  it("returns the newest matching decision for a gate", () => {
    const state = validateThread({
      kind: "runx.thread.v1",
      adapter: {
        type: "local-conversation",
      },
      thread_kind: "signal",
      thread_locator: "local://conversation/42",
      entries: [],
      decisions: [
        {
          decision_id: "plan-1",
          gate_id: "issue-triage.plan",
          decision: "deny",
          recorded_at: "2026-04-21T08:00:00Z",
        },
        {
          decision_id: "plan-2",
          gate_id: "issue-triage.plan",
          decision: "allow",
          recorded_at: "2026-04-21T08:05:00Z",
        },
      ],
      outbox: [],
      source_refs: [],
    });

    expect(latestDecisionForGate(state, "issue-triage.plan")?.decision_id).toBe("plan-2");
    expect(threadAllowsGate(state, "issue-triage.plan")).toBe(true);
  });

  it("renders a stable provider-agnostic summary", () => {
    const state = validateThread({
      kind: "runx.thread.v1",
      adapter: {
        type: "ticketing",
        provider: "linear",
        surface: "ticket_thread",
      },
      thread_kind: "signal",
      thread_locator: "linear://issue/ENG-42",
      entries: [
        {
          entry_id: "entry-1",
          entry_kind: "message",
          recorded_at: "2026-04-21T09:00:00Z",
        },
        {
          entry_id: "entry-2",
          entry_kind: "status",
          recorded_at: "2026-04-21T09:01:00Z",
        },
      ],
      decisions: [],
      outbox: [
        {
          entry_id: "draft-1",
          kind: "draft_change",
          status: "proposed",
        },
      ],
      source_refs: [],
    });

    expect(summarizeThread(state)).toBe(
      "signal:linear://issue/ENG-42 via ticketing | entries=2 decisions=0 outbox=draft_change",
    );
  });

  it("finds the latest control outbox entries without product-specific logic", () => {
    const state = validateThread({
      kind: "runx.thread.v1",
      adapter: {
        type: "github",
      },
      thread_kind: "signal",
      thread_locator: "github://example/repo/issues/123",
      entries: [],
      decisions: [],
      outbox: [
        {
          entry_id: "message:task:review",
          kind: "message",
          status: "published",
          thread_locator: "github://example/repo/issues/123",
          metadata: {
            updated_at: "2026-04-23T00:00:00Z",
          },
        },
        {
          entry_id: "message:task:controlled-review",
          kind: "message",
          status: "published",
          thread_locator: "github://example/repo/issues/123",
          metadata: {
            updated_at: "2026-04-24T00:00:00Z",
            control: {
              workflow: "docs",
              lane: "review",
            },
          },
        },
        {
          entry_id: "message:task:signal",
          kind: "message",
          status: "published",
          thread_locator: "github://example/repo/issues/123",
          metadata: {
            updated_at: "2026-04-25T00:00:00Z",
            control: {
              workflow: "docs",
              lane: "signal",
            },
          },
        },
        {
          entry_id: "pull_request:task",
          kind: "pull_request",
          locator: "https://github.com/example/repo/pull/1",
          metadata: {
            pushed_at: "2026-04-26T00:00:00Z",
          },
        },
      ],
      source_refs: [],
    });

    expect(readOutboxEntryControl(state.outbox[1])?.lane).toBe("review");
    expect(findLatestControlOutboxEntry(state, {
      kinds: ["message"],
      workflow: "docs",
      lanes: ["review"],
      entryIdPattern: /^message:[^:]+:review$/i,
    })?.entry_id).toBe("message:task:controlled-review");
    expect(findLatestControlOutboxEntry(state, {
      kinds: ["message"],
      workflow: "docs",
      lanes: ["signal"],
    })?.entry_id).toBe("message:task:signal");
    expect(findLatestControlOutboxEntry(state, {
      kinds: ["message"],
      entryIdPattern: /^message:[^:]+:review$/i,
    })?.entry_id).toBe("message:task:review");
    const globalPattern = /^message:[^:]+:signal$/gi;
    expect(findLatestControlOutboxEntry(state, {
      kinds: ["message"],
      entryIdPattern: globalPattern,
    })?.entry_id).toBe("message:task:signal");
    expect(findLatestControlOutboxEntry(state, {
      kinds: ["message"],
      entryIdPattern: globalPattern,
    })?.entry_id).toBe("message:task:signal");
    expect(findLatestOutboxEntry(state, { kinds: ["pull_request"] })?.locator)
      .toBe("https://github.com/example/repo/pull/1");
    expect(sortOutboxEntriesByRecency(state.outbox).map((entry) => entry.entry_id)).toEqual([
      "pull_request:task",
      "message:task:signal",
      "message:task:controlled-review",
      "message:task:review",
    ]);
  });

  it("centralizes generic handoff transition checks", () => {
    expect(handoffStateAllowsSignalDisposition({ status: "accepted" }, "approved_to_send")).toBe(true);
    expect(handoffStateAllowsSignalDisposition({ status: "needs_revision" }, "approved_to_send")).toBe(false);
    expect(handoffStateAllowsSignalDisposition({ status: "needs_revision" }, "requested_changes")).toBe(true);
    expect(handoffStateAllowsOutboxPush({ status: "approved_to_send" })).toBe(true);
    expect(handoffStateAllowsOutboxPush({ status: "accepted" })).toBe(false);
  });

  it("materializes reviewed outbox files through a generic reader", async () => {
    const files = await materializeOutboxEntryFiles({
      outboxEntry: {
        metadata: {
          changed_files: ["README.md", " ./docs/index.md ", "README.md"],
        },
      },
      async readFile(relativePath) {
        return `contents:${relativePath}`;
      },
    });

    expect(files).toEqual([
      { path: "README.md", contents: "contents:README.md" },
      { path: "docs/index.md", contents: "contents:docs/index.md" },
    ]);
    await expect(materializeOutboxEntryFiles({
      outboxEntry: {
        entry_id: "pull_request:task",
        kind: "pull_request",
        metadata: {
          changed_files: ["../secret.txt"],
        },
      },
      async readFile() {
        return "never";
      },
    })).rejects.toThrow(/relative path inside the workspace/);
  });

  it("sanitizes public lifecycle story text before publication", () => {
    expect(sanitizePublicMarkdown(
      "RUNX_BIN=/Users/kam/dev/runx/dist/index.js ghp_123456789012345678901234567890123456",
    )).toBe("RUNX_BIN=[local-path] [secret]");
    expect(sanitizePublicMarkdown(
      "wrote /tmp/runx/output.json with OPENAI_API_KEY=sk-test",
    )).toBe("wrote [local-path] with OPENAI_API_KEY=[secret]");
    expect(sanitizePublicMarkdown(
      "provider returned sk-proj-abcdefghijklmnopqrstuvwx1234567890",
    )).toBe("provider returned [secret]");
    expect(sanitizePublicMarkdown(
      "Status: super-secret-token",
    )).toBe("Status: [secret]");
    expect(sanitizePublicMarkdown(
      "Blockers: leaked bearer abc123",
    )).toBe("Blockers: leaked bearer [secret]");
    expect(sanitizePublicMarkdown(
      "Status: material_ref=nango:github:conn_1",
    )).toBe("Status: material_ref=[secret]");
  });

  it("renders stable feed entry markdown and message outbox entries", () => {
    const story = validateFeedStory({
      thread_locator: "github://example/repo/issues/123",
      title: "Checkout fails",
      next_action: "Human reviewer merges the draft PR after checks pass.",
      milestones: [
        {
          kind: "decision",
          status: "passed",
          summary: "Bug is reproducible and bounded to checkout validation.",
          details: [
            "Validation command uses repo-local tests, not [local-path].",
          ],
        },
        {
          kind: "merge_gate",
          status: "ready",
          summary: "Draft PR is ready for human review; runx will not merge it.",
        },
      ],
    });

    expect(renderFeedStoryMarkdown(story)).toBe(
      [
        "## Checkout fails",
        "",
        "Source thread: `github://example/repo/issues/123`",
        "",
        "### Gate Summary",
        "- Decision (passed): Bug is reproducible and bounded to checkout validation.",
        "  - Validation command uses repo-local tests, not [local-path].",
        "- Human merge gate (ready): Draft PR is ready for human review; runx will not merge it.",
        "",
        "Next: Human reviewer merges the draft PR after checks pass.",
        "",
      ].join("\n"),
    );

    expect(buildFeedStoryOutboxEntry({
      taskId: "Checkout Fix",
      threadLocator: "github://example/repo/issues/123",
      title: "Review gate",
      milestone: {
        kind: "review",
        status: "passed",
        summary: "Review passed with no blocking findings.",
      },
      updatedAt: "2026-05-14T12:00:00Z",
    })).toMatchObject({
      entry_id: "message:checkout-fix:review",
      kind: "message",
      title: "Review gate",
      status: "proposed",
      thread_locator: "github://example/repo/issues/123",
      metadata: {
        schema_version: "runx.outbox-entry.feed-entry.v1",
        workflow: "issue-to-pr",
        milestone_kind: "review",
        outbox_receipt_id: expect.stringMatching(/^feed:issue-to-pr:checkout-fix:review:[a-f0-9]{20}$/),
        updated_at: "2026-05-14T12:00:00Z",
        control: {
          workflow: "issue-to-pr",
          lane: "review",
        },
      },
    });

    const localStory = validateFeedStory({
      thread_locator: "/tmp/runx/thread.json",
      milestones: [
        {
          kind: "signal",
          summary: "Local fixture signal.",
        },
      ],
    });
    expect(renderFeedStoryMarkdown(localStory)).not.toContain("/tmp/runx");
    expect(renderFeedStoryMarkdown(localStory)).toContain("Source thread: `[local-path]`");
  });

  it("renders sanitized issue-to-PR reviewer markdown from shared knowledge helpers", () => {
    const markdown = renderIssueToPrReviewerMarkdown({
      taskId: "fixture-task",
      title: "Fix fixture behavior",
      sourceTitle: "Checkout failure",
      sourceLocator: "github://example/repo/issues/123",
      branch: "fixture-task",
      base: "main",
      governanceStatus: "completed",
      checkStatus: "success",
      buildPassed: 2,
      buildFailed: 0,
      reviewVerdict: "pass",
      blockingCount: 0,
      nonBlockingCount: 1,
      handoffMarkdown: [
        "# Handoff: Fix fixture behavior",
        "Status: completed",
        "Next: none",
        "RUNX_BIN=/Users/kam/dev/runx/dist/index.js",
        "Changed /tmp/workspace/app.txt",
        "provider returned sk-proj-abcdefghijklmnopqrstuvwx1234567890",
      ].join("\n"),
    });

    expect(markdown).toContain("## Source Thread");
    expect(markdown).toContain("## Human Merge Gate");
    expect(markdown).toContain("Blocking findings: 0");
    expect(markdown).toContain("Status: completed");
    expect(markdown).toContain("Detailed handoff output omitted");
    expect(markdown).not.toContain("RUNX_BIN=");
    expect(markdown).not.toContain("/Users/kam");
    expect(markdown).not.toContain("/tmp/workspace");
    expect(markdown).not.toContain("sk-proj-");
  });

  it("summarizes public handoff markdown instead of publishing raw command logs", () => {
    expect(summarizePublicHandoffMarkdown([
      "# Handoff: Fixture",
      "Status: review",
      "Next: scafld review fixture",
      "RUNX_BIN=/Users/kam/dev/runx/dist/index.js",
      "node --test very-long-private-command",
    ].join("\n"))).toBe([
      "# Handoff: Fixture",
      "Status: review",
      "Next: scafld review fixture",
      "",
      "Detailed handoff output omitted from public markdown; run `scafld handoff` in the workspace for private evidence.",
    ].join("\n"));
  });

  it("redacts secret-looking values on allowlisted handoff summary lines", () => {
    const summary = summarizePublicHandoffMarkdown([
      "# Handoff: Fixture",
      "Status: super-secret-token",
      "Blockers: leaked bearer abc123",
      "Next: material_ref=nango:github:conn_1",
    ].join("\n"));

    expect(summary).toBe([
      "# Handoff: Fixture",
      "Status: [secret]",
      "Blockers: leaked bearer [secret]",
      "Next: material_ref=[secret]",
    ].join("\n"));
  });

  it("rejects missing thread locator fields", () => {
    expect(
      () =>
        validateThread({
          kind: "runx.thread.v1",
          adapter: {
            type: "github",
          },
          thread_kind: "signal",
          title: "missing locator",
          entries: [],
          decisions: [],
          outbox: [],
          source_refs: [],
        }),
    ).toThrow(/thread_locator/);
  });

  it("rejects nested subject payloads in the thread contract", () => {
    expect(
      () =>
        validateThread({
          kind: "runx.thread.v1",
          adapter: {
            type: "github",
          },
          subject: {
            thread_kind: "signal",
            thread_locator: "github://example/repo/issues/123",
          },
          entries: [],
          decisions: [],
          outbox: [],
          source_refs: [],
        }),
    ).toThrow(/thread_kind/);
  });

  it("pushes and rehydrates through the file thread adapter", async () => {
    const tempDir = await mkdtemp(path.join(os.tmpdir(), "runx-thread-file-"));
    const statePath = path.join(tempDir, "thread.json");
    const pushedAt = "2026-05-22T01:02:03.000Z";
    const initial = {
      kind: "runx.thread.v1",
      adapter: {
        type: "file",
        adapter_ref: statePath,
      },
      thread_kind: "signal",
      thread_locator: "local://provider/issues/123",
      canonical_uri: "https://example.test/issues/123",
      entries: [],
      decisions: [],
      outbox: [],
      source_refs: [],
    };

    try {
      await writeFile(statePath, `${JSON.stringify(initial, null, 2)}\n`);
      vi.useFakeTimers();
      vi.setSystemTime(new Date(pushedAt));
      const state = validateThread(initial);
      const result = await pushOutboxEntryViaAdapter({
        thread: state,
        entry: {
          entry_id: "pull_request:fixture-task",
          kind: "pull_request",
          title: "Fixture PR",
          status: "proposed",
        },
        next_status: "draft",
      });
      vi.useRealTimers();

      expect(result.status).toBe("pushed");
      expect(result.outbox_entry).toMatchObject({
        entry_id: "pull_request:fixture-task",
        kind: "pull_request",
        status: "draft",
        locator: expect.stringContaining("#outbox/pull_request%3Afixture-task"),
        thread_locator: "local://provider/issues/123",
      });
      expect(result.thread.outbox).toEqual([
        expect.objectContaining({
          entry_id: "pull_request:fixture-task",
          status: "draft",
        }),
      ]);
      expect(result.thread.entries.at(-1)?.entry_id).toBe(`entry_${sha256Hex(canonicalJsonStringify({
        thread: "local://provider/issues/123",
        outbox_entry: "pull_request:fixture-task",
        pushed_at: pushedAt,
      })).slice(0, 24)}`);
      expect(result.thread.adapter.cursor).toBe(`push:${sha256Hex(canonicalJsonStringify({
        outbox_entry: "pull_request:fixture-task",
        pushed_at: pushedAt,
      })).slice(0, 12)}`);
      expect(result.thread.adapter.cursor).not.toContain("sha256:");
      expect(result.thread.entries.at(-1)).toMatchObject({
        entry_kind: "status",
        structured_data: {
          event: "push_outbox_entry",
          outbox_entry_id: "pull_request:fixture-task",
          status: "draft",
        },
      });

      const fetched = await fetchThreadViaAdapter(result.thread.adapter, {
        thread_kind: "signal",
        thread_locator: "local://provider/issues/123",
        include_outbox: true,
      });
      expect(fetched?.outbox).toEqual([
        expect.objectContaining({
          entry_id: "pull_request:fixture-task",
          status: "draft",
        }),
      ]);
    } finally {
      vi.useRealTimers();
      await rm(tempDir, { recursive: true, force: true });
    }
  });

  it("skips push when no runtime adapter is registered", async () => {
    const state = validateThread({
      kind: "runx.thread.v1",
      adapter: {
        type: "github",
      },
      thread_kind: "signal",
      thread_locator: "github://example/repo/issues/123",
      entries: [],
      decisions: [],
      outbox: [],
      source_refs: [],
    });

    const result = await pushOutboxEntryViaAdapter({
      thread: state,
      entry: {
        entry_id: "pull_request:fixture-task",
        kind: "pull_request",
      },
    });

    expect(result).toEqual({
      status: "skipped",
      reason: "no thread adapter is registered for 'github'; provider outbox adapters require a ratified thread-outbox provider protocol and Rust-supervised CredentialDelivery",
      outbox_entry: {
        entry_id: "pull_request:fixture-task",
        kind: "pull_request",
      },
      thread: state,
    });
  });
});

describe("handoff state reduction", () => {
  it("keeps the post-boundary model generic instead of docs-specific", () => {
    const firstSignal = validateHandoffSignal({
      schema: "runx.handoff_signal.v1",
      signal_id: "sig_1",
      handoff_id: "docs-pr:example/repo:001",
      boundary_kind: "external_maintainer",
      target_repo: "example/repo",
      target_locator: "github://example/repo/pulls/42",
      thread_locator: "github://example/repo/pulls/42",
      source: "pull_request_comment",
      disposition: "acknowledged",
      recorded_at: "2026-04-24T03:00:00Z",
    });
    const secondSignal = validateHandoffSignal({
      schema: "runx.handoff_signal.v1",
      signal_id: "sig_2",
      handoff_id: "docs-pr:example/repo:001",
      boundary_kind: "external_maintainer",
      target_repo: "example/repo",
      target_locator: "github://example/repo/pulls/42",
      source: "pull_request_review",
      disposition: "requested_changes",
      recorded_at: "2026-04-24T03:05:00Z",
    });

    expect(latestHandoffSignal([firstSignal, secondSignal], "docs-pr:example/repo:001")?.signal_id).toBe("sig_2");
    expect(reduceHandoffState({
      handoff_id: "docs-pr:example/repo:001",
      boundary_kind: "external_maintainer",
      target_repo: "example/repo",
      target_locator: "github://example/repo/pulls/42",
      signals: [firstSignal, secondSignal],
    })).toMatchObject({
      schema: "runx.handoff_state.v1",
      status: "needs_revision",
      signal_count: 2,
      last_signal_id: "sig_2",
      last_signal_disposition: "requested_changes",
    });
    expect(validateHandoffState({
      schema: "runx.handoff_state.v1",
      handoff_id: "docs-pr:example/repo:001",
      target_repo: "example/repo",
      status: "needs_revision",
      signal_count: 2,
      last_signal_id: "sig_2",
      last_signal_at: "2026-04-24T03:05:00Z",
      last_signal_disposition: "requested_changes",
    })).toMatchObject({
      status: "needs_revision",
      signal_count: 2,
    });
  });

  it("applies scoped suppression records across future outreach attempts", () => {
    const repoSuppression = validateSuppressionRecord({
      schema: "runx.suppression_record.v1",
      record_id: "sup_repo",
      scope: "repo",
      key: "example/repo",
      reason: "operator_block",
      recorded_at: "2026-04-24T03:10:00Z",
    });
    const contactSuppression = validateSuppressionRecord({
      schema: "runx.suppression_record.v1",
      record_id: "sup_contact",
      scope: "contact",
      key: "mailto:maintainer@example.org",
      reason: "requested_no_contact",
      recorded_at: "2026-04-24T03:12:00Z",
    });

    expect(findActiveSuppressionRecord({
      handoff_id: "docs-outreach:charity:001",
      target_repo: "example/repo",
      contact_locator: "mailto:maintainer@example.org",
    }, [repoSuppression, contactSuppression], "2026-04-24T03:20:00Z")?.record_id).toBe("sup_contact");
    expect(handoffIsSuppressed({
      handoff_id: "docs-outreach:charity:001",
      target_repo: "example/repo",
      contact_locator: "mailto:maintainer@example.org",
    }, [repoSuppression, contactSuppression], "2026-04-24T03:20:00Z")).toBe(true);
    expect(reduceHandoffState({
      handoff_id: "docs-outreach:charity:001",
      boundary_kind: "external_contact",
      target_repo: "example/repo",
      contact_locator: "mailto:maintainer@example.org",
      suppressions: [repoSuppression, contactSuppression],
      now: "2026-04-24T03:20:00Z",
    })).toMatchObject({
      status: "suppressed",
      suppression_record_id: "sup_contact",
      suppression_reason: "requested_no_contact",
    });
  });
});

describe("file local knowledge store", () => {
  it("initializes an idempotent filesystem index and stores project-scoped projections", async () => {
    const tempDir = await mkdtemp(path.join(os.tmpdir(), "runx-knowledge-"));
    const knowledgeDir = path.join(tempDir, "knowledge");

    try {
      const store = createFileKnowledgeStore(knowledgeDir);
      await expect(store.init()).resolves.toMatchObject({
        schema_version: "runx.knowledge.v1",
        entries: [],
      });
      await expect(store.init()).resolves.toMatchObject({
        schema_version: "runx.knowledge.v1",
      });

      const receipt = {
        id: "rx_knowledge_1",
        kind: `skill_${"execution"}` as const,
        status: "sealed" as const,
        [`skill_${"name"}`]: "echo",
        source_type: "cli-tool",
        started_at: "2026-04-10T00:00:00Z",
        completed_at: "2026-04-10T00:00:01Z",
      };

      const project = path.join(tempDir, "project");
      await store.indexReceipt({
        receipt,
        [`receipt${"Path"}`]: path.join(tempDir, "receipts", `${receipt.id}.json`),
        project,
        indexedAt: "2026-04-10T00:00:02Z",
      });
      await store.addProjection({
        project,
        scope: "project",
        key: "homepage_url",
        value: "https://example.test",
        source: "test",
        confidence: 0.9,
        freshness: "fresh",
        receiptId: receipt.id,
        createdAt: "2026-04-10T00:00:03Z",
      });

      await expect(store.listReceipts({ project })).resolves.toEqual([
        expect.objectContaining({
          receipt_id: receipt.id,
          receipt_ref: {
            type: "receipt",
            uri: `runx:receipt:${receipt.id}`,
            label: "echo",
          },
          execution_name: "echo",
          source_type: "cli-tool",
          indexed_at: "2026-04-10T00:00:02Z",
        }),
      ]);
      await expect(store.listProjections({ project })).resolves.toEqual([
        expect.objectContaining({
          key: "homepage_url",
          value: "https://example.test",
          receipt_id: receipt.id,
        }),
      ]);
    } finally {
      await rm(tempDir, { recursive: true, force: true });
    }
  });

  it("preserves concurrent projection writes through the filesystem index", async () => {
    const tempDir = await mkdtemp(path.join(os.tmpdir(), "runx-knowledge-concurrent-"));
    const knowledgeDir = path.join(tempDir, "knowledge");
    const project = path.join(tempDir, "project");

    try {
      const store = createFileKnowledgeStore(knowledgeDir);
      await store.init();

      await Promise.all(
        Array.from({ length: 20 }, async (_, index) =>
          createFileKnowledgeStore(knowledgeDir).addProjection({
            project,
            scope: "project",
            key: `projection_${index}`,
            value: index,
            source: "concurrency-test",
            confidence: 1,
            freshness: "fresh",
            createdAt: `2026-04-10T00:00:${String(index).padStart(2, "0")}Z`,
          }),
        ),
      );

      const projections = await store.listProjections({ project });
      expect(projections).toHaveLength(20);
      expect(projections.map((projection) => projection.key).sort()).toEqual(
        Array.from({ length: 20 }, (_, index) => `projection_${index}`).sort(),
      );
    } finally {
      await rm(tempDir, { recursive: true, force: true });
    }
  });

  it("skips malformed stored index entries instead of throwing", async () => {
    const tempDir = await mkdtemp(path.join(os.tmpdir(), "runx-knowledge-malformed-"));
    const knowledgeDir = path.join(tempDir, "knowledge");
    const project = path.join(tempDir, "project");

    try {
      await mkdir(knowledgeDir, { recursive: true });
      await writeFile(
        path.join(knowledgeDir, "index.json"),
        `${JSON.stringify({
          schema_version: "runx.knowledge.v1",
          entries: [
            {
              entry_id: "receipt_rx_valid",
              entry_kind: "receipt",
              receipt_id: "rx_valid",
              receipt_ref: {
                type: "receipt",
                uri: "runx:receipt:rx_valid",
                label: "echo",
              },
              status: "sealed",
              execution_name: "echo",
              indexed_at: "2026-04-10T00:00:00Z",
              project,
            },
            { receipt_id: 123, indexed_at: 1 },
            {
              entry_id: "projection_valid",
              entry_kind: "projection",
              project,
              scope: "project",
              key: "homepage_url",
              value: "https://example.test",
              source: "test",
              confidence: 0.9,
              freshness: "fresh",
              created_at: "2026-04-10T00:00:01Z",
            },
            { id: "projection_bad", key: 42 },
          ],
        }, null, 2)}\n`,
      );

      const warnings: string[] = [];
      const warn = console.warn;
      console.warn = (message?: unknown) => {
        warnings.push(String(message ?? ""));
      };

      try {
        const store = createFileKnowledgeStore(knowledgeDir);
        const knowledge = await store.read();
        expect(knowledge.entries.filter((entry) => entry.entry_kind === "receipt")).toEqual([
          expect.objectContaining({
            receipt_id: "rx_valid",
            receipt_ref: {
              type: "receipt",
              uri: "runx:receipt:rx_valid",
              label: "echo",
            },
            execution_name: "echo",
          }),
        ]);
        expect(knowledge.entries.filter((entry) => entry.entry_kind === "projection")).toEqual([
          expect.objectContaining({
            entry_id: "projection_valid",
            key: "homepage_url",
          }),
        ]);
      } finally {
        console.warn = warn;
      }

      expect(warnings).toHaveLength(2);
      expect(warnings[0]).toContain("malformed local knowledge entry");
      expect(warnings[1]).toContain("malformed local knowledge entry");
    } finally {
      await rm(tempDir, { recursive: true, force: true });
    }
  });
});
