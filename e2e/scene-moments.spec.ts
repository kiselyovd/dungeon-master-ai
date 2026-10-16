import { randomUUID } from 'node:crypto';
import { expect, test } from '@playwright/test';
import { mockTauri } from './fixtures/tauri-mock';

const backend = process.env.DMAI_ADVENTURE_BACKEND_URL;
test.skip(!backend, 'Requires isolated real backend and canonical actions.');

test('canonical major accent keeps actions/composer available and consumes a background stream without replay', async ({
  page,
  request,
}) => {
  const base = backend as string;
  const created = await request.post(`${base}/campaigns`, {
    data: {
      schema_version: 1,
      ruleset_id: 'srd-5.2.1',
      title: 'Scene moment acceptance',
      premise: 'Isolated UI verification',
      seed: { kind: 'bundled', adventure_id: 'bell-beneath-water' },
    },
  });
  expect(created.ok()).toBe(true);
  const campaign = await created.json(),
    characterId = randomUUID();
  const committed = await request.put(`${base}/campaigns/${campaign.id}/character`, {
    data: {
      schema_version: 1,
      ruleset_id: 'srd-5.2.1',
      session_id: campaign.last_session_id,
      character_id: characterId,
      expected_revision: 1,
      causation_id: randomUUID(),
      character: {
        name: 'Moment hero',
        heroClass: 'fighter',
        level: 1,
        hp: 12,
        hpMax: 12,
        ac: 18,
        abilities: { str: 15, dex: 14, con: 13, int: 12, wis: 10, cha: 8 },
        inventory: [],
      },
    },
  });
  expect(committed.ok()).toBe(true);
  await mockTauri(page, {
    onboarding_completed: true,
    hero_class: 'fighter',
    ui_language: 'en',
    image_enabled: false,
    video_enabled: false,
    scene_transitions_enabled: true,
    active_campaign_id: campaign.id,
    active_session_id: campaign.last_session_id,
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
  });
  await page.route(
    /^https?:\/\/[^/]+\/(?:campaigns|sessions|saves|combat)(?:\/|$)/,
    async (route) => {
      const url = new URL(route.request().url());
      const reply = await route.fetch({ url: `${base}${url.pathname}${url.search}` });
      await route.fulfill({ response: reply });
    },
  );
  await page.goto('/');
  await expect(page.locator('.dm-splash')).toHaveCount(0);
  const trigger = page
    .getByRole('button', { name: 'Adventure', exact: true })
    .and(page.locator('[aria-expanded]'));
  await trigger.click();
  await page.getByRole('tab', { name: 'Scene', exact: true }).click();
  for (const label of [
    'Go to Old quay',
    'Go to Flooded archive',
    'Call for Tikhon in the upper chamber',
  ]) {
    const response = page.waitForResponse(
      (reply) => reply.url().endsWith('/adventure/actions') && reply.request().method() === 'POST',
    );
    await page.getByRole('button', { name: label, exact: true }).click();
    expect((await response).status()).toBe(200);
    await expect(page.locator('.dm-scene-transition')).toHaveCount(0);
    await expect(page.locator('.dm-video-disabled-toast')).toHaveCount(0);
  }
  const response = page.waitForResponse(
    (reply) => reply.url().endsWith('/adventure/actions') && reply.request().method() === 'POST',
  );
  await page.getByRole('button', { name: 'Hold the passage for Tikhon', exact: true }).click();
  expect((await response).status()).toBe(200);
  await page.keyboard.press('Escape');
  const moment = page.locator('.dm-scene-transition');
  await expect(moment).toHaveAttribute('role', 'status');
  await expect(moment).toHaveCSS('position', 'absolute');
  await expect(moment).toHaveCSS('pointer-events', 'none');
  await expect(moment.locator('img')).toHaveAttribute('data-asset-id', 'scene.bell.archive.v2');
  await expect(page.getByRole('dialog')).toHaveCount(0);
  const composer = page.getByPlaceholder('What do you do?');
  await composer.fill('Unsent next action');
  await expect(composer).toBeFocused();
  await page.screenshot({
    path: '.artifacts/product-uiux/task-33-major-moment.png',
    animations: 'disabled',
  });

  let received = false,
    release: (() => void) | undefined;
  const held = new Promise<void>((resolve) => {
    release = resolve;
  });
  await page.route('**/combat/action', async (route) => {
    const reply = await route.fetch({ url: `${base}/combat/action` });
    expect(reply.ok()).toBe(true);
    received = true;
    await held;
    await route.fulfill({ response: reply });
  });
  await page.getByTestId('action-btn-end_turn').click();
  await expect.poll(() => received).toBe(true);
  // Headless Chromium keeps tabs focused; inject only the focus sensor.
  // The operation and held response still come from the actual backend.
  await page.evaluate(() => {
    Object.defineProperty(document, 'hasFocus', { configurable: true, value: () => false });
    window.dispatchEvent(new Event('blur'));
  });
  await expect.poll(() => page.evaluate(() => document.hasFocus())).toBe(false);
  await expect(page.locator('.dm-app')).toHaveAttribute('data-motion-paused', 'true');
  release?.();
  await expect(page.getByTestId('combat-overlay')).toHaveAttribute('data-revision', '1');
  await expect(moment).toHaveCount(0);
  await page.evaluate(() => {
    Reflect.deleteProperty(document, 'hasFocus');
    window.dispatchEvent(new Event('focus'));
  });
  await expect(moment).toHaveCount(0);
  await expect(composer).toHaveValue('Unsent next action');
  const state = await (await request.get(`${base}/campaigns/${campaign.id}/state`)).json();
  expect(state.projection.combat.revision).toBe(1);
});
