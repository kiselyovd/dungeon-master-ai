import { createHash } from 'node:crypto';
import { readFile } from 'node:fs/promises';
import { expect, test } from '@playwright/test';
import { mockTauri } from './fixtures/tauri-mock';

test('production wizard persists an uploaded portrait reference and reloads the same face', async ({
  page,
}) => {
  const bytes = await readFile('src/assets/living-tabletop/v2/hero-fighter-01-v2.webp');
  const assetId = 'a2843520-2a22-4d82-b34d-33ae04389f41';
  const reference = {
    source: 'owned',
    asset_id: assetId,
    category: 'portrait',
    origin: 'uploaded',
    descriptor: {
      sha256: createHash('sha256').update(bytes).digest('hex'),
      mime_type: 'image/webp',
      byte_len: bytes.length,
      width: 512,
      height: 512,
    },
  };
  let draftId: string | null = null;
  let uploads = 0;
  let reads = 0;
  await mockTauri(page, {
    onboarding_completed: true,
    hero_class: 'fighter',
    active_provider: 'local-mistralrs',
    ui_language: 'en',
    image_enabled: false,
  });
  await page.route('**/srd/**', (route) =>
    route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: route.request().url().endsWith('/equipment')
        ? '{"weapons":[],"armor":[],"adventuring_gear":[]}'
        : '[]',
    }),
  );
  await page.route('**/media/assets', async (route) => {
    const body = route.request().postDataJSON();
    expect(body.category).toBe('portrait');
    expect(body.scope.kind).toBe('draft');
    expect(Buffer.from(body.data_b64, 'base64')).toEqual(bytes);
    draftId = body.scope.draft_id;
    uploads++;
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(reference),
    });
  });
  await page.route(`**/media/assets/${assetId}?*`, async (route) => {
    expect(new URL(route.request().url()).searchParams.get('draft_id')).toBe(draftId);
    reads++;
    await route.fulfill({ status: 200, contentType: 'image/webp', body: bytes });
  });
  async function openPortrait() {
    await page.getByRole('button', { name: 'Settings', exact: true }).click();
    await page.getByRole('button', { name: /Re-create character/i }).click();
    await page
      .locator('.dm-wizard')
      .getByRole('tab', { name: /Portrait$/ })
      .click();
  }
  await page.goto('/');
  await openPortrait();
  await page
    .getByLabel('Upload portrait', { exact: true })
    .setInputFiles({ name: 'portrait.webp', mimeType: 'image/webp', buffer: bytes });
  const image = page.getByAltText('Character portrait', { exact: true });
  await expect(image).toBeVisible();
  await expect(image).toHaveAttribute('src', /^blob:/);
  await expect
    .poll(() =>
      page.evaluate(() => {
        const raw = localStorage.getItem('dmai-e2e-store:settings.json');
        const prefs = raw ? JSON.parse(raw) : null;
        return prefs?.char_creation_draft?.portraitMediaRef?.asset_id ?? null;
      }),
    )
    .toBe(assetId);
  const stored = await page.evaluate(() => localStorage.getItem('dmai-e2e-store:settings.json'));
  expect(stored).not.toContain('data:image/');
  expect(stored).not.toContain('blob:');
  expect(uploads).toBe(1);
  await page.reload();
  await openPortrait();
  await expect(image).toBeVisible();
  await expect(image).toHaveAttribute('src', /^blob:/);
  expect(uploads).toBe(1);
  expect(reads).toBeGreaterThanOrEqual(2);
  expect(
    await image.evaluate(
      (node) => node instanceof HTMLImageElement && node.complete && node.naturalWidth === 512,
    ),
  ).toBe(true);
  await page.screenshot({ path: '.artifacts/product-uiux/custom-portrait-production.png' });
});
