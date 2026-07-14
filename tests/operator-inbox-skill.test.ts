import { existsSync, mkdtempSync, readdirSync, rmSync } from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";

import { describe, expect, it } from "vitest";

const PLAN_RUNNER = path.resolve("skills/operator-inbox/graph/plan/run.mjs");
const REDUCE_RUNNER = path.resolve("skills/operator-inbox/graph/reduce/run.mjs");
const SKILL_PATH = path.resolve("skills/operator-inbox");
const QUERY_DIGEST = `sha256:${"0".repeat(64)}`;

describe("operator-inbox skill", () => {
  it("preserves explicit dispositions and reopens only for newer external work", () => {
    let projection = emptyProjection();

    const first = plan({
      operation: "scan_page",
      expected_version: 0,
      projection,
      observed_at: "2026-07-14T09:05:00.000Z",
      scan: scan("scan-1", 1, "complete"),
      messages: [message({
        messageLocator: "slack://workspace/analytics/100.1",
        occurredAt: "2026-07-14T09:00:00.000Z",
        authorId: "george",
        preview: "Can you check the analytics claim?",
      })],
    });
    projection = fold(projection, 0, 1, [eventRecord(1, first.event)]);

    expect(projection).toMatchObject({
      version: 1,
      page: { complete: true, next_after_version: 1, stream_version: 1 },
      items: [{
        thread_locator: "slack://workspace/analytics/thread-100",
        status: "open",
        requester: { external_id: "george", display_name: "George" },
        conversation: { external_id: "analytics", display_name: "analytics", type: "channel" },
      }],
      scans: [{ scan_id: "scan-1", status: "complete", pages_observed: 1 }],
    });

    const resolved = plan({
      operation: "disposition",
      expected_version: 1,
      projection,
      observed_at: "2026-07-14T10:00:00.000Z",
      disposition: {
        thread_locator: "slack://workspace/analytics/thread-100",
        status: "resolved",
        actor: "Kam",
        reason: "Addressed in the sending-at-scale article",
        evidence_url: "https://example.com/sending-at-scale",
      },
    });
    expect(resolved.projection).not.toHaveProperty("page");
    projection = fold(resolved.projection, 2, 2, []);

    const oldHistory = planScan(projection, 2, "scan-2", message({
      messageLocator: "slack://workspace/analytics/99.9",
      occurredAt: "2026-07-14T08:30:00.000Z",
      authorId: "george",
      preview: "An older reminder",
    }));
    expect(oldHistory.projection.items[0]).toMatchObject({
      status: "resolved",
      disposition: { actor: "Kam", reason: "Addressed in the sending-at-scale article" },
    });

    const ownReply = planScan(oldHistory.projection, 3, "scan-3", message({
      messageLocator: "slack://workspace/analytics/101.1",
      occurredAt: "2026-07-14T10:30:00.000Z",
      authorId: "operator-1",
      authorName: "Kam",
      preview: "I followed this up",
    }));
    expect(ownReply.projection.items[0].status).toBe("resolved");

    const newerExternal = planScan(ownReply.projection, 4, "scan-4", message({
      messageLocator: "slack://workspace/analytics/102.1",
      occurredAt: "2026-07-14T09:30:00.000Z",
      authorId: "nick",
      authorName: "Nick",
      preview: "An unseen follow-up arrived before the disposition",
    }));
    expect(newerExternal.projection.items[0]).toMatchObject({
      status: "open",
      requester: { external_id: "george", display_name: "George" },
      latest_message: {
        message_locator: "slack://workspace/analytics/102.1",
        preview: "An unseen follow-up arrived before the disposition",
      },
      disposition: {
        status: "resolved",
        actor: "Kam",
        covered_occurrence_at: "2026-07-14T09:00:00.000Z",
      },
    });
  });

  it("rejects non-contiguous pages and unbounded observations", () => {
    const event = planScan(emptyProjection(), 0, "scan-bounds", message({
      messageLocator: "slack://workspace/analytics/100.1",
      occurredAt: "2026-07-14T09:00:00.000Z",
      authorId: "george",
      preview: "Bounded",
    })).event;

    const gap = runStageRaw(REDUCE_RUNNER, {
      projection: emptyProjection(),
      after_version: 0,
      stream_version: 2,
      events: [eventRecord(2, event)],
    });
    expect(gap.status).not.toBe(0);
    expect(gap.stderr).toContain("not contiguous at version 1");

    const tooMany = runStageRaw(PLAN_RUNNER, {
      operation: "scan_page",
      expected_version: 0,
      projection: emptyProjection(),
      observed_at: "2026-07-14T09:05:00.000Z",
      scan: scan("scan-too-many", 1, "complete"),
      messages: Array.from({ length: 21 }, (_, index) => message({
        messageLocator: `slack://workspace/analytics/${index}`,
        occurredAt: "2026-07-14T09:00:00.000Z",
        authorId: "george",
        preview: "Bounded",
      })),
    });
    expect(tooMany.status).not.toBe(0);
    expect(tooMany.stderr).toContain("at most 20");
  });

  it("composes graph writes through the default local SQLite data source", () => {
    const workspace = mkdtempSync(path.join(os.tmpdir(), "runx-operator-inbox-"));
    try {
      const result = spawnSync(nativeRunxBinaryForTest(), ["harness", SKILL_PATH, "--json"], {
        cwd: workspace,
        encoding: "utf8",
        env: {
          ...process.env,
          RUNX_CWD: workspace,
          RUNX_RECEIPT_SIGN_KID: "operator-inbox-test-key",
          RUNX_RECEIPT_SIGN_ED25519_SEED_BASE64: "QkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkI=",
          RUNX_RECEIPT_SIGN_ISSUER_TYPE: "hosted",
        },
      });
      expect(result.status, result.stderr).toBe(0);
      expect(JSON.parse(result.stdout)).toMatchObject({ status: "passed", case_count: 1 });

      const dataDir = path.join(workspace, ".runx", "data", "local-sources");
      expect(readdirSync(dataDir).some((entry) => entry.endsWith(".sqlite"))).toBe(true);
    } finally {
      rmSync(workspace, { recursive: true, force: true });
    }
  });
});

