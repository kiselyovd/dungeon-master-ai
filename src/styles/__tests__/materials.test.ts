import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';

describe('living tabletop materials', () => {
  const css = readFileSync(resolve(__dirname, '../materials.css'), 'utf8');

  it.each([
    'leather',
    'stone',
    'oak',
    'parchment',
    'velvet',
    'bronze',
  ])('maps the %s semantic material to living-tabletop art', (name) => {
    expect(css).toContain(`--dm-texture-${name}`);
    expect(css).toContain(`material-${name}.webp`);
  });

  it('retains reduced-transparency fallback', () => {
    expect(css).toContain('@media (prefers-reduced-transparency: reduce)');
    expect(css).toContain('background-image: none');
  });
});
