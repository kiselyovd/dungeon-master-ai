import { backendUrl } from './client';
import type { ChatMessage } from '../state/chat';

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
  onDone: (reason: string) => void;
  onError: (err: { code: string; message: string }) => void;
  signal?: AbortSignal;
}

export async function streamChat(opts: StreamChatOptions): Promise<void> {
  const url = await backendUrl('/chat');
  const init: RequestInit = {
    method: 'POST',
    headers: { 'content-type': 'application/json', accept: 'text/event-stream' },
    body: JSON.stringify({ messages: opts.messages, model: opts.model }),
  };
  if (opts.signal) init.signal = opts.signal;
  const resp = await fetch(url, init);

  if (!resp.ok) {
    const body = (await resp.json().catch(() => ({}))) as {
      error?: { code: string; message: string };
    };
    opts.onError(body.error ?? { code: 'http_error', message: `HTTP ${resp.status}` });
    return;
  }

  if (!resp.body) {
    opts.onError({ code: 'no_body', message: 'response body is empty' });
    return;
  }

  const reader = resp.body.getReader();
  const decoder = new TextDecoder();
  let buffer = '';

  while (true) {
    const { done, value } = await reader.read();
    if (done) break;
    buffer += decoder.decode(value, { stream: true });
    const lastBlockEnd = buffer.lastIndexOf('\n\n');
    if (lastBlockEnd === -1) continue;
    const completePart = buffer.slice(0, lastBlockEnd + 2);
    buffer = buffer.slice(lastBlockEnd + 2);
    const events = parseSseEvents(completePart);
    for (const ev of events) {
      handleEvent(ev, opts);
    }
  }

  if (buffer.trim()) {
    const events = parseSseEvents(buffer + '\n\n');
    for (const ev of events) {
      handleEvent(ev, opts);
    }
  }
}

function handleEvent(ev: SseEvent, opts: StreamChatOptions): void {
  switch (ev.event) {
    case 'text_delta': {
      const data = ev.data as { text?: string };
      if (typeof data.text === 'string') opts.onTextDelta(data.text);
      break;
    }
    case 'done': {
      const data = ev.data as { reason?: string };
      opts.onDone(data.reason ?? 'stop');
      break;
    }
    case 'error': {
      const data = ev.data as { code?: string; message?: string };
      opts.onError({ code: data.code ?? 'unknown', message: data.message ?? 'unknown' });
      break;
    }
  }
}
