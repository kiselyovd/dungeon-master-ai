import { expect, test } from '@playwright/test';
import { mockTauri } from './fixtures/tauri-mock';

test.beforeEach(async ({ page }, testInfo) => {
  if (testInfo.title.startsWith('onboarding welcome')) {
    await mockTauri(page, { ui_language: 'en' });
    return;
  }
  await mockTauri(page, {
    onboarding_completed: true,
    hero_class: 'fighter',
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
    ui_language: 'en',
  });
  await page.route('**/sessions/**', async (route) => {
    if (route.request().url().includes('/saves')) {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify([
          {
            id: 'save-1',
            session_id: 'session-1',
            kind: 'manual',
            title: 'Old bridge watch',
            summary: 'Mira warned the party about the bridge.',
            tag: 'exploration',
            created_at: '2026-08-12T08:00:00Z',
            turn_number: 3,
          },
        ]),
      });
      return;
    }
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ messages: [] }),
    });
  });
  await page.route('**/agent/turn', async (route) => {
    const body =
      'event: tool_call_start\n' +
      'data: {"id":"npc-1","tool_name":"remember_npc","round":1}\n\n' +
      'event: tool_call_result\n' +
      'data: {"id":"npc-1","tool_name":"remember_npc","args":{"name":"Mira","role":"Innkeeper","disposition":"friendly","fact":"She watches the old bridge."},"result":{"npc_id":"npc-1"},"is_error":false,"round":1,"handled_by":"engine"}\n\n' +
      'event: tool_call_result\n' +
      'data: {"id":"scene-1","tool_name":"set_scene","args":{"title":"Combat at the old bridge"},"result":{"scene_id":"scene-1"},"is_error":false,"round":1,"handled_by":"engine"}\n\n' +
      'event: agent_done\ndata: {"total_rounds":1}\n\n';
    await route.fulfill({
      status: 200,
      headers: { 'content-type': 'text/event-stream' },
      body,
    });
  });
});

test('onboarding welcome and four class portraits use living-tabletop art', async ({ page }) => {
  await page.goto('/');

  await expect(page.getByTestId('onboarding-hero-art')).toHaveAttribute(
    'src',
    /onboarding.*\.webp/,
  );
  await page.getByRole('button', { name: 'Start adventure', exact: true }).click();
  await page.getByRole('button', { name: 'Choose hero', exact: true }).click();

  for (const role of ['fighter', 'wizard', 'rogue', 'cleric']) {
    const card = page.getByRole('radio', { name: new RegExp(role, 'i') });
    await expect(card.locator('..').locator('img')).toBeVisible();
  }
});

test('empty VTT uses living-tabletop art and retains product labels', async ({ page }) => {
  await page.goto('/');

  await expect(page.getByText('DUNGEON MASTER AI')).toBeVisible();
  await expect(page.getByText('Fixture Chronicle', { exact: true }).first()).toBeVisible();
  await expect(page.locator('.dm-vtt-empty-art')).toHaveAttribute('data-asset-id', 'vtt.empty.v2');
  await expect(page.locator('.dm-vtt-empty[data-art-direction="living-tabletop"]')).toBeVisible();
});

test('NPC, save, and combat scene use their semantic images', async ({ page }) => {
  await page.goto('/');
  await page.getByPlaceholder(/what do you do/i).fill('approach the old bridge');
  await page.getByRole('button', { name: 'Send', exact: true }).click();

  const art = page.getByTestId('scene-transition-art');
  await expect(art).toBeVisible();
  await expect(art).toHaveAttribute('src', /scene-combat.*\.webp/);
  await page.keyboard.press('Escape');
  await expect(art).toHaveCount(0);

  await page.getByRole('button', { name: 'NPCs', exact: true }).click();
  await expect(page.locator('img[data-archetype="innkeeper"]')).toHaveAttribute(
    'src',
    /npc-innkeeper.*\.webp/,
  );
  await page
    .getByRole('complementary', { name: 'Adventure', exact: true })
    .getByRole('button', { name: 'Close', exact: true })
    .click();

  await page.getByRole('button', { name: 'Saves', exact: true }).click();
  await expect(page.locator('.dm-save-thumb[data-tag="exploration"] img').first()).toHaveAttribute(
    'src',
    /save-exploration.*\.webp/,
  );
});

test('reduced motion stops every living-tabletop descendant animation', async ({ page }) => {
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto('/');
  await page.getByPlaceholder(/what do you do/i).fill('approach the old bridge');
  await page.getByRole('button', { name: 'Send', exact: true }).click();

  const art = page.getByTestId('scene-transition-art');
  await expect(art).toBeVisible();
  const animatedDescendants = page.locator('[data-art-direction="living-tabletop"] *');
  await expect
    .poll(async () =>
      animatedDescendants.evaluateAll((elements) =>
        elements.every((element) => getComputedStyle(element).animationName === 'none'),
      ),
    )
    .toBe(true);
});
