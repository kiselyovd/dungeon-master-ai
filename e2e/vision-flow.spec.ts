import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { expect, test } from '@playwright/test';
import { mockTauri } from './fixtures/tauri-mock';

// __dirname is a CommonJS global; under ESM (which Playwright uses when
// "type": "module" is set in package.json) it is undefined. Derive it from
// import.meta.url so the fixture path resolves on the CI runner.
const __dirname = path.dirname(fileURLToPath(import.meta.url));

test.beforeEach(async ({ page }) => {
  // active_provider keeps PreflightCheck satisfied (local-mistralrs is
  // exempt from the missing_chat check) so its blocking modal never mounts.
  await mockTauri(page, {
    onboarding_completed: true,
    hero_class: 'fighter',
    active_provider: 'local-mistralrs',
  });

  // Mock the /chat SSE endpoint with a short scripted response.
  await page.route('**/chat', async (route) => {
    const body =
      'event: text_delta\ndata: {"text":"got it"}\n\n' + 'event: done\ndata: {"reason":"stop"}\n\n';
    await route.fulfill({
      status: 200,
      headers: { 'content-type': 'text/event-stream' },
      body,
    });
  });
});

// Skipped: image attachments are paste/drag-drop only (no visible file input),
// and synthesising a real DataTransfer drop with image bytes is unreliable in
// headless chromium. Attachment behaviour is covered by the ComposerAttachments
// + ChatPanel vitest unit tests; chat send/stream is covered end-to-end in
// chat.spec.ts. Kept as a placeholder for a future drag-drop harness.
test.skip('user can attach an image via file input and see it after sending', async ({ page }) => {
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
