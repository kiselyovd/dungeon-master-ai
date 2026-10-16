import { expect, type Page, test } from '@playwright/test';
import { mockTauri } from './fixtures/tauri-mock';

const campaignId = 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa';
const sessionId = 'bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb';
const actorId = '92414394-f4b1-477f-9278-10f75c3c4fb7';
const requestId = 'cccccccc-cccc-4ccc-8ccc-cccccccccccc';
const original = 'Иду к причалу.';
async function seed(page: Page) {
  await mockTauri(page, {
    onboarding_completed: true,
    ui_language: 'ru',
    hero_class: 'fighter',
    active_campaign_id: campaignId,
    active_session_id: sessionId,
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
}
function progress() {
  return {
    schema_version: 1,
    scope: {
      request_id: requestId,
      campaign_id: campaignId,
      session_id: sessionId,
      actor_id: actorId,
      authority: { kind: 'solo', restore_generation: 0 },
    },
    model: 'fixture-dm',
    phase: 'narration_pending',
    allowed_action: 'resume',
    progress_known: true,
    commands: [{ command_id: 1, state: 'finished' }],
    confirmed_effects: [
      {
        command_id: 1,
        step: 1,
        action: 'adventure',
        outcome: { kind: 'performed', check_success: null },
      },
    ],
    clarification: null,
  };
}
function input() {
  return {
    operation: {
      request_id: requestId,
      model: 'fixture-dm',
      request: {
        campaign_id: campaignId,
        session_id: sessionId,
        player_message: original,
        history: [],
        images: [],
        board: null,
      },
    },
    clarification: null,
  };
}
test('production root restores a request before sending and retains a new draft on continuation', async ({
  page,
}) => {
  await seed(page);
  let pending = true;
  await page.route('**/agent/recovery?*', (route) =>
    route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(pending ? progress() : null),
    }),
  );
  await page.route('**/agent/recovery-input?*', (route) =>
    route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(pending ? input() : null),
    }),
  );
  const calls: Record<string, unknown>[] = [];
  await page.route('**/agent/turn', async (route) => {
    calls.push(route.request().postDataJSON());
    pending = false;
    await route.fulfill({
      status: 200,
      contentType: 'text/event-stream',
      body: 'event: text_delta\ndata: {"text":"Вы у причала."}\n\nevent: agent_done\ndata: {"total_rounds":1}\n\n',
    });
  });
  await page.goto('/');
  await expect(page.getByText('Завершено шагов: 1 из 1')).toBeVisible();
  const draft = page.getByRole('textbox', { name: 'Что ты делаешь?' });
  await draft.fill('Новый черновик');
  await expect(page.getByRole('button', { name: 'Отправить', exact: true })).toBeDisabled();
  expect(calls).toHaveLength(0);
  await page.screenshot({ path: '.artifacts/product-uiux/recovery-pending-root.png' });
  await page.getByRole('button', { name: 'Продолжить ответ', exact: true }).click();
  await expect(page.getByText('Вы у причала.', { exact: true })).toBeVisible();
  await expect(page.getByTestId('agent-recovery-panel')).toHaveCount(0);
  await expect(draft).toHaveValue('Новый черновик');
  expect(calls).toHaveLength(1);
  expect(calls[0]).toMatchObject({
    request_id: requestId,
    player_message: original,
    model: 'fixture-dm',
    resume: true,
  });
});
test('an unreadable pending state blocks new turns until an explicit successful read', async ({
  page,
}) => {
  await seed(page);
  let unavailable = true;
  await page.route('**/agent/recovery?*', (route) =>
    route.fulfill({
      status: unavailable ? 503 : 200,
      contentType: 'application/json',
      body: unavailable ? '{"error":{"code":"agent_recovery_read_failed"}}' : 'null',
    }),
  );
  await page.goto('/');
  await expect(page.getByRole('button', { name: 'Проверить ещё раз' })).toBeVisible();
  const draft = page.getByRole('textbox', { name: 'Что ты делаешь?' });
  await draft.fill('Черновик не пропадает');
  await expect(page.getByRole('button', { name: 'Отправить', exact: true })).toBeDisabled();
  unavailable = false;
  await page.getByRole('button', { name: 'Проверить ещё раз' }).click();
  await expect(page.getByTestId('agent-recovery-panel')).toHaveCount(0);
  await expect(draft).toHaveValue('Черновик не пропадает');
  await expect(page.getByRole('button', { name: 'Отправить', exact: true })).toBeEnabled();
});
