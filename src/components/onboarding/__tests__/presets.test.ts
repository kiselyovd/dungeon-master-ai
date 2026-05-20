import { describe, expect, it } from 'vitest';
import { computeSteps } from '../presets';

describe('computeSteps', () => {
  it('local-only includes image but not video', () => {
    expect(computeSteps('local-only')).toEqual(['welcome', 'preset', 'chat', 'image', 'hero']);
  });

  it('hybrid includes image but not video', () => {
    expect(computeSteps('hybrid')).toEqual(['welcome', 'preset', 'chat', 'image', 'hero']);
  });

  it('manual includes image but not video', () => {
    expect(computeSteps('manual')).toEqual(['welcome', 'preset', 'chat', 'image', 'hero']);
  });

  it('cloud-cinematic includes both image and video', () => {
    expect(computeSteps('cloud-cinematic')).toEqual([
      'welcome',
      'preset',
      'chat',
      'image',
      'video',
      'hero',
    ]);
  });

  it('text-only skips image and video', () => {
    expect(computeSteps('text-only')).toEqual(['welcome', 'preset', 'chat', 'hero']);
  });
});
