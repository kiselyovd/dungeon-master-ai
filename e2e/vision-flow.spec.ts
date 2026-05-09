import path from 'node:path';
import { expect, test } from '@playwright/test';

test.beforeEach(async ({ page }) => {
  await page.addInitScript(() => {
    (window as unknown as { __TAURI_INTERNALS__: unknown }).__TAURI_INTERNALS__ = {
      invoke: async () => null,
      transformCallback: () => 0,
    };
  });

  // Mock the /chat SSE endpoint with a short scripted response.
  await page.route('**/chat', async (route) => {
    const body =
      'event: text_delta\ndata: {"text":"got it"}\n\n' +
      'event: done\ndata: {"reason":"stop"}\n\n';
    await route.fulfill({
      status: 200,
      headers: { 'content-type': 'text/event-stream' },
      body,
    });
  });
});

test('user can attach an image via file input and see it after sending', async ({ page }) => {
  await page.goto('/');

  const fileInput = page.locator('input[type=file]');
  await fileInput.setInputFiles(path.join(__dirname, 'fixtures', 'sample.png'));

  // Thumbnail strip shows the staged image.
  await expect(page.locator('[role=list] img')).toHaveCount(1);

  // Type a description and send.
  const textarea = page.getByRole('textbox');
  await textarea.fill('what is in this image?');
  await page.getByRole('button', { name: /^Send$/i }).click();

  // The user bubble in history now contains the image.
  await expect(page.locator('[data-testid="bubble"][data-role="user"] img')).toHaveCount(1);

  // The mocked assistant text streams in.
  await expect(page.getByText('got it')).toBeVisible();
});
