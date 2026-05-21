import { expect, test } from '@playwright/test';
import { mockTauri } from './fixtures/tauri-mock';

/**
 * Verifies the e2e Tauri mock keeps plugin-store and plugin-stronghold
 * state across a page reload. Before M11-DM Batch A the mock rebuilt its
 * buckets on every `addInitScript` run, so nothing survived a reload and
 * the persistence regression (audit F1) could not be caught in e2e.
 *
 * These specs drive the mock directly through `__TAURI_INTERNALS__.invoke`
 * - no fragile UI selectors. They use an isolated probe store file and a
 * probe Stronghold client that the app itself never touches, so the
 * assertion measures only the mock's reload-survival and never races the
 * app's own persistence writes.
 */

type MockInvoke = { invoke: (cmd: string, args: unknown) => Promise<unknown> };

const PROBE_STORE = 'e2e-probe.json';
const PROBE_CLIENT = 'e2e-probe-client';

test.beforeEach(async ({ page }) => {
  await mockTauri(page);
});

test('plugin-store values survive a page reload', async ({ page }) => {
  await page.goto('/');

  // Write to an isolated probe store the app never loads.
  await page.evaluate(async (store) => {
    const inv = (window as unknown as { __TAURI_INTERNALS__: MockInvoke }).__TAURI_INTERNALS__;
    const rid = (await inv.invoke('plugin:store|load', { path: store })) as number;
    await inv.invoke('plugin:store|set', { rid, key: 'probe', value: 'kept' });
    await inv.invoke('plugin:store|save', { rid });
  }, PROBE_STORE);

  await page.reload();

  // Re-open the same store by filename; the value must still be there.
  const survived = await page.evaluate(async (store) => {
    const inv = (window as unknown as { __TAURI_INTERNALS__: MockInvoke }).__TAURI_INTERNALS__;
    const rid = (await inv.invoke('plugin:store|load', { path: store })) as number;
    return inv.invoke('plugin:store|get', { rid, key: 'probe' });
  }, PROBE_STORE);

  expect(survived).toEqual(['kept', true]);
});

test('stronghold records survive a page reload', async ({ page }) => {
  await page.goto('/');

  await page.evaluate(async (client) => {
    const inv = (window as unknown as { __TAURI_INTERNALS__: MockInvoke }).__TAURI_INTERNALS__;
    await inv.invoke('plugin:stronghold|save_store_record', {
      snapshotPath: '/mock/app-data-dir/e2e-probe.hold',
      client,
      key: 'probe',
      value: [1, 2, 3],
    });
    await inv.invoke('plugin:stronghold|save', {
      snapshotPath: '/mock/app-data-dir/e2e-probe.hold',
    });
  }, PROBE_CLIENT);

  await page.reload();

  const survived = await page.evaluate(async (client) => {
    const inv = (window as unknown as { __TAURI_INTERNALS__: MockInvoke }).__TAURI_INTERNALS__;
    return inv.invoke('plugin:stronghold|get_store_record', {
      snapshotPath: '/mock/app-data-dir/e2e-probe.hold',
      client,
      key: 'probe',
    });
  }, PROBE_CLIENT);

  expect(survived).toEqual([1, 2, 3]);
});
