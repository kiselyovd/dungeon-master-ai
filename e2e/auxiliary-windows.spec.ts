import { expect, test } from '@playwright/test';

const campaign = 'bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb';
const actor = 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa';
for (const kind of ['hero', 'journal'] as const)
  test(`production ${kind} root renders scoped read-only data and clears revoked access`, async ({
    page,
  }) => {
    const errors: string[] = [];
    page.on('pageerror', (error) => errors.push(error.message));
    await page.addInitScript(
      ({ kind, campaign, actor }) => {
        const runtime = window as unknown as {
          isTauri: boolean;
          auxFixture: unknown;
          auxCommands: string[];
          __TAURI_INTERNALS__: unknown;
        };
        runtime.isTauri = true;
        runtime.auxCommands = [];
        runtime.auxFixture = {
          schemaVersion: 1,
          sequence: 1,
          scopeId: 'room:guest',
          campaignId: campaign,
          actorId: actor,
          revision: 3,
          language: 'ru',
          ready: true,
          hero:
            kind === 'hero'
              ? {
                  characterId: actor,
                  name: 'Мира',
                  heroClass: 'fighter',
                  hp: 9,
                  hpMax: 12,
                  portraitUrl: null,
                  backstory: 'Own biography',
                  inventory: [{ id: 'rope', name: 'Верёвка', count: 2 }],
                }
              : null,
          journal:
            kind === 'journal'
              ? [
                  {
                    id: 'entry',
                    campaign_id: campaign,
                    chapter: 'Открытое знание',
                    entry_html: '<p>Общий след</p>',
                    created_at: '2026-10-14T00:00:00Z',
                  },
                ]
              : [],
        };
        runtime.__TAURI_INTERNALS__ = {
          metadata: { currentWindow: { label: kind }, currentWebview: { label: kind } },
          invoke: async (command: string) => {
            runtime.auxCommands.push(command);
            if (command === 'desktop_aux_read') return runtime.auxFixture;
            if (command === 'desktop_aux_focus_main' || command === 'desktop_aux_close')
              return null;
            throw Error(`unexpected_aux_command:${command}`);
          },
        };
      },
      { kind, campaign, actor },
    );
    await page.setViewportSize({ width: 960, height: 720 });
    await page.goto('/auxiliary.html');
    await expect(page.getByText('Dungeon Master AI', { exact: true })).toBeVisible();
    await expect(page.getByText('Только просмотр', { exact: true })).toBeVisible();
    await expect(
      page.getByText(kind === 'hero' ? 'Мира' : 'Общий след', { exact: true }),
    ).toBeVisible();
    await expect(page.getByRole('button', { name: /Отдых|Создать персонажа/ })).toHaveCount(0);
    await page.screenshot({
      path: `.artifacts/product-uiux/task-42-${kind}-browser.png`,
      animations: 'disabled',
    });
    if (kind === 'hero') {
      await page.mouse.move(700, 550);
      await page.mouse.wheel(0, 1400);
      await expect(page.getByText('Верёвка', { exact: true })).toBeVisible();
      await expect(page.getByRole('button', { name: 'Вернуться к игре' })).toBeVisible();
    }
    await page.getByRole('button', { name: 'Вернуться к игре' }).click();
    await page.evaluate(() => {
      (window as unknown as { auxFixture: unknown }).auxFixture = null;
    });
    await expect(
      page.getByText(kind === 'hero' ? 'Мира' : 'Общий след', { exact: true }),
    ).toHaveCount(0);
    await expect(page.getByRole('status')).toBeVisible();
    const commands = await page.evaluate(
      () => (window as unknown as { auxCommands: string[] }).auxCommands,
    );
    expect(
      commands.every((command) => ['desktop_aux_read', 'desktop_aux_focus_main'].includes(command)),
    ).toBe(true);
    expect(errors).toEqual([]);
  });
