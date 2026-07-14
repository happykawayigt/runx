import crypto from "node:crypto";

const SCHEMA = "runx.operator_inbox.projection.v1";
const STATUSES = new Set(["open", "waiting", "followed_up", "resolved", "dismissed"]);
const SCAN_STATUSES = new Set(["running", "complete", "truncated", "failed"]);
const MAX_MESSAGES_PER_PAGE = 20;
const MAX_ITEMS = 5_000;
const MAX_SCANS = 100;
const MAX_LOCATORS = 50;

export function emptyProjection() {
  return { schema: SCHEMA, version: 0, items: [], scans: [] };
}

export function foldEventPage({ projection, events, afterVersion, streamVersion }) {
  const state = normalizeProjection(projection);
  const after = nonNegativeInteger(afterVersion, "after_version");
  const current = nonNegativeInteger(streamVersion, "stream_version");
  if (state.version !== after) {
    throw new Error(`operator inbox projection version ${state.version} does not match after_version ${after}`);
  }
  if (current < after) throw new Error("operator inbox stream_version precedes after_version");
  if (!Array.isArray(events) || events.length > 500) throw new Error("operator inbox events must be a bounded array");

  let expected = after + 1;
  for (const entry of events) {
    const record = object(entry, "event entry");
    const version = nonNegativeInteger(record.version, "event version");
    if (version !== expected) throw new Error(`operator inbox event page is not contiguous at version ${expected}`);
    applyDomainEvent(state, object(record.event, "event"));
    state.version = version;
    expected += 1;
  }
  if (state.version > current) throw new Error("operator inbox event page exceeds stream_version");
  return {
    ...state,
    page: {
      next_after_version: state.version,
      stream_version: current,
      complete: state.version === current,
    },
  };
}

export function planTransition(input) {
  const state = normalizeProjection(input.projection);
  const expectedVersion = nonNegativeInteger(input.expectedVersion, "expected_version");
  if (state.version !== expectedVersion) {
    throw new Error(`operator inbox projection version ${state.version} does not match expected_version ${expectedVersion}`);
  }
  const observedAt = isoTime(input.observedAt, "observed_at");
  const operation = text(input.operation, 30, "operation");
  let event;
  if (operation === "scan_page") {
    event = scanPageEvent(input.scan, input.messages, observedAt);
  } else if (operation === "disposition") {
    event = dispositionEvent(input.disposition, observedAt, state);
  } else {
    throw new Error("operator inbox operation must be scan_page or disposition");
  }
  const eventDigest = sha256(event);
  const idempotencyKey = operation === "scan_page"
    ? `operator-inbox:scan:${event.payload.scan.scan_id}:${event.payload.scan.page_index}:${eventDigest.slice(7, 31)}`
    : `operator-inbox:disposition:${event.payload.disposition.thread_locator}:${eventDigest.slice(7, 31)}`;
  const next = clone(state);
  applyDomainEvent(next, event);
  next.version = expectedVersion + 1;
  return {
    effect_family: "operator-inbox",
    operation,
    expected_version: expectedVersion,
    idempotency_key: idempotencyKey,
    event,
    projection: next,
  };
}

function scanPageEvent(rawScan, rawMessages, observedAt) {
  const scanInput = object(rawScan, "scan");
  const status = text(scanInput.status, 30, "scan.status");
  if (!SCAN_STATUSES.has(status)) throw new Error(`unsupported operator inbox scan status '${status}'`);
  const messages = Array.isArray(rawMessages) ? rawMessages.map(normalizeMessage) : null;
  if (!messages || messages.length > MAX_MESSAGES_PER_PAGE) {
    throw new Error(`operator inbox messages must be an array of at most ${MAX_MESSAGES_PER_PAGE}`);
  }
  const error = optionalText(scanInput.error, 500, "scan.error");
  if (status === "failed" && !error) throw new Error("failed operator inbox scans require a bounded error");
  return {
    type: "operator_inbox.scan_page_recorded",
    effect_family: "operator-inbox",
    operation: "scan_page",
    payload: {
      observed_at: observedAt,
      scan: {
        scan_id: text(scanInput.scan_id, 200, "scan.scan_id"),
        provider: text(scanInput.provider, 100, "scan.provider"),
        query_digest: digest(scanInput.query_digest, "scan.query_digest"),
        page_index: positiveInteger(scanInput.page_index, "scan.page_index"),
        status,
        ...(error ? { error } : {}),
      },
      messages,
    },
  };
}

