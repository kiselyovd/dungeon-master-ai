import { expect, test } from '@playwright/test';
import { mockTauri } from './fixtures/tauri-mock';

test.setTimeout(60_000);
for (const lang of ['ru', 'en'] as const) {
  test(`model readiness ${lang}: actual status, advanced context and optional media`, async ({
    page,
  }) => {
    await page.setViewportSize({ width: 960, height: 640 });
    await mockTauri(page, { onboarding_completed: true, ui_language: lang, image_enabled: false });
    let unavailable = false;
    let starts = 0;
    let configuredContext = 8192;
    await page.route('**/local/runtime/status', async (route) => {
      if (unavailable) {
        await route.abort('failed');
        return;
      }
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          llm: { state: 'ready', port: 34567 },
          image: { state: 'off' },
          model_id: 'gemma4_e2b',
        }),
      });
    });
    await page.route('**/local-llm/manifest', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          system: [
            {
              id: 'gemma-4-e2b',
              display_name: 'Gemma 4 E2B',
              hf_repo: 'fixture/model',
              hf_filename: 'fixture.gguf',
              arch: 'gemma',
              quant: 'q4',
              size_gb: 3,
              license: 'apache-2.0',
            },
          ],
          user: [],
          installed_ids: ['gemma-4-e2b'],
          download_states: {},
        }),
      }),
    );
    await page.route('**/local/runtime/start', async (route) => {
      starts++;
      await route.fulfill({ status: 200, body: '{}' });
    });
    await page.route('**/local/runtime/stop', (route) =>
      route.fulfill({ status: 200, body: '{}' }),
    );
    await page.route('**/local-mode/config', async (route) => {
      if (route.request().method() === 'POST')
        configuredContext = route.request().postDataJSON().context_window;
      await route.fulfill({ status: 200, body: '{}' });
    });
    await page.goto('/');
    await page
      .getByRole('button', { name: lang === 'ru' ? 'Настройки' : 'Settings', exact: true })
      .click();
    const dialog = page.getByRole('dialog');
    await expect(dialog.getByText(/Hugging Face|GGUF/)).toHaveCount(0);
    await dialog
      .getByRole('tab', { name: lang === 'ru' ? 'Модель на компьютере' : 'Model on this computer' })
      .click();
    await expect(dialog.getByRole('radio', { name: /Gemma 4 E2B/ })).toBeChecked();
    await expect(
      dialog.getByText(lang === 'ru' ? 'Установлена на компьютере' : 'Installed on this computer', {
        exact: true,
      }),
    ).toBeVisible();
    const check = dialog.getByRole('button', {
      name: lang === 'ru' ? 'Проверить состояние' : 'Check status',
    });
    await expect(check).toBeVisible();
    await expect(
      dialog.getByLabel(lang === 'ru' ? 'Предел контекста (токены)' : 'Context limit (tokens)'),
    ).toHaveCount(0);
    await page.screenshot({
      path: `.artifacts/product-uiux/settings-readiness-${lang}-960.png`,
      animations: 'disabled',
    });
    unavailable = true;
    await check.click();
    await expect(
      dialog.getByText(
        lang === 'ru' ? 'Не удалось подтвердить состояние.' : 'Status could not be confirmed.',
        { exact: true },
      ),
    ).toBeVisible();
    unavailable = false;
    await check.click();
    await expect(
      dialog.getByText(
        lang === 'ru' ? 'Не удалось подтвердить состояние.' : 'Status could not be confirmed.',
        { exact: true },
      ),
    ).toHaveCount(0);
    await dialog
      .getByRole('button', { name: lang === 'ru' ? 'Дополнительно' : 'Advanced', exact: true })
      .click();
    await dialog
      .getByLabel(lang === 'ru' ? 'Предел контекста (токены)' : 'Context limit (tokens)')
      .fill('4096');
    await dialog
      .getByRole('button', {
        name: lang === 'ru' ? 'Запустить модель' : 'Start model',
        exact: true,
      })
      .click();
    await expect.poll(() => starts).toBe(1);
    expect(configuredContext).toBe(4096);
    const save = dialog.getByRole('button', {
      name: lang === 'ru' ? 'Сохранить' : 'Save',
      exact: true,
    });
    const box = await save.boundingBox();
    expect(box && box.y + box.height).toBeLessThanOrEqual(640);
  });
}
