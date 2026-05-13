import { afterEach, describe, expect, it, vi } from 'vitest';
import { EMPTY_DRAFT } from '../../state/charCreation';
import { streamCharacterField, streamFullCharacter, streamTestChat } from '../characterAssist';

vi.mock('../client', () => ({
  backendUrl: vi.fn(async (path: string) => `http://test.local${path}`),
}));

function mockSseResponse(events: { event: string; data: string }[]): Response {
  const text = events.flatMap((e) => [`event: ${e.event}`, `data: ${e.data}`, '']).join('\n');
  const body = new ReadableStream({
    start(controller) {
      controller.enqueue(new TextEncoder().encode(text));
      controller.close();
    },
  });
  return new Response(body, {
    status: 200,
    headers: { 'content-type': 'text/event-stream' },
  });
}

afterEach(() => {
  vi.restoreAllMocks();
});

describe('characterAssist clients', () => {
  it('streamCharacterField calls onToken per token + onDone', async () => {
    globalThis.fetch = vi.fn().mockResolvedValueOnce(
      mockSseResponse([
        { event: 'token', data: '{"type":"token","text":"Alpha "}' },
        { event: 'token', data: '{"type":"token","text":"Beta"}' },
        { event: 'done', data: '{"type":"done"}' },
      ]),
    );

    const onToken = vi.fn();
    const onDone = vi.fn();
    await streamCharacterField({
      field: 'name',
      draft: EMPTY_DRAFT,
      locale: 'en',
      onToken,
      onError: vi.fn(),
      onDone,
    });
    expect(onToken).toHaveBeenCalledWith('Alpha ');
    expect(onToken).toHaveBeenCalledWith('Beta');
    expect(onDone).toHaveBeenCalledOnce();
  });

  it('streamFullCharacter calls onPatch per draft_patch + onDone', async () => {
    globalThis.fetch = vi.fn().mockResolvedValueOnce(
      mockSseResponse([
        {
          event: 'draft_patch',
          data: '{"type":"draft_patch","patch":{"classId":"fighter"}}',
        },
        { event: 'done', data: '{"type":"done"}' },
      ]),
    );

    const onPatch = vi.fn();
    const onDone = vi.fn();
    await streamFullCharacter({
      draft: EMPTY_DRAFT,
      locale: 'en',
      onPatch,
      onError: vi.fn(),
      onDone,
    });
    expect(onPatch).toHaveBeenCalledWith({ classId: 'fighter' });
    expect(onDone).toHaveBeenCalledOnce();
  });

  it('streamTestChat calls onToken per token + onDone', async () => {
    globalThis.fetch = vi.fn().mockResolvedValueOnce(
      mockSseResponse([
        { event: 'token', data: '{"type":"token","text":"Hello"}' },
        { event: 'done', data: '{"type":"done"}' },
      ]),
    );

    const onToken = vi.fn();
    const onDone = vi.fn();
    await streamTestChat({
      draft: EMPTY_DRAFT,
      history: [],
      userMessage: 'Hi',
      locale: 'en',
      onToken,
      onError: vi.fn(),
      onDone,
    });
    expect(onToken).toHaveBeenCalledWith('Hello');
    expect(onDone).toHaveBeenCalledOnce();
  });

  it('streamCharacterField propagates error events', async () => {
    globalThis.fetch = vi.fn().mockResolvedValueOnce(
      mockSseResponse([
        {
          event: 'error',
          data: '{"type":"error","code":"provider_error","message":"oops"}',
        },
      ]),
    );

    const onError = vi.fn();
    await streamCharacterField({
      field: 'name',
      draft: EMPTY_DRAFT,
      locale: 'en',
      onToken: vi.fn(),
      onError,
      onDone: vi.fn(),
    });
    expect(onError).toHaveBeenCalledOnce();
    expect((onError.mock.calls[0]?.[0] as Error).message).toMatch(/oops/);
  });
});
