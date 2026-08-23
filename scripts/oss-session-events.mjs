import crypto from "node:crypto";

const MAX_EVENTS = 512;
const EVENT_KINDS = new Set(["started", "attached", "checkpoint", "paused", "resumed", "cancelled", "completed", "forked"]);
const FORBIDDEN_KEYS = new Set(["prompt", "messages", "input", "output", "response", "body", "headers", "authorization", "token", "secret", "credential"].map((key) => key.toLowerCase()));

function identifier(value, label) {
  if (typeof value !== "string" || value.trim() === "" || value.length > 128 || [...value].some((character) => character.charCodeAt(0) < 32)) throw new Error(`${label} must be a bounded identifier`);
  return value.trim();
}

function validateEvent(event, index, sessionId) {
  if (!event || typeof event !== "object" || Array.isArray(event)) throw new Error(`event ${index} must be an object`);
  for (const key of Object.keys(event)) {
    if (FORBIDDEN_KEYS.has(key.toLowerCase())) throw new Error(`session event contains forbidden field: ${key}`);
    if (!["eventId", "sessionId", "sequence", "kind", "parentEventId"].includes(key)) throw new Error(`session event field is not allowed: ${key}`);
  }
  const eventSessionId = identifier(event.sessionId, `event ${index} sessionId`);
  if (eventSessionId !== sessionId) throw new Error(`event ${index} belongs to another session`);
  identifier(event.eventId, `event ${index} eventId`);
  if (!Number.isSafeInteger(event.sequence) || event.sequence < 0) throw new Error(`event ${index} sequence must be a non-negative safe integer`);
  if (!EVENT_KINDS.has(event.kind)) throw new Error(`event ${index} has unsupported kind`);
  if (event.parentEventId !== undefined) identifier(event.parentEventId, `event ${index} parentEventId`);
}

export function buildSessionEventLedger({ sessionId, events }) {
  const normalizedSessionId = identifier(sessionId, "sessionId");
  if (!Array.isArray(events) || events.length > MAX_EVENTS) throw new Error(`session ledger requires at most ${MAX_EVENTS} events`);
  const seen = new Set();
  events.forEach((event, index) => {
    validateEvent(event, index, normalizedSessionId);
    if (seen.has(event.eventId)) throw new Error(`duplicate session event: ${event.eventId}`);
    seen.add(event.eventId);
    if (event.sequence !== index) throw new Error(`session event sequence must be contiguous at index ${index}`);
  });
  const last = events.at(-1);
  const status = last?.kind === "cancelled" ? "cancelled" : last?.kind === "completed" ? "completed" : last?.kind === "paused" ? "paused" : "active";
  return { sessionId: normalizedSessionId, eventCount: events.length, lastSequence: last?.sequence ?? null, status, executionMode: "observe_only", events: events.map(({ eventId, sessionId: id, sequence, kind, parentEventId }) => ({ eventId, sessionId: id, sequence, kind, ...(parentEventId ? { parentEventId } : {}) })) };
}

export function transitionSession(ledger, action) {
  const current = ledger.status;
  const allowed = { active: ["pause", "cancel", "complete"], paused: ["resume", "cancel"], cancelled: [], completed: [] };
  if (!allowed[current]?.includes(action)) throw new Error(`session action ${action} is not allowed from ${current}`);
  const kind = { pause: "paused", cancel: "cancelled", complete: "completed", resume: "resumed" }[action];
  const event = { eventId: `${ledger.sessionId}:${ledger.eventCount}`, sessionId: ledger.sessionId, sequence: ledger.eventCount, kind };
  return buildSessionEventLedger({ sessionId: ledger.sessionId, events: [...ledger.events, event] });
}

export function forkSessionAtEvent(ledger, eventId) {
  identifier(eventId, "fork eventId");
  if (!ledger.events.some((event) => event.eventId === eventId)) throw new Error("fork event is not in the session ledger");
  const sessionId = `fork:${crypto.createHash("sha256").update(`${ledger.sessionId}:${eventId}`).digest("hex").slice(0, 32)}`;
  return { sessionId, parentSessionId: ledger.sessionId, forkEventId: eventId, executionMode: "observe_only", automaticPromotion: "disabled" };
}
