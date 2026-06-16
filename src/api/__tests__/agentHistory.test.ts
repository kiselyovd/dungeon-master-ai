import { afterEach, describe, expect, it, vi } from 'vitest';
import type { ChatMessage } from '../../state/chat';
import { streamAgentTurn } from '../agent';
import { setBackendPortForTesting } from '../client';

/**
 * The backend `/agent/turn` deserializes `history` into `Vec<ChatMessage>`, an
 * enum tagged on `role` where `User` REQUIRES a `parts` array and assistant
 * carries `content`. Sending the raw chat-slice message (content only) made the
 * backend 422 with "history[i]: missing field `parts`" on every turn that had
 * history - found via the live real-Tauri playthrough.
 */
function sse(body: string): Response {
  const stream = new ReadableStream<Uint8Array>({
    start(c) {
      c.enqueue(new TextEncoder().encode(body));
      c.close();
    },
  });
  return new Response(stream, {
    status: 200,
    headers: { 'content-type': 'text/event-stream' },
  });
}

const noop = () => {};
const callbacks = {
  onTextDelta: noop,
  onToolCallStart: noop,
  onToolCallResult: noop,
  onAgentDone: noop,
};

describe('streamAgentTurn history serialization', () => {
  afterEach(() => {
    setBackendPortForTesting(null);
    vi.unstubAllGlobals();
  });

  async function capture(history: ChatMessage[]): Promise<Record<string, unknown>> {
    setBackendPortForTesting(45678);
    let body: Record<string, unknown> = {};
    vi.stubGlobal(
      'fetch',
      vi.fn(async (_url: string, init: RequestInit) => {
        body = JSON.parse(init.body as string);
        return sse('event: agent_done\ndata: {"total_rounds":0}\n\n');
      }),
    );
    await streamAgentTurn({
      campaignId: 'c',
      sessionId: 's',
      playerMessage: 'go north',
      history,
      ...callbacks,
    });
    return body;
  }

  it('serializes a user history message with a parts array', async () => {
    const body = await capture([{ id: '1', role: 'user', content: 'hello' } as ChatMessage]);
    expect((body.history as unknown[])[0]).toEqual({
      role: 'user',
      parts: [{ type: 'text', text: 'hello' }],
    });
  });

  it('keeps existing user parts (e.g. image attachments) verbatim', async () => {
    const parts = [{ type: 'image', image: { mime: 'image/png', data_base64: 'x' } }];
    const body = await capture([
      { id: '2', role: 'user', content: '', parts } as unknown as ChatMessage,
    ]);
    const first = (body.history as { parts: unknown[] }[])[0];
    expect(first?.parts).toEqual(parts);
  });

  it('serializes an assistant history message with content (no parts)', async () => {
    const body = await capture([
      { id: '3', role: 'assistant', content: 'A dim tavern.' } as ChatMessage,
    ]);
    expect((body.history as unknown[])[0]).toEqual({
      role: 'assistant',
      content: 'A dim tavern.',
    });
  });
});
