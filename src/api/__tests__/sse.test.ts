import { describe, expect, it } from 'vitest';
import { parseSseEvents } from '../sse';

describe('parseSseEvents - SSE spec edge cases', () => {
  it('parses single event with data', () => {
    const events = parseSseEvents('event: text_delta\ndata: {"text":"hi"}\n\n');
    expect(events).toEqual([{ event: 'text_delta', data: { text: 'hi' } }]);
  });

  it('parses multiple events back-to-back', () => {
    const raw =
      'event: text_delta\ndata: {"text":"a"}\n\n' +
      'event: text_delta\ndata: {"text":"b"}\n\n' +
      'event: done\ndata: {"reason":"stop"}\n\n';
    const events = parseSseEvents(raw);
    expect(events).toHaveLength(3);
    expect(events[2]).toEqual({ event: 'done', data: { reason: 'stop' } });
  });

  it('returns empty for incomplete events (no double newline)', () => {
    expect(parseSseEvents('event: text_delta\ndata: {"text":"hi"}')).toEqual([]);
  });

  it('keeps non-JSON data as raw string', () => {
    expect(parseSseEvents('event: text_delta\ndata: not json\n\n')).toEqual([
      { event: 'text_delta', data: 'not json' },
    ]);
  });

  it('handles CRLF separators (Windows / proxy emitted)', () => {
    const events = parseSseEvents(
      'event: text_delta\r\ndata: {"text":"crlf"}\r\n\r\nevent: done\r\ndata: {"reason":"stop"}\r\n\r\n',
    );
    expect(events).toHaveLength(2);
    expect(events[0]).toEqual({ event: 'text_delta', data: { text: 'crlf' } });
  });

  it('skips comment lines (keepalives starting with `:`)', () => {
    const raw = ': keepalive ping\n: another comment\nevent: text_delta\ndata: {"text":"x"}\n\n';
    expect(parseSseEvents(raw)).toEqual([{ event: 'text_delta', data: { text: 'x' } }]);
  });

  it('joins multi-line data fields with \\n per spec', () => {
    const raw = 'event: text_delta\ndata: line1\ndata: line2\n\n';
    expect(parseSseEvents(raw)).toEqual([{ event: 'text_delta', data: 'line1\nline2' }]);
  });

  it('drops a single leading space after the field colon', () => {
    // "data: foo" -> "foo" (one space stripped); "data:  foo" -> " foo"
    expect(parseSseEvents('event: x\ndata:  with-leading-space\n\n')).toEqual([
      { event: 'x', data: ' with-leading-space' },
    ]);
  });

  it('ignores unknown fields like id and retry', () => {
    const raw = 'id: 42\nretry: 3000\nevent: text_delta\ndata: {"text":"y"}\n\n';
    expect(parseSseEvents(raw)).toEqual([{ event: 'text_delta', data: { text: 'y' } }]);
  });

  it('drops blocks without an event field', () => {
    expect(parseSseEvents('data: orphaned\n\n')).toEqual([]);
  });

  it('handles a field with no colon by treating the whole line as field name', () => {
    // Per spec, a line without ":" is "treated as field, value is empty
    // string". We keep the event empty, so the block is dropped (no event).
    expect(parseSseEvents('text_delta\ndata: x\n\n')).toEqual([]);
  });
});
