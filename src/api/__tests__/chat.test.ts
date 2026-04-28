import { describe, it, expect } from 'vitest';
import { parseSseEvents } from '../chat';

describe('parseSseEvents', () => {
  it('parses single event with data', () => {
    const raw = 'event: text_delta\ndata: {"text":"hi"}\n\n';
    const events = parseSseEvents(raw);
    expect(events).toEqual([{ event: 'text_delta', data: { text: 'hi' } }]);
  });

  it('parses multiple events', () => {
    const raw =
      'event: text_delta\ndata: {"text":"a"}\n\nevent: text_delta\ndata: {"text":"b"}\n\nevent: done\ndata: {"reason":"stop"}\n\n';
    const events = parseSseEvents(raw);
    expect(events).toHaveLength(3);
    expect(events[2]).toEqual({ event: 'done', data: { reason: 'stop' } });
  });

  it('returns empty for incomplete events (no double newline)', () => {
    const raw = 'event: text_delta\ndata: {"text":"hi"}';
    expect(parseSseEvents(raw)).toEqual([]);
  });

  it('skips events with non-json data gracefully', () => {
    const raw = 'event: text_delta\ndata: not json\n\n';
    const events = parseSseEvents(raw);
    expect(events).toEqual([{ event: 'text_delta', data: 'not json' }]);
  });
});
