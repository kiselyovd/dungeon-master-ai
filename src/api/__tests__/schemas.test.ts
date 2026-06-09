import { describe, expect, it } from 'vitest';
import { safeParseImageGenerated } from '../schemas';

describe('safeParseImageGenerated kind', () => {
  it('parses kind when present', () => {
    const p = safeParseImageGenerated({ mime_type: 'image/png', image_b64: 'AAA', kind: 'map' });
    expect(p?.kind).toBe('map');
  });
  it('tolerates missing kind', () => {
    const p = safeParseImageGenerated({ mime_type: 'image/png', image_b64: 'AAA' });
    expect(p).not.toBeNull();
    expect(p?.kind).toBeUndefined();
  });
});
