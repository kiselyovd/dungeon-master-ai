import { expect, test } from '@playwright/test';
import { mockTauri } from './fixtures/tauri-mock';

test('production chat reload recovers the server question and resumes the original turn', async ({
  page,
}) => {
  await mockTauri(page, {
    onboarding_completed: true,
    ui_language: 'en',
    hero_class: 'fighter',
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
  const id = 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa';
  const requests: Array<Record<string, unknown>> = [];
  await page.route('**/agent/pending?*', (route) => {
    const original = requests[0];
    const body = original
      ? {
          operation: {
            request_id: original.request_id,
            model: original.model ?? null,
            request: {
              campaign_id: original.campaign_id,
              session_id: original.session_id,
              player_message: original.player_message,
              history: original.history,
              images: original.images ?? [],
              board: original.board ?? null,
            },
          },
          clarification: { id, command_id: 1, question: 'Which quay do you mean?', reply: null },
        }
      : null;
    return route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(body),
    });
  });
  await page.route('**/sessions/*/messages', (route) =>
    route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        messages: requests.length
          ? [
              { role: 'user', parts: [{ type: 'text', text: 'Go to the quay' }] },
              { role: 'assistant', content: 'Which quay do you mean?' },
            ]
          : [],
      }),
    }),
  );
  await page.route('**/agent/turn', async (route) => {
    requests.push(route.request().postDataJSON());
    const body =
      requests.length === 1
        ? `event: text_delta\ndata: {"text":"Which quay do you mean?"}\n\nevent: agent_needs_input\ndata: {"clarification_id":"${id}","command_id":1,"total_rounds":1}\n\n`
        : 'event: text_delta\ndata: {"text":"You arrive at the old quay."}\n\nevent: agent_done\ndata: {"total_rounds":1}\n\n';
    await route.fulfill({ status: 200, contentType: 'text/event-stream', body });
  });
  await page.goto('/');
  const composer = page.getByRole('textbox', { name: 'What do you do?' });
  await composer.fill('Go to the quay');
  await page.getByRole('button', { name: 'Send', exact: true }).click();
  await expect(page.getByText('Which quay do you mean?', { exact: true })).toBeVisible();
  await page.reload();
  await expect(page.getByText('Which quay do you mean?', { exact: true })).toBeVisible();
  await composer.fill('The old one');
  await page.getByRole('button', { name: 'Send', exact: true }).click();
  await expect(page.getByText('You arrive at the old quay.', { exact: true })).toBeVisible();
  expect(requests).toHaveLength(2);
  expect(requests[1]?.request_id).toBe(requests[0]?.request_id);
  expect(requests[1]?.player_message).toBe('Go to the quay');
  expect(requests[1]?.history).toEqual(requests[0]?.history);
  expect(requests[1]?.clarification).toEqual({ id, reply: 'The old one' });
  expect(requests[1]?.resume).toBe(true);
  await page.screenshot({
    path: '.artifacts/ai-durable/clarification-reloaded.png',
    fullPage: true,
  });
});

test('production chat keeps a clarification and sends the answer with its original turn', async ({
  page,
}) => {
  await mockTauri(page, {
    onboarding_completed: true,
    ui_language: 'en',
    hero_class: 'fighter',
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
  await page.route('**/sessions/*/messages', (route) =>
    route.fulfill({ status: 200, contentType: 'application/json', body: '{"messages":[]}' }),
  );
  const requests: Array<Record<string, unknown>> = [];
  const clarificationId = 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa';
  await page.route('**/agent/turn', async (route) => {
    requests.push(route.request().postDataJSON());
    const body =
      requests.length === 1
        ? `event: text_delta\ndata: {"text":"Which quay do you mean?"}\n\nevent: agent_needs_input\ndata: {"clarification_id":"${clarificationId}","command_id":1,"total_rounds":1}\n\n`
        : 'event: text_delta\ndata: {"text":"You arrive at the old quay."}\n\nevent: agent_done\ndata: {"total_rounds":1}\n\n';
    await route.fulfill({ status: 200, contentType: 'text/event-stream', body });
  });
  await page.goto('/');
  const composer = page.getByRole('textbox', { name: 'What do you do?' });
  await composer.fill('Go to the quay');
  await page.getByRole('button', { name: 'Send', exact: true }).click();
  await expect(page.getByText('Which quay do you mean?', { exact: true })).toBeVisible();
  await expect(page.locator('[data-error-code="network"]')).toHaveCount(0);
  await composer.fill('The old one');
  await page.getByRole('button', { name: 'Send', exact: true }).click();
  await expect(page.getByText('You arrive at the old quay.', { exact: true })).toBeVisible();
  await expect(page.getByText('Which quay do you mean?', { exact: true })).toBeVisible();
  await expect(page.getByText('The old one', { exact: true })).toBeVisible();
  expect(requests).toHaveLength(2);
  expect(requests[1]?.request_id).toBe(requests[0]?.request_id);
  expect(requests[1]?.player_message).toBe('Go to the quay');
  expect(requests[1]?.history).toEqual(requests[0]?.history);
  expect(requests[1]?.clarification).toEqual({ id: clarificationId, reply: 'The old one' });
  expect(requests[1]?.resume).toBe(true);
  await page.screenshot({
    path: '.artifacts/ai-durable/clarification-confirmed.png',
    fullPage: true,
  });
});
