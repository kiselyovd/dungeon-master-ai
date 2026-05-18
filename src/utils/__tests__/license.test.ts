import { describe, expect, it } from 'vitest';
import { isOssLicense } from '../license';

describe('isOssLicense', () => {
  it.each(['Apache 2.0', 'Apache 2.0 (Qwen)', 'MIT'])('accepts OSS license: %s', (lic) => {
    expect(isOssLicense(lic)).toBe(true);
  });

  it.each([
    'SAI NC',
    'FLUX-dev NC',
    'varies per model',
    'Anthropic ToS',
    'LTX (re-check before GA)',
    '',
    'unknown',
  ])('rejects non-OSS license: %s', (lic) => {
    expect(isOssLicense(lic)).toBe(false);
  });
});
