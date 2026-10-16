import { readFileSync } from 'node:fs';
import { expect, test } from '@playwright/test';
import { mockTauri } from './fixtures/tauri-mock';

const wire = JSON.parse(
  readFileSync(new URL('../src/api/__fixtures__/soloRecovery.json', import.meta.url), 'utf8'),
);
const scene = {
  id: '824e4fa6-b156-444c-984b-f891fe7f578d',
  title: 'Quiet crossing',
  mode: 'exploration',
};
for (const stage of [wire.before, wire.after, wire.afterRest]) stage.state.projection.scene = scene;
for (const loseReply of [false, true]) {
  test(`production solo recovery continues to rest${loseReply ? ' after lost reply and reload' : ''}`, async ({
    page,
  }) => {
    const first = wire.before.state;
    await mockTauri(page, {
      onboarding_completed: true,
      ui_language: 'ru',
      hero_class: 'fighter',
      active_campaign_id: first.campaign.id,
      active_session_id: first.session_id,
      active_provider: 'openai-compat',
      providers: {
        'openai-compat': {
          kind: 'openai-compat',
          baseUrl: 'http://127.0.0.1:31415/v1',
          model: 'fixture-dm',
          apiKey: 'fixture-key',
        },
        'local-mistralrs': null,
      },
      image_enabled: false,
      video_enabled: false,
      pc: first.projection.character,
    });
    let stage: 'before' | 'after' | 'rested' = 'before';
    const mutations: string[] = [];
    let originalNonce: string | null = null;
    await page.route(
      /\/campaigns(?:\/[^/?]+(?:\/(?:state|rest|recovery))?)?(?:\?|$)/,
      async (route) => {
        const request = route.request(),
          path = new URL(request.url()).pathname;
        const state =
          stage === 'before'
            ? wire.before.state
            : stage === 'after'
              ? wire.after.state
              : wire.afterRest.state;
        let body: unknown;
        if (path.endsWith('/recovery')) {
          if (request.method() === 'POST') {
            const input = request.postDataJSON();
            expect(input.intent).toEqual({ kind: 'wait', target_character_id: first.character_id });
            expect(input.expected_revision).toBe(wire.before.status.revision);
            expect(input).not.toHaveProperty('rng_seed');
            if (originalNonce === null) {
              originalNonce = input.causation_id;
              mutations.push('recover');
              stage = 'after';
              if (loseReply) {
                await route.abort('failed');
                return;
              }
              body = wire.after.result;
            } else {
              expect(input.causation_id).toBe(originalNonce);
              mutations.push('retry');
              body = {
                ...wire.after.result,
                receipt: { ...wire.after.result.receipt, duplicate: true },
              };
            }
          } else
            body =
              stage === 'before'
                ? wire.before.status
                : stage === 'after'
                  ? wire.after.result.status
                  : wire.afterRest.recovery;
        } else if (path.endsWith('/rest')) {
          if (request.method() === 'POST') {
            expect(request.postDataJSON().intent).toEqual({ kind: 'long' });
            mutations.push('rest');
            stage = 'rested';
            body = wire.afterRest.result;
          } else
            body =
              stage === 'before'
                ? wire.before.rest
                : stage === 'after'
                  ? wire.after.rest
                  : wire.afterRest.result.status;
        } else
          body = path.endsWith('/state')
            ? state
            : path === '/campaigns'
              ? [state.campaign]
              : state.campaign;
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify(body),
        });
      },
    );
    await page.goto('/');
    await expect(page.locator('.dm-char-fab')).toContainText('Mira');
    await page.locator('.dm-char-fab').click();
    await page.getByRole('button', { name: 'Полный лист персонажа', exact: true }).click();
    await page.getByRole('button', { name: 'Отдых', exact: true }).click();
    const rest = page.getByRole('dialog', { name: 'Отдых', exact: true });
    await expect(rest.getByText('Стабилен, без сознания', { exact: true })).toBeVisible();
    await expect(rest.getByRole('button', { name: 'Подтвердить', exact: true })).toHaveCount(0);
    if (!loseReply)
      await page.screenshot({
        path: '.artifacts/solo-recovery-stable.png',
        fullPage: true,
        animations: 'disabled',
      });
    await rest.getByRole('button', { name: 'Дождаться восстановления', exact: true }).click();
    const confirmation = page.getByRole('dialog', {
      name: 'Дождаться восстановления',
      exact: true,
    });
    await expect(confirmation.getByText(/Продвинуть игровые часы/)).toBeVisible();
    expect(mutations).toEqual([]);
    await confirmation
      .getByRole('button', { name: 'Дождаться восстановления', exact: true })
      .click();
    if (loseReply) {
      await expect(
        rest.getByRole('button', { name: 'Повторить тот же запрос', exact: true }),
      ).toBeVisible();
      await expect(rest.getByRole('button', { name: 'Закрыть', exact: true })).toBeDisabled();
      await page.reload();
      await expect(page.locator('.dm-char-fab')).toContainText('Mira');
      await page.locator('.dm-char-fab').click();
      await page.getByRole('button', { name: 'Полный лист персонажа', exact: true }).click();
      await page.getByRole('button', { name: 'Отдых', exact: true }).click();
      await rest.getByRole('button', { name: 'Повторить тот же запрос', exact: true }).click();
    }
    await expect(rest.getByText(/Хиты: 1 \/ 12/)).toBeVisible();
    await rest.getByRole('combobox', { name: 'Занятие' }).click();
    await page.getByRole('option', { name: 'Длительный отдых · 8 часов' }).click();
    await rest.getByRole('button', { name: 'Подтвердить', exact: true }).click();
    await expect(rest.getByText(/Хиты: 12 \/ 12/)).toBeVisible();
    expect(mutations).toEqual(loseReply ? ['recover', 'retry', 'rest'] : ['recover', 'rest']);
    await page.screenshot({
      path: `.artifacts/solo-recovery-${loseReply ? 'retry' : 'production'}.png`,
      fullPage: true,
      animations: 'disabled',
    });
  });
}

