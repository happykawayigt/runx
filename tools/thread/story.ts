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
