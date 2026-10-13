import { existsSync, readFileSync } from 'node:fs';
import { expect, test } from '@playwright/test';
import { mockTauri } from './fixtures/tauri-mock';

const fixturePath = '.artifacts/product-uiux/task-31-illustration-wire.json';
const fixtures = (
  existsSync(fixturePath)
    ? JSON.parse(readFileSync(fixturePath, 'utf8'))
    : [
        'tavern',
        'village',
        'forest-road',
        'ruins',
        'cave',
        'dungeon',
        'temple',
        'wizard-tower',
      ].map((name) => ({
        id: `illustration-${name}`,
        prompt: name.replaceAll('-', ' '),
        image_b64: readFileSync(
          `crates/adapter-media/assets/bundled/illustrations/illustration-${name}.webp`,
        ).toString('base64'),
      }))
) as Array<{ id: string; prompt: string; image_b64: string }>;
for (const fixture of fixtures)
  if (
    !Buffer.from(fixture.image_b64, 'base64').equals(
      readFileSync(`crates/adapter-media/assets/bundled/illustrations/${fixture.id}.webp`),
    )
  )
    throw Error('packaged_illustration_fixture_changed');

test('all eight real resilient fallback illustrations render in chat and accessible full views without becoming portraits/maps', async ({
  page,
}) => {
  await page.setViewportSize({ width: 1280, height: 800 });
  await mockTauri(page, {
    onboarding_completed: true,
    hero_class: 'fighter',
    ui_language: 'en',
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
  await page.route('**/sessions/**', (route) =>
    route.fulfill({ status: 200, contentType: 'application/json', body: '{"messages":[]}' }),
  );
  let index = 0;
  await page.route('**/agent/turn', (route) => {
    const fixture = fixtures[index++];
    if (!fixture) throw Error('unexpected extra image turn');
    const event = (name: string, data: unknown) =>
        `event: ${name}\ndata: ${JSON.stringify(data)}\n\n`,
      id = `picture-${index}`;
    return route.fulfill({
      status: 200,
      contentType: 'text/event-stream',
      body:
        event('tool_call_start', { id, tool_name: 'generate_illustration', round: 1 }) +
        event('image_generated', {
          tool_call_id: id,
          mime_type: 'image/webp',
          image_b64: fixture.image_b64,
          kind: 'chat',
          source: 'bundled',
          asset_id: fixture.id,
        }) +
        event('tool_call_result', {
          id,
          tool_name: 'generate_illustration',
          args: { prompt: fixture.prompt },
          result: { ok: true },
          is_error: false,
          round: 1,
          handled_by: 'engine',
        }) +
        event('agent_done', { total_rounds: 1 }),
    });
  });
  await page.goto('/');
  for (const [offset, fixture] of fixtures.entries()) {
    const draft = page.getByPlaceholder('What do you do?');
    await draft.fill(`Show ${fixture.prompt}`);
    await page.getByRole('button', { name: 'Send', exact: true }).click();
    const card = page.getByTestId(`tool-call-card-picture-${offset + 1}`),
      image = card.locator('img');
    await expect(card).toHaveAttribute('data-image-kind', 'chat');
    await expect(card).toHaveAttribute('data-image-asset-id', fixture.id);
    await expect
      .poll(() => image.evaluate((node: HTMLImageElement) => node.complete && node.naturalWidth))
      .toBe(1280);
    await expect(page.getByTestId('authored-map')).toHaveCount(0);
    const opener = card.getByRole('button', { name: 'Open image', exact: true });
    await opener.click();
    const modal = page.getByRole('dialog', { name: 'Image', exact: true });
    await expect(modal).toBeVisible();
    await expect(modal.locator('img')).toHaveCSS('object-fit', 'contain');
    await page.screenshot({
      path: `.artifacts/visual-production/${fixture.id}-consumer.png`,
      animations: 'disabled',
    });
    await page.keyboard.press('Escape');
    await expect(modal).toHaveCount(0);
    await expect(opener).toBeFocused();
  }
  expect(index).toBe(8);
});
