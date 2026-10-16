type Waiter = { resolve: () => void; reject: (error: unknown) => void };

/** Serialize asynchronous disk writes and fold a burst into its latest pending value. */
export function createCoalescingWriter<T>(
  write: (value: T) => Promise<void>,
  merge: (pending: T, latest: T) => T = (_pending, latest) => latest,
): (value: T) => Promise<void> {
  let pending: { value: T; waiters: Waiter[] } | null = null;
  let draining = false;

  async function drain() {
    draining = true;
    while (pending) {
      const batch = pending;
      pending = null;
      try {
        await write(batch.value);
        for (const waiter of batch.waiters) waiter.resolve();
      } catch (error) {
        for (const waiter of batch.waiters) waiter.reject(error);
      }
    }
    draining = false;
  }

  return (value) =>
    new Promise<void>((resolve, reject) => {
      if (pending) {
        pending.value = merge(pending.value, value);
        pending.waiters.push({ resolve, reject });
      } else pending = { value, waiters: [{ resolve, reject }] };
      if (!draining) void drain();
    });
}
