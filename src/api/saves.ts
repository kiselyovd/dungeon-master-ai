/**
 * Saves API client (M5 P2.13).
 *
 * Five typed wrappers around the Saves backend (`crates/app-server/src/routes/saves.rs`).
 * Linear-save model only - branching deferred to v2.
 */

import { backendUrl } from './client';
import { ChatError } from './errors';
import { discardResponseBody } from './response';

export type SaveKind = 'manual' | 'auto' | 'checkpoint';
export type SaveTag = 'combat' | 'exploration' | 'dialog' | 'npc';

export interface SaveSummary {
  id: string;
  session_id: string;
  kind: SaveKind;
  title: string;
  summary: string;
  tag: SaveTag;
  created_at: string;
  turn_number: number;
}

export interface SaveRow extends SaveSummary {
  game_state: unknown;
}

export interface CreateSaveRequest {
  kind: SaveKind;
  title: string;
  summary: string;
  tag: SaveTag;
}

export interface RestoreSaveResponse {
  game_state: unknown;
  messages: SessionMessageWire[];
}

function invalidResponse(label: string): never {
  throw new ChatError('invalid_response', `${label} response is invalid`);
}

function record(value: unknown, label: string): Record<string, unknown> {
  if (value === null || typeof value !== 'object' || Array.isArray(value)) invalidResponse(label);
  return value as Record<string, unknown>;
}

function stringField(value: unknown, label: string): string {
  if (typeof value !== 'string') invalidResponse(label);
  return value;
}

function numberField(value: unknown, label: string): number {
  if (typeof value !== 'number' || !Number.isFinite(value)) invalidResponse(label);
  return value;
}

function parseSaveSummary(value: unknown): SaveSummary {
  const row = record(value, 'save');
  const kind = stringField(row.kind, 'save.kind');
  const tag = stringField(row.tag, 'save.tag');
  if (kind !== 'manual' && kind !== 'auto' && kind !== 'checkpoint') invalidResponse('save.kind');
  if (tag !== 'combat' && tag !== 'exploration' && tag !== 'dialog' && tag !== 'npc') {
    invalidResponse('save.tag');
  }
  return {
    id: stringField(row.id, 'save.id'),
    session_id: stringField(row.session_id, 'save.session_id'),
    kind,
    title: stringField(row.title, 'save.title'),
    summary: stringField(row.summary, 'save.summary'),
    tag,
    created_at: stringField(row.created_at, 'save.created_at'),
    turn_number: numberField(row.turn_number, 'save.turn_number'),
  };
}

function parseMessage(value: unknown): SessionMessageWire {
  const message = record(value, 'message');
  const role = stringField(message.role, 'message.role');
  if (
    role !== 'user' &&
    role !== 'assistant' &&
    role !== 'system' &&
    role !== 'assistant_with_tool_calls' &&
    role !== 'tool_result'
  ) {
    invalidResponse('message.role');
  }
  const parsed: SessionMessageWire = { role };
  if (message.content !== undefined)
    parsed.content = stringField(message.content, 'message.content');
  if (message.parts !== undefined) {
    if (!Array.isArray(message.parts)) invalidResponse('message.parts');
    parsed.parts = message.parts.map((partValue) => {
      const part = record(partValue, 'message.part');
      const parsedPart: NonNullable<SessionMessageWire['parts']>[number] = {
        type: stringField(part.type, 'message.part.type'),
      };
      if (part.text !== undefined) parsedPart.text = stringField(part.text, 'message.part.text');
      if (part.mime !== undefined) parsedPart.mime = stringField(part.mime, 'message.part.mime');
      if (part.data_b64 !== undefined) {
        parsedPart.data_b64 = stringField(part.data_b64, 'message.part.data_b64');
      }
      if (part.name === null) parsedPart.name = null;
      else if (part.name !== undefined)
        parsedPart.name = stringField(part.name, 'message.part.name');
      return parsedPart;
    });
  }
  if (message.tool_calls !== undefined) {
    if (!Array.isArray(message.tool_calls)) invalidResponse('message.tool_calls');
    parsed.tool_calls = message.tool_calls;
  }
  return parsed;
}

async function readError(resp: Response, label: string): Promise<ChatError> {
  let message = `${label} failed: ${resp.status}`;
  try {
    const body = (await resp.json()) as { error?: { message?: string } };
    if (body?.error?.message) message = body.error.message;
  } catch {
    // Body may be empty or non-JSON; the default `message` covers it.
  }
  return new ChatError('http_error', message);
}

export async function fetchSessionSaves(sessionId: string): Promise<SaveSummary[]> {
  const url = await backendUrl(`/sessions/${encodeURIComponent(sessionId)}/saves`);
  const resp = await fetch(url);
  if (!resp.ok) throw await readError(resp, 'list saves');
  const json: unknown = await resp.json();
  if (!Array.isArray(json)) invalidResponse('list saves');
  return json.map(parseSaveSummary);
}

