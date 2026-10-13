import { expect, test } from '@playwright/test';
import wire from '../src/api/__fixtures__/soloCombatVitality.json' with { type: 'json' };
import { mockTauri } from './fixtures/tauri-mock';

const archetypes = [
  'water_echo',
  'goblin',
  'skeleton',
  'zombie',
  'wolf',
  'bandit',
  'orc',
  'giant_rat',
];
function encounter() {
  const source = structuredClone(wire.ended);
  const base = source.snapshot.combatants.find((actor) => !actor.character_id);
  const human = source.snapshot.combatants.find((actor) => actor.character_id);
  if (!base || !human) throw Error('real combat fixture missing actors');
  const enemies = [...archetypes, 'future_opponent'].map((_appearance, index) => ({
    ...structuredClone(base),
    id: `12345678-1234-4234-8234-${String(index).padStart(12, '0')}`,
    display_name_key: null,
    name: `Visible opponent ${index + 1}`,
    current_hp: index === 6 ? 0 : 6,
    max_hp: 6,
    ac: 11,
    is_dead: index === 6,
    conditions: index === 2 ? ['poisoned', 'prone'] : [],
    // Place actors beside the fixed initiative overlay. The second row has
    // adjacent cells so the decorative silhouettes deliberately overlap.
    position: {
      x: 11 + (index % 4) * (index >= 4 && index < 8 ? 1 : 2),
      y: 7 + Math.floor(index / 4) * 3,
    },
  }));
  const hero = {
    ...human,
    id: 'human-token',
    character_id: '92414394-f4b1-477f-9278-10f75c3c4fb7',
    name: 'Hero',
    current_hp: 12,
    is_dead: false,
    conditions: [],
    position: { x: 1, y: 6 },
  };
  return {
    ...source,
    authored: null,
    adventure: null,
    campaign_scope: null,
    appearances: Object.fromEntries(
      enemies.map((actor, index) => [actor.id, [...archetypes, 'future_opponent'][index]]),
    ),
    snapshot: {
      ...source.snapshot,
      active: true,
      current_combatant: hero.id,
      combatants: [hero, ...enemies],
      initiative: [hero, ...enemies].map((actor, index) => ({
        id: actor.id,
        roll: 20 - index,
        dex_tiebreak: 0,
      })),
    },
  };
}

