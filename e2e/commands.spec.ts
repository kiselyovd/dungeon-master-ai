import { expect, test } from '@playwright/test';
import { mockTauri } from './fixtures/tauri-mock';

test('production command palette preserves typing, focus, nested dialogs and layout', async ({
  page,
}) => {
  await mockTauri(page, { onboarding_completed: true, hero_class: 'fighter', ui_language: 'en' });
  await page.goto('/');
  await expect(page.locator('.dm-splash')).toHaveCount(0);
  const draft = page.getByPlaceholder('What do you do?');
  await draft.fill('Unsent command draft');
  await draft.press('Shift+Enter');
  await expect(draft).toHaveValue('Unsent command draft\n');
  await draft.press('Control+Shift+J');
  await expect(page.getByRole('region', { name: "Bard's Journal" })).toHaveCount(0);
  await draft.evaluate((element) =>
    element.dispatchEvent(
      new KeyboardEvent('keydown', { key: 'k', ctrlKey: true, isComposing: true, bubbles: true }),
    ),
  );
  await expect(page.getByRole('dialog', { name: 'Commands' })).toHaveCount(0);
  await draft.press('Control+k');
  const palette = page.getByRole('dialog', { name: 'Commands' });
  await expect(palette).toBeVisible();
  await expect(palette.getByRole('button', { name: 'End your turn' })).toBeDisabled();
  await expect(palette.getByRole('button', { name: 'Tool diagnostics' })).toHaveCount(0);
  await palette.getByRole('checkbox', { name: 'Advanced commands' }).check();
  await expect(palette.getByRole('button', { name: /Tool diagnostics/ })).toBeVisible();
  const search = palette.getByRole('textbox', { name: 'Find a command' });
  await search.fill('journal');
  await search.press('Enter');
  await expect(palette).toHaveCount(0);
  await expect(page.getByRole('region', { name: "Bard's Journal" })).toBeVisible();
  await expect(draft).toHaveValue('Unsent command draft\n');
  await page.getByRole('button', { name: 'Commands', exact: true }).click();
  await search.fill('focus chat');
  await search.press('Enter');
  await expect(draft).toBeFocused();
  await page.getByRole('button', { name: 'Commands', exact: true }).click();
  await search.fill('settings');
  await search.press('Enter');
  await expect(page.getByRole('dialog', { name: 'Settings', exact: true })).toBeVisible();
  await page.keyboard.press('Control+k');
  await expect(palette).toBeVisible();
  await page.keyboard.press('Escape');
  await expect(palette).toHaveCount(0);
  await expect(page.getByRole('dialog', { name: 'Settings', exact: true })).toBeVisible();
  await page.keyboard.press('Escape');
  await expect(page.getByRole('dialog')).toHaveCount(0);
  await page.getByRole('button', { name: 'Commands', exact: true }).click();
  await search.fill('reset');
  await search.press('Enter');
  await expect(page.getByRole('region', { name: "Bard's Journal" })).toHaveCount(0);
  await expect(draft).toHaveValue('Unsent command draft\n');
  await page.getByRole('button', { name: 'Commands', exact: true }).click();
  await search.fill('focus map');
  await search.press('Enter');
  await expect(page.getByTestId('dm-vtt')).toBeFocused();
  await page.reload();
  await expect(draft).toHaveValue('Unsent command draft\n');
  await page.getByRole('button', { name: 'Commands', exact: true }).click();
  await page.screenshot({
    path: '.artifacts/product-uiux/task-35-palette.png',
    animations: 'disabled',
  });
});

test('Ctrl+S and F11 use the shared registry once while editing, and search is localized', async ({
  page,
}) => {
  await mockTauri(page, { onboarding_completed: true, hero_class: 'fighter', ui_language: 'ru' });
  await page.addInitScript(() => {
    Object.defineProperty(window, 'isTauri', { value: true });
    const native = (
      window as unknown as {
        __TAURI_INTERNALS__: { invoke(command: string, args?: unknown): Promise<unknown> };
      }
    ).__TAURI_INTERNALS__;
    Object.assign(native, {
      metadata: { currentWindow: { label: 'main' }, currentWebview: { label: 'main' } },
    });
    const invoke = native.invoke.bind(native);
    (window as unknown as { commandWindowCalls: string[] }).commandWindowCalls = [];
    native.invoke = async (command, args) => {
      if (command.startsWith('plugin:window|')) {
        (window as unknown as { commandWindowCalls: string[] }).commandWindowCalls.push(command);
        if (command.endsWith('is_fullscreen')) return false;
      }
      return invoke(command, args);
    };
  });
  let saves = 0;
  await page.route('**/sessions/*/saves/quick', async (route) => {
    saves++;
    await new Promise((resolve) => setTimeout(resolve, 100));
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ id: crypto.randomUUID() }),
    });
  });
  await page.route('**/sessions/*/saves', (route) =>
    route.fulfill({ status: 200, contentType: 'application/json', body: '[]' }),
  );
  await page.goto('/');
  await expect(page.locator('.dm-splash')).toHaveCount(0);
  const draft = page.locator('[data-command-target="chat"]');
  await draft.fill('Черновик');
  await draft.press('Control+s');
  await draft.evaluate((element) =>
    element.dispatchEvent(
      new KeyboardEvent('keydown', { key: 's', ctrlKey: true, repeat: true, bubbles: true }),
    ),
  );
  await expect.poll(() => saves).toBe(1);
  await expect(draft).toHaveValue('Черновик');
  await draft.press('F11');
  await expect
    .poll(() =>
      page.evaluate(
        () =>
          (window as unknown as { commandWindowCalls: string[] }).commandWindowCalls.filter(
            (call) => call.endsWith('set_fullscreen'),
          ).length,
      ),
    )
    .toBe(1);
  await draft.press('Control+k');
  await page.getByRole('textbox', { name: 'Найти команду' }).fill('журнал');
  await expect(page.getByRole('button', { name: /Открыть журнал/ })).toBeVisible();
});
