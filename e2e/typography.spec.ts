import { expect, test } from '@playwright/test';
import { emptyPcData, PRESETS } from '../src/state/pc';
import { mockTauri } from './fixtures/tauri-mock';

const campaignId = '59ed5276-bb7c-45a7-8e27-8fb11890e8ba';
const sessionId = 'e8da1a25-4020-434f-bf30-d95e61cebdce';
const actorId = '08105c64-17ef-4d39-8200-639f81808206';
for (const language of ['ru', 'en'] as const) {
  for (const viewport of [
    { width: 960, height: 640 },
    { width: 1280, height: 800 },
  ]) {
    test(`production typography ${language} ${viewport.width}: local glyphs, reading preference and reachable controls`, async ({
      page,
    }) => {
      await page.setViewportSize(viewport);
      const fontFailures: string[] = [];
      page.on('request', (request) => {
        if (request.resourceType() === 'font' && new URL(request.url()).hostname !== '127.0.0.1')
          fontFailures.push(request.url());
      });
      await page.addInitScript(() => {
        const observations: number[] = [];
        Object.assign(window, { typographyShifts: observations });
        new PerformanceObserver((list) => {
          for (const entry of list.getEntries()) {
            const shift = entry as PerformanceEntry & { hadRecentInput: boolean; value: number };
            if (!shift.hadRecentInput) observations.push(shift.value);
          }
        }).observe({ type: 'layout-shift', buffered: true });
      });
      await mockTauri(page, {
        onboarding_completed: true,
        hero_class: 'fighter',
        active_campaign_id: campaignId,
        active_session_id: sessionId,
        ui_language: language,
        image_enabled: false,
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
      const prose =
        language === 'ru'
          ? 'Мира Mira вошла в архив. Ёжик и звёзды, ÅÉÖŁ, № 12. **Щит готов.** *Тихий шаг.*'
          : 'Mira Мира entered the archive. Ёжик, ÅÉÖŁ, № 12. **Shield ready.** *A quiet step.*';
      const state = {
        schema_version: 1,
        ruleset_id: 'srd-5.2.1',
        session_id: sessionId,
        character_id: actorId,
        campaign: {
          id: campaignId,
          schema_version: 1,
          ruleset_id: 'srd-5.2.1',
          title: language === 'ru' ? 'Архив звёзд' : 'The Star Archive',
          premise: 'Typography fixture',
          seed: { kind: 'collaborative', answers: { genre: ['classic_fantasy'] } },
          last_session_id: sessionId,
          revision: 3,
          created_at: '2026-10-12T00:00:00Z',
          updated_at: '2026-10-12T00:00:00Z',
        },
        projection: {
          character: {
            ...emptyPcData(),
            ...PRESETS.fighter,
            heroClass: 'fighter',
            name: 'Мира Mira',
            characterId: actorId,
          },
          messages: [{ id: 'narration', role: 'assistant', content: prose }],
          combat: null,
          scene: {
            id: '824e4fa6-b156-444c-984b-f891fe7f578d',
            title: language === 'ru' ? 'Архив звёзд' : 'The Star Archive',
            mode: 'exploration',
          },
          adventure: null,
          journal: [],
          npcs: [],
        },
      };
      await page.route('**/campaigns/*/state', (route) =>
        route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify(state),
        }),
      );
      // Simulate an offline installation: all font bytes must come from the app.
      await page.route('https://**', (route) => route.abort());
      await page.goto('/');
      const bubble = page.locator('[data-testid="bubble"][data-role="assistant"]');
      await expect(bubble).toBeVisible();
      await page.evaluate(async () => {
        await Promise.all([
          document.fonts.load('600 16px Manrope', 'Мира Mira Ёжик № ÅÉÖŁ'),
          document.fonts.load('400 17px Literata', 'Мира Mira Ёжик № ÅÉÖŁ'),
          document.fonts.load('italic 400 17px Literata', 'Мира Mira Ёжик №'),
          document.fonts.load('700 17px Literata', 'Мира Mira Щит'),
        ]);
        await document.fonts.ready;
      });
      expect(fontFailures).toEqual([]);
      expect(await bubble.evaluate((node) => getComputedStyle(node).fontSize)).toBe('17px');
      expect(await bubble.evaluate((node) => getComputedStyle(node).lineHeight)).toBe('28.9px');
      const cdp = await page.context().newCDPSession(page);
      await cdp.send('DOM.enable');
      await cdp.send('CSS.enable');
      const { root } = await cdp.send('DOM.getDocument');
      const { nodeId } = await cdp.send('DOM.querySelector', {
        nodeId: root.nodeId,
        selector: '[data-testid="bubble"][data-role="assistant"] p',
      });
      const { fonts } = await cdp.send('CSS.getPlatformFontsForNode', { nodeId });
      expect(fonts.length).toBeGreaterThan(0);
      expect(fonts.every((font) => font.isCustomFont && font.familyName === 'Literata')).toBe(true);
      const size = page.getByRole('combobox', {
        name: language === 'ru' ? 'Размер текста для чтения' : 'Reading text size',
      });
      await size.focus();
      await size.press('Enter');
      await size.press('ArrowDown');
      await size.press('Enter');
      await expect(bubble).toHaveCSS('font-size', '18px');
      const send = page.getByRole('button', {
        name: language === 'ru' ? 'Отправить' : 'Send',
        exact: true,
      });
      const composer = page.getByPlaceholder(
        language === 'ru' ? 'Что ты делаешь?' : 'What do you do?',
      );
      await expect(composer).toBeVisible();
      await expect(page.getByTestId('preflight-finish-setup')).toHaveCount(0);
      await composer.click({ trial: true });
      await expect(send).toBeVisible();
      const sendBox = await send.boundingBox();
      expect(sendBox && sendBox.y + sendBox.height).toBeLessThanOrEqual(viewport.height);
      expect(
        await send.evaluate((node) => Number.parseFloat(getComputedStyle(node).fontSize)),
      ).toBeGreaterThanOrEqual(14);
      const cls = await page.evaluate(
        () =>
          (window as Window & { typographyShifts?: number[] }).typographyShifts?.reduce(
            (sum, value) => sum + value,
            0,
          ) ?? 0,
      );
      expect(cls).toBeLessThan(0.1);
      await page.reload();
      await expect(bubble).toHaveCSS('font-size', '18px');
      await expect(page.locator('.dm-splash')).toHaveCount(0);
      await page.evaluate(() => document.fonts.ready);
      await page.screenshot({
        path: `.artifacts/product-uiux/typography-${language}-${viewport.width}.png`,
      });
      await cdp.detach();
    });
  }
}
