/**
 * Saves API client (M5 P2.13).
 *
 * Five typed wrappers around the Saves backend (`crates/app-server/src/routes/saves.rs`).
 * Linear-save model only - branching deferred to v2.
 */

import { backendUrl } from './client';
import { ChatError } from './errors';

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

interface CreateSaveResponse {
  id: string;
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
  return (await resp.json()) as SaveSummary[];
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
  const json = (await resp.json()) as CreateSaveResponse;
  return { id: json.id };
}

export async function quickSaveSession(sessionId: string): Promise<{ id: string }> {
  const url = await backendUrl(`/sessions/${encodeURIComponent(sessionId)}/saves/quick`);
  const resp = await fetch(url, { method: 'POST' });
  if (!resp.ok) throw await readError(resp, 'quick save');
  const json = (await resp.json()) as CreateSaveResponse;
  return { id: json.id };
}

export async function fetchSaveById(saveId: string): Promise<SaveRow> {
  const url = await backendUrl(`/saves/${encodeURIComponent(saveId)}`);
  const resp = await fetch(url);
  if (!resp.ok) throw await readError(resp, 'load save');
  return (await resp.json()) as SaveRow;
}

export async function deleteSaveById(saveId: string): Promise<void> {
  const url = await backendUrl(`/saves/${encodeURIComponent(saveId)}`);
  const resp = await fetch(url, { method: 'DELETE' });
  if (!resp.ok) throw await readError(resp, 'delete save');
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

interface MessagesResponse {
  messages: SessionMessageWire[];
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
  const json = (await resp.json()) as MessagesResponse;
  // Client-side enforcement of the limit: slice to the last N messages in
  // chronological order. This is the authoritative guard - see NOTE above.
  const messages = json.messages;
  if (opts?.limit !== undefined && messages.length > opts.limit) {
    return messages.slice(-opts.limit);
  }
  return messages;
}
