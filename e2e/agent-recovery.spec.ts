import { expect, test } from '@playwright/test';
import { mockTauri } from './fixtures/tauri-mock';

test('retry keeps the turn identity and replaces incomplete narration in the production root', async ({
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
  await page.route('**/agent/turn', async (route) => {
    requests.push(route.request().postDataJSON());
    const body =
      requests.length === 1
        ? 'event: text_delta\ndata: {"text":"Incomplete reply"}\n\n'
        : 'event: text_delta\ndata: {"text":"Confirmed reply"}\n\nevent: agent_done\ndata: {"total_rounds":1}\n\n';
    await route.fulfill({ status: 200, contentType: 'text/event-stream', body });
  });
  await page.goto('/');
  const composer = page.getByRole('textbox', { name: 'What do you do?' });
  await composer.fill('I inspect the quay');
  await page.getByRole('button', { name: 'Send', exact: true }).click();
  await expect(page.locator('[data-error-code="network"]')).toBeVisible();
  await expect(page.getByText('Incomplete reply', { exact: true })).toBeVisible();
  await composer.fill('I inspect the quay');
  await page.getByRole('button', { name: 'Send', exact: true }).click();
  await expect(page.getByText('Confirmed reply', { exact: true })).toBeVisible();
  await expect(page.getByText('Incomplete reply', { exact: true })).toHaveCount(0);
  await expect(page.getByText('I inspect the quay', { exact: true })).toHaveCount(1);
  expect(requests).toHaveLength(2);
  expect(requests[0]?.request_id).toBeTruthy();
  expect({ ...requests[1], resume: false }).toEqual(requests[0]);
  expect(requests[1]?.resume).toBe(true);
  await page.screenshot({ path: '.artifacts/ai-durable/retry-confirmed.png', fullPage: true });
});
