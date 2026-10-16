import { describe, expect, it } from 'vitest';
import { serializeJavaScriptLiteral } from './cdp';

describe('CDP JavaScript argument literals', () => {
  it.each([
    null,
    'actor-1',
    '"\\); globalThis.changed = true; //',
    '</script><script>bad()</script>',
    '\u2028\u2029',
  ])('preserves the value without raw code-boundary characters: %j', (value) => {
    const literal = serializeJavaScriptLiteral(value);
    expect(JSON.parse(literal)).toEqual(value);
    expect(literal).not.toMatch(/[<>\u2028\u2029]/u);
  });
});
