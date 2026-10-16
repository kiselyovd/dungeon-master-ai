import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';

describe('living tabletop motion boundary', () => {
  const css = readFileSync(resolve(__dirname, '../living-tabletop.css'), 'utf8');

  it('limits ambient animation to approved surfaces', () => {
    expect(css).toContain('[data-art-direction="living-tabletop"]');
    expect(css).not.toContain('.dm-chat-panel');
    expect(css).not.toContain('.dm-composer');
  });

  it('stops non-essential motion while preserving interactive geometry transforms', () => {
    expect(css).toContain('@media (prefers-reduced-motion: reduce)');
    expect(css).toContain('animation: none;');
    expect(css).not.toMatch(/\[data-art-direction="living-tabletop"\]\s*\*\s*\{/);
    expect(css).not.toMatch(
      /\[data-art-direction="living-tabletop"\]\s*\*\s*\{[^}]*transform:\s*none/,
    );
  });
});
