import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { expect, test } from '@playwright/test';
import { rasterHeader } from '../scripts/lib/raster-header';
import { mockTauri } from './fixtures/tauri-mock';

test('production main derives the owned portrait for its detached view without persisting image bytes', async ({
  page,
}) => {
  const manifest = JSON.parse(readFileSync('docs/visual/asset-manifest.json', 'utf8')) as {
    assets: { id: string; path: string }[];
  };
  const portrait = manifest.assets.find((asset) => asset.id === 'hero.fighter.01.v2');
  if (!portrait) throw Error('portrait_fixture_missing');
  const bytes = readFileSync(portrait.path);
  const dimensions = rasterHeader(bytes);
  const asset = 'cccccccc-cccc-4ccc-8ccc-cccccccccccc';
  const reference = {
    source: 'owned',
    asset_id: asset,
    category: 'portrait',
    origin: 'uploaded',
    descriptor: {
      sha256: createHash('sha256').update(bytes).digest('hex'),
      mime_type: 'image/webp',
      byte_len: bytes.length,
      width: dimensions.width,
      height: dimensions.height,
    },
  };
  await mockTauri(page, {
    onboarding_completed: true,
    hero_class: 'fighter',
    ui_language: 'en',
    pc: { name: 'Mira', portraitMediaRef: reference, portraitAssetId: null },
  });
  await page.route(`**/media/assets/${asset}?*`, (route) =>
    route.fulfill({
      status: 200,
      headers: { 'content-type': 'image/webp', 'content-length': String(bytes.length) },
      body: bytes,
    }),
  );
  await page.route('**/sessions/**', (route) =>
    route.fulfill({ status: 200, contentType: 'application/json', body: '{"messages":[]}' }),
  );
  await page.addInitScript(() => {
    const runtime = window as unknown as {
      isTauri: boolean;
      portraitFrames: Record<string, unknown>[];
      __TAURI_INTERNALS__: {
        invoke(command: string, args?: Record<string, unknown>): Promise<unknown>;
      };
    };
    runtime.isTauri = true;
    runtime.portraitFrames = [];
    const invoke = runtime.__TAURI_INTERNALS__.invoke.bind(runtime.__TAURI_INTERNALS__);
    runtime.__TAURI_INTERNALS__.invoke = async (command, args) => {
      if (command === 'desktop_aux_publish') {
        runtime.portraitFrames.push(args?.frame as Record<string, unknown>);
        return null;
      }
      if (command === 'desktop_aux_heartbeat') return null;
      return invoke(command, args);
    };
  });
  await page.goto('/');
  await expect(
    page.getByRole('button', { name: 'Open character sheet for Mira', exact: true }),
  ).toBeVisible();
  await expect
    .poll(() =>
      page.evaluate(() =>
        Boolean(
          (
            window as unknown as { portraitFrames: { portrait?: { dataUrl: string } }[] }
          ).portraitFrames.some((frame) => frame.portrait?.dataUrl.startsWith('data:image/')),
        ),
      ),
    )
    .toBe(true);
  const proof = await page.evaluate(async () => {
    const frames = (
      window as unknown as {
        portraitFrames: {
          hero: { characterId: string; portraitMediaRef: { asset_id: string } };
          portrait?: { assetId: string; dataUrl: string };
        }[];
      }
    ).portraitFrames;
    const frame = frames.findLast((frame) => frame.portrait);
    if (!frame?.portrait) throw Error('preview_missing');
    const image = new Image();
    const dataUrl = frame.portrait.dataUrl;
    image.src = dataUrl;
    await image.decode();
    return {
      assetId: frame.portrait.assetId,
      sourceAssetId: frame.hero.portraitMediaRef.asset_id,
      width: image.naturalWidth,
      height: image.naturalHeight,
      persisted: Object.values(localStorage).some((value) => value.includes(dataUrl)),
    };
  });
  expect(proof.assetId).toBe(asset);
  expect(proof.sourceAssetId).toBe(asset);
  expect(proof.width).toBeLessThanOrEqual(256);
  expect(proof.height).toBeLessThanOrEqual(256);
  expect(Math.max(proof.width, proof.height)).toBeLessThan(
    Math.max(dimensions.width, dimensions.height),
  );
  expect(proof.persisted).toBe(false);
});
