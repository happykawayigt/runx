#!/usr/bin/env node
import { spawnSync } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  buildCreateFrame,
  buildLifecycleFrame,
  buildMessageFrame,
} from "../tools/thread/thread_desired_state.mjs";
import { firstNonEmptyString, parseGitHubIssueRef, prune } from "../tools/thread/github_adapter.mjs";

// Generic declarative thread-mirror driver. It pulls a tenant's desired thread
// state (provider-agnostic, no domain meaning), reconciles each thread's provider
// representation to match via the governed thread_outbox provider, and reports
// observations back so the tenant can re-link the resulting threads. It is
// stateless and idempotent: every run compares desired vs live and applies only
// the difference, so there is no cursor and drift self-heals on the next tick.

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const PROVIDER_SCRIPT = path.join(__dirname, "../tools/thread/thread_outbox_provider/github-provider.mjs");

async function main() {
  const config = readConfig(process.env);
  const payload = await fetchDesiredState(config);
  const threads = Array.isArray(payload.threads) ? payload.threads : [];
  const observations = [];
  const results = [];

  for (const thread of threads) {
    try {
      const outcome = reconcileThread(thread, config);
      results.push(outcome.summary);
      if (outcome.observation) observations.push(outcome.observation);
    } catch (error) {
      results.push({ identity_key: thread?.identity_key, error: messageOf(error) });
      if (!config.continueOnError) throw error;
    }
  }

  if (!config.dryRun && observations.length > 0) {
    await postObservations(config, observations);
  }

  process.stdout.write(`${JSON.stringify({
    ok: true,
    reconciled: threads.length,
    observed: observations.length,
    results,
  })}\n`);
}

function reconcileThread(thread, config) {
  if (config.dryRun) {
    return {
      summary: {
        identity_key: thread.identity_key,
        would_create: !firstNonEmptyString(thread.thread_locator),
        state: thread.state,
        labels: Array.isArray(thread.labels) ? thread.labels.length : 0,
        comments: Array.isArray(thread.comments) ? thread.comments.length : 0,
        dry_run: true,
      },
    };
  }

  // 1. Locate or create. An existing locator is the source's hint; without one we
  //    find-or-create by the identity marker, so a brand-new thread is created and
  //    a lost link self-heals to the existing issue.
  let locator = firstNonEmptyString(thread.thread_locator);
  let providerThreadId;
  let created = false;
  if (!locator) {
    const pushed = runProvider(buildCreateFrame(thread, frameOptions(config)), config.env);
    locator = threadLocatorFrom(pushed);
    providerThreadId = providerThreadIdFrom(pushed);
    created = true;
    if (!locator) {
      throw new Error(`provider create yielded no locator for ${thread.identity_key}`);
    }
  }

  // 2. Reconcile labels + open/closed to the desired state (the engine's core
  //    job; idempotent, so a no-op when already correct).
  const lifecycle = runProvider(buildLifecycleFrame(thread, locator, frameOptions(config)), config.env);
  locator = threadLocatorFrom(lifecycle) ?? locator;
  providerThreadId = providerThreadId ?? providerThreadIdFrom(lifecycle);

  // 3. Append any comment not already on the thread (provider dedupes by marker).
  const comments = Array.isArray(thread.comments) ? thread.comments : [];
  for (const comment of comments) {
    runProvider(buildMessageFrame(thread, comment, locator, frameOptions(config)), config.env);
  }

  return {
    summary: {
      identity_key: thread.identity_key,
      locator,
      created,
      state: thread.state,
      comments: comments.length,
    },
    observation: buildObservation(thread, locator, providerThreadId),
  };
}

function buildObservation(thread, locator, providerThreadId) {
  if (!locator) return undefined;
  const issueRef = safeIssueRef(locator);
  return prune({
    schema_version: 1,
    provider: thread.provider,
    target_repo: thread.target_repo,
    identity_key: thread.identity_key,
    thread_locator: issueRef?.thread_locator ?? locator,
    thread_url: issueRef?.issue_url ?? locator,
    provider_thread_id: firstNonEmptyString(providerThreadId, issueRef?.issue_number),
    ref: thread.ref,
    observed_at: new Date().toISOString(),
  });
}

