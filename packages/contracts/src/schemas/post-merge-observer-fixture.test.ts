import { readFileSync } from "node:fs";

import { describe, expect, it } from "vitest";

import { validateHarnessReceiptContract } from "./spine.js";

const fixtureUrl = new URL(
  "../../../../fixtures/contracts/harness-spine/post-merge-observer-merged-verified.json",
  import.meta.url,
);

describe("post-merge observer harness fixture", () => {
  it("validates the merged verified closure receipt without retired peer packets", () => {
    const rawFixture = readFileSync(fixtureUrl, "utf8");
    const fixture = JSON.parse(rawFixture) as { readonly expected: unknown };
    const receipt = validateHarnessReceiptContract(fixture.expected, "post-merge observer fixture");

    expect(receipt.seal.reason_code).toBe("merged_verified");
    expect(receipt.harness.idempotency.intent_key).toBe(
      "post-merge:github://runxhq/nitrosend/issues/77:github://runxhq/nitrosend/pulls/188",
    );
    expect(receipt.harness.acts.map((act) => act.form)).toEqual([
      "observation",
      "verification",
      "reply",
      "revision",
    ]);

    const sealCriteria = receipt.seal.criteria.map((criterion) => criterion.criterion_id);
    expect(sealCriteria).toEqual([
      "post_merge.provider_state",
      "post_merge.human_gate",
      "post_merge.verification_passed",
      "post_merge.source_thread_target_present",
      "post_merge.close_policy_authorized",
    ]);

    const replyAct = receipt.harness.acts.find((act) => act.act_id === "act_publish_final_threads");
    expect(replyAct?.target_refs.some((ref) => {
      return ref.type === "slack_thread"
        && typeof ref.locator === "string"
        && ref.locator.split("/").length >= 3;
    })).toBe(true);

    const publicationCriterion = receipt.seal.criteria.find((criterion) => {
      return criterion.criterion_id === "post_merge.source_thread_target_present";
    });
    expect(publicationCriterion?.verification_refs).toHaveLength(1);
    expect(publicationCriterion?.evidence_refs.some((ref) => ref.type === "slack_thread")).toBe(true);

    for (const retiredToken of [
      ["runx.issue", "_to_pr_", "outcome.v1"].join(""),
      ["issue", "_to_pr_", "outcome"].join(""),
      "effect",
      ["verification", "_", "report"].join(""),
      ["verification", "-", "report"].join(""),
      ["target", "_", "outcome"].join(""),
    ]) {
      expect(rawFixture).not.toContain(retiredToken);
    }
  });
});
