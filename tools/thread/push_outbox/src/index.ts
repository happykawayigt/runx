import { mkdir, readFile, rename, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

import { canonicalJsonStringify, sha256Hex } from "@runxhq/contracts";
import {
  artifact,
  defineTool,
  optionalArtifact,
  prune,
  recordInput,
  stringInput,
} from "@runxhq/authoring";
import {
  fetchGitHubIssueThread,
  firstNonEmptyString,
  isRecord,
  optionalRecord,
  pushGitHubMessage,
  pushGitHubPullRequest,
} from "../../github_adapter.mjs";
import {
  assertStoryMilestoneId,
} from "../../story.js";

const githubPublishEnvAllowlist = [
  "PATH",
  "TMPDIR",
  "TMP",
  "TEMP",
  "GH_TOKEN",
  "GITHUB_TOKEN",
  "RUNX_GITHUB_TOKEN",
  "RUNX_GIT_AUTHOR_NAME",
  "RUNX_GIT_AUTHOR_EMAIL",
  "GIT_AUTHOR_NAME",
  "GIT_AUTHOR_EMAIL",
  "GIT_COMMITTER_NAME",
  "GIT_COMMITTER_EMAIL",
  "GITHUB_ACTIONS",
];

export default defineTool({
  name: "thread.push_outbox",
  description: "Push an outbox entry through the current thread adapter and return the refreshed thread.",
  source: {
    type: "cli-tool",
    command: "node",
    args: ["./run.mjs"],
    sandbox: {
      profile: "workspace-write",
      cwd_policy: "skill-directory",
      env_allowlist: githubPublishEnvAllowlist,
      network: true,
      writable_paths: ["{{workspace_path}}", "{{fixture}}"],
    },
  },
  inputs: {
    thread: recordInput({ optional: true, description: "Current hydrated thread for the bounded provider surface." }),
    outbox_entry: artifact({ description: "Outbox entry to push through the thread adapter." }),
    draft_pull_request: optionalArtifact({ description: "Provider-agnostic draft pull-request packet paired with the outbox entry." }),
    fixture: stringInput({ optional: true, description: "Optional governed workspace root inherited from issue-to-pr style lanes." }),
    workspace_path: stringInput({ optional: true, description: "Optional workspace root used by adapters that need local git state to publish outputs upstream." }),
    next_status: stringInput({ optional: true, description: "Optional status to apply after a successful provider push, such as `draft`." }),
  },
  scopes: ["thread:push"],
  run: runPushOutbox,
});

async function runPushOutbox({ inputs, env }) {
  const thread = isRecord(inputs.thread) ? inputs.thread : undefined;
  const outboxEntry = inputs.outbox_entry;
  const draftPullRequest = inputs.draft_pull_request;
  const sourceThread = sourceThreadRequirement(outboxEntry);
  const nextStatus = firstNonEmptyString(inputs.next_status);
  const workspacePath = firstNonEmptyString(
    inputs.workspace_path,
    inputs.fixture,
    env.RUNX_CWD,
  );

  if (!thread) {
    if (sourceThread?.required) {
      throw new Error("thread is required for this outbox entry; source_thread.missing_behavior is fail_closed.");
    }
    return {
      draft_pull_request: draftPullRequest,
      outbox_entry: outboxEntry,
      thread: null,
      push: {
        status: "skipped",
        reason: "thread not provided",
      },
    };
  }

  const adapter = isRecord(thread.adapter) ? thread.adapter : {};
  const adapterType = firstNonEmptyString(adapter.type);
  const adapterRef = firstNonEmptyString(adapter.adapter_ref);
  const outboxKind = firstNonEmptyString(outboxEntry.kind);
  validateStoryMessageMilestone(outboxEntry);
  const pullRequestControlMetadata = buildPullRequestControlMetadata({
    outboxEntry,
    draftPullRequest,
  });

  validateRequiredSourceThread({
    sourceThread,
    thread,
    outboxEntry,
  });

  if (!adapterType) {
    throw new Error("thread.adapter.type is required.");
  }

  if (adapterType === "github") {
    if (!adapterRef) {
      return {
        draft_pull_request: draftPullRequest,
        outbox_entry: outboxEntry,
        thread: thread,
        push: {
          status: "skipped",
          reason: `thread adapter '${adapterType}' requires adapter_ref.`,
          adapter: {
            type: adapterType,
          },
        },
      };
    }
    if (outboxKind === "message") {
      const pushed = pushGitHubMessage({
        thread,
        outboxEntry,
        workspacePath,
        nextStatus,
        env,
      });
      const refreshedState = fetchGitHubIssueThread({
        adapterRef,
        env,
        cwd: workspacePath,
      });
      const refreshedOutboxEntry =
        selectMatchingOutboxEntry(refreshedState, pushed.outbox_entry) ??
        pushed.outbox_entry;

      return {
        draft_pull_request: draftPullRequest,
        outbox_entry: refreshedOutboxEntry,
        thread: refreshedState,
        push: {
          status: "pushed",
          adapter: {
            type: adapterType,
            adapter_ref: adapterRef,
          },
          pushed_at: firstNonEmptyString(
            optionalRecord(refreshedOutboxEntry.metadata)?.pushed_at,
          ),
          message: {
            locator: firstNonEmptyString(
              refreshedOutboxEntry.locator,
              optionalRecord(refreshedOutboxEntry.metadata)?.locator,
            ),
            comment_id: firstNonEmptyString(
              optionalRecord(refreshedOutboxEntry.metadata)?.comment_id,
            ),
          },
        },
      };
    }
    if (!workspacePath) {
      return {
        draft_pull_request: draftPullRequest,
        outbox_entry: outboxEntry,
        thread: thread,
        push: {
          status: "skipped",
          reason: "workspace_path is required for the GitHub thread adapter.",
          adapter: {
            type: adapterType,
            adapter_ref: adapterRef,
          },
        },
      };
    }
    if (outboxKind !== "pull_request") {
      return {
        draft_pull_request: draftPullRequest,
        outbox_entry: outboxEntry,
        thread: thread,
        push: {
          status: "skipped",
          reason: `GitHub thread adapter does not support outbox kind '${outboxKind}'.`,
          adapter: {
            type: adapterType,
            adapter_ref: adapterRef,
          },
        },
      };
    }
    if (!draftPullRequest) {
      return {
        outbox_entry: outboxEntry,
        thread: thread,
        push: {
          status: "skipped",
          reason:
            "draft_pull_request is required to push through the GitHub thread adapter.",
          adapter: {
            type: adapterType,
            adapter_ref: adapterRef,
          },
        },
      };
    }
    const pushed = pushGitHubPullRequest({
      thread,
      draftPullRequest,
      outboxEntry,
      workspacePath,
      nextStatus,
      env,
    });
    const refreshedState = fetchGitHubIssueThread({
      adapterRef,
      env,
      cwd: workspacePath,
    });
    const refreshedOutboxEntry =
      selectMatchingOutboxEntry(refreshedState, pushed.outbox_entry) ??
      pushed.outbox_entry;
    const returnedOutboxEntry = preservePullRequestControlMetadata(
      refreshedOutboxEntry,
      pullRequestControlMetadata,
    );
    const returnedThread = replaceThreadOutboxEntry(refreshedState, returnedOutboxEntry);

    return {
      draft_pull_request: draftPullRequest,
      outbox_entry: returnedOutboxEntry,
      thread: returnedThread,
      push: {
        status: "pushed",
        adapter: {
          type: adapterType,
          adapter_ref: adapterRef,
        },
        pushed_at: firstNonEmptyString(
          optionalRecord(returnedOutboxEntry.metadata)?.pushed_at,
        ),
        pull_request: {
          number: firstNonEmptyString(pushed.pull_request.number),
          url: firstNonEmptyString(pushed.pull_request.url),
        },
      },
    };
  }

  if (adapterType !== "file") {
    return {
      draft_pull_request: draftPullRequest,
      outbox_entry: outboxEntry,
      thread: thread,
      push: {
        status: "skipped",
        reason: `no thread adapter is registered for '${adapterType}'`,
        adapter: {
          type: adapterType,
        },
      },
    };
  }

  if (!adapterRef) {
    throw new Error(`thread adapter '${adapterType}' requires adapter_ref.`);
  }

  const statePath = resolveAdapterRefPath(adapterRef);
  const adapterUri = pathToFileURL(statePath).href;
  const currentState = asRecord(
    JSON.parse(await readFile(statePath, "utf8")),
    "thread_file",
  );
  const threadLocator = firstNonEmptyString(
    outboxEntry.thread_locator,
    currentState.thread_locator,
  );

  if (!threadLocator) {
    throw new Error("thread locator is required to push an outbox entry.");
  }

  const existingOutbox = Array.isArray(currentState.outbox)
    ? currentState.outbox.filter(isRecord)
    : [];
  const existing = existingOutbox.find(
    (candidate) =>
      candidate.entry_id === outboxEntry.entry_id ||
      (firstNonEmptyString(outboxEntry.locator) &&
        candidate.locator === outboxEntry.locator),
  );
  const pushedAt = new Date().toISOString();
  const pushedEntry = preservePullRequestControlMetadata({
    ...existing,
    ...outboxEntry,
    locator: firstNonEmptyString(
      outboxEntry.locator,
      existing?.locator,
      `${adapterUri}#outbox/${encodeURIComponent(String(outboxEntry.entry_id || ""))}`,
    ),
    status: firstNonEmptyString(
      nextStatus,
      outboxEntry.status,
      existing?.status,
      "draft",
    ),
    thread_locator: threadLocator,
  }, pullRequestControlMetadata);

  const pushEvent = {
    entry_id: `entry_${opaqueCanonicalJsonHashFragment({
      thread_locator: threadLocator,
      outbox_entry_id: pushedEntry.entry_id,
      pushed_at: pushedAt,
    }, 24)}`,
    entry_kind: "status",
    recorded_at: pushedAt,
    body: `Pushed ${pushedEntry.kind} ${pushedEntry.entry_id}`,
    structured_data: {
      event: "push_outbox_entry",
      outbox_entry_id: pushedEntry.entry_id,
      kind: pushedEntry.kind,
      locator: pushedEntry.locator,
      status: pushedEntry.status,
    },
    source_ref: {
      type: "thread_adapter",
      uri: adapterUri,
      recorded_at: pushedAt,
    },
  };
  const refreshedState = {
    ...currentState,
    adapter: {
      ...adapter,
      adapter_ref: adapterRef,
      cursor: `push:${opaqueCanonicalJsonHashFragment({ outbox_entry: pushedEntry.entry_id, pushed_at: pushedAt }, 12)}`,
    },
    entries: [
      ...(Array.isArray(currentState.entries) ? currentState.entries : []),
      pushEvent,
    ],
    outbox: upsertOutboxEntry(existingOutbox, pushedEntry),
    generated_at: pushedAt,
    watermark: pushedEntry.entry_id,
  };

  await writeThreadFile(statePath, refreshedState);

  return {
    draft_pull_request: draftPullRequest,
    outbox_entry: pushedEntry,
    thread: refreshedState,
    push: {
      status: "pushed",
      adapter: {
        type: adapterType,
        adapter_ref: adapterRef,
      },
      pushed_at: pushedAt,
    },
  };
}

function asRecord(value, label) {
  if (!isRecord(value)) {
    throw new Error(`${label} must be an object.`);
  }
  return value;
}

function resolveAdapterRefPath(adapterRefValue) {
  if (adapterRefValue.startsWith("file://")) {
    return path.resolve(fileURLToPath(adapterRefValue));
  }
  return path.resolve(adapterRefValue);
}

function selectMatchingOutboxEntry(threadValue, pushedEntry) {
  const outbox = Array.isArray(threadValue?.outbox)
    ? threadValue.outbox.filter(isRecord)
    : [];
  const pushedKind = firstNonEmptyString(pushedEntry.kind);
  const pushedReceiptId = firstNonEmptyString(optionalRecord(pushedEntry.metadata)?.outbox_receipt_id);
  return outbox.find(
    (candidate) => {
      if (firstNonEmptyString(pushedEntry.locator) && candidate.locator === pushedEntry.locator) {
        return true;
      }
      if (candidate.entry_id !== pushedEntry.entry_id) {
        return false;
      }
      if (pushedKind !== "message") {
        return true;
      }
      const candidateReceiptId = firstNonEmptyString(optionalRecord(candidate.metadata)?.outbox_receipt_id);
      return Boolean(pushedReceiptId && candidateReceiptId === pushedReceiptId);
    },
  );
}

function replaceThreadOutboxEntry(threadValue, pushedEntry) {
  if (!Array.isArray(threadValue?.outbox)) {
    return threadValue;
  }
  let replaced = false;
  const outbox = threadValue.outbox.map((candidate) => {
    if (isRecord(candidate) && outboxEntriesMatch(candidate, pushedEntry)) {
      replaced = true;
      return pushedEntry;
    }
    return candidate;
  });
  return {
    ...threadValue,
    outbox: replaced ? outbox : [...outbox, pushedEntry],
  };
}

function outboxEntriesMatch(candidate, pushedEntry) {
  if (firstNonEmptyString(pushedEntry.locator) && candidate.locator === pushedEntry.locator) {
    return true;
  }
  return Boolean(pushedEntry.entry_id && candidate.entry_id === pushedEntry.entry_id);
}

function buildPullRequestControlMetadata({ outboxEntry, draftPullRequest }) {
  if (firstNonEmptyString(outboxEntry?.kind) !== "pull_request") {
    return undefined;
  }
  const metadata = optionalRecord(outboxEntry.metadata) ?? {};
  const draft = optionalRecord(draftPullRequest) ?? {};
  const target = optionalRecord(draft.target) ?? {};
  const governance = optionalRecord(draft.governance) ?? {};
  return prune({
    schema_version: safeMetadataString(metadata.schema_version, "outbox_entry.metadata.schema_version"),
    packet_schema_version: safeMetadataString(metadata.packet_schema_version, "outbox_entry.metadata.packet_schema_version"),
    action: safeMetadataString(metadata.action, "outbox_entry.metadata.action"),
    task_id: safeMetadataString(metadata.task_id, draft.task_id, "outbox_entry.metadata.task_id"),
    repo: safeMetadataString(metadata.repo, target.repo, "outbox_entry.metadata.repo"),
    branch: safeMetadataString(metadata.branch, target.branch, "outbox_entry.metadata.branch"),
    base: safeMetadataString(metadata.base, target.base, "outbox_entry.metadata.base"),
    harness_context: optionalRecord(metadata.harness_context),
    operational_policy: optionalRecord(metadata.operational_policy),
    title: safeMetadataString(metadata.title, "outbox_entry.metadata.title"),
    review_verdict: safeMetadataString(metadata.review_verdict, "outbox_entry.metadata.review_verdict"),
    check_status: safeMetadataString(metadata.check_status, "outbox_entry.metadata.check_status"),
    sync_status: safeMetadataString(metadata.sync_status, "outbox_entry.metadata.sync_status"),
    push_ready: typeof metadata.push_ready === "boolean" ? metadata.push_ready : undefined,
    changed_files: normalizeChangedFiles(metadata.changed_files ?? governance.changed_files),
    dedupe: normalizeDedupeMetadata(optionalRecord(metadata.dedupe)),
    source_thread: normalizeSourceThreadMetadata(optionalRecord(metadata.source_thread)),
    human_merge_gate: safeMetadataString(metadata.human_merge_gate, "outbox_entry.metadata.human_merge_gate"),
    post_merge_observation: safeMetadataString(metadata.post_merge_observation, "outbox_entry.metadata.post_merge_observation"),
    story_milestones: normalizeStoryMilestones(metadata.story_milestones, "outbox_entry.metadata.story_milestones"),
  });
}

function preservePullRequestControlMetadata(pushedEntry, controlMetadata) {
  if (!controlMetadata || firstNonEmptyString(pushedEntry?.kind) !== "pull_request") {
    return pushedEntry;
  }
  const providerMetadata = optionalRecord(pushedEntry.metadata) ?? {};
  return prune({
    ...pushedEntry,
    metadata: prune({
      ...controlMetadata,
      ...providerMetadata,
      changed_files: controlMetadata.changed_files ?? normalizeChangedFiles(providerMetadata.changed_files),
      dedupe: controlMetadata.dedupe ?? normalizeDedupeMetadata(optionalRecord(providerMetadata.dedupe)),
      source_thread: controlMetadata.source_thread ?? normalizeSourceThreadMetadata(optionalRecord(providerMetadata.source_thread)),
      harness_context: controlMetadata.harness_context ?? optionalRecord(providerMetadata.harness_context),
      operational_policy: controlMetadata.operational_policy ?? optionalRecord(providerMetadata.operational_policy),
    }),
  });
}

function normalizeSourceThreadMetadata(sourceThread) {
  if (!sourceThread) {
    return undefined;
  }
  const threadLocator = safeMetadataString(sourceThread.thread_locator, "source_thread.thread_locator");
  return prune({
    required: sourceThread.required === true,
    publish_mode: safeMetadataString(sourceThread.publish_mode, "source_thread.publish_mode"),
    missing_behavior: safeMetadataString(sourceThread.missing_behavior, "source_thread.missing_behavior"),
    thread_locator: threadLocator,
  });
}

function normalizeDedupeMetadata(dedupe) {
  if (!dedupe) {
    return undefined;
  }
  return prune({
    strategy: safeMetadataString(dedupe.strategy, "outbox_entry.metadata.dedupe.strategy"),
    key: safeMetadataString(dedupe.key, "outbox_entry.metadata.dedupe.key"),
    result: safeMetadataString(dedupe.result, "outbox_entry.metadata.dedupe.result"),
    existing_entry_id: safeMetadataString(dedupe.existing_entry_id, "outbox_entry.metadata.dedupe.existing_entry_id"),
    existing_locator: safeMetadataString(dedupe.existing_locator, "outbox_entry.metadata.dedupe.existing_locator"),
  });
}

function normalizeChangedFiles(value) {
  if (!Array.isArray(value)) {
    return undefined;
  }
  const paths = value
    .map((entry) => safeMetadataString(entry, "changed_files"))
    .filter((entry) => entry !== undefined)
    .map((entry) => normalizeChangedFilePath(entry));
  return paths.length > 0 ? [...new Set(paths)] : undefined;
}

function normalizeChangedFilePath(value) {
  const normalized = value.trim().replace(/\\/g, "/").replace(/^\.\/+/, "");
  if (
    normalized.length === 0 ||
    normalized.startsWith("/") ||
    /^[A-Za-z]:\//.test(normalized) ||
    normalized.split("/").some((segment) => segment === ".." || segment.length === 0)
  ) {
    throw new Error("outbox changed file path must be a relative path inside the workspace.");
  }
  return normalized;
}

function safeStringArray(value, label) {
  if (!Array.isArray(value)) {
    return undefined;
  }
  const items = value
    .map((entry) => safeMetadataString(entry, label))
    .filter((entry) => entry !== undefined);
  return items.length > 0 ? items : undefined;
}

function normalizeStoryMilestones(value, label) {
  const items = safeStringArray(value, label);
  if (!items) {
    return undefined;
  }
  return items.map((item) => assertStoryMilestoneId(item, label));
}

function validateStoryMessageMilestone(outboxEntry) {
  if (firstNonEmptyString(outboxEntry?.kind) !== "message") {
    return;
  }
  const metadata = optionalRecord(outboxEntry.metadata) ?? {};
  const milestone = firstNonEmptyString(metadata.milestone_kind);
  if (!milestone) {
    return;
  }
  assertStoryMilestoneId(milestone, "outbox_entry.metadata.milestone_kind");
}

function safeMetadataString(...values) {
  const label = values.pop();
  const value = firstNonEmptyString(...values);
  if (!value) {
    return undefined;
  }
  if (containsLocalFilesystemPath(value)) {
    throw new Error(`${label} must not contain local filesystem paths.`);
  }
  if (containsSecretMaterial(value)) {
    throw new Error(`${label} must not contain secret material.`);
  }
  return value;
}

function containsLocalFilesystemPath(value) {
  return /(?:^|[\s=("'`])(?:\/Users|\/home|\/var|\/private|\/tmp)\/[^\s)"'`]+/.test(value) ||
    /[A-Za-z]:\\[^\s)"'`]+/.test(value);
}

function containsSecretMaterial(value) {
  return /\b(gh[pousr]_[A-Za-z0-9_]{20,}|xox[baprs]-[A-Za-z0-9-]{20,}|sk-(?:proj-)?[A-Za-z0-9_-]{16,})\b/.test(value) ||
    /\b((?:bearer|authorization)\s+)[A-Za-z0-9._:-]{6,}\b/i.test(value) ||
    /\b[A-Z][A-Z0-9_]*(?:TOKEN|SECRET|PASSWORD|API[_-]?KEY|MATERIAL[_-]?REF)[A-Z0-9_]*=/.test(value);
}

function sourceThreadRequirement(outboxEntry) {
  const metadata = optionalRecord(outboxEntry?.metadata) ?? {};
  const sourceThread = optionalRecord(metadata.source_thread);
  if (!sourceThread) {
    return undefined;
  }
  return {
    required: sourceThread.required === true,
    publishMode: firstNonEmptyString(sourceThread.publish_mode),
    threadLocator: firstNonEmptyString(sourceThread.thread_locator),
    missingBehavior: firstNonEmptyString(sourceThread.missing_behavior),
  };
}

function validateRequiredSourceThread({
  sourceThread,
  thread,
  outboxEntry,
}) {
  if (!sourceThread?.required) {
    return;
  }
  if (sourceThread.missingBehavior !== "fail_closed") {
    throw new Error("source_thread.missing_behavior must be fail_closed for required source-thread publishing.");
  }
  const outboxThreadLocator = firstNonEmptyString(outboxEntry.thread_locator);
  const threadLocator = firstNonEmptyString(thread.thread_locator);
  const requiredThreadLocator = firstNonEmptyString(sourceThread.threadLocator);
  if (!firstNonEmptyString(requiredThreadLocator, outboxThreadLocator, threadLocator)) {
    throw new Error("source_thread.thread_locator is required for required source-thread publishing.");
  }
  if (requiredThreadLocator && outboxThreadLocator && requiredThreadLocator !== outboxThreadLocator) {
    throw new Error("outbox_entry.thread_locator must match source_thread.thread_locator.");
  }
  if (requiredThreadLocator && threadLocator && requiredThreadLocator !== threadLocator) {
    throw new Error("thread.thread_locator must match source_thread.thread_locator.");
  }
  if (sourceThread.publishMode === "none") {
    throw new Error("source_thread.publish_mode cannot be none when source_thread.required is true.");
  }
}

function upsertOutboxEntry(existingEntries, entry) {
  const filtered = existingEntries.filter(
    (candidate) =>
      candidate.entry_id !== entry.entry_id &&
      candidate.locator !== entry.locator,
  );
  return [...filtered, entry];
}

async function writeThreadFile(statePath, state) {
  await mkdir(path.dirname(statePath), { recursive: true });
  const tempPath = `${statePath}.${process.pid}.${Date.now()}.${Math.random().toString(36).slice(2)}.tmp`;
  await writeFile(tempPath, `${JSON.stringify(state, null, 2)}\n`, {
    mode: 0o600,
  });
  await rename(tempPath, statePath);
}

function opaqueCanonicalJsonHashFragment(value, length) {
  // Internal truncated ID material only; callers must not treat this as a sha256: commitment.
  return sha256Hex(canonicalJsonStringify(value)).slice(0, length);
}
