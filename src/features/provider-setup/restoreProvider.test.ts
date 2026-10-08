import { expect, it, vi } from 'vitest';
import { restoreProviderOnStartup } from './restoreProvider';

it('waits for hydrated settings and restores again for a fresh backend', async () => {
  let hydrated = false;
  let onHydrated!: () => void;
  let onReady!: () => void;
  const apply = vi.fn(async () => {}),
    failed = vi.fn(),
    detach = vi.fn();
  const stop = restoreProviderOnStartup({
    hydrated: () => hydrated,
    configured: () => true,
    apply,
    failed,
    onHydrated: (c) => {
      onHydrated = c;
      return detach;
    },
    onBackendReady: (c) => {
      onReady = c;
      return detach;
    },
  });
  await Promise.resolve();
  expect(apply).not.toHaveBeenCalled();
  hydrated = true;
  onHydrated();
  await Promise.resolve();
  expect(apply).toHaveBeenCalledTimes(1);
  onReady();
  await Promise.resolve();
  expect(apply).toHaveBeenCalledTimes(2);
  stop();
  onReady();
  await Promise.resolve();
  expect(apply).toHaveBeenCalledTimes(2);
  expect(detach).toHaveBeenCalledTimes(2);
});
it('a guest without a local provider does not publish a default configuration', async () => {
  const apply = vi.fn(async () => {});
  const stop = restoreProviderOnStartup({
    hydrated: () => true,
    configured: () => false,
    apply,
    failed: vi.fn(),
    onHydrated: () => () => {},
    onBackendReady: () => () => {},
  });
  await Promise.resolve();
  expect(apply).not.toHaveBeenCalled();
  stop();
});
it('a synchronous invalid runtime configuration is reported without breaking bootstrap', async () => {
  const failed = vi.fn();
  const stop = restoreProviderOnStartup({
    hydrated: () => true,
    configured: () => true,
    apply: () => {
      throw new Error('not ready');
    },
    failed,
    onHydrated: () => () => {},
    onBackendReady: () => () => {},
  });
  await new Promise((r) => setTimeout(r, 0));
  expect(failed).toHaveBeenCalledTimes(1);
  stop();
});
