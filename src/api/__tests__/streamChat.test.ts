import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { streamChat } from '../chat';
import { ChatError } from '../errors';

vi.mock('../client', () => ({
  backendUrl: vi.fn(async (path: string) => `http://test.local${path}`),
}));

interface FetchScript {
  status?: number;
  contentType?: string;
  chunks?: string[];
  body?: string | null;
  jsonError?: { code?: string; message?: string };
  throwOnFetch?: unknown;
}

function buildResponse({
  status = 200,
  contentType = 'text/event-stream',
  chunks,
  body,
  jsonError,
}: FetchScript): Response {
  const headers = new Headers({ 'content-type': contentType });
  if (jsonError !== undefined) {
    return new Response(JSON.stringify({ error: jsonError }), {
      status,
      headers: new Headers({ 'content-type': 'application/json' }),
    });
  }
  if (body === null) {
    return new Response(null, { status, headers });
  }
  if (chunks) {
    const encoder = new TextEncoder();
    const stream = new ReadableStream<Uint8Array>({
      start(controller) {
        for (const c of chunks) controller.enqueue(encoder.encode(c));
        controller.close();
      },
    });
    return new Response(stream, { status, headers });
  }
  return new Response(body ?? '', { status, headers });
}

function stubFetch(script: FetchScript): void {
  vi.stubGlobal(
    'fetch',
    vi.fn(async () => {
      if (script.throwOnFetch !== undefined) throw script.throwOnFetch;
      return buildResponse(script);
    }),
  );
}

const baseMessages = [{ id: 'm1', role: 'user' as const, content: 'hi' }];

beforeEach(() => {
  vi.unstubAllGlobals();
});

afterEach(() => {
  vi.unstubAllGlobals();
});

describe('streamChat happy path', () => {
  it('streams text deltas and resolves with the done reason', async () => {
    stubFetch({
      chunks: [
        'event: text_delta\ndata: {"text":"Hello"}\n\n',
        'event: text_delta\ndata: {"text":", world"}\n\n',
        'event: done\ndata: {"reason":"stop"}\n\n',
      ],
    });

    const seen: string[] = [];
    const result = await streamChat({
      messages: baseMessages,
      onTextDelta: (t) => seen.push(t),
    });

    expect(seen).toEqual(['Hello', ', world']);
    expect(result.reason).toBe('stop');
  });

  it('reassembles SSE events split across read() chunks', async () => {
    stubFetch({
      chunks: [
        'event: text_de',
        'lta\ndata: {"text":"chu',
        'nked"}\n\nevent: done\ndata: {"reason":"stop"}\n\n',
      ],
    });

    const seen: string[] = [];
    const result = await streamChat({
      messages: baseMessages,
      onTextDelta: (t) => seen.push(t),
    });

    expect(seen).toEqual(['chunked']);
    expect(result.reason).toBe('stop');
  });

  it('survives a UTF-8 multi-byte split mid-token', async () => {
    // 'Привет' encoded as UTF-8 spans 12 bytes (2 each). Split bytes 0..7 vs 7..12
    // so a multi-byte char straddles the boundary.
    const encoder = new TextEncoder();
    const full = encoder.encode('event: text_delta\ndata: {"text":"Привет"}\n\n');
    const cut = 32; // mid the russian word
    const a = full.slice(0, cut);
    const b = full.slice(cut);
    const done = encoder.encode('event: done\ndata: {"reason":"stop"}\n\n');

    vi.stubGlobal(
      'fetch',
      vi.fn(async () => {
        const stream = new ReadableStream<Uint8Array>({
          start(controller) {
            controller.enqueue(a);
            controller.enqueue(b);
            controller.enqueue(done);
            controller.close();
          },
        });
        return new Response(stream, {
          status: 200,
          headers: new Headers({ 'content-type': 'text/event-stream' }),
        });
      }),
    );

    const seen: string[] = [];
    const result = await streamChat({
      messages: baseMessages,
      onTextDelta: (t) => seen.push(t),
    });

    expect(seen.join('')).toBe('Привет');
    expect(result.reason).toBe('stop');
  });

  it('treats connection-close-without-done as disconnected', async () => {
    stubFetch({
      chunks: ['event: text_delta\ndata: {"text":"partial"}\n\n'],
    });

    const seen: string[] = [];
    const result = await streamChat({
      messages: baseMessages,
      onTextDelta: (t) => seen.push(t),
    });

    expect(seen).toEqual(['partial']);
    expect(result.reason).toBe('disconnected');
  });
});

describe('streamChat error paths', () => {
  it('throws ChatError(auth_failed) on 401', async () => {
    stubFetch({ status: 401, jsonError: { code: 'auth_failed', message: 'bad key' } });

    await expect(
      streamChat({ messages: baseMessages, onTextDelta: () => {} }),
    ).rejects.toMatchObject({
      name: 'ChatError',
      code: 'auth_failed',
      message: 'bad key',
    });
  });

  it('falls back to status-derived code when error body lacks code', async () => {
    stubFetch({ status: 429, jsonError: {} });

    const err = await streamChat({
      messages: baseMessages,
      onTextDelta: () => {},
    }).catch((e: unknown) => e);

    expect(err).toBeInstanceOf(ChatError);
    expect((err as ChatError).code).toBe('rate_limit');
  });

  it('throws ChatError(no_body) when response body is null', async () => {
    stubFetch({ status: 200, body: null });

    await expect(
      streamChat({ messages: baseMessages, onTextDelta: () => {} }),
    ).rejects.toMatchObject({ code: 'no_body' });
  });

  it('throws ChatError on mid-stream error events', async () => {
    stubFetch({
      chunks: [
        'event: text_delta\ndata: {"text":"ok"}\n\n',
        'event: error\ndata: {"code":"provider_error","message":"boom"}\n\n',
      ],
    });

    const err = await streamChat({
      messages: baseMessages,
      onTextDelta: () => {},
    }).catch((e: unknown) => e);

    expect(err).toBeInstanceOf(ChatError);
    expect((err as ChatError).code).toBe('provider_error');
    expect((err as ChatError).message).toBe('boom');
  });

  it('coerces network errors into ChatError(network)', async () => {
    stubFetch({ throwOnFetch: new TypeError('fetch failed: ECONNREFUSED') });

    const err = await streamChat({
      messages: baseMessages,
      onTextDelta: () => {},
    }).catch((e: unknown) => e);

    expect(err).toBeInstanceOf(ChatError);
    expect((err as ChatError).code).toBe('network');
  });

  it('coerces aborts into ChatError(aborted)', async () => {
    stubFetch({
      throwOnFetch: new DOMException('The user aborted a request.', 'AbortError'),
    });

    const err = await streamChat({
      messages: baseMessages,
      onTextDelta: () => {},
    }).catch((e: unknown) => e);

    expect(err).toBeInstanceOf(ChatError);
    expect((err as ChatError).code).toBe('aborted');
  });
});
