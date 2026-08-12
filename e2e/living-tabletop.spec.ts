import { expect, test } from '@playwright/test';
import { mockTauri } from './fixtures/tauri-mock';

test.beforeEach(async ({ page }) => {
  await mockTauri(page, {
    onboarding_completed: true,
    hero_class: 'fighter',
    active_provider: 'local-mistralrs',
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
  await mockTauri(page);
  await page.goto('/');

  await expect(page.getByTestId('onboarding-hero-art')).toHaveAttribute(
    'src',
    /onboarding.*\.webp/,
  );
  await page.getByRole('button', { name: /Continue/i }).click();
  await page.getByRole('radio', { name: /Skip setup/i }).click();
  await page.getByRole('button', { name: /Continue/i }).click();

  for (const role of ['fighter', 'wizard', 'rogue', 'cleric']) {
    await expect(page.locator(`img[src*="hero-${role}"]`)).toBeVisible();
  }
});

test('empty VTT uses living-tabletop art and retains product labels', async ({ page }) => {
  await page.goto('/');

  await expect(page.getByText('DUNGEON MASTER AI')).toBeVisible();
  await expect(page.getByText('Untitled Campaign')).toBeVisible();
  await expect(page.locator('.dm-vtt-empty-art')).toHaveAttribute('src', /vtt-empty.*\.webp/);
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
  await page.getByRole('dialog').getByRole('button', { name: /close/i }).first().click();

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
  const count = await animatedDescendants.count();
  for (let index = 0; index < count; index += 1) {
    await expect(animatedDescendants.nth(index)).toHaveCSS('animation-name', 'none');
  }
});
