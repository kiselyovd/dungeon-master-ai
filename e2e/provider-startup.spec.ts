import { readFileSync } from 'node:fs';
import { expect, test } from '@playwright/test';
import { mockTauri } from './fixtures/tauri-mock';

const wire = JSON.parse(
  readFileSync(new URL('../src/api/__fixtures__/soloRecovery.json', import.meta.url), 'utf8'),
);
test('production startup restores persisted settings before the first DM request', async ({
  page,
}) => {
  const state = structuredClone(wire.afterRest.state);
  state.projection.scene = {
    id: '824e4fa6-b156-444c-984b-f891fe7f578d',
    title: 'Quiet crossing',
    mode: 'exploration',
  };
  await mockTauri(page, {
    onboarding_completed: true,
    ui_language: 'ru',
    hero_class: 'fighter',
    active_campaign_id: state.campaign.id,
    active_session_id: state.session_id,
    active_provider: 'openai-compat',
    providers: {
      'openai-compat': {
        kind: 'openai-compat',
        baseUrl: 'http://127.0.0.1:31416/v1',
        model: 'fixture-dm',
        apiKey: 'fixture-key',
      },
      'local-mistralrs': null,
    },
    image_enabled: false,
    video_enabled: false,
    pc: state.projection.character,
  });
  let release!: () => void;
  const blocked = new Promise<void>((r) => {
    release = r;
  });
  let configured = false;
  const requests: string[] = [];
  await page.route('**/settings/v2', async (route) => {
    const body = route.request().postDataJSON();
    expect(body.chat.active_provider_id).toBe('openai-compat');
    expect(body.chat.active_model_id).toBe('fixture-dm');
    requests.push('configure');
    await blocked;
    configured = true;
    await route.fulfill({ status: 200, contentType: 'application/json', body: '{}' });
  });
  await page.route(/\/campaigns(?:\/[^/?]+(?:\/state)?)?(?:\?|$)/, (route) => {
    const path = new URL(route.request().url()).pathname;
    return route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(
        path.endsWith('/state') ? state : path === '/campaigns' ? [state.campaign] : state.campaign,
      ),
    });
  });
  await page.route('**/agent/turn', async (route) => {
    expect(configured).toBe(true);
    requests.push('turn');
    await route.fulfill({
      status: 200,
      contentType: 'text/event-stream',
      body: 'event: text_delta\ndata: {"text":"Река спокойна. Что предпримешь?"}\n\nevent: agent_done\ndata: {"total_rounds":1}\n\n',
    });
  });
  await page.goto('/');
  await expect(page.locator('.dm-char-fab')).toContainText('Mira');
  await expect.poll(() => requests.length).toBe(1);
  await page.locator('textarea').fill('Осматриваю переправу.');
  await page.getByRole('button', { name: 'Отправить', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Стоп', exact: true })).toBeVisible();
  expect(requests).toEqual(['configure']);
  release();
  await expect(page.getByText('Река спокойна. Что предпримешь?', { exact: true })).toBeVisible();
  expect(requests).toEqual(['configure', 'turn']);
  await page.screenshot({
    path: '.artifacts/provider-startup-production.png',
    fullPage: true,
    animations: 'disabled',
  });
});
