import { readFileSync } from "node:fs";

import { describe, expect, it } from "vitest";

import { validateReceiptContract } from "./receipt.js";

const fixtureUrl = new URL(
  "../../../../fixtures/contracts/harness-spine/post-merge-observer-merged-verified.json",
  import.meta.url,
);

describe("post-merge observer flat receipt fixture", () => {
  it("validates the merged verified closure receipt without retired peer packets", () => {
    const rawFixture = readFileSync(fixtureUrl, "utf8");
    const fixture = JSON.parse(rawFixture) as { readonly expected: unknown };
    const receipt = validateReceiptContract(fixture.expected, "post-merge observer fixture");

    expect(receipt.seal.reason_code).toBe("merged_verified");
    expect(receipt.idempotency.intent_key).toBe(
      "post-merge:github://runxhq/nitrosend/issues/77:github://runxhq/nitrosend/pulls/188",
    );
    expect(receipt.acts.map((act) => act.form)).toEqual([
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

    const publicationCriterion = receipt.seal.criteria.find((criterion) => {
      return criterion.criterion_id === "post_merge.source_thread_target_present";
    });
    expect(publicationCriterion?.verification_refs).toHaveLength(1);
    expect(publicationCriterion?.evidence_refs.some((ref) => ref.type === "slack_thread")).toBe(true);

    for (const retiredToken of [
      ["harness", "_receipt"].join(""),
      ["verification", "_", "summary"].join(""),
    ]) {
      expect(rawFixture).not.toContain(retiredToken);
    }
  });
});
