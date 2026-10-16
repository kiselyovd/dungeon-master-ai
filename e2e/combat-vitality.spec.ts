import { readFileSync } from 'node:fs';
import { expect, test } from '@playwright/test';
import { mockTauri } from './fixtures/tauri-mock';

const wire = JSON.parse(
  readFileSync(new URL('../src/api/__fixtures__/soloCombatVitality.json', import.meta.url), 'utf8'),
);
test('production tabletop removes a finished encounter while keeping the living downed hero state', async ({
  page,
}) => {
  const before = wire.before,
    hero = before.projection.combat.snapshot.combatants.find(
      (a: { character_id?: string; id: string }) => a.character_id === before.character_id,
    );
  await mockTauri(page, {
    onboarding_completed: true,
    ui_language: 'ru',
    hero_class: 'fighter',
    active_campaign_id: before.campaign.id,
    active_session_id: before.session_id,
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
    pc: before.projection.character,
  });
  await page.route(/\/campaigns(?:\/[^/?]+(?:\/state)?)?(?:\?|$)/, (route) => {
    const path = new URL(route.request().url()).pathname;
    return route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(
        path.endsWith('/state')
          ? before
          : path === '/campaigns'
            ? [before.campaign]
            : before.campaign,
      ),
    });
  });
  let ended = false;
  await page.route('**/combat/action', async (route) => {
    const input = route.request().postDataJSON();
    expect(input.action_type).toBe('end_turn');
    expect(input.args.combatant_id).toBe(hero.id);
    ended = true;
    await route.fulfill({
      status: 200,
      contentType: 'text/event-stream',
      body: `event: combat_projection\ndata: ${JSON.stringify({ type: 'combat_projection', projection: wire.ended })}\n\n`,
    });
  });
  await page.goto('/');
  await expect(page.locator('.dm-char-fab')).toContainText('Дарен');
  await expect(page.getByTestId(`combat-token-${hero.id}`)).toBeVisible();
  await expect(page.getByTestId('combat-token-skull')).toHaveCount(0);
  await page.getByRole('button', { name: /Закончить ход/ }).click();
  await expect.poll(() => ended).toBe(true);
  await expect(page.locator('.dm-char-fab')).toContainText('0/11');
  await expect(page.getByTestId(`combat-token-${hero.id}`)).toHaveCount(0);
  await expect(page.getByTestId('combat-token-skull')).toHaveCount(0);
  await page.screenshot({
    path: '.artifacts/combat-vitality-production.png',
    fullPage: true,
    animations: 'disabled',
  });
});