function planScan(projection: Projection, expectedVersion: number, scanId: string, observedMessage: Message) {
  return plan({
    operation: "scan_page",
    expected_version: expectedVersion,
    projection,
    observed_at: "2026-07-14T11:00:00.000Z",
    scan: scan(scanId, 1, "complete"),
    messages: [observedMessage],
  });
}

function plan(inputs: Record<string, unknown>): Transition {
  return (runStage(PLAN_RUNNER, inputs) as { transition: Transition }).transition;
}

function fold(
  projection: Projection,
  afterVersion: number,
  streamVersion: number,
  events: readonly Record<string, unknown>[],
): Projection {
  return (runStage(REDUCE_RUNNER, {
    projection,
    after_version: afterVersion,
    stream_version: streamVersion,
    events,
  }) as { projection: Projection }).projection;
}

function runStage(stage: string, inputs: Record<string, unknown>): unknown {
  const result = runStageRaw(stage, inputs);
  expect(result.status, result.stderr).toBe(0);
  return JSON.parse(result.stdout);
}

function runStageRaw(stage: string, inputs: Record<string, unknown>) {
  return spawnSync(process.execPath, [stage], {
    encoding: "utf8",
    env: { ...process.env, RUNX_INPUTS_JSON: JSON.stringify(inputs) },
  });
}

function eventRecord(version: number, event: Record<string, unknown>) {
  return { version, event };
}

function scan(scanId: string, pageIndex: number, status: string) {
  return {
    scan_id: scanId,
    provider: "slack",
    query_digest: QUERY_DIGEST,
    page_index: pageIndex,
    status,
  };
}

function message({
  messageLocator,
  occurredAt,
  authorId,
  authorName = "George",
  preview,
}: {
  readonly messageLocator: string;
  readonly occurredAt: string;
  readonly authorId: string;
  readonly authorName?: string;
  readonly preview: string;
}): Message {
  return {
    provider: "slack",
    external_tenant_ref: "workspace",
    connected_subject_ref: "operator-1",
    message_locator: messageLocator,
    thread_locator: "slack://workspace/analytics/thread-100",
    author: { external_id: authorId, display_name: authorName },
    conversation: { external_id: "analytics", display_name: "analytics", type: "channel" },
    occurred_at: occurredAt,
    preview,
    permalink: "https://example.slack.com/archives/analytics/p100",
    context: [],
  };
}

function emptyProjection(): Projection {
  return { schema: "runx.operator_inbox.projection.v1", version: 0, items: [], scans: [] };
}

function nativeRunxBinaryForTest(): string {
  const configured = process.env.RUNX_DEV_RUST_CLI_BIN;
  if (configured) return configured;
  const candidate = path.resolve("crates/target/debug/runx");
  return existsSync(candidate) ? candidate : "runx";
}

type Projection = {
  readonly schema: string;
  readonly version: number;
  readonly items: readonly InboxItem[];
  readonly scans: readonly Record<string, unknown>[];
  readonly page?: Record<string, unknown>;
};

type InboxItem = Record<string, unknown> & {
  readonly status: string;
};

type Message = Record<string, unknown>;

type Transition = {
  readonly event: Record<string, unknown>;
  readonly projection: Projection;
};
