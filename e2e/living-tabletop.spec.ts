import { expect, test } from '@playwright/test';
import { mockTauri } from './fixtures/tauri-mock';

test.beforeEach(async ({ page }) => {
  await mockTauri(page, {
    onboarding_completed: true,
    hero_class: 'fighter',
    active_provider: 'local-mistralrs',
  });
  await page.route('**/sessions/**', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ messages: [] }),
    });
  });
  await page.route('**/agent/turn', async (route) => {
    const body =
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

test('a deterministic scene change uses calm semantic art', async ({ page }) => {
  await page.goto('/');
  await page.getByPlaceholder(/what do you do/i).fill('approach the old bridge');
  await page.getByRole('button', { name: 'Send', exact: true }).click();

  const art = page.getByTestId('scene-transition-art');
  await expect(art).toBeVisible();
  await expect(art).toHaveAttribute('src', /scene-combat.*\.webp/);
});

test('reduced motion keeps scene art still', async ({ page }) => {
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto('/');
  await page.getByPlaceholder(/what do you do/i).fill('approach the old bridge');
  await page.getByRole('button', { name: 'Send', exact: true }).click();

  const art = page.getByTestId('scene-transition-art');
  await expect(art).toBeVisible();
  await expect(art).toHaveCSS('animation-name', 'none');
  await expect(page.locator('.dm-ambient-dust')).toHaveCSS('animation-name', 'none');
});
