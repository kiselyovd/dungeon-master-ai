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
