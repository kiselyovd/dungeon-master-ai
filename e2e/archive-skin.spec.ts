import { randomUUID } from 'node:crypto';
import { expect, test } from '@playwright/test';
import { mockTauri } from './fixtures/tauri-mock';

const backend = process.env.DMAI_ADVENTURE_BACKEND_URL;
test.skip(!backend, 'Requires isolated actual backend for geometry and movement evidence.');
for (const dpr of [1, 1.5, 2])
  test.describe(`Archive skin DPR${dpr}`, () => {
    test.use({ deviceScaleFactor: dpr, viewport: { width: 1440, height: 960 } });
    test('exact wall/passage/exit and sprites stay aligned through zoom/fit/layers; movement remains authoritative', async ({
      page,
      request,
    }) => {
      const base = backend as string;
      const created = await request.post(`${base}/campaigns`, {
        data: {
          schema_version: 1,
          ruleset_id: 'srd-5.2.1',
          title: 'Archive calibration',
          premise: 'Geometry acceptance',
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
            name: 'Calibration hero',
            heroClass: 'fighter',
            level: 1,
            hp: 12,
            hpMax: 12,
            ac: 14,
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
        active_campaign_id: campaign.id,
        active_session_id: campaign.last_session_id,
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
        'Hold the passage for Tikhon',
      ]) {
        const response = page.waitForResponse(
          (reply) =>
            reply.url().endsWith('/adventure/actions') && reply.request().method() === 'POST',
        );
        await page.getByRole('button', { name: label, exact: true }).click();
        expect((await response).status()).toBe(200);
      }
      await page.keyboard.press('Escape');
      const readState = async () =>
        (await request.get(`${base}/campaigns/${campaign.id}/state`)).json();
      const state = await readState(),
        combat = state.projection.combat;
      expect(combat.authored.map_skin_asset_id).toBe('map.bell_archive.v2');
      const own = combat.snapshot.combatants.find(
        (actor: { character_id: string }) => actor.character_id === characterId,
      );
      expect(own).toBeTruthy();
      const actor = page.getByTestId(`combat-token-${own.id}`),
        map = page.getByTestId('authored-map'),
        skin = page.getByTestId('authored-map-skin');
      await expect(skin).toHaveAttribute('data-asset-id', 'map.bell_archive.v2');
      await expect
        .poll(() =>
          skin.evaluate(async (node) => {
            const image = new Image();
            image.src = node.getAttribute('href') ?? '';
            await image.decode();
            return [image.naturalWidth, image.naturalHeight];
          }),
        )
        .toEqual([1536, 1024]);
      const aligned = async () => {
        const box = await map.boundingBox(),
          image = await skin.boundingBox();
        if (!box || !image) throw Error('missing calibrated frame');
        for (const key of ['x', 'y', 'width', 'height'] as const)
          expect(Math.abs(box[key] - image[key])).toBeLessThan(0.5);
        for (const y of [0, 1, 2, 4, 5, 6, 7]) {
          const wall = await page
            .locator(`[data-testid="authored-wall"][data-cell-x="6"][data-cell-y="${y}"]`)
            .evaluate((node) => {
              const rect = node as SVGRectElement,
                matrix = rect.getScreenCTM();
              if (!matrix) throw Error('wall transform missing');
              const origin = new DOMPoint(
                rect.x.baseVal.value,
                rect.y.baseVal.value,
              ).matrixTransform(matrix);
              return { x: origin.x, y: origin.y };
            });
          if (!wall) throw Error('wall missing');
          expect(Math.abs(wall.x - (box.x + (box.width * 6) / 12))).toBeLessThan(0.5);
          expect(Math.abs(wall.y - (box.y + (box.height * y) / 8))).toBeLessThan(0.5);
        }
        await expect(page.locator('[data-testid="authored-wall"][data-cell-y="3"]')).toHaveCount(0);
        const exit = await page
          .getByTestId('authored-exit')
          .locator('rect')
          .evaluate((node) => {
            const rect = node as SVGRectElement,
              matrix = rect.getScreenCTM();
            if (!matrix) throw Error('exit transform missing');
            const origin = new DOMPoint(rect.x.baseVal.value, rect.y.baseVal.value).matrixTransform(
              matrix,
            );
            return { x: origin.x, y: origin.y };
          });
        if (!exit) throw Error('exit missing');
        expect(Math.abs(exit.x - (box.x + box.width / 12))).toBeLessThan(0.5);
        expect(Math.abs(exit.y - (box.y + (box.height * 3) / 8))).toBeLessThan(0.5);
      };
      await aligned();
      await page.getByRole('button', { name: 'Zoom in', exact: true }).click();
      await aligned();
      await page.getByRole('button', { name: 'Zoom out', exact: true }).click();
      await aligned();
      await page.getByRole('button', { name: 'Fit to view', exact: true }).click();
      await aligned();
      await page.getByRole('button', { name: 'Measure', exact: true }).click();
      const ruler = page.getByTestId('measure-overlay'),
        rulerBox = await ruler.boundingBox(),
        mapBox = await map.boundingBox();
      if (!rulerBox || !mapBox) throw Error('ruler frame missing');
      const start = {
        x: mapBox.x + (mapBox.width * 1.5) / 12,
        y: mapBox.y + (mapBox.height * 3.5) / 8,
      };
      await ruler.click({ position: { x: start.x - rulerBox.x, y: start.y - rulerBox.y } });
      await page.mouse.move(start.x + (mapBox.width * 2) / 12, start.y);
      await expect(page.getByTestId('measure-tooltip')).toHaveText('10 ft');
      await page.getByRole('button', { name: 'Measure', exact: true }).click();
      await page.getByRole('button', { name: 'Layers', exact: true }).click();
      await page.getByLabel('Scene image', { exact: true }).uncheck();
      await expect(skin).toHaveCount(0);
      await expect(page.getByTestId('authored-wall')).toHaveCount(7);
      await page.getByLabel('Scene image', { exact: true }).check();
      await aligned();
      await page.getByRole('button', { name: 'Layers', exact: true }).click();
      const before = await actor.boundingBox();
      if (!before) throw Error('hero missing');
      const reply = page.waitForResponse(
        (response) =>
          response.url().endsWith('/combat/action') && response.request().method() === 'POST',
      );
      await page.mouse.move(before.x + before.width / 2, before.y + before.height / 2);
      await page.mouse.down();
      await page.mouse.move(before.x + before.width * 1.5, before.y + before.height / 2, {
        steps: 5,
      });
      await page.mouse.up();
      expect((await reply).status()).toBe(200);
      const moved = (await readState()).projection.combat.snapshot.combatants.find(
        (candidate: { id: string }) => candidate.id === own.id,
      );
      expect(moved.position).toEqual({ x: own.position.x + 1, y: own.position.y });
      expect(moved.budget.movement_ft).toBe(own.budget.movement_ft - 5);
      await aligned();
      await page.screenshot({
        path: `.artifacts/visual-production/archive-skin-dpr-${dpr}.png`,
        animations: 'disabled',
      });
      await page.reload();
      await expect(skin).toBeVisible();
      await aligned();
    });
  });
