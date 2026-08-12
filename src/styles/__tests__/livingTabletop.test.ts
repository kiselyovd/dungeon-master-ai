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

  it('stops non-essential motion for reduced-motion users', () => {
    expect(css).toContain('@media (prefers-reduced-motion: reduce)');
    expect(css).toContain('animation: none !important');
    expect(css).toContain('transform: none !important');
  });
});