export async function createSave(
  sessionId: string,
  body: CreateSaveRequest,
): Promise<{ id: string }> {
  const url = await backendUrl(`/sessions/${encodeURIComponent(sessionId)}/saves`);
  const resp = await fetch(url, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(body),
  });
  if (!resp.ok) throw await readError(resp, 'create save');
  const json = record(await resp.json(), 'create save');
  return { id: stringField(json.id, 'create save.id') };
}

export async function quickSaveSession(sessionId: string): Promise<{ id: string }> {
  const url = await backendUrl(`/sessions/${encodeURIComponent(sessionId)}/saves/quick`);
  const resp = await fetch(url, { method: 'POST' });
  if (!resp.ok) throw await readError(resp, 'quick save');
  const json = record(await resp.json(), 'quick save');
  return { id: stringField(json.id, 'quick save.id') };
}

export async function fetchSaveById(saveId: string): Promise<SaveRow> {
  const url = await backendUrl(`/saves/${encodeURIComponent(saveId)}`);
  const resp = await fetch(url);
  if (!resp.ok) throw await readError(resp, 'load save');
  const json = record(await resp.json(), 'load save');
  return { ...parseSaveSummary(json), game_state: json.game_state };
}

export async function deleteSaveById(saveId: string): Promise<void> {
  const url = await backendUrl(`/saves/${encodeURIComponent(saveId)}`);
  const resp = await fetch(url, { method: 'DELETE' });
  if (!resp.ok) throw await readError(resp, 'delete save');
  await discardResponseBody(resp);
}

/**
 * Restore a save's combat + scene state on the backend and return the full
 * schema-version 2 game_state so the frontend can rehydrate Zustand slices.
 * POST /saves/{saveId}/restore?session_id={sessionId}   [W2.3]
 */
export async function restoreSave(saveId: string, sessionId: string): Promise<RestoreSaveResponse> {
  const url = await backendUrl(
    `/saves/${encodeURIComponent(saveId)}/restore?session_id=${encodeURIComponent(sessionId)}`,
  );
  const resp = await fetch(url, { method: 'POST' });
  if (!resp.ok) throw await readError(resp, 'restore save');
  const json = record(await resp.json(), 'restore save');
  if (!Array.isArray(json.messages)) invalidResponse('restore save.messages');
  return {
    game_state: json.game_state,
    messages: json.messages.map(parseMessage),
  };
}

/** Overwrite an existing save's metadata in place (PUT /saves/{id}). [F3] */
export async function updateSaveById(saveId: string, body: CreateSaveRequest): Promise<void> {
  const url = await backendUrl(`/saves/${encodeURIComponent(saveId)}`);
  const resp = await fetch(url, {
    method: 'PUT',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(body),
  });
  if (!resp.ok) throw await readError(resp, 'overwrite save');
  await discardResponseBody(resp);
}

/**
 * A single chat message as returned by GET /sessions/{id}/messages.
 * Mirrors the backend `ChatMessage` enum serialized with `serde(tag = "role",
 * rename_all = "snake_case")`.
 *
 * The full set of role variants the backend can return:
 *   - "system"                   -> System { content }
 *   - "user"                     -> User { parts }
 *   - "assistant"                -> Assistant { content }
 *   - "assistant_with_tool_calls"-> AssistantWithToolCalls { content?, tool_calls }
 *   - "tool_result"              -> ToolResult(ToolResult)
 *
 * The chat UI (ChatRole in src/state/chat.ts) only renders "user" | "assistant" | "system".
 * The extra variants are typed here accurately but filtered during V1 rehydration -
 * see the comment in rehydrateFromSave.
 */
export interface SessionMessageWire {
  role: 'user' | 'assistant' | 'system' | 'assistant_with_tool_calls' | 'tool_result';
  content?: string;
  parts?: Array<{
    type: string;
    text?: string;
    mime?: string;
    data_b64?: string;
    name?: string | null;
  }>;
  /** Present on assistant_with_tool_calls variant. */
  tool_calls?: unknown[];
}

export async function fetchSessionMessages(
  sessionId: string,
  opts?: { limit?: number },
): Promise<SessionMessageWire[]> {
  const params = new URLSearchParams();
  // NOTE: The backend GET /sessions/{id}/messages handler currently ignores
  // query parameters and returns the FULL message history. The ?limit= param
  // is sent for forward-compatibility only; the client-side slice below is
  // what actually enforces the "last N messages" contract.
  if (opts?.limit !== undefined) params.set('limit', String(opts.limit));
  const query = params.size > 0 ? `?${params.toString()}` : '';
  const url = await backendUrl(`/sessions/${encodeURIComponent(sessionId)}/messages${query}`);
  const resp = await fetch(url);
  if (!resp.ok) throw await readError(resp, 'fetch messages');
  const json = record(await resp.json(), 'fetch messages');
  if (!Array.isArray(json.messages)) invalidResponse('fetch messages.messages');
  // Client-side enforcement of the limit: slice to the last N messages in
  // chronological order. This is the authoritative guard - see NOTE above.
  const messages = json.messages.map(parseMessage);
  if (opts?.limit !== undefined && messages.length > opts.limit) {
    return messages.slice(-opts.limit);
  }
  return messages;
}
