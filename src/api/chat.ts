import type { ChatMessage } from '../state/chat';
import { backendUrl } from './client';
import { ChatError, type ChatErrorCode } from './errors';
import { safeParseDone, safeParseHttpError, safeParseStreamError, safeParseText } from './schemas';
import { parseSseEvents } from './sse';

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
      // Match the (non-greedy) SSE separators: \n\n, \r\r, or \r\n\r\n.
      const splitIdx = findLastSeparator(buffer);
      if (splitIdx === null) continue;
      const completePart = buffer.slice(0, splitIdx.endIdx);
      buffer = buffer.slice(splitIdx.endIdx);
      doneReason = handleBlock(completePart, opts) ?? doneReason;
      if (doneReason !== null) {
        await reader.cancel();
        return { reason: doneReason };
      }
    }
  } catch (e) {
    throw ChatError.from(e);
  }

  if (buffer.trim()) {
    doneReason = handleBlock(`${buffer}\n\n`, opts) ?? doneReason;
  }
  return { reason: doneReason ?? 'disconnected' };
}

interface SeparatorMatch {
  endIdx: number; // exclusive index after the separator
}

/**
 * Find the END of the last `\n\n`/`\r\r`/`\r\n\r\n` separator in `buffer`.
 * Returns null if no separator is present yet.
 */
function findLastSeparator(buffer: string): SeparatorMatch | null {
  let best: SeparatorMatch | null = null;
  for (const sep of ['\r\n\r\n', '\n\n', '\r\r']) {
    const idx = buffer.lastIndexOf(sep);
    if (idx !== -1) {
      const candidate = { endIdx: idx + sep.length };
      if (best === null || candidate.endIdx > best.endIdx) best = candidate;
    }
  }
  return best;
}

function handleBlock(raw: string, opts: StreamChatOptions): string | null {
  const events = parseSseEvents(raw);
  let doneReason: string | null = null;
  for (const ev of events) {
    switch (ev.event) {
      case 'text_delta': {
        const payload = safeParseText(ev.data);
        if (payload) opts.onTextDelta(payload.text);
        break;
      }
      case 'done': {
        const payload = safeParseDone(ev.data);
        doneReason = payload?.reason ?? 'stop';
        break;
      }
      case 'error': {
        const payload = safeParseStreamError(ev.data);
        const code: ChatErrorCode = payload?.code ?? 'provider_error';
        const message = payload?.message ?? 'provider error';
        throw new ChatError(code, message);
      }
    }
  }
  return doneReason;
}

async function readHttpError(resp: Response): Promise<ChatError> {
  let parsed: ReturnType<typeof safeParseHttpError> = null;
  try {
    parsed = safeParseHttpError(await resp.json());
  } catch {
    // Body may be empty or non-JSON; fall through to defaults.
  }
  const codeRaw = parsed?.error?.code;
  const code: ChatErrorCode = isChatErrorCode(codeRaw) ? codeRaw : statusToCode(resp.status);
  const message = parsed?.error?.message ?? `HTTP ${resp.status}`;
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

// Re-exports for convenience / backwards compat with existing tests.
export { parseSseEvents, type SseEvent } from './sse';
