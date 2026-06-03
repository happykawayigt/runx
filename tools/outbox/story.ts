import { createHash } from "node:crypto";

export const STORY_MILESTONE_IDS = [
  "accepted",
  "hydrated",
  "triaged",
  "reply_drafted",
  "ask_for_info",
  "proposal_ready",
  "escalation_proposed",
  "tracking_item_created",
  "spec_ready",
  "build_started",
  "review_requested",
  "change_request_created",
  "review_fixup",
  "human_gate",
  "outcome_observed",
  "final_outcome",
  "no_action",
  "monitor",
] as const;

export type StoryMilestoneId = typeof STORY_MILESTONE_IDS[number];
export type ThreadStorySectionId = StoryMilestoneId;

export const ISSUE_TO_PR_STORY_MILESTONES = [
  "accepted",
  "triaged",
  "spec_ready",
  "build_started",
  "review_requested",
  "change_request_created",
  "human_gate",
  "final_outcome",
] as const satisfies readonly StoryMilestoneId[];

export const LEGACY_STORY_MILESTONE_ID_MAP = {
  signal: "accepted",
  decision: "triaged",
  spec: "spec_ready",
  build: "build_started",
  review: "review_requested",
  pull_request: "change_request_created",
  merge_gate: "human_gate",
  outcome: "final_outcome",
  initial_issue: "accepted",
  triage_results: "triaged",
  pr_created: "change_request_created",
  human_merge_gate: "human_gate",
  completion_update: "final_outcome",
} as const satisfies Record<string, StoryMilestoneId>;

export const STORY_MILESTONE_LABELS = {
  accepted: "Accepted",
  hydrated: "Context Hydrated",
  triaged: "Triaged",
  reply_drafted: "Reply Drafted",
  ask_for_info: "Ask For Info",
  proposal_ready: "Proposal Ready",
  escalation_proposed: "Escalation Proposed",
  tracking_item_created: "Tracking Item Created",
  spec_ready: "Spec Ready",
  build_started: "Build Started",
  review_requested: "Review Requested",
  change_request_created: "Change Request Created",
  review_fixup: "Review Fixup",
  human_gate: "Human Gate",
  outcome_observed: "Outcome Observed",
  final_outcome: "Final Outcome",
  no_action: "No Action",
  monitor: "Monitor",
} as const satisfies Record<StoryMilestoneId, string>;

export interface StoryMilestone {
  readonly kind: StoryMilestoneId;
  readonly status?: string;
  readonly summary?: string;
  readonly details?: readonly string[];
  readonly proposal_kind?: string;
  readonly source_ref?: string;
  readonly source_thread_ref?: string;
  readonly result_refs?: readonly string[];
  readonly publication_refs?: readonly string[];
}

export interface ThreadStory {
  readonly title?: string;
  readonly next_action?: string;
  readonly source_ref?: string;
  readonly source_thread_ref?: string;
  readonly result_refs?: readonly string[];
  readonly publication_refs?: readonly string[];
  readonly milestones?: readonly StoryMilestone[];
}

export type FeedStoryMilestoneKind = StoryMilestoneId;

export interface FeedStoryOutboxEntryInput {
  readonly taskId?: unknown;
  readonly threadLocator?: unknown;
  readonly title?: unknown;
  readonly milestone?: Partial<StoryMilestone> & Record<string, unknown>;
  readonly bodyMarkdown?: unknown;
  readonly updatedAt?: unknown;
  readonly workflow?: unknown;
  readonly laneId?: unknown;
  readonly sourceId?: unknown;
  readonly provider?: unknown;
  readonly targetRef?: unknown;
  readonly proposalId?: unknown;
}

interface StoryOutboxMetadataInput {
  readonly sourceId?: unknown;
  readonly provider?: unknown;
  readonly sourceThreadRef?: unknown;
  readonly workflowId?: unknown;
  readonly laneId?: unknown;
  readonly milestoneId?: unknown;
  readonly targetRef?: unknown;
  readonly proposalId?: unknown;
  readonly bodyMarkdown?: unknown;
  readonly requiresSourceThreadPublication?: boolean;
}

interface StoryOutboxIdempotencyMetadata {
  readonly key: string;
  readonly content_hash: string;
}

const STORY_MILESTONE_ID_SET = new Set<string>(STORY_MILESTONE_IDS);
const LEGACY_STORY_MILESTONE_ID_SET = new Set<string>(Object.keys(LEGACY_STORY_MILESTONE_ID_MAP));
const LEGACY_STORY_MILESTONE_ID_LOOKUP: Readonly<Record<string, StoryMilestoneId>> = LEGACY_STORY_MILESTONE_ID_MAP;

