import type { SseEvent } from './sse';
import { parseSseEvents } from './sse';

const EVENT_SEPARATOR = /\r\n\r\n|\r\r|\n\n/g;

/** Incrementally decodes an SSE byte stream and flushes a final unterminated event at EOF. */
export class SseStreamDecoder {
  private readonly decoder = new TextDecoder();
  private buffer = '';

  push(chunk: Uint8Array): SseEvent[] {
    this.buffer += this.decoder.decode(chunk, { stream: true });
    return this.consume(false);
  }

  finish(): SseEvent[] {
    this.buffer += this.decoder.decode();
    return this.consume(true);
  }

  private consume(flush: boolean): SseEvent[] {
    const events: SseEvent[] = [];
    let consumedThrough = 0;
    EVENT_SEPARATOR.lastIndex = 0;
    let match = EVENT_SEPARATOR.exec(this.buffer);
    while (match) {
      const block = this.buffer.slice(consumedThrough, match.index);
      if (block.trim()) events.push(...parseSseEvents(`${block}\n\n`));
      consumedThrough = match.index + match[0].length;
      match = EVENT_SEPARATOR.exec(this.buffer);
    }
    this.buffer = this.buffer.slice(consumedThrough);
    if (flush && this.buffer.trim()) {
      events.push(...parseSseEvents(`${this.buffer}\n\n`));
      this.buffer = '';
    }
    return events;
  }
}
