import type { CharacterDraft, TestChatTurn } from '../state/charCreation';
import { backendUrl } from './client';
import { ChatError } from './errors';
import { parseSseEvents } from './sse';

export type AssistField =
  | 'name'
  | 'backstory'
  | 'ideals'
  | 'bonds'
  | 'flaws'
  | 'portrait_prompt'
  | 'personality_flag'
  | 'item_name';

interface StreamCommon {
  draft: CharacterDraft;
  locale: 'en' | 'ru';
  signal?: AbortSignal | undefined;
  onError: (err: Error) => void;
  onDone: () => void;
}

export interface StreamCharacterFieldArgs extends StreamCommon {
  field: AssistField;
  onToken: (text: string) => void;
}

export interface StreamFullCharacterArgs extends StreamCommon {
  onPatch: (patch: Partial<CharacterDraft>) => void;
}

export interface StreamTestChatArgs extends StreamCommon {
  history: TestChatTurn[];
  userMessage: string;
  onToken: (text: string) => void;
}

async function postAssist(body: unknown, signal?: AbortSignal): Promise<Response> {
  const url = await backendUrl('/character/assist');
  const init: RequestInit = {
    method: 'POST',
    headers: { 'content-type': 'application/json', accept: 'text/event-stream' },
    body: JSON.stringify(body),
  };
  if (signal) init.signal = signal;
  return fetch(url, init);
}

interface AssistEnvelope {
  type: 'token' | 'draft_patch' | 'error' | 'done';
  text?: string;
  patch?: Partial<CharacterDraft>;
  code?: string;
  message?: string;
}

function isAssistEnvelope(value: unknown): value is AssistEnvelope {
  if (typeof value !== 'object' || value === null) return false;
  const t = (value as { type?: unknown }).type;
  return t === 'token' || t === 'draft_patch' || t === 'error' || t === 'done';
}

async function consume(resp: Response, handler: (ev: AssistEnvelope) => boolean): Promise<void> {
  if (!resp.ok) throw new ChatError('http_error', `assist failed: ${resp.status}`);
  if (!resp.body) throw new ChatError('no_body', 'response body is empty');

  const reader = resp.body.getReader();
  const decoder = new TextDecoder();
  let buffer = '';
  while (true) {
    const { done, value } = await reader.read();
    if (done) break;
    buffer += decoder.decode(value, { stream: true });
    const idx = lastSepEnd(buffer);
    if (idx === null) continue;
    const block = buffer.slice(0, idx);
    buffer = buffer.slice(idx);
    const events = parseSseEvents(block);
    for (const ev of events) {
      if (!isAssistEnvelope(ev.data)) continue;
      const stop = handler(ev.data);
      if (stop) {
        await reader.cancel();
        return;
      }
    }
  }
  if (buffer.trim().length > 0) {
    const events = parseSseEvents(`${buffer}\n\n`);
    for (const ev of events) {
      if (!isAssistEnvelope(ev.data)) continue;
      handler(ev.data);
    }
  }
}

function lastSepEnd(s: string): number | null {
  const candidates = ['\r\n\r\n', '\n\n', '\r\r'];
  let best: number | null = null;
  for (const sep of candidates) {
    const i = s.lastIndexOf(sep);
    if (i !== -1) {
      const end = i + sep.length;
      if (best === null || end > best) best = end;
    }
  }
  return best;
}

export async function streamCharacterField(args: StreamCharacterFieldArgs): Promise<void> {
  let resp: Response;
  try {
    resp = await postAssist(
      {
        kind: 'field',
        context: args.draft,
        params: { field: args.field },
        locale: args.locale,
      },
      args.signal,
    );
  } catch (e) {
    args.onError(e as Error);
    return;
  }
  try {
    await consume(resp, (env) => {
      if (env.type === 'token' && env.text !== undefined) args.onToken(env.text);
      else if (env.type === 'error') {
        args.onError(new Error(env.message ?? env.code ?? 'assist error'));
      } else if (env.type === 'done') {
        args.onDone();
        return true;
      }
      return false;
    });
  } catch (e) {
    args.onError(e as Error);
  }
}

export async function streamFullCharacter(args: StreamFullCharacterArgs): Promise<void> {
  let resp: Response;
  try {
    resp = await postAssist(
      { kind: 'full', context: args.draft, params: {}, locale: args.locale },
      args.signal,
    );
  } catch (e) {
    args.onError(e as Error);
    return;
  }
  try {
    await consume(resp, (env) => {
      if (env.type === 'draft_patch' && env.patch) args.onPatch(env.patch);
      else if (env.type === 'error') {
        args.onError(new Error(env.message ?? env.code ?? 'assist error'));
      } else if (env.type === 'done') {
        args.onDone();
        return true;
      }
      return false;
    });
  } catch (e) {
    args.onError(e as Error);
  }
}

export async function streamTestChat(args: StreamTestChatArgs): Promise<void> {
  let resp: Response;
  try {
    resp = await postAssist(
      {
        kind: 'test_chat',
        context: args.draft,
        params: { user_message: args.userMessage, history: args.history },
        locale: args.locale,
      },
      args.signal,
    );
  } catch (e) {
    args.onError(e as Error);
    return;
  }
  try {
    await consume(resp, (env) => {
      if (env.type === 'token' && env.text !== undefined) args.onToken(env.text);
      else if (env.type === 'error') {
        args.onError(new Error(env.message ?? env.code ?? 'assist error'));
      } else if (env.type === 'done') {
        args.onDone();
        return true;
      }
      return false;
    });
  } catch (e) {
    args.onError(e as Error);
  }
}
