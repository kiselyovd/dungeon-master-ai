import { expect, test } from '@playwright/test';
import { mockTauri } from './fixtures/tauri-mock';

/**
 * Chat composer + agent turn streaming. Mocks the session-messages fetch
 * (empty) and the /agent/turn SSE stream so a send produces a user bubble and
 * a streamed assistant reply without a real backend.
 */
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
      'event: text_delta\ndata: {"text":"You enter the tavern."}\n\n' +
      'event: agent_done\ndata: {"total_rounds":1}\n\n';
    await route.fulfill({
      status: 200,
      headers: { 'content-type': 'text/event-stream' },
      body,
    });
  });
});

test('Send is disabled until text is entered', async ({ page }) => {
  await page.goto('/');
  const send = page.getByRole('button', { name: 'Send', exact: true });
  await expect(send).toBeDisabled();
  await page.getByPlaceholder(/what do you do/i).fill('look around');
  await expect(send).toBeEnabled();
});

test('sending a message shows the user bubble and the streamed assistant reply', async ({
  page,
}) => {
  await page.goto('/');
  await page.getByPlaceholder(/what do you do/i).fill('look around');
  await page.getByRole('button', { name: 'Send', exact: true }).click();

  // User bubble echoes the input.
  await expect(page.locator('[data-testid="bubble"][data-role="user"]')).toContainText(
    /look around/i,
  );
  // Assistant reply streams in from the mocked SSE.
  await expect(page.getByText('You enter the tavern.')).toBeVisible();
});