export function isStoryMilestoneId(value: unknown): value is StoryMilestoneId {
  return typeof value === "string" && STORY_MILESTONE_ID_SET.has(value);
}

export function assertStoryMilestoneId(value: unknown, label = "milestone_id"): StoryMilestoneId {
  if (isStoryMilestoneId(value)) {
    return value;
  }
  if (typeof value === "string" && LEGACY_STORY_MILESTONE_ID_SET.has(value)) {
    throw new Error(`${label} uses legacy milestone id '${value}'; use '${LEGACY_STORY_MILESTONE_ID_LOOKUP[value]}'.`);
  }
  throw new Error(`${label} has unknown_milestone '${String(value)}'.`);
}

export function canonicalStoryMilestoneIdForPublishedRefresh(value: unknown): StoryMilestoneId | undefined {
  if (isStoryMilestoneId(value)) {
    return value;
  }
  if (typeof value === "string") {
    return LEGACY_STORY_MILESTONE_ID_LOOKUP[value];
  }
  return undefined;
}

export function assertSourceThreadPublicationAllowed(input: {
  readonly requiresSourceThreadPublication?: boolean;
  readonly sourceThreadRef?: unknown;
  readonly missingBehavior?: unknown;
}): string | undefined {
  const sourceThreadRef = clean(input.sourceThreadRef);
  if (!input.requiresSourceThreadPublication) {
    return sourceThreadRef;
  }
  const missingBehavior = clean(input.missingBehavior) ?? "fail_closed";
  if (missingBehavior !== "fail_closed") {
    throw new Error("source_thread.missing_behavior must be fail_closed for source-thread publication.");
  }
  if (!sourceThreadRef) {
    throw new Error("missing_thread_locator: root_thread_fallback_rejected; source-thread publication must fail_closed.");
  }
  return sourceThreadRef;
}

export function renderThreadStoryMarkdown(story: ThreadStory): string {
  const title = clean(story.title) ?? "Operational story";
  const nextAction = clean(story.next_action);
  const milestones = Array.isArray(story.milestones) ? story.milestones : [];
  const refs = renderStoryRefs({
    source_ref: story.source_ref,
    source_thread_ref: story.source_thread_ref,
    result_refs: story.result_refs,
    publication_refs: story.publication_refs,
  });
  return [
    `# ${title}`,
    "",
    nextAction ? `Next: ${nextAction}` : undefined,
    refs,
    ...milestones.map((milestone) => renderStoryMilestoneMarkdown(milestone)),
  ].filter((line) => line !== undefined && line !== "").join("\n").trimEnd();
}

export function renderStoryMilestoneMarkdown(milestone: StoryMilestone): string {
  const kind = assertStoryMilestoneId(milestone.kind, "story milestone");
  const status = clean(milestone.status);
  const summary = clean(milestone.summary);
  const proposalKind = clean(milestone.proposal_kind);
  const proposalLabel = proposalKind ? friendlyProposalLabel(proposalKind) : undefined;
  const details = Array.isArray(milestone.details)
    ? milestone.details.map((detail) => clean(detail)).filter((detail): detail is string => Boolean(detail))
    : [];
  const refs = renderStoryRefs(milestone);
  return [
    `## ${proposalLabel ?? STORY_MILESTONE_LABELS[kind]}${status ? ` (${status})` : ""}`,
    summary,
    refs,
    ...details.map((detail) => `- ${detail}`),
    "",
  ].filter((line) => line !== undefined && line !== "").join("\n");
}

