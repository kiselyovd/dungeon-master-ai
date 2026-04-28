import type { ChatMessage } from '../state/chat';
import { backendUrl } from './client';
import { ChatError, type ChatErrorCode } from './errors';

export interface SseEvent {
  event: string;
  data: unknown;
}

export function parseSseEvents(raw: string): SseEvent[] {
  const blocks = raw.split('\n\n');
  // Only the blocks BEFORE the final split element are complete. If `raw`
  // ends with "\n\n", the last element is "" (still skipped). If it does
  // not, the last element is a partial block and must be left to the
  // caller to accumulate.
  const completeBlocks = blocks.slice(0, -1);
  const events: SseEvent[] = [];
  for (const block of completeBlocks) {
    if (!block.trim()) continue;
    let eventName = '';
    const dataLines: string[] = [];
    for (const line of block.split('\n')) {
      if (line.startsWith('event:')) {
        eventName = line.slice('event:'.length).trim();
      } else if (line.startsWith('data:')) {
        dataLines.push(line.slice('data:'.length).trim());
      }
    }
    if (!eventName) continue;
    const dataStr = dataLines.join('\n');
    let data: unknown;
    try {
      data = JSON.parse(dataStr);
    } catch {
      data = dataStr;
    }
    events.push({ event: eventName, data });
  }
  return events;
}

export interface StreamChatOptions {
  messages: ChatMessage[];
  model?: string;
  onTextDelta: (text: string) => void;
  signal?: AbortSignal;
}

export interface StreamChatResult {
  reason: string;
}

/**
 * Drive the SSE chat pipeline. Resolves with the finish `reason` when the
 * server emits a `done` event; throws `ChatError` on every failure path so
 * callers can rely on a single try/catch for unified handling.
 *
 * Stream-time text deltas are surfaced via `onTextDelta`. Mid-stream `error`
 * events become a thrown `ChatError`. Aborts surface as `code: 'aborted'`.
 */
export async function streamChat(opts: StreamChatOptions): Promise<StreamChatResult> {
  const url = await backendUrl('/chat');
  const init: RequestInit = {
    method: 'POST',
    headers: { 'content-type': 'application/json', accept: 'text/event-stream' },
    body: JSON.stringify({ messages: opts.messages, model: opts.model }),
  };
  if (opts.signal) init.signal = opts.signal;

  let resp: Response;
  try {
    resp = await fetch(url, init);
  } catch (e) {
    throw ChatError.from(e);
  }

  if (!resp.ok) {
    throw await readHttpError(resp);
  }

  if (!resp.body) {
    throw new ChatError('no_body', 'response body is empty');
  }

  const reader = resp.body.getReader();
  const decoder = new TextDecoder();
  let buffer = '';
  let doneReason: string | null = null;

  try {
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      buffer += decoder.decode(value, { stream: true });
      const lastBlockEnd = buffer.lastIndexOf('\n\n');
      if (lastBlockEnd === -1) continue;
      const completePart = buffer.slice(0, lastBlockEnd + 2);
      buffer = buffer.slice(lastBlockEnd + 2);
      doneReason = handleBlock(completePart, opts) ?? doneReason;
      if (doneReason !== null) {
        // Server sent done - nothing more to read; release the lock cleanly.
        await reader.cancel();
        return { reason: doneReason };
      }
    }
  } catch (e) {
    throw ChatError.from(e);
  }

  // Connection closed without an explicit `done` event. Drain the trailing
  // partial first; if it carries the done event we honor it, otherwise we
  // treat it as a graceful disconnect.
  if (buffer.trim()) {
    doneReason = handleBlock(`${buffer}\n\n`, opts) ?? doneReason;
  }
  return { reason: doneReason ?? 'disconnected' };
}

function handleBlock(raw: string, opts: StreamChatOptions): string | null {
  const events = parseSseEvents(raw);
  let doneReason: string | null = null;
  for (const ev of events) {
    switch (ev.event) {
      case 'text_delta': {
        const data = ev.data as { text?: string };
        if (typeof data.text === 'string') opts.onTextDelta(data.text);
        break;
      }
      case 'done': {
        const data = ev.data as { reason?: string };
        doneReason = typeof data.reason === 'string' ? data.reason : 'stop';
        break;
      }
      case 'error': {
        const data = ev.data as { code?: string; message?: string };
        const code = isChatErrorCode(data.code) ? data.code : 'provider_error';
        throw new ChatError(code, data.message ?? 'provider error');
      }
    }
  }
  return doneReason;
}

async function readHttpError(resp: Response): Promise<ChatError> {
  let parsed: { error?: { code?: string; message?: string } } = {};
  try {
    parsed = (await resp.json()) as typeof parsed;
  } catch {
    // Body may be empty or non-JSON; fall through to defaults.
  }
  const codeRaw = parsed.error?.code;
  const code: ChatErrorCode = isChatErrorCode(codeRaw) ? codeRaw : statusToCode(resp.status);
  const message = parsed.error?.message ?? `HTTP ${resp.status}`;
  return new ChatError(code, message);
}

function statusToCode(status: number): ChatErrorCode {
  if (status === 401 || status === 403) return 'auth_failed';
  if (status === 429) return 'rate_limit';
  if (status >= 500) return 'provider_error';
  return 'http_error';
}

const KNOWN_CODES: ReadonlySet<ChatErrorCode> = new Set<ChatErrorCode>([
  'auth_failed',
  'rate_limit',
  'network',
  'provider_error',
  'no_body',
  'http_error',
  'aborted',
  'invalid_response',
  'unknown',
]);

function isChatErrorCode(value: unknown): value is ChatErrorCode {
  return typeof value === 'string' && KNOWN_CODES.has(value as ChatErrorCode);
}
