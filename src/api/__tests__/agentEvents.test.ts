import { afterEach, describe, expect, it, vi } from 'vitest';
import { type AgentTurnOptions, streamAgentTurn } from '../agent';
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

function callbacks(
  events: string[],
): Pick<
  AgentTurnOptions,
  | 'onTextDelta'
  | 'onToolCallStart'
  | 'onToolCallResult'
  | 'onAgentDone'
  | 'onReasoningDelta'
  | 'onImageGenerated'
  | 'onVideoGenerated'
> {
  return {
    onTextDelta: (text) => events.push(`text:${text}`),
    onToolCallStart: (id, toolName, round) => events.push(`tool-start:${id}:${toolName}:${round}`),
    onToolCallResult: (id, toolName, _args, _result, isError, round, handledBy) =>
      events.push(`tool-result:${id}:${toolName}:${isError}:${round}:${handledBy}`),
    onAgentDone: (rounds) => events.push(`done:${rounds}`),
    onReasoningDelta: (text) => events.push(`reasoning:${text}`),
    onImageGenerated: (url, toolCallId, kind) => events.push(`image:${toolCallId}:${kind}:${url}`),
    onVideoGenerated: (url, toolCallId, kind) => events.push(`video:${toolCallId}:${kind}:${url}`),
  };
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
    ...callbacks(events),
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
        'event: image_generated\ndata: {"tool_call_id":"img-1","round":1,"mime_type":"image/png","image_b64":"aQ==","kind":"map"}\n\n',
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
      'image:img-1:map:data:image/png;base64,aQ==',
      'text:hello',
      'tool-start:call-1:roll_dice:1',
      'tool-result:call-1:roll_dice:false:1:engine',
      'video:vid-1:chat:data:video/mp4;base64,dg==',
      'done:1',
    ]);
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
        ...callbacks(events),
      }),
    ).rejects.toMatchObject({ code: 'aborted' });
    expect(events).toEqual([]);
  });
});
