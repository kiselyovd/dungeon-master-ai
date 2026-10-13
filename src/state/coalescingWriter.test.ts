import { describe, expect, it, vi } from 'vitest';
import { createCoalescingWriter } from './coalescingWriter';

describe('coalescing persistence writes', () => {
  it('bounds a burst to the in-flight value and the newest pending value', async () => {
    let release: () => void = () => {};
    const gate = new Promise<void>((resolve) => {
      release = resolve;
    });
    const write = vi.fn(async (value: number) => {
      if (value === 1) await gate;
    });
    const enqueue = createCoalescingWriter(write);
    const writes = [enqueue(1), enqueue(2), enqueue(3), enqueue(4)];
    expect(write.mock.calls.map(([value]) => value)).toEqual([1]);
    release();
    await Promise.all(writes);
    expect(write.mock.calls.map(([value]) => value)).toEqual([1, 4]);
  });
  it('allows a later write after a failed in-flight value', async () => {
    const write = vi.fn(async (value: number) => {
      if (value === 1) throw Error('disk unavailable');
    });
    const enqueue = createCoalescingWriter(write);
    const failure = enqueue(1);
    const success = enqueue(2);
    await expect(failure).rejects.toThrow('disk unavailable');
    await expect(success).resolves.toBeUndefined();
    expect(write.mock.calls.map(([value]) => value)).toEqual([1, 2]);
  });
});