function frameOptions(config) {
  return { adapterId: config.adapterId, sourceId: config.sourceId };
}

function readConfig(env) {
  const apiBaseUrl = trim(env.THREAD_SYNC_API_BASE_URL);
  if (!apiBaseUrl) {
    throw new Error("THREAD_SYNC_API_BASE_URL is required.");
  }
  const internalSecret = trim(env.THREAD_SYNC_INTERNAL_SECRET);
  if (!internalSecret) {
    throw new Error("THREAD_SYNC_INTERNAL_SECRET is required.");
  }
  return {
    apiBaseUrl: apiBaseUrl.replace(/\/+$/, ""),
    internalSecret,
    provider: trim(env.THREAD_SYNC_PROVIDER) ?? "github",
    targetRepo: trim(env.THREAD_SYNC_TARGET_REPO),
    sourceId: trim(env.THREAD_SYNC_SOURCE_ID) ?? "tenant",
    adapterId: trim(env.THREAD_SYNC_ADAPTER_ID) ?? "runx-github-thread-adapter",
    limit: positiveInteger(env.THREAD_SYNC_LIMIT, 50),
    dryRun: env.THREAD_SYNC_DRY_RUN === "1" || env.THREAD_SYNC_DRY_RUN === "true",
    continueOnError: env.THREAD_SYNC_FAIL_FAST !== "1" && env.THREAD_SYNC_FAIL_FAST !== "true",
    env,
  };
}

async function fetchDesiredState(config) {
  const url = new URL("/internal/thread-desired-state", config.apiBaseUrl);
  url.searchParams.set("provider", config.provider);
  url.searchParams.set("limit", String(config.limit));
  if (config.targetRepo) {
    url.searchParams.set("target_repo", config.targetRepo);
  }
  const response = await fetch(url, {
    headers: { authorization: `Bearer ${config.internalSecret}` },
  });
  if (!response.ok) {
    throw new Error(`thread desired-state fetch failed: ${response.status} ${await response.text()}`);
  }
  return response.json();
}

async function postObservations(config, observations) {
  const response = await fetch(new URL("/internal/thread-state-observations", config.apiBaseUrl), {
    method: "POST",
    headers: {
      authorization: `Bearer ${config.internalSecret}`,
      "content-type": "application/json",
    },
    body: JSON.stringify({ schema_version: 1, observations }),
  });
  if (!response.ok) {
    throw new Error(`thread observation post failed: ${response.status} ${await response.text()}`);
  }
}

function runProvider(frame, env) {
  const child = spawnSync(process.execPath, [PROVIDER_SCRIPT], {
    input: JSON.stringify(frame),
    encoding: "utf8",
    env: env ?? process.env,
  });
  if (child.status !== 0) {
    throw new Error([
      `thread provider failed for ${frame.outbox_entry_id}.`,
      child.stderr?.trim(),
      child.stdout?.trim(),
    ].filter(Boolean).join("\n"));
  }
  return JSON.parse(child.stdout);
}

function threadLocatorFrom(pushed) {
  const output = pushed?.output ?? {};
  return firstNonEmptyString(
    output.outbox_entry?.thread_locator,
    output.push?.provider_thread?.thread_locator,
    output.push?.lifecycle?.locator && safeIssueRef(output.push.lifecycle.locator)?.thread_locator,
  );
}

function providerThreadIdFrom(pushed) {
  const output = pushed?.output ?? {};
  return firstNonEmptyString(
    output.outbox_entry?.metadata?.provider_thread_id,
    output.push?.provider_thread?.issue_number,
  );
}

function safeIssueRef(locator) {
  try {
    return parseGitHubIssueRef(locator);
  } catch {
    return undefined;
  }
}

function messageOf(error) {
  return error instanceof Error ? error.message : String(error);
}

function positiveInteger(value, fallback) {
  const parsed = Number.parseInt(String(value ?? ""), 10);
  return Number.isFinite(parsed) && parsed > 0 ? parsed : fallback;
}

function trim(value) {
  return typeof value === "string" && value.trim().length > 0 ? value.trim() : undefined;
}

main().catch((error) => {
  console.error(messageOf(error));
  process.exit(1);
});
