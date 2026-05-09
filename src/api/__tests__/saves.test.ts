import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { ChatError } from '../errors';
import {
  createSave,
  deleteSaveById,
  fetchSaveById,
  fetchSessionSaves,
  quickSaveSession,
} from '../saves';

vi.mock('../client', () => ({
  backendUrl: vi.fn(async (path: string) => `http://test.local${path}`),
}));

beforeEach(() => {
  vi.unstubAllGlobals();
});
afterEach(() => {
  vi.unstubAllGlobals();
});

describe('saves API', () => {
  it('fetchSessionSaves returns the parsed list', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(
        async () =>
          new Response(
            JSON.stringify([
              {
                id: 's1',
                session_id: 'sess1',
                kind: 'manual',
                title: 'before boss',
                summary: 'rest at gate',
                tag: 'exploration',
                created_at: '2026-05-09T10:00:00Z',
                turn_number: 0,
              },
            ]),
            { status: 200, headers: new Headers({ 'content-type': 'application/json' }) },
          ),
      ),
    );
    const list = await fetchSessionSaves('sess1');
    expect(list).toHaveLength(1);
    expect(list[0]?.title).toBe('before boss');
  });

  it('fetchSessionSaves throws ChatError on non-2xx', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => new Response('', { status: 500 })),
    );
    await expect(fetchSessionSaves('sess1')).rejects.toBeInstanceOf(ChatError);
  });

  it('createSave POSTs the JSON body and returns the new id', async () => {
    const calls: { url: string; init: RequestInit }[] = [];
    vi.stubGlobal(
      'fetch',
      vi.fn(async (url: string, init: RequestInit) => {
        calls.push({ url, init });
        return new Response(JSON.stringify({ id: 'new-1' }), {
          status: 201,
          headers: new Headers({ 'content-type': 'application/json' }),
        });
      }),
    );

    const result = await createSave('sess1', {
      kind: 'manual',
      title: 'before boss',
      summary: 'rest',
      tag: 'exploration',
    });
    expect(result.id).toBe('new-1');
    expect(calls).toHaveLength(1);
    expect(calls[0]?.url).toBe('http://test.local/sessions/sess1/saves');
    const body = JSON.parse(calls[0]?.init.body as string) as Record<string, unknown>;
    expect(body).toEqual({
      kind: 'manual',
      title: 'before boss',
      summary: 'rest',
      tag: 'exploration',
    });
    expect(calls[0]?.init.method).toBe('POST');
  });

  it('quickSaveSession POSTs to /quick and returns the new id', async () => {
    const calls: { url: string; init: RequestInit | undefined }[] = [];
    vi.stubGlobal(
      'fetch',
      vi.fn(async (url: string, init?: RequestInit) => {
        calls.push({ url, init });
        return new Response(JSON.stringify({ id: 'q1' }), {
          status: 201,
          headers: new Headers({ 'content-type': 'application/json' }),
        });
      }),
    );
    const result = await quickSaveSession('sess1');
    expect(result.id).toBe('q1');
    expect(calls[0]?.url).toBe('http://test.local/sessions/sess1/saves/quick');
    expect(calls[0]?.init?.method).toBe('POST');
  });

  it('fetchSaveById returns the full row', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(
        async () =>
          new Response(
            JSON.stringify({
              id: 's1',
              session_id: 'sess1',
              kind: 'manual',
              title: 't',
              summary: 's',
              tag: 'combat',
              created_at: '2026-05-09T00:00:00Z',
              turn_number: 0,
              game_state: { schema_version: 1, state: {} },
            }),
            { status: 200, headers: new Headers({ 'content-type': 'application/json' }) },
          ),
      ),
    );
    const row = await fetchSaveById('s1');
    expect(row.tag).toBe('combat');
    // biome-ignore lint/suspicious/noExplicitAny: test introspection of the unknown game_state envelope
    expect((row.game_state as any).schema_version).toBe(1);
  });

  it('deleteSaveById issues a DELETE and resolves on 204', async () => {
    const calls: { url: string; init: RequestInit | undefined }[] = [];
    vi.stubGlobal(
      'fetch',
      vi.fn(async (url: string, init?: RequestInit) => {
        calls.push({ url, init });
        return new Response(null, { status: 204 });
      }),
    );
    await expect(deleteSaveById('s1')).resolves.toBeUndefined();
    expect(calls[0]?.init?.method).toBe('DELETE');
    expect(calls[0]?.url).toBe('http://test.local/saves/s1');
  });

  it('deleteSaveById throws on non-2xx', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => new Response('', { status: 404 })),
    );
    await expect(deleteSaveById('missing')).rejects.toBeInstanceOf(ChatError);
  });
});
