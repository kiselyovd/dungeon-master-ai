import { expect, test } from '@playwright/test';
import { mockTauri } from './fixtures/tauri-mock';

const campaignId = 'bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb';
for (const language of ['ru', 'en'] as const) {
  test(`production ${language}: quiet footer, public outcome, diagnostics and visit metadata`, async ({
    page,
  }) => {
    await page.setViewportSize({ width: 960, height: 640 });
    await mockTauri(page, {
      onboarding_completed: true,
      hero_class: 'fighter',
      ui_language: language,
      active_campaign_id: campaignId,
      active_session_id: 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa',
      active_provider: 'openai-compat',
      image_enabled: false,
      video_enabled: false,
      providers: {
        'openai-compat': {
          kind: 'openai-compat',
          baseUrl: 'http://127.0.0.1:9/v1',
          model: 'fixture-technical-model-id',
          apiKey: 'fixture-key',
        },
        'local-mistralrs': null,
      },
    });
    const response = language === 'ru' ? 'Дверь закрыта.' : 'The door is closed.';
    await page.route('**/agent/turn', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'text/event-stream',
        body:
          'event: reasoning_text\ndata: {"text":"fixture hidden model reasoning"}\n\n' +
          'event: tool_call_start\ndata: {"id":"copy_roll","tool_name":"roll_dice","round":2}\n\n' +
          'event: tool_call_result\ndata: {"id":"copy_roll","tool_name":"roll_dice","args":{"dice":"1d20","internal_note":"fixture hidden args"},"result":{"total":18,"private_note":"fixture hidden result"},"is_error":false,"round":2}\n\n' +
          `event: text_delta\ndata: ${JSON.stringify({ text: response })}\n\n` +
          'event: agent_done\ndata: {"total_rounds":2}\n\n',
      }),
    );
    await page.goto('/');
    await expect(page.locator('.dm-splash')).toHaveCount(0);
    await expect
      .poll(() =>
        page.evaluate((id) => {
          const prefs = JSON.parse(localStorage.getItem('dmai-e2e-store:settings.json') ?? '{}');
          return Boolean(prefs.campaign_visits?.[id]);
        }, campaignId),
      )
      .toBe(true);
    await expect(page.locator('footer')).not.toContainText('fixture-technical-model-id');
    await page
      .getByPlaceholder(language === 'ru' ? 'Что ты делаешь?' : 'What do you do?')
      .fill('Inspect the door.');
    await page
      .getByRole('button', { name: language === 'ru' ? 'Отправить' : 'Send', exact: true })
      .click();
    const card = page.getByTestId('tool-call-card-copy_roll');
    await expect(card).toContainText(language === 'ru' ? 'Бросок: 18' : 'Roll: 18');
    await expect(page.getByText(response, { exact: true })).toBeVisible();
    await expect(page.locator('body')).not.toContainText('fixture hidden');
    await expect(card).not.toContainText('roll_dice');
    await expect(card).not.toContainText('args');
    await expect(card).not.toContainText(language === 'ru' ? 'Раунд 2' : 'Round 2');
    await card
      .getByRole('button', {
        name: language === 'ru' ? 'Технические детали действия' : 'Action technical details',
      })
      .click();
    const closeInspector = page.getByRole('button', {
      name: language === 'ru' ? 'Закрыть инспектор' : /close inspector/i,
    });
    await expect(closeInspector).toBeVisible();
    await closeInspector.click();
    await page
      .getByRole('button', { name: language === 'ru' ? 'Кампании' : 'Campaigns', exact: true })
      .click();
    const main = page.locator('main[data-art-direction="living-tabletop"]');
    await expect(main).not.toContainText('srd-5.2.1');
    await expect(main).not.toContainText(language === 'ru' ? 'Глава' : 'Chapter');
    await expect(
      main.getByText(language === 'ru' ? /^Сохранено:/ : /^Saved:/).first(),
    ).toBeVisible();
    await expect(
      main.getByText(language === 'ru' ? /^Открывали:/ : /^Opened:/).first(),
    ).toBeVisible();
    await page.reload();
    await expect(page.locator('.dm-splash')).toHaveCount(0);
    if (!(await main.isVisible()))
      await page
        .getByRole('button', { name: language === 'ru' ? 'Кампании' : 'Campaigns', exact: true })
        .click();
    await expect(
      main.getByText(language === 'ru' ? /^Открывали:/ : /^Opened:/).first(),
    ).toBeVisible();
    await page.screenshot({
      path: `.artifacts/product-uiux/copy-${language}-960.png`,
      animations: 'disabled',
    });
  });
}