function dispositionEvent(rawDisposition, observedAt, state) {
  const input = object(rawDisposition, "disposition");
  const threadLocator = text(input.thread_locator, 500, "disposition.thread_locator");
  const item = state.items.find((candidate) => candidate.thread_locator === threadLocator);
  if (!item) {
    throw new Error(`operator inbox item ${threadLocator} was not found`);
  }
  const status = text(input.status, 30, "disposition.status");
  if (!STATUSES.has(status) || status === "open") {
    throw new Error("operator inbox human disposition must be waiting, followed_up, resolved, or dismissed");
  }
  const evidenceUrl = optionalHttpsUrl(input.evidence_url);
  return {
    type: "operator_inbox.disposition_recorded",
    effect_family: "operator-inbox",
    operation: "disposition",
    payload: {
      observed_at: observedAt,
      disposition: {
        thread_locator: threadLocator,
        status,
        actor: text(input.actor, 200, "disposition.actor"),
        reason: text(input.reason, 500, "disposition.reason"),
        covered_occurrence_at: isoTime(item.latest_message.occurred_at, "item.latest_message.occurred_at"),
        ...(evidenceUrl ? { evidence_url: evidenceUrl } : {}),
      },
    },
  };
}

function applyDomainEvent(state, event) {
  if (event.type === "operator_inbox.scan_page_recorded") {
    const payload = object(event.payload, "scan event payload");
    const observedAt = isoTime(payload.observed_at, "event observed_at");
    const scan = object(payload.scan, "event scan");
    for (const message of array(payload.messages, "event messages").map(normalizeMessage)) {
      applyMessage(state, message, observedAt);
    }
    applyScan(state, scan, observedAt);
    return;
  }
  if (event.type === "operator_inbox.disposition_recorded") {
    const payload = object(event.payload, "disposition event payload");
    const observedAt = isoTime(payload.observed_at, "event observed_at");
    const disposition = object(payload.disposition, "event disposition");
    const threadLocator = text(disposition.thread_locator, 500, "event disposition.thread_locator");
    const status = text(disposition.status, 30, "event disposition.status");
    if (!STATUSES.has(status) || status === "open") {
      throw new Error("operator inbox human disposition must be waiting, followed_up, resolved, or dismissed");
    }
    const actor = text(disposition.actor, 200, "event disposition.actor");
    const reason = text(disposition.reason, 500, "event disposition.reason");
    const coveredOccurrenceAt = isoTime(disposition.covered_occurrence_at, "event disposition.covered_occurrence_at");
    const evidenceUrl = optionalHttpsUrl(disposition.evidence_url);
    const item = state.items.find((candidate) => candidate.thread_locator === threadLocator);
    if (!item) throw new Error(`operator inbox item ${threadLocator} was not found`);
    item.status = status;
    item.disposition = {
      status,
      actor,
      reason,
      at: observedAt,
      covered_occurrence_at: coveredOccurrenceAt,
      ...(evidenceUrl ? { evidence_url: evidenceUrl } : {}),
    };
    return;
  }
  throw new Error(`unsupported operator inbox event '${event.type}'`);
}

function applyMessage(state, message, observedAt) {
  if (message.author.external_id === message.connected_subject_ref) return;
  let item = state.items.find((candidate) => candidate.thread_locator === message.thread_locator);
  if (!item) {
    if (state.items.length >= MAX_ITEMS) throw new Error(`operator inbox capacity of ${MAX_ITEMS} items was reached`);
    item = {
      thread_locator: message.thread_locator,
      provider: message.provider,
      external_tenant_ref: message.external_tenant_ref,
      connected_subject_ref: message.connected_subject_ref,
      requester: message.author,
      conversation: message.conversation,
      latest_message: latestMessage(message),
      message_locators: [message.message_locator],
      status: "open",
      disposition: null,
      first_observed_at: observedAt,
      last_observed_at: observedAt,
    };
    state.items.push(item);
    return;
  }
  item.last_observed_at = observedAt;
  item.message_locators = [...new Set([...item.message_locators, message.message_locator])].slice(-MAX_LOCATORS);
  if (Date.parse(message.occurred_at) > Date.parse(item.latest_message.occurred_at)) {
    item.latest_message = latestMessage(message);
    item.conversation = message.conversation;
  }
  if (item.disposition && Date.parse(message.occurred_at) > Date.parse(item.disposition.covered_occurrence_at)) {
    item.status = "open";
  }
}

function applyScan(state, scan, observedAt) {
  const scanId = text(scan.scan_id, 200, "scan.scan_id");
  const status = text(scan.status, 30, "scan.status");
  if (!SCAN_STATUSES.has(status)) throw new Error(`unsupported operator inbox scan status '${status}'`);
  const error = optionalText(scan.error, 500, "scan.error");
  if (status === "failed" && !error) throw new Error("failed operator inbox scans require a bounded error");
  const existing = state.scans.find((entry) => entry.scan_id === scanId);
  const record = existing ?? {
    scan_id: scanId,
    provider: text(scan.provider, 100, "scan.provider"),
    query_digest: digest(scan.query_digest, "scan.query_digest"),
    started_at: observedAt,
    pages_observed: 0,
    status: "running",
  };
  record.pages_observed = Math.max(record.pages_observed, positiveInteger(scan.page_index, "scan.page_index"));
  record.status = status;
  record.updated_at = observedAt;
  record.error = error ?? null;
  if (!existing) state.scans.push(record);
  state.scans = state.scans.slice(-MAX_SCANS);
}

