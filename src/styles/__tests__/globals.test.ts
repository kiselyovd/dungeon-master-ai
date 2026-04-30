import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';

describe('globals.css prefers-reduced-motion', () => {
  it('contains @media prefers-reduced-motion block with 0.01ms timings', () => {
    const css = readFileSync(resolve(__dirname, '../../styles/globals.css'), 'utf-8');
    expect(css).toContain('prefers-reduced-motion');
    expect(css).toContain('0.01ms');
  });
});
