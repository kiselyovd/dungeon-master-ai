import { afterEach, describe, expect, it, vi } from 'vitest';
import { type AgentEvent, streamAgentTurn } from '../agent';
import { setBackendPortForTesting } from '../client';

const encoder = new TextEncoder();

function streamingResponse(chunks: string[]): Response {
  return new Response(
    new ReadableStream<Uint8Array>({
      start(controller) {
        for (const chunk of chunks) controller.enqueue(encoder.encode(chunk));
        controller.close();
      },
    }),
    { status: 200, headers: { 'content-type': 'text/event-stream' } },
  );
}

function captureEvent(events: string[], event: AgentEvent): void {
  switch (event.type) {
    case 'text_delta':
      events.push(`text:${event.text}`);
      break;
    case 'reasoning_text':
      events.push(`reasoning:${event.text}`);
      break;
    case 'tool_call_start':
      events.push(`tool-start:${event.id}:${event.toolName}:${event.round}`);
      break;
    case 'tool_call_result':
      events.push(
        `tool-result:${event.id}:${event.toolName}:${event.isError}:${event.round}:${event.handledBy}`,
      );
      break;
    case 'image_generated':
      events.push(
        `image:${event.toolCallId}:${event.kind}:${event.source.type}:${event.source.type === 'bundled' ? event.source.assetId : '-'}:${event.dataUrl}`,
      );
      break;
    case 'video_generated':
      events.push(`video:${event.toolCallId}:${event.kind}:${event.dataUrl}`);
      break;
    case 'agent_done':
      events.push(`done:${event.totalRounds}`);
      break;
    case 'done':
    case 'error':
      break;
  }
}

async function consume(chunks: string[], events: string[]): Promise<void> {
  setBackendPortForTesting(45678);
  vi.stubGlobal(
    'fetch',
    vi.fn(async () => streamingResponse(chunks)),
  );
  await streamAgentTurn({
    campaignId: 'campaign-1',
    sessionId: 'session-1',
    playerMessage: 'advance',
    history: [],
    onEvent: (event) => captureEvent(events, event),
  });
}

describe('streamAgentTurn event framing', () => {
  afterEach(() => {
    setBackendPortForTesting(null);
    vi.unstubAllGlobals();
  });

  it('preserves the backend event order and exact payload mapping', async () => {
    const events: string[] = [];
    await consume(
      [
        'event: reasoning_text\ndata: {"text":"plan"}\n\n',
        'event: image_generated\ndata: {"tool_call_id":"img-1","round":1,"mime_type":"image/png","image_b64":"aQ==","kind":"map","source":"bundled","asset_id":"map-forest-crossing"}\n\n',
        'event: text_delta\ndata: {"text":"hello"}\n\n',
        'event: tool_call_start\ndata: {"id":"call-1","tool_name":"roll_dice","round":1}\n\n',
        'event: tool_call_result\ndata: {"id":"call-1","tool_name":"roll_dice","args":{},"result":{"total":7},"is_error":false,"round":1,"handled_by":"engine"}\n\n',
        'event: video_generated\ndata: {"tool_call_id":"vid-1","round":1,"mime_type":"video/mp4","video_b64":"dg==","kind":"chat"}\n\n',
        'event: agent_done\ndata: {"total_rounds":1}\n\n',
      ],
      events,
    );

    expect(events).toEqual([
      'reasoning:plan',
      'image:img-1:map:bundled:map-forest-crossing:data:image/png;base64,aQ==',
      'text:hello',
      'tool-start:call-1:roll_dice:1',
      'tool-result:call-1:roll_dice:false:1:engine',
      'video:vid-1:chat:data:video/mp4;base64,dg==',
      'done:1',
    ]);
  });

  it('defaults legacy image events without provenance to generated', async () => {
    const events: string[] = [];
    await consume(
      [
        'event: image_generated\ndata: {"tool_call_id":"legacy-1","mime_type":"image/png","image_b64":"aQ==","kind":"chat"}\n\n',
      ],
      events,
    );

    expect(events).toEqual(['image:legacy-1:chat:generated:-:data:image/png;base64,aQ==']);
  });

  it('decodes fragmented CRLF and CR events and flushes the final EOF event', async () => {
    const events: string[] = [];
    await consume(
      [
        'event: text_delta\r\ndata: {"text":"hel',
        'lo"}\r\n\r\nevent: reasoning_text\rdata: {"text":"think"}\r\r',
        'event: agent_done\r\ndata: {"total_rounds":2}',
      ],
      events,
    );

    expect(events).toEqual(['text:hello', 'reasoning:think', 'done:2']);
  });

  it('ignores malformed payloads without dispatching partial callbacks', async () => {
    const events: string[] = [];
    await consume(
      [
        'event: tool_call_start\ndata: {"id":4,"tool_name":"roll_dice","round":1}\n\n',
        'event: text_delta\ndata: not-json\n\n',
        'event: agent_done\ndata: {"total_rounds":0}\n\n',
      ],
      events,
    );

    expect(events).toEqual(['done:0']);
  });

  it('surfaces a structured stream error without dispatching finalization', async () => {
    const events: string[] = [];

    await expect(
      consume(
        ['event: error\ndata: {"code":"provider_error","message":"provider unavailable"}\n\n'],
        events,
      ),
    ).rejects.toMatchObject({ code: 'provider_error' });
    expect(events).toEqual([]);
  });

  it('classifies an aborted transport without dispatching callbacks', async () => {
    const events: string[] = [];
    setBackendPortForTesting(45678);
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => {
        throw new DOMException('aborted', 'AbortError');
      }),
    );

    await expect(
      streamAgentTurn({
        campaignId: 'campaign-1',
        sessionId: 'session-1',
        playerMessage: 'advance',
        history: [],
        onEvent: (event) => captureEvent(events, event),
      }),
    ).rejects.toMatchObject({ code: 'aborted' });
    expect(events).toEqual([]);
  });

  it('cancels and releases the response reader after agent_done', async () => {
    const cancel = vi.fn(async () => undefined);
    const releaseLock = vi.fn();
    const read = vi
      .fn()
      .mockResolvedValueOnce({
        done: false,
        value: encoder.encode('event: agent_done\ndata: {"total_rounds":1}\n\n'),
      })
      .mockResolvedValueOnce({ done: true, value: undefined });
    setBackendPortForTesting(45678);
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => ({
        ok: true,
        status: 200,
        body: { getReader: () => ({ read, cancel, releaseLock }) },
      })),
    );

    await streamAgentTurn({
      campaignId: 'campaign-1',
      sessionId: 'session-1',
      playerMessage: 'advance',
      history: [],
      onEvent: () => undefined,
    });

    expect(releaseLock).toHaveBeenCalledTimes(1);
    expect(read).toHaveBeenCalledOnce();
    expect(cancel).toHaveBeenCalledOnce();
  });
});
