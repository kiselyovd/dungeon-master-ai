import { describe, expect, it, vi } from 'vitest';
import { flushClosePreparations, registerClosePreparation } from '../closePreparations';

describe('presentation disk barriers', () => {
  it('waits for every mounted draft and removes disposed scopes', async () => {
    let release: (() => void) | undefined;
    const first = vi.fn(
      () =>
        new Promise<void>((resolve) => {
          release = resolve;
        }),
    );
    const second = vi.fn(async () => {});
    const removeFirst = registerClosePreparation(first),
      removeSecond = registerClosePreparation(second);
    let done = false;
    const pending = flushClosePreparations().then(() => {
      done = true;
    });
    expect(first).toHaveBeenCalledTimes(1);
    expect(second).toHaveBeenCalledTimes(1);
    expect(done).toBe(false);
    release?.();
    await pending;
    removeFirst();
    removeSecond();
    await flushClosePreparations();
    expect(first).toHaveBeenCalledTimes(1);
  });
  it('fails the barrier rather than discarding a failed draft write', async () => {
    const remove = registerClosePreparation(async () => {
      throw Error('disk-failed');
    });
    await expect(flushClosePreparations()).rejects.toThrow('disk-failed');
    remove();
  });
});
