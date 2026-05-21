import { readFileSync } from "node:fs";

import { Value } from "@sinclair/typebox/value";
import { describe, expect, it } from "vitest";

import {
  threadOutboxProviderFetchV1Schema,
  threadOutboxProviderManifestV1Schema,
  threadOutboxProviderObservationV1Schema,
  threadOutboxProviderPushV1Schema,
  validateThreadOutboxProviderFetchContract,
  validateThreadOutboxProviderManifestContract,
  validateThreadOutboxProviderObservationContract,
  validateThreadOutboxProviderPushContract,
} from "./thread-outbox-provider.js";

const fixtureRoot = new URL("../../../../fixtures/contracts/thread-outbox-provider/", import.meta.url);

const forbiddenSecretFields = [
  "token",
  "access_token",
  "api_key",
  "secret",
  "password",
  "authorization",
] as const;

describe("thread outbox provider protocol schemas", () => {
  it("validates manifest, push, fetch, and observation frames", () => {
    expect(validateThreadOutboxProviderManifestContract(readExpected("manifest.json")).schema)
      .toBe("runx.thread_outbox_provider.manifest.v1");
    expect(validateThreadOutboxProviderPushContract(readExpected("push.json")).schema)
      .toBe("runx.thread_outbox_provider.push.v1");
    expect(validateThreadOutboxProviderFetchContract(readExpected("fetch.json")).schema)
      .toBe("runx.thread_outbox_provider.fetch.v1");
    expect(validateThreadOutboxProviderObservationContract(readExpected("observation.json")).schema)
      .toBe("runx.thread_outbox_provider.observation.v1");
  });

  it("rejects raw secret-like fields on public frames", () => {
    for (const field of forbiddenSecretFields) {
      expect(Value.Check(threadOutboxProviderManifestV1Schema, {
        ...(readExpected("manifest.json") as Record<string, unknown>),
        [field]: "super-secret-token",
      })).toBe(false);
      expect(Value.Check(threadOutboxProviderPushV1Schema, {
        ...(readExpected("push.json") as Record<string, unknown>),
        [field]: "super-secret-token",
      })).toBe(false);
      expect(Value.Check(threadOutboxProviderFetchV1Schema, {
        ...(readExpected("fetch.json") as Record<string, unknown>),
        [field]: "super-secret-token",
      })).toBe(false);
      expect(Value.Check(threadOutboxProviderObservationV1Schema, {
        ...(readExpected("observation.json") as Record<string, unknown>),
        [field]: "super-secret-token",
      })).toBe(false);
    }
  });

  it("rejects raw secret-like fields inside nested credential and provider frames", () => {
    const push = {
      ...(readExpected("push.json") as Record<string, unknown>),
      provider_profile: {
        ...((readExpected("push.json") as Record<string, Record<string, unknown>>).provider_profile),
        access_token: "super-secret-token",
      },
    };
    const observation = {
      ...(readExpected("observation.json") as Record<string, unknown>),
      readback_summary: {
        ...((readExpected("observation.json") as Record<string, Record<string, unknown>>).readback_summary),
        authorization: "Bearer super-secret-token",
      },
    };

    expect(Value.Check(threadOutboxProviderPushV1Schema, push)).toBe(false);
    expect(() => validateThreadOutboxProviderPushContract(push)).toThrow();
    expect(Value.Check(threadOutboxProviderObservationV1Schema, observation)).toBe(false);
    expect(() => validateThreadOutboxProviderObservationContract(observation)).toThrow();
  });

  it("rejects provider mutations without source-thread routing", () => {
    const push = { ...(readExpected("push.json") as Record<string, unknown>) };
    delete push.thread_locator;

    expect(Value.Check(threadOutboxProviderPushV1Schema, push)).toBe(false);
    expect(() => validateThreadOutboxProviderPushContract(push)).toThrow();
  });

  it("rejects readback requests without an explicit target", () => {
    const fetch = { ...(readExpected("fetch.json") as Record<string, unknown>) };
    delete fetch.target;

    expect(Value.Check(threadOutboxProviderFetchV1Schema, fetch)).toBe(false);
    expect(() => validateThreadOutboxProviderFetchContract(fetch)).toThrow();
  });

  it("keeps v1 provider adapters process-supervised", () => {
    const manifest = {
      ...(readExpected("manifest.json") as Record<string, unknown>),
      transport: {
        kind: "http",
        endpoint: "https://example.test/thread-provider",
      },
    };

    expect(Value.Check(threadOutboxProviderManifestV1Schema, manifest)).toBe(false);
    expect(() => validateThreadOutboxProviderManifestContract(manifest)).toThrow();
  });

  it("rejects unknown fields on all top-level frame shapes", () => {
    expect(Value.Check(threadOutboxProviderManifestV1Schema, withExtra("manifest.json"))).toBe(false);
    expect(Value.Check(threadOutboxProviderPushV1Schema, withExtra("push.json"))).toBe(false);
    expect(Value.Check(threadOutboxProviderFetchV1Schema, withExtra("fetch.json"))).toBe(false);
    expect(Value.Check(threadOutboxProviderObservationV1Schema, withExtra("observation.json"))).toBe(false);
  });
});

function readExpected(fixtureName: string): unknown {
  const fixture = JSON.parse(readFileSync(new URL(fixtureName, fixtureRoot), "utf8")) as {
    readonly expected: unknown;
  };
  return fixture.expected;
}

function withExtra(fixtureName: string): unknown {
  return {
    ...(readExpected(fixtureName) as Record<string, unknown>),
    unexpected: true,
  };
}
