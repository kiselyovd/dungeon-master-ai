import { expect, test } from '@playwright/test';
import { mockTauri } from './fixtures/tauri-mock';

test('DM failure is localized and retry preserves the original operation', async ({ page }) => {
  await mockTauri(page, {
    onboarding_completed: true,
    ui_language: 'ru',
    hero_class: 'fighter',
    pc: { name: 'Нора', heroClass: 'fighter' },
    active_campaign_id: 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa',
    active_session_id: 'bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb',
    active_provider: 'openai-compat',
    providers: {
      'openai-compat': {
        kind: 'openai-compat',
        baseUrl: 'http://127.0.0.1:9/v1',
        model: 'fixture-dm',
        apiKey: 'fixture-key',
      },
      'local-mistralrs': null,
    },
    image_enabled: false,
    video_enabled: false,
  });
  const requests: Record<string, unknown>[] = [];
  await page.route('**/agent/turn', async (route) => {
    requests.push(route.request().postDataJSON());
    await route.fulfill({
      status: 200,
      contentType: 'text/event-stream',
      body:
        requests.length === 1
          ? 'event: error\ndata: {"code":"agent_turn_failed"}\n\n'
          : 'event: text_delta\ndata: {"text":"Тихон благодарит вас."}\n\nevent: agent_done\ndata: {"total_rounds":1}\n\n',
    });
  });
  await page.goto('/');
  const input = page.getByRole('textbox', { name: 'Что ты делаешь?' });
  await input.fill('Осматриваю оберег.');
  await page.getByRole('button', { name: 'Отправить', exact: true }).click();
  await expect(page.locator('[data-error-code="agent_turn_failed"]')).toHaveText(
    'Ответ ведущего не завершён. Повторите запрос, чтобы продолжить.',
  );
  await input.fill('Осматриваю оберег.');
  await page.getByRole('button', { name: 'Отправить', exact: true }).click();
  await expect(page.getByText('Тихон благодарит вас.', { exact: true })).toBeVisible();
  expect(requests).toHaveLength(2);
  const [original, retry] = requests;
  if (!original || !retry) throw new Error('Two requests expected');
  expect(retry.request_id).toBe(original.request_id);
  expect(retry.player_message).toBe(original.player_message);
  expect(retry.resume).toBe(true);
  await expect(page.locator('[data-error-code]')).toHaveCount(0);
});
