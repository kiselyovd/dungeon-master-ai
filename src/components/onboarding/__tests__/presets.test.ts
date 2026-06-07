import { describe, expect, it } from 'vitest';
import { computeSteps } from '../presets';

describe('computeSteps', () => {
  // Hero comes right after the preset choice, before any technical setup:
  // the narrative choice (who you are) precedes provider/modality wiring.
  it('local-only: hero before chat, includes image but not video', () => {
    expect(computeSteps('local-only')).toEqual(['welcome', 'preset', 'hero', 'chat', 'image']);
  });

  it('hybrid: hero before chat, includes image but not video', () => {
    expect(computeSteps('hybrid')).toEqual(['welcome', 'preset', 'hero', 'chat', 'image']);
  });

  it('cloud-cinematic: hero before chat, includes both image and video', () => {
    expect(computeSteps('cloud-cinematic')).toEqual([
      'welcome',
      'preset',
      'hero',
      'chat',
      'image',
      'video',
    ]);
  });

  it('text-only: hero before chat, skips image and video', () => {
    expect(computeSteps('text-only')).toEqual(['welcome', 'preset', 'hero', 'chat']);
  });

  it('manual is an honest skip: only welcome, preset, hero (no chat/image/video)', () => {
    expect(computeSteps('manual')).toEqual(['welcome', 'preset', 'hero']);
  });
});
