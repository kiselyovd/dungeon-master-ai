import { describe, expect, it } from 'vitest';
import { withTimeout } from '../withTimeout';

describe('withTimeout', () => {
  it('resolves with the value when it settles before the deadline', async () => {
    const result = await withTimeout(Promise.resolve('ok'), 50, 'fallback');
    expect(result).toBe('ok');
  });

  it('resolves with the fallback when the promise hangs past the deadline', async () => {
    // A promise that never settles - models a Stronghold vault read that hangs
    // and would otherwise block hydration forever (audit blocker 1).
    const never = new Promise<string>(() => {});
    const result = await withTimeout(never, 10, 'fallback');
    expect(result).toBe('fallback');
  });

  it('propagates rejection so the caller can decide how to handle it', async () => {
    await expect(withTimeout(Promise.reject(new Error('boom')), 50, 'fallback')).rejects.toThrow(
      'boom',
    );
  });
});