for (const language of ['en', 'ru']) {
  test(`opponents ${language}: real root, explicit art, selection, small zoom and cold restore`, async ({
    page,
  }) => {
    const failures: string[] = [];
    page.on('pageerror', (error) => failures.push(error.message));
    await page.setViewportSize({ width: 1440, height: 960 });
    const projection = encounter();
    await mockTauri(page, {
      onboarding_completed: true,
      hero_class: 'fighter',
      ui_language: language,
      public_combat: projection,
      pc: { portraitAssetId: 'hero.fighter.01.v2' },
    });
    await page.route('**/sessions/**', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ messages: [] }),
      }),
    );
    await page.goto('/');
    const board = page.getByTestId('dm-vtt');
    await expect(board).toBeVisible();
    const compactQueue = page.locator('.dm-init-summary');
    if (await compactQueue.count()) await compactQueue.click();
    for (const [index, archetype] of archetypes.entries()) {
      const id = projection.snapshot.combatants[index + 1]?.id;
      if (!id) throw Error('missing fixture actor');
      const sprite = page.getByTestId(`miniature-${id}`);
      await expect(sprite).toHaveAttribute('data-asset-id', `opponent.${archetype}.v2`);
      await expect
        .poll(() =>
          sprite
            .locator('img')
            .evaluate((image: HTMLImageElement) => image.complete && image.naturalHeight === 768),
        )
        .toBe(true);
      const cell = page.getByTestId(`combat-token-${id}`);
      await cell.click();
      await expect(cell).toHaveAttribute('data-selected', 'true');
      expect(await sprite.evaluate((node) => getComputedStyle(node).pointerEvents)).toBe('none');
      await expect(
        page.locator(`.dm-init-card img[data-asset-id="opponent.${archetype}.v2"]`),
      ).toHaveCount(1);
    }
    const unknown = projection.snapshot.combatants.at(-1);
    if (!unknown) throw Error('missing fallback');
    await expect(page.getByTestId(`miniature-${unknown.id}`)).toHaveCount(0);
    const dead = projection.snapshot.combatants[7];
    if (!dead) throw Error('missing dead actor');
    await expect(page.getByTestId(`combat-token-${dead.id}`)).toHaveAttribute('data-dead', 'true');
    await expect(page.getByTestId('combat-token-skull')).toHaveCount(1);
    await page.screenshot({
      path: `.artifacts/visual-production/opponent-board-${language}.png`,
      animations: 'disabled',
    });
    await page
      .getByRole('button', { name: language === 'ru' ? 'Измерить' : 'Measure', exact: true })
      .click();
    const ruler = page.getByTestId('measure-overlay');
    const rulerBox = await ruler.boundingBox();
    const size = await page
      .getByTestId('combat-token-human-token')
      .evaluate((node) => parseFloat(node.style.width));
    if (!rulerBox) throw Error('ruler missing');
    await ruler.click({ position: { x: 400, y: 450 } });
    await page.mouse.move(rulerBox.x + 400 + size * 2, rulerBox.y + 450);
    await expect(page.getByTestId('measure-tooltip')).toHaveText('10 ft');
    await page
      .getByRole('button', { name: language === 'ru' ? 'Измерить' : 'Measure', exact: true })
      .click();
    const zoomOut = page.getByRole('button', {
      name: language === 'ru' ? 'Отдалить' : 'Zoom out',
      exact: true,
    });
    for (let index = 0; index < 3; index++) await zoomOut.click();
    await page.screenshot({
      path: `.artifacts/visual-production/opponent-low-zoom-${language}.png`,
      animations: 'disabled',
    });
    await page.reload();
    await expect(
      page.locator('[data-testid^="miniature-"][data-asset-id^="opponent."]'),
    ).toHaveCount(8);
    expect(failures).toEqual([]);
    await page.setViewportSize({ width: 1440, height: 1550 });

    // Inspect the exact decoded production images on light and dark grounds.
    // This disposable QA sheet is never shipped as a product screen.
    const images = await page
      .locator('[data-testid^="miniature-"][data-asset-id^="opponent."] img')
      .evaluateAll((nodes) =>
        nodes.map((node) => {
          const frame = node.parentElement,
            cell = frame?.parentElement;
          if (!frame || !cell) throw Error('production frame missing');
          return {
            src: (node as HTMLImageElement).src,
            id: frame.dataset.assetId ?? '',
            width: parseFloat(frame.style.width),
            height: parseFloat(frame.style.height),
            bottom: parseFloat(frame.style.bottom),
            transform: frame.style.transform,
            cell: parseFloat(cell.style.width),
          };
        }),
      );
    await page.evaluate((images) => {
      const sheet = document.createElement('div');
      sheet.id = 'alpha-review';
      sheet.style.cssText =
        'position:fixed;inset:0;z-index:999999;overflow:auto;background:#fff;padding:24px;display:grid;grid-template-columns:repeat(8,1fr);gap:8px;font:12px sans-serif;color:#111';
      for (const background of ['#eee6d3', '#17151c'])
        for (const size of [30, 48, 64])
          for (const image of images) {
            const card = document.createElement('div');
            card.style.cssText = `height:220px;background:${background};position:relative`;
            const base = document.createElement('div');
            base.style.cssText = `position:absolute;width:${size}px;height:${size}px;left:50%;bottom:24px;transform:translateX(-50%)`;
            const frame = document.createElement('div');
            const scale = size / image.cell;
            frame.style.cssText = `position:absolute;left:50%;width:${image.width * scale}px;height:${image.height * scale}px;bottom:${image.bottom * scale}px;transform:${image.transform}`;
            const sprite = document.createElement('img');
            sprite.src = image.src;
            sprite.style.cssText = 'width:100%;height:100%;object-fit:contain';
            const label = document.createElement('span');
            label.textContent = `${image.id} / ${size}`;
            label.style.cssText = `position:absolute;bottom:4px;color:${background === '#17151c' ? '#eee' : '#111'};font-size:10px`;
            frame.append(sprite);
            base.append(frame);
            card.append(base, label);
            sheet.append(card);
          }
      document.body.append(sheet);
    }, images);
    await page
      .locator('#alpha-review')
      .screenshot({ path: `.artifacts/visual-production/opponent-alpha-${language}.png` });
  });
}
