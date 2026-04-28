/**
 * Server-Sent Events parsing - WHATWG-spec-compliant subset.
 *
 * The W3C SSE spec (https://html.spec.whatwg.org/multipage/server-sent-events.html)
 * splits the stream by `\n\n` (or `\r\n\r\n` or `\r\r`) into events. Each
 * event is a sequence of lines `field: value`. We support `event:`, `data:`,
 * and ignore (per spec) `id:`, `retry:`, plus comment lines starting with `:`.
 *
 * Multi-line `data:` values are joined with `\n` per spec.
 *
 * Caller passes the entire accumulated buffer; only blocks ending in a
 * blank-line separator are emitted. The trailing partial block stays in the
 * caller's buffer until more bytes arrive.
 */

export interface SseEvent {
  event: string;
  data: unknown;
}

const FIELD_SEPARATORS = /\r\n\r\n|\r\r|\n\n/;

export function parseSseEvents(raw: string): SseEvent[] {
  const blocks = raw.split(FIELD_SEPARATORS);
  // Only blocks BEFORE the final element are complete - the last element is
  // the partial block (or '' if the input ended cleanly with a separator).
  const complete = blocks.slice(0, -1);
  const events: SseEvent[] = [];
  for (const block of complete) {
    if (!block.trim()) continue;
    const ev = parseBlock(block);
    if (ev !== null) events.push(ev);
  }
  return events;
}

function parseBlock(block: string): SseEvent | null {
  let eventName = '';
  const dataLines: string[] = [];
  // Each line within a block; lines may use \n or \r as separators per spec.
  for (const rawLine of block.split(/\r\n|\r|\n/)) {
    if (rawLine.length === 0) continue;
    if (rawLine.startsWith(':')) {
      // Comment - keepalive pings, server hints. Discard.
      continue;
    }
    const colon = rawLine.indexOf(':');
    let field: string;
    let value: string;
    if (colon === -1) {
      // Spec: "If the line contains no U+003A COLON character, process the
      // line by treating the line as the field name and treating the value
      // as the empty string."
      field = rawLine;
      value = '';
    } else {
      field = rawLine.slice(0, colon);
      value = rawLine.slice(colon + 1);
      // Spec: leading single space is dropped. "data: foo" -> "foo".
      if (value.startsWith(' ')) value = value.slice(1);
    }
    switch (field) {
      case 'event':
        eventName = value;
        break;
      case 'data':
        dataLines.push(value);
        break;
      // id, retry, and any other field names are ignored.
    }
  }
  if (!eventName) return null;
  const dataStr = dataLines.join('\n');
  let data: unknown;
  try {
    data = JSON.parse(dataStr);
  } catch {
    data = dataStr;
  }
  return { event: eventName, data };
}