export function sanitizePublicMarkdown(value: unknown): string | undefined {
  if (value === undefined || value === null) {
    return undefined;
  }
  return String(value)
    .replace(/\b([A-Z][A-Z0-9_]*(?:TOKEN|SECRET|PASSWORD|API[_-]?KEY|MATERIAL[_-]?REF)[A-Z0-9_]*)=("[^"]*"|'[^']*'|\S+)/gi, "$1=[secret]")
    .replace(/\b((?:bearer|authorization|token|secret|password|api[_-]?key|material[_-]?ref|materialRef)\s*[:=]\s*)(["']?)[^\s`),;]+/gi, "$1[secret]")
    .replace(/\b((?:bearer|authorization)\s+)[A-Za-z0-9._:-]{6,}\b/gi, "$1[secret]")
    .replace(/\b(gh[pousr]_[A-Za-z0-9_]{20,}|xox[baprs]-[A-Za-z0-9-]{20,})\b/g, "[secret]")
    .replace(/\bsk-(?:proj-)?[A-Za-z0-9_-]{16,}\b/g, "[secret]")
    .replace(/\b[A-Za-z0-9]+(?:[-_](?:secret|token|password|api[-_]?key))+[A-Za-z0-9_-]*\b(?!\s*=)/gi, "[secret]")
    .replace(/\b([A-Z][A-Z0-9_]*=)(?:\/Users|\/home|\/var|\/private|\/tmp|[A-Za-z]:\\)[^\s`)]+/g, "$1[local-path]")
    .replace(/(^|[\s=("'`])(?:\/Users|\/home|\/var|\/private|\/tmp)\/[^\s`)]+/g, "$1[local-path]")
    .replace(/[A-Za-z]:\\[^\s`)]+/g, "[local-path]");
}

export function summarizePublicHandoffMarkdown(value: unknown): string | undefined {
  const sanitized = clean(value);
  if (!sanitized) {
    return undefined;
  }
  if (containsRedactionMarker(sanitized)) {
    return "Detailed handoff omitted from public markdown because it contains local paths or sensitive runtime details.";
  }
  return limitLines(sanitized, 18);
}

export function friendlyProposalLabel(proposalKind: string): string {
  return proposalKind
    .split(/[_-]+/)
    .filter(Boolean)
    .map((part) => `${part[0]?.toUpperCase() ?? ""}${part.slice(1)}`)
    .join(" ");
}

export function renderFeedStoryMarkdown(story: ThreadStory): string {
  return renderThreadStoryMarkdown(story);
}

export function buildFeedStoryOutboxEntry(input: FeedStoryOutboxEntryInput): Record<string, unknown> {
  const taskId = clean(input.taskId) ?? "unknown-task";
  const threadLocator = assertSourceThreadPublicationAllowed({
    requiresSourceThreadPublication: true,
    sourceThreadRef: input.threadLocator,
    missingBehavior: "fail_closed",
  });
  const milestone = input.milestone ?? {};
  const milestoneKind = assertStoryMilestoneId(milestone.kind, "outbox_entry.metadata.milestone_kind");
  const bodyMarkdown = clean(input.bodyMarkdown) ?? "";
  const workflow = clean(input.workflow) ?? "issue-to-pr";
  const coreMetadata = buildCoreStoryOutboxMetadata({
    sourceId: input.sourceId ?? taskId,
    provider: input.provider ?? "source_thread",
    sourceThreadRef: threadLocator,
    workflowId: workflow,
    laneId: input.laneId ?? workflow,
    milestoneId: milestoneKind,
    targetRef: input.targetRef,
    proposalId: input.proposalId,
    bodyMarkdown,
    requiresSourceThreadPublication: true,
  });
  const receiptHash = hashString(JSON.stringify({
    taskId,
    threadLocator,
    milestoneKind,
    bodyMarkdown,
    updatedAt: clean(input.updatedAt),
  })).slice(0, 20);

  return {
    entry_id: `message:${taskId}:${milestoneKind}`,
    kind: "message",
    status: "proposed",
    thread_locator: threadLocator,
    title: clean(input.title) ?? "Issue-to-PR story",
    metadata: {
      schema_version: "runx.outbox-entry.feed-entry.v1",
      workflow,
      milestone_kind: milestoneKind,
      outbox_receipt_id: `feed:${workflow}:${taskId}:${milestoneKind}:${receiptHash}`,
      idempotency: coreMetadata.idempotency,
      replay: coreMetadata.replay,
      source_thread: {
        required: true,
        publish_mode: "reply",
        missing_behavior: "fail_closed",
        thread_locator: threadLocator,
      },
      body_markdown: bodyMarkdown,
    },
  };
}

export function storyMilestoneRefreshesPublishedEntry(existing: unknown, requested: StoryMilestoneId): boolean {
  const existingCanonical = canonicalStoryMilestoneIdForPublishedRefresh(existing);
  if (existingCanonical === requested) {
    return true;
  }
  return existing === "merge_gate" && requested === "final_outcome";
}

export function canonicalStoryEntryIdForRefresh(
  entryId: string | undefined,
  existing: unknown,
  requested: StoryMilestoneId,
): string | undefined {
  if (!entryId || typeof existing !== "string") {
    return entryId;
  }
  if (!storyMilestoneRefreshesPublishedEntry(existing, requested)) {
    return entryId;
  }
  return entryId.replace(new RegExp(`:${escapeRegExp(existing)}$`, "u"), `:${requested}`);
}

function buildCoreStoryOutboxMetadata(input: StoryOutboxMetadataInput): {
  readonly milestone_kind: StoryMilestoneId;
  readonly idempotency: StoryOutboxIdempotencyMetadata;
  readonly replay: {
    readonly same_key: "update_or_reuse";
    readonly different_milestones: "distinct_entries";
  };
} {
  const milestoneId = assertStoryMilestoneId(input.milestoneId, "outbox_entry.metadata.milestone_kind");
  return {
    milestone_kind: milestoneId,
    idempotency: buildStoryOutboxIdempotencyMetadata({
      ...input,
      milestoneId,
    }),
    replay: {
      same_key: "update_or_reuse",
      different_milestones: "distinct_entries",
    },
  };
}

function buildStoryOutboxIdempotencyMetadata(input: StoryOutboxMetadataInput): StoryOutboxIdempotencyMetadata {
  const milestoneId = assertStoryMilestoneId(input.milestoneId, "outbox_entry.metadata.milestone_kind");
  const sourceThreadRef = assertSourceThreadPublicationAllowed({
    requiresSourceThreadPublication: input.requiresSourceThreadPublication,
    sourceThreadRef: input.sourceThreadRef,
    missingBehavior: "fail_closed",
  });
  const contentHash = hashString(sanitizePublicMarkdown(input.bodyMarkdown)?.trim() ?? "");
  const keyMaterial = {
    source_id: clean(input.sourceId),
    provider: clean(input.provider),
    source_thread_ref: sourceThreadRef,
    workflow_id: clean(input.workflowId),
    lane_id: clean(input.laneId),
    milestone_id: milestoneId,
    target_ref: clean(input.targetRef),
    proposal_id: clean(input.proposalId),
    content_hash: contentHash,
  };
  return {
    key: `story:${hashStable(keyMaterial).slice(0, 32)}`,
    content_hash: contentHash,
  };
}

function renderStoryRefs(input: {
  readonly source_ref?: unknown;
  readonly source_thread_ref?: unknown;
  readonly result_refs?: readonly unknown[];
  readonly publication_refs?: readonly unknown[];
}): string | undefined {
  const sourceRef = clean(input.source_ref);
  const sourceThreadRef = clean(input.source_thread_ref);
  const resultRefs = Array.isArray(input.result_refs)
    ? input.result_refs.map((entry) => clean(entry)).filter(Boolean)
    : [];
  const publicationRefs = Array.isArray(input.publication_refs)
    ? input.publication_refs.map((entry) => clean(entry)).filter(Boolean)
    : [];
  const lines = [
    sourceRef ? `- source_ref: ${sourceRef}` : undefined,
    sourceThreadRef ? `- source_thread_ref: ${sourceThreadRef}` : undefined,
    resultRefs.length > 0 ? `- result_refs: ${resultRefs.join(", ")}` : undefined,
    publicationRefs.length > 0 ? `- publication_refs: ${publicationRefs.join(", ")}` : undefined,
  ].filter(Boolean);
  return lines.length > 0 ? lines.join("\n") : undefined;
}

function clean(value: unknown): string | undefined {
  const sanitized = sanitizePublicMarkdown(value)?.trim();
  return sanitized || undefined;
}

function containsRedactionMarker(value: string): boolean {
  return value.includes("[local-path]") || value.includes("[secret]");
}

function limitLines(value: string, maxLines: number): string {
  const lines = value.split(/\r?\n/u);
  if (lines.length <= maxLines) {
    return value;
  }
  return `${lines.slice(0, maxLines).join("\n")}\n...`;
}

function stableStringify(value: unknown): string {
  if (value === null || typeof value !== "object") {
    return JSON.stringify(value);
  }
  if (Array.isArray(value)) {
    return `[${value.map((item) => stableStringify(item)).join(",")}]`;
  }
  const entries = Object.entries(value as Record<string, unknown>)
    .filter(([, entryValue]) => entryValue !== undefined)
    .sort(([left], [right]) => left.localeCompare(right));
  return `{${entries.map(([key, entryValue]) => `${JSON.stringify(key)}:${stableStringify(entryValue)}`).join(",")}}`;
}

function hashStable(value: unknown): string {
  return hashString(stableStringify(value));
}

function hashString(value: string): string {
  return createHash("sha256").update(value).digest("hex");
}

function escapeRegExp(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}
