import { describe, expect, it } from 'vitest';
import { SseStreamDecoder } from '../sseStream';

const encoder = new TextEncoder();

describe('SseStreamDecoder', () => {
  it('decodes fragmented LF, CRLF, and CR framed events in order', () => {
    const decoder = new SseStreamDecoder();
    expect(decoder.push(encoder.encode('event: text_delta\r\ndata: {"text":"hel'))).toEqual([]);
    expect(decoder.push(encoder.encode('lo"}\r\n\r\nevent: done\rdata: {}\r\r'))).toEqual([
      { event: 'text_delta', data: { text: 'hello' } },
      { event: 'done', data: {} },
    ]);
  });

  it('flushes an unterminated final event exactly once', () => {
    const decoder = new SseStreamDecoder();
    expect(decoder.push(encoder.encode('event: agent_done\ndata: {"total_rounds":2}'))).toEqual([]);
    expect(decoder.finish()).toEqual([{ event: 'agent_done', data: { total_rounds: 2 } }]);
    expect(decoder.finish()).toEqual([]);
  });
});