test('production solo death-save control shows the actual recorded die', async ({ page }) => {
  const wire = JSON.parse(
    readFileSync(new URL('../src/api/__fixtures__/soloDeathSave.json', import.meta.url), 'utf8'),
  );
  const first = wire.before.state;
  first.projection.scene = scene;
  await mockTauri(page, {
    onboarding_completed: true,
    ui_language: 'ru',
    hero_class: 'fighter',
    active_campaign_id: first.campaign.id,
    active_session_id: first.session_id,
    active_provider: 'openai-compat',
    providers: {
      'openai-compat': {
        kind: 'openai-compat',
        baseUrl: 'http://127.0.0.1:31415/v1',
        model: 'fixture-dm',
        apiKey: 'fixture-key',
      },
      'local-mistralrs': null,
    },
    image_enabled: false,
    video_enabled: false,
    pc: first.projection.character,
  });
  let completed = false;
  let requests = 0;
  await page.route(
    /\/campaigns(?:\/[^/?]+(?:\/(?:state|rest|recovery))?)?(?:\?|$)/,
    async (route) => {
      const req = route.request(),
        path = new URL(req.url()).pathname;
      let body: unknown;
      if (path.endsWith('/recovery')) {
        if (req.method() === 'POST') {
          const input = req.postDataJSON();
          expect(input.intent).toEqual({ kind: 'death_save' });
          expect(input).not.toHaveProperty('roll');
          expect(input).not.toHaveProperty('rng_seed');
          requests++;
          completed = true;
          body = wire.after.result;
        } else body = completed ? wire.after.result.status : wire.before.status;
      } else if (path.endsWith('/rest')) body = completed ? wire.after.rest : wire.before.rest;
      else
        body = path.endsWith('/state')
          ? first
          : path === '/campaigns'
            ? [first.campaign]
            : first.campaign;
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify(body),
      });
    },
  );
  await page.goto('/');
  await expect(page.locator('.dm-char-fab')).toContainText('Mira');
  await page.locator('.dm-char-fab').click();
  await page.getByRole('button', { name: 'Полный лист персонажа', exact: true }).click();
  await page.getByRole('button', { name: 'Отдых', exact: true }).click();
  const rest = page.getByRole('dialog', { name: 'Отдых', exact: true });
  await expect(rest.getByText('При смерти', { exact: true })).toBeVisible();
  await rest.getByRole('button', { name: 'Спасбросок от смерти', exact: true }).click();
  const face = wire.after.result.receipt.death_saves[0].result.roll;
  await expect(rest.getByText(`Спасбросок от смерти: ${face}`, { exact: true })).toBeVisible();
  expect(requests).toBe(1);
  await page.screenshot({
    path: '.artifacts/solo-death-save-production.png',
    fullPage: true,
    animations: 'disabled',
  });
});
