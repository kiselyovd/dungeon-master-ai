import { existsSync, readFileSync } from 'node:fs';
import { expect, test } from '@playwright/test';
import { mockTauri } from './fixtures/tauri-mock';

const fixturePath = '.artifacts/product-uiux/task-30-bundled-wire.json';
const fixtures = (
  existsSync(fixturePath)
    ? JSON.parse(readFileSync(fixturePath, 'utf8'))
    : [
        'tavern-interior',
        'forest-crossing',
        'ruined-courtyard',
        'cave-chamber',
        'dungeon-hall',
        'temple-floor',
        'village-square',
        'tower-chamber',
      ].map((name) => ({
        id: `map-${name}`,
        prompt: name.replaceAll('-', ' '),
        image_b64: readFileSync(
          `crates/adapter-media/assets/bundled/maps/map-${name}.webp`,
        ).toString('base64'),
      }))
) as Array<{ id: string; prompt: string; image_b64: string }>;
for (const fixture of fixtures)
  if (
    !Buffer.from(fixture.image_b64, 'base64').equals(
      readFileSync(`crates/adapter-media/assets/bundled/maps/${fixture.id}.webp`),
    )
  )
    throw Error('packaged_map_fixture_changed');
test('all eight actual packaged fallback maps decode through image events as scenery, never inferred terrain', async ({
  page,
}) => {
  await page.setViewportSize({ width: 1440, height: 960 });
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
    if (!fixture) throw Error('unexpected extra turn');
    const event = (name: string, data: unknown) =>
      `event: ${name}\ndata: ${JSON.stringify(data)}\n\n`;
    const id = `map-${index}`;
    const body =
      event('tool_call_start', { id, tool_name: 'generate_map', round: 1 }) +
      event('image_generated', {
        tool_call_id: id,
        mime_type: 'image/webp',
        image_b64: fixture.image_b64,
        kind: 'map',
        source: 'bundled',
        asset_id: fixture.id,
      }) +
      event('tool_call_result', {
        id,
        tool_name: 'generate_map',
        args: { prompt: fixture.prompt },
        result: { ok: true },
        is_error: false,
        round: 1,
        handled_by: 'engine',
      }) +
      event('agent_done', { total_rounds: 1 });
    return route.fulfill({ status: 200, contentType: 'text/event-stream', body });
  });
  await page.goto('/');
  for (const fixture of fixtures) {
    const draft = page.getByPlaceholder('What do you do?');
    await draft.fill(`Show ${fixture.prompt}`);
    await page.getByRole('button', { name: 'Send', exact: true }).click();
    const image = page.getByTestId('current-scene-art').locator('img');
    await expect
      .poll(() => image.evaluate((node: HTMLImageElement) => node.complete && node.naturalWidth))
      .toBe(1280);
    await expect(page.getByTestId('authored-map')).toHaveCount(0);
    await expect(page.locator('.dm-vtt-controls')).toBeHidden();
    await page.getByRole('button', { name: 'Open painting: Scene painting', exact: true }).click();
    const preview = page.getByTestId('scene-art-preview');
    await expect(preview.locator('img')).toHaveCSS('object-fit', 'contain');
    await page.screenshot({
      path: `.artifacts/visual-production/${fixture.id}-consumer.png`,
      animations: 'disabled',
    });
    await page.getByRole('button', { name: 'Close painting', exact: true }).click();
  }
  expect(index).toBe(8);
});