function normalizeProjection(value) {
  if (value === undefined || value === null) return emptyProjection();
  const input = object(value, "projection");
  if (input.schema !== SCHEMA) throw new Error("operator inbox projection has an unsupported schema");
  const projection = {
    schema: SCHEMA,
    version: nonNegativeInteger(input.version, "projection.version"),
    items: clone(array(input.items, "projection.items")),
    scans: clone(array(input.scans, "projection.scans")),
  };
  if (projection.items.length > MAX_ITEMS || projection.scans.length > MAX_SCANS) {
    throw new Error("operator inbox projection exceeds capacity");
  }
  return projection;
}

function normalizeMessage(value) {
  const input = object(value, "message");
  const author = object(input.author, "message.author");
  const conversation = object(input.conversation, "message.conversation");
  const context = array(input.context, "message.context").slice(0, 40).map((entry) => {
    const message = object(entry, "message.context entry");
    return {
      relation: message.relation === "before" ? "before" : "after",
      message_locator: text(message.message_locator, 500, "context.message_locator"),
      author: normalizeAuthor(message.author),
      occurred_at: isoTime(message.occurred_at, "context.occurred_at"),
      preview: optionalText(message.preview, 500, "context.preview") ?? "",
    };
  });
  return {
    provider: text(input.provider, 100, "message.provider"),
    external_tenant_ref: text(input.external_tenant_ref, 300, "message.external_tenant_ref"),
    connected_subject_ref: text(input.connected_subject_ref, 300, "message.connected_subject_ref"),
    message_locator: text(input.message_locator, 500, "message.message_locator"),
    thread_locator: text(input.thread_locator, 500, "message.thread_locator"),
    author: normalizeAuthor(author),
    conversation: {
      external_id: text(conversation.external_id, 300, "conversation.external_id"),
      ...(optionalText(conversation.display_name, 300, "conversation.display_name") ? { display_name: optionalText(conversation.display_name, 300, "conversation.display_name") } : {}),
      type: text(conversation.type, 30, "conversation.type"),
    },
    occurred_at: isoTime(input.occurred_at, "message.occurred_at"),
    preview: optionalText(input.preview, 2_000, "message.preview") ?? "",
    ...(optionalHttpsUrl(input.permalink) ? { permalink: optionalHttpsUrl(input.permalink) } : {}),
    ...(Number.isSafeInteger(input.reply_count) && input.reply_count >= 0 ? { reply_count: input.reply_count } : {}),
    context,
  };
}

function normalizeAuthor(value) {
  const author = object(value, "author");
  const displayName = optionalText(author.display_name, 300, "author.display_name");
  return {
    external_id: text(author.external_id, 300, "author.external_id"),
    ...(displayName ? { display_name: displayName } : {}),
  };
}

function latestMessage(message) {
  return {
    message_locator: message.message_locator,
    occurred_at: message.occurred_at,
    preview: message.preview,
    ...(message.permalink ? { permalink: message.permalink } : {}),
    ...(message.reply_count !== undefined ? { reply_count: message.reply_count } : {}),
  };
}

function object(value, field) {
  if (!value || typeof value !== "object" || Array.isArray(value)) throw new Error(`${field} must be an object`);
  return value;
}

function array(value, field) {
  if (!Array.isArray(value)) throw new Error(`${field} must be an array`);
  return value;
}

function text(value, max, field) {
  if (typeof value !== "string") throw new Error(`${field} must be a string`);
  const result = value.trim();
  if (!result || result.length > max || /[\u0000-\u001f]/.test(result)) throw new Error(`${field} is invalid`);
  return result;
}

function optionalText(value, max, field) {
  return value === undefined || value === null ? undefined : text(value, max, field);
}

function isoTime(value, field) {
  const result = text(value, 100, field);
  if (!Number.isFinite(Date.parse(result))) throw new Error(`${field} must be ISO-8601`);
  return new Date(result).toISOString();
}

function digest(value, field) {
  const result = text(value, 100, field);
  if (!/^sha256:[a-f0-9]{64}$/.test(result)) throw new Error(`${field} must be a sha256 digest`);
  return result;
}

function optionalHttpsUrl(value) {
  if (value === undefined || value === null) return undefined;
  const result = text(value, 2_000, "URL");
  const url = new URL(result);
  if (url.protocol !== "https:") throw new Error("operator inbox URLs must use HTTPS");
  return url.toString();
}

function nonNegativeInteger(value, field) {
  if (!Number.isSafeInteger(value) || value < 0) throw new Error(`${field} must be a non-negative integer`);
  return value;
}

function positiveInteger(value, field) {
  if (!Number.isSafeInteger(value) || value < 1) throw new Error(`${field} must be a positive integer`);
  return value;
}

function sha256(value) {
  return `sha256:${crypto.createHash("sha256").update(canonical(value)).digest("hex")}`;
}

function canonical(value) {
  if (value === null || typeof value !== "object") return JSON.stringify(value);
  if (Array.isArray(value)) return `[${value.map(canonical).join(",")}]`;
  return `{${Object.keys(value).sort().map((key) => `${JSON.stringify(key)}:${canonical(value[key])}`).join(",")}}`;
}

function clone(value) {
  return structuredClone(value);
}
