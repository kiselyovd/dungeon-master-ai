import { readFileSync } from 'node:fs';
import { expect, type Page, test } from '@playwright/test';
import type { RoomUpdate } from '../src/api/contracts/room';
import type { CombatProjectionDto } from '../src/features/combat/model/types';

const wire = JSON.parse(
  readFileSync(new URL('../src/api/__fixtures__/roomUpdate.json', import.meta.url), 'utf8'),
) as RoomUpdate;

import { mockTauri } from './fixtures/tauri-mock';

async function installRoomStub(
  page: Page,
  snapshot: RoomUpdate,
  completed = true,
  recoveryStages?: { aid: RoomUpdate; wait: RoomUpdate },
) {
  await mockTauri(page, { onboarding_completed: completed, ui_language: 'ru' });
  await page.addInitScript(
    ({ snapshot, recoveryStages }) => {
      type Internals = {
        invoke: (command: string, args?: Record<string, unknown>) => Promise<unknown>;
      };
      let current: Internals | undefined;
      const install = (value: Internals) => {
        const original = value.invoke.bind(value);
        const streams = new Map<string, () => void>();
        const outboxKey = `dmai-e2e-room-outbox:${snapshot.state.room.room_id}`;
        value.invoke = async (command, args = {}) => {
          const operation = args.operation as Record<string, unknown> | undefined;
          if (command === 'stream_room' && operation?.kind === 'dm_turn') {
            window.localStorage.setItem(outboxKey, JSON.stringify(operation));
            const channel = args.onChunk as { onmessage: (packet: { bytes: number[] }) => void };
            const onMessage = channel.onmessage.bind(channel);
            const decoder = new TextDecoder();
            let received = '';
            channel.onmessage = (packet) => {
              received += decoder.decode(Uint8Array.from(packet.bytes), { stream: true });
              let boundary = received.indexOf('\n\n');
              while (boundary !== -1) {
                const frame = received.slice(0, boundary);
                received = received.slice(boundary + 2);
                const event = frame.match(/^event: (.+)$/m)?.[1];
                const data = frame.match(/^data: (.+)$/m)?.[1];
                if (data && (event === 'agent_done' || event === 'agent_needs_input')) {
                  const payload = JSON.parse(data) as Record<string, unknown>;
                  if (event === 'agent_done') window.localStorage.removeItem(outboxKey);
                  else {
                    window.localStorage.setItem(
                      outboxKey,
                      JSON.stringify({
                        ...operation,
                        awaiting_clarification: payload.clarification_id,
                      }),
                    );
                  }
                }
                boundary = received.indexOf('\n\n');
              }
              onMessage(packet);
            };
          }
          if (command === 'recover_host_room')
            return {
              room: snapshot.state.room,
              member_id: snapshot.state.room.members.find((member) => member.role === 'host')
                ?.member_id,
              address: '127.0.0.1:4123',
              invitation: 'fixture',
            };
          if (
            command === 'room_operation' &&
            (args.operation as { kind: string }).kind === 'recovery' &&
            recoveryStages
          ) {
            const operation = args.operation as { intent: { kind: string } };
            snapshot = structuredClone(
              operation.intent.kind === 'aid' ? recoveryStages.aid : recoveryStages.wait,
            );
            const event = [...snapshot.events]
              .reverse()
              .find((event) => (event.payload as { operation?: string }).operation === 'recovery');
            return { status: 200, body: (event?.payload as { receipt: unknown }).receipt };
          }
          if (
            command === 'room_operation' &&
            (args.operation as { kind: string }).kind === 'leave'
          ) {
            (window as unknown as { __confirmedLeave?: unknown }).__confirmedLeave = args.operation;
            return {
              status: 200,
              body: {
                receipt_revision: snapshot.state.revision,
                duplicate: false,
                departure_epoch: (snapshot.state.room.epoch ?? 1) + 1,
              },
            };
          }
          if (command === 'saved_guest_rooms') return [];
          if (command === 'postpone_guest_room') return;
          if (command === 'join_room' || command === 'recover_guest_room') {
            const request = args.request as { join_id: string };
            const member = snapshot.state.room.members.find((member) => member.role === 'player');
            return {
              room: snapshot.state.room,
              member_id: member?.member_id,
              join_id:
                command === 'recover_guest_room'
                  ? '77777777-7777-4777-8777-777777777777'
                  : request.join_id,
              address: '127.0.0.1:4123',
            };
          }
          if (
            command === 'room_operation' &&
            (args.operation as { kind: string }).kind === 'combat'
          ) {
            const combat = snapshot.state.combat as {
              revision: number;
              snapshot: {
                current_combatant: string | null;
                combatants: Array<{ id: string; character_id: string | null }>;
              };
            };
            const own = String(snapshot.state.own_character?.characterId);
            combat.snapshot.current_combatant =
              combat.snapshot.combatants.find(
                (actor) => actor.character_id && actor.character_id !== own,
              )?.id ?? null;
            combat.revision++;
            snapshot.state.revision++;
            snapshot.head++;
            snapshot.cursor = snapshot.head;
            snapshot.events = [];
            return {
              status: 200,
              body: {
                projection: combat,
                duplicate: false,
                receipt_revision: combat.revision,
                receipt_events: [],
              },
            };
          }
          if (command === 'room_operation')
            return {
              status: 200,
              body:
                operation?.kind === 'agent_recovery'
                  ? null
                  : operation?.kind === 'pending_action'
                    ? JSON.parse(window.localStorage.getItem(outboxKey) ?? 'null')
                    : snapshot,
            };
          if (command === 'stream_room')
            return await new Promise<void>((resolve) =>
              streams.set(String(args.streamId), resolve),
            );
          if (command === 'cancel_room_stream') {
            streams.get(String(args.streamId))?.();
            streams.delete(String(args.streamId));
            return;
          }
          return original(command, args);
        };
        current = value;
      };
      const target = window as unknown as { __TAURI_INTERNALS__?: Internals };
      const existing = target.__TAURI_INTERNALS__;
      Object.defineProperty(window, '__TAURI_INTERNALS__', {
        configurable: true,
        get: () => current,
        set: install,
      });
      if (existing) install(existing);
    },
    { snapshot, recoveryStages },
  );
}

test('production co-op root joins a guest and preserves its own hero', async ({ page }) => {
  await installRoomStub(page, wire);
  await page.goto('/');
  await page.getByRole('button', { name: 'Совместная игра' }).click();
  await expect(page.getByRole('heading', { name: 'Совместная игра' })).toBeVisible();
  await page.getByLabel('Адрес и порт организатора').fill('127.0.0.1:4123');
  await page.getByLabel('Код комнаты').fill(wire.state.room.room_id);
  await page.getByLabel('Ваше имя игрока').fill('Tara');
  await page.getByLabel('Приглашение').fill('fixture-invitation');
  await page.getByRole('button', { name: 'Подключиться' }).click();
  await expect(page.getByRole('heading', { name: 'Мастер игры · ИИ' })).toBeVisible();
  await expect(page.getByRole('group', { name: 'Герои игроков' })).toBeVisible();
  await expect(page.locator('[data-game-table]')).toHaveAttribute('data-table-mode', 'guest');
  await expect(page.getByLabel('Что делает ваш герой?')).toBeEnabled();
  await expect(page.getByText('Tara', { exact: true })).toBeVisible();
  await page.keyboard.press('Control+k');
  const palette = page.getByRole('dialog', { name: 'Команды' });
  await expect(palette.getByRole('button', { name: /Быстрое сохранение/ })).toBeDisabled();
  await expect(palette.getByRole('button', { name: /Открыть настройки/ })).toBeDisabled();
  await expect(palette.getByRole('button', { name: /Завершить свой ход/ })).toBeDisabled();
  await expect(palette.getByRole('button', { name: /Перейти к карте/ })).toBeDisabled();
  await palette.getByRole('textbox', { name: 'Найти команду' }).fill('журнал');
  await palette.getByRole('button', { name: /Открыть журнал/ }).click();
  await expect(page.getByRole('region', { name: 'Хроника Барда' })).toBeVisible();
  await page.keyboard.press('Escape');
  await page.screenshot({
    path: '.artifacts/room-ui-production-browser.png',
    fullPage: true,
    mask: [page.locator('[data-room-invitation]')],
  });
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= document.documentElement.clientWidth,
    ),
  ).toBe(true);
  await page.getByRole('button', { name: 'Комната', exact: true }).click();
  await page.getByRole('button', { name: 'Покинуть комнату' }).click();
  expect(
    await page.evaluate(
      () => (window as unknown as { __confirmedLeave?: { kind: string } }).__confirmedLeave?.kind,
    ),
  ).toBe('leave');
  await expect(page.getByRole('heading', { name: 'Ваши приключения' })).toBeVisible();
});

test('production room battle renders actual server map and gates the next human turn', async ({
  page,
}) => {
  const snapshot = JSON.parse(
    readFileSync(new URL('../src/api/__fixtures__/roomCombatUpdate.json', import.meta.url), 'utf8'),
  ) as RoomUpdate;
  await installRoomStub(page, snapshot);
  await page.goto('/');
  await page.getByRole('button', { name: 'Совместная игра' }).click();
  await page.getByLabel('Адрес и порт организатора').fill('127.0.0.1:4123');
  await page.getByLabel('Код комнаты').fill(snapshot.state.room.room_id);
  await page.getByLabel('Ваше имя игрока').fill('Guest');
  await page.getByLabel('Приглашение').fill('fixture-invitation');
  await page.getByRole('button', { name: 'Подключиться' }).click();
  await expect(page.getByTestId('authored-map')).toBeVisible();
  await expect(page.getByLabel('Инициатива', { exact: true })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Закончить ход' })).toBeEnabled();
  await page.screenshot({
    path: '.artifacts/room-combat-production-browser.png',
    fullPage: true,
    mask: [page.locator('[data-room-invitation]')],
  });
  await page.getByRole('button', { name: 'Закончить ход' }).click();
  await expect(page.getByRole('button', { name: 'Закончить ход' })).toBeDisabled();
  await page.keyboard.press('Control+k');
  await expect(
    page
      .getByRole('dialog', { name: 'Команды' })
      .getByRole('button', { name: /Завершить свой ход/ }),
  ).toBeDisabled();
});

test('a clean guest enters co-op directly and returns to the campaign library', async ({
  page,
}) => {
  await installRoomStub(page, wire, false);
  await page.goto('/');
  await page.getByRole('button', { name: 'Присоединиться к игре' }).click();
  await expect(page.getByRole('heading', { name: 'Совместная игра' })).toBeVisible();
  await page.getByLabel('Адрес и порт организатора').fill('127.0.0.1:4123');
  await page.getByLabel('Код комнаты').fill(wire.state.room.room_id);
  await page.getByLabel('Ваше имя игрока').fill('Tara');
  await page.getByLabel('Приглашение').fill('fixture-invitation');
  await page.getByRole('button', { name: 'Подключиться' }).click();
  await expect(page.getByRole('heading', { name: 'Мастер игры · ИИ' })).toBeVisible();
  await page.getByRole('button', { name: 'Комната', exact: true }).click();
  await page.getByRole('button', { name: 'Покинуть комнату' }).click();
  await expect(page.getByRole('heading', { name: 'Ваши приключения' })).toBeVisible();
});

test('guest restores its native selection without entering invitation again', async ({ page }) => {
  await installRoomStub(page, wire);
  await page.goto('/');
  await page.getByRole('button', { name: 'Совместная игра' }).click();
  await page.getByRole('button', { name: 'Восстановить подключение гостя' }).click();
  await expect(page.getByRole('heading', { name: 'Мастер игры · ИИ' })).toBeVisible();
  await expect(page.getByText('Tara', { exact: true })).toBeVisible();
});

test('host recovers a saved room from the production screen without creation input', async ({
  page,
}) => {
  await installRoomStub(page, wire);
  await page.goto('/');
  await page.getByRole('button', { name: 'Совместная игра' }).click();
  await page.getByRole('button', { name: 'Восстановить комнату', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Мастер игры · ИИ' })).toBeVisible();
  await expect(page.locator('[data-game-table]')).toHaveAttribute('data-table-mode', 'host');
  await page.getByRole('button', { name: 'Комната', exact: true }).click();
  await page.getByRole('button', { name: 'Обновить приглашение' }).click({ timeout: 5000 });
  await expect(page.getByRole('textbox', { name: 'Приглашение' })).toHaveValue('fixture');
});

test('production recovery controls aid a dying hero and confirm waiting using real engine snapshots', async ({
  page,
}) => {
  const fixture = (name: string) =>
    JSON.parse(
      readFileSync(new URL(`../src/api/__fixtures__/${name}.json`, import.meta.url), 'utf8'),
    ) as RoomUpdate;
  await installRoomStub(page, fixture('roomRecoveryUpdate'), true, {
    aid: fixture('roomRecoveryAidUpdate'),
    wait: fixture('roomRecoveryWaitUpdate'),
  });
  await page.goto('/');
  await page.getByRole('button', { name: 'Совместная игра' }).click();
  await page.getByRole('button', { name: 'Восстановить комнату', exact: true }).click();
  await page.getByRole('button', { name: 'Приключение', exact: true }).click();
  await page.getByRole('tab', { name: 'Герой', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Помощь раненым героям' })).toBeVisible();
  await page.getByRole('button', { name: 'Попытаться помочь' }).click();
  await expect(page.getByText('Tara · Стабилен, без сознания')).toBeVisible();
  await page.getByRole('button', { name: 'Дождаться восстановления' }).click();
  await expect(
    page.getByRole('dialog', { name: 'Дождаться восстановления', exact: true }),
  ).toBeVisible();
  await page
    .getByRole('dialog', { name: 'Дождаться восстановления', exact: true })
    .getByRole('button', { name: 'Дождаться восстановления' })
    .click();
  await expect(page.getByRole('heading', { name: 'Помощь раненым героям' })).not.toBeVisible();
  await expect(
    page.getByText('Tara', { exact: true }).locator('..').getByText('ОЗ 1/12 · КД 14'),
  ).toBeVisible();
});

test('production room table omits a withdrawn human without hiding the remaining turn', async ({
  page,
}) => {
  const snapshot = JSON.parse(
    readFileSync(new URL('../src/api/__fixtures__/roomCombatUpdate.json', import.meta.url), 'utf8'),
  ) as RoomUpdate;
  const hostActor = snapshot.state.actors.find((a) => a.role === 'host');
  if (!hostActor || !snapshot.state.own_character) throw new Error('host fixture required');
  const own = hostActor.character_id;
  snapshot.state.own_character.characterId = own;
  snapshot.state.actors = snapshot.state.actors.filter((a) => a.character_id === own);
  snapshot.state.room.members = snapshot.state.room.members.map((m) => ({
    ...m,
    active: m.role === 'host',
  }));
  snapshot.state.recovery = snapshot.state.recovery.filter((hero) => hero.character_id === own);
  const combat = snapshot.state.combat as CombatProjectionDto | null;
  if (!combat) throw new Error('combat fixture required');
  if (combat.campaign_scope) combat.campaign_scope.character_id = own;
  const withdrawn = combat.snapshot.combatants.find(
    (a) => a.character_id && a.character_id !== own,
  );
  if (!withdrawn) throw new Error('second human required');
  withdrawn.withdrawn = true;
  withdrawn.name = 'Departed hero';
  combat.snapshot.current_combatant =
    combat.snapshot.combatants.find((a) => a.character_id === own)?.id ?? null;
  await installRoomStub(page, snapshot);
  await page.goto('/');
  await page.getByRole('button', { name: 'Совместная игра' }).click();
  await page.getByRole('button', { name: 'Восстановить комнату' }).click();
  const initiative = page.getByLabel('Инициатива', { exact: true });
  await expect(initiative).toBeVisible();
  await expect(initiative.getByText(withdrawn.name, { exact: true })).toHaveCount(0);
  await expect(page.getByRole('button', { name: 'Закончить ход' })).toBeEnabled();
  await page.screenshot({
    path: '.artifacts/browser-session/withdrawn-room.png',
    fullPage: true,
    mask: [page.locator('[data-room-invitation]')],
  });
});

for (const role of ['host', 'guest'] as const) {
  for (const language of ['ru', 'en'] as const) {
    for (const width of [960, 1280, 1920]) {
      test(`persistent table ${role} ${language} ${width}: scene, composer, resizing and private room controls`, async ({
        page,
      }) => {
        test.setTimeout(60_000);
        const height = width === 960 ? 640 : width === 1280 ? 800 : 1080;
        await page.setViewportSize({ width, height });
        const active = JSON.parse(
          readFileSync(
            new URL('../src/api/__fixtures__/roomCombatUpdate.json', import.meta.url),
            'utf8',
          ),
        ) as RoomUpdate;
        await installRoomStub(page, active);
        await page.goto('/');
        await page.getByRole('button', { name: 'Совместная игра' }).click();
        await page
          .getByRole('button', {
            name: role === 'host' ? 'Восстановить комнату' : 'Восстановить подключение гостя',
            exact: true,
          })
          .click();
        if (language === 'en') await page.getByRole('button', { name: 'EN', exact: true }).click();
        const composer = page.getByLabel(
          language === 'en' ? 'What does your hero do?' : 'Что делает ваш герой?',
        );
        await expect(composer).toBeVisible();
        const box = await composer.boundingBox();
        expect(box && box.y + box.height).toBeLessThanOrEqual(height);
        await expect(page.locator('[data-table-layout]')).toHaveAttribute(
          'data-table-layout',
          'persistent',
        );
        await expect(page.locator('[data-table-layout]')).toHaveAttribute('data-mode', role);
        await expect(page.getByTestId('authored-map')).toBeVisible();
        expect(
          await page.evaluate(
            () =>
              document.documentElement.scrollWidth <= window.innerWidth &&
              document.documentElement.scrollHeight <= window.innerHeight,
          ),
        ).toBe(true);
        await expect(page.locator('[data-room-invitation]')).toHaveCount(0);
        const divider = page.getByRole('separator');
        const before = Number(await divider.getAttribute('aria-valuenow'));
        await divider.focus();
        await divider.press('ArrowLeft');
        await expect(divider).toHaveAttribute('aria-valuenow', String(before + 16));
        await composer.fill('Unsent scoped draft');
        const room = page.getByRole('button', {
          name: language === 'en' ? 'Room' : 'Комната',
          exact: true,
        });
        await room.click();
        await expect(
          page.getByRole('dialog', { name: language === 'en' ? 'Room' : 'Комната', exact: true }),
        ).toBeVisible();
        if (role === 'host') await expect(page.locator('[data-room-invitation]')).toBeVisible();
        await page.keyboard.press('Escape');
        await expect(page.getByRole('dialog')).toHaveCount(0);
        await expect(room).toBeFocused();
        await expect(composer).toHaveValue('Unsent scoped draft');
        await page.screenshot({
          path: `.artifacts/product-uiux/table-${role}-${language}-${width}.png`,
          animations: 'disabled',
        });
      });
    }
  }
}
test('public same-name actors retain distinct portrait-miniature pairs and cell hit targets', async ({
  page,
}) => {
  test.setTimeout(60_000);
  const snapshot = JSON.parse(
    readFileSync(new URL('../src/api/__fixtures__/roomCombatUpdate.json', import.meta.url), 'utf8'),
  ) as RoomUpdate;
  if (snapshot.state.actors.length < 2) throw Error('public actors fixture missing');
  const first = snapshot.state.actors[0];
  const second = snapshot.state.actors[1];
  if (!first || !second) throw Error('public actors missing');
  first.portrait_asset_id = 'hero.fighter.01.v2';
  second.portrait_asset_id = 'hero.rogue.legacy.v1';
  await installRoomStub(page, snapshot);
  await page.goto('/');
  await page.getByRole('button', { name: 'Совместная игра' }).click();
  await page.getByRole('button', { name: 'Восстановить подключение гостя' }).click();
  const mini = page.locator('[data-asset-id="miniature.fighter.01.v2"]');
  const other = page.locator('[data-asset-id="miniature.rogue.legacy.v1"]');
  await expect(mini).toHaveCount(1);
  await expect(other).toHaveCount(1);
  await expect
    .poll(() =>
      mini
        .locator('img')
        .evaluate((img: HTMLImageElement) => img.complete && img.naturalWidth === 512),
    )
    .toBe(true);
  await expect(mini).toHaveCSS('pointer-events', 'none');
  const cell = mini.locator('..');
  const before = await cell.boundingBox();
  const figure = await mini.boundingBox();
  expect(before?.width).toBeCloseTo(30, 0);
  expect(figure && before && figure.height > before.height).toBe(true);
  await page.screenshot({
    path: '.artifacts/product-uiux/whole-miniatures-room.png',
    animations: 'disabled',
  });
  await page.getByRole('button', { name: 'Приблизить', exact: true }).click();
  const after = await cell.boundingBox();
  expect(after?.width).toBeGreaterThan(before?.width ?? 0);
  expect(await cell.getAttribute('style')).toContain('width: 30px');
  await expect(mini).toHaveCount(1);
  await expect(other).toHaveCount(1);
  await cell.focus();
  await cell.press('Space');
  await expect(cell).toHaveAttribute('aria-pressed', 'true');
  await page.locator('.dm-init-summary').click();
  await expect(page.locator('.dm-init-list [aria-pressed="true"]')).toHaveCount(1);
  await page.locator('.dm-init-summary').click();
  // Exercise actual production cells at 48 and 64 screen pixels through the
  // existing continuous wheel zoom. The authored grid remains 30 world pixels.
  let currentSize = after?.width ?? 36;
  for (const screenSize of [48, 64]) {
    const canvas = page.locator('.dm-vtt-canvas');
    const box = await canvas.boundingBox();
    if (!box) throw Error('map viewport unavailable');
    await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
    await page.mouse.wheel(0, -Math.log(screenSize / currentSize) / Math.log(1.0015));
    await expect
      .poll(async () => Math.round((await cell.boundingBox())?.width ?? 0))
      .toBe(screenSize);
    const scaledFigure = await mini.boundingBox();
    const scaledCell = await cell.boundingBox();
    expect(scaledFigure && scaledCell && scaledFigure.height > scaledCell.height).toBe(true);
    expect(await cell.getAttribute('style')).toContain('width: 30px');
    await expect(cell).toHaveAttribute('aria-pressed', 'true');
    await page.screenshot({
      path: `.artifacts/product-uiux/whole-miniatures-room-${screenSize}.png`,
      animations: 'disabled',
    });
    currentSize = screenSize;
  }
});

for (const heroClass of ['fighter', 'rogue', 'wizard', 'cleric']) {
  for (const number of ['01', '02', '03', '04']) {
    test(`${heroClass} ${number} retains its public portrait and whole miniature at table scale`, async ({
      page,
    }) => {
      const snapshot = JSON.parse(
        readFileSync(
          new URL('../src/api/__fixtures__/roomCombatUpdate.json', import.meta.url),
          'utf8',
        ),
      ) as RoomUpdate;
      const actor = snapshot.state.actors[0];
      if (!actor) throw Error('fighter fixture actor missing');
      actor.portrait_asset_id = `hero.${heroClass}.${number}.v2`;
      await installRoomStub(page, snapshot);
      await page.goto('/');
      await page.getByRole('button', { name: 'Совместная игра' }).click();
      await page.getByRole('button', { name: 'Восстановить подключение гостя' }).click();
      const miniature = page.locator(`[data-asset-id="miniature.${heroClass}.${number}.v2"]`);
      await expect(miniature).toHaveCount(1);
      await expect
        .poll(() =>
          miniature
            .locator('img')
            .evaluate((image: HTMLImageElement) => image.complete && image.naturalWidth === 512),
        )
        .toBe(true);
      await expect(miniature).toHaveCSS('pointer-events', 'none');
      const cell = miniature.locator('..');
      let current = 30;
      for (const size of [30, 48, 64]) {
        const canvas = page.locator('.dm-vtt-canvas');
        const bounds = await canvas.boundingBox();
        if (!bounds) throw Error('viewport missing');
        await page.mouse.move(bounds.x + bounds.width / 2, bounds.y + bounds.height / 2);
        if (size !== current)
          await page.mouse.wheel(0, -Math.log(size / current) / Math.log(1.0015));
        await expect
          .poll(async () => Math.round((await cell.boundingBox())?.width ?? 0))
          .toBe(size);
        expect(await cell.getAttribute('style')).toContain('width: 30px');
        await page.screenshot({
          path: `.artifacts/visual-production/${heroClass}-${number}-dark-${size}.png`,
          animations: 'disabled',
        });
        current = size;
      }
      // Contrast specimen of the same production geometry/renderer. No cells,
      // blockers, actor positions or mechanical data are modified.
      await page.getByTestId('authored-map').evaluate((svg) => {
        (svg as SVGElement).style.setProperty('--color-bg-base', '#dbcda9');
        (svg as SVGElement).style.setProperty('--color-bg-raised', '#b7a782');
      });
      await page.screenshot({
        path: `.artifacts/visual-production/${heroClass}-${number}-light-64.png`,
        animations: 'disabled',
      });
      await expect(miniature).toHaveCount(1);
    });
  }
}

test('room narration preserves reading position and restores a scoped draft after cold reload', async ({
  page,
}) => {
  const snapshot = structuredClone(wire);
  snapshot.state.messages = Array.from({ length: 30 }, (_, index) => ({
    id: `12345678-1234-4234-8234-${String(index).padStart(12, '0')}`,
    role: index % 2 === 0 ? 'user' : 'assistant',
    text: `Публичное сообщение ${index}.\n\n${'Длинное повествование о речном городе и знакомых героях. '.repeat(12)}`,
    truncated: index === 1,
  }));
  await installRoomStub(page, snapshot);
  await page.goto('/');
  await page.getByRole('button', { name: 'Совместная игра' }).click();
  await page.getByRole('button', { name: 'Восстановить подключение гостя' }).click();
  const history = page.getByTestId('chat-history');
  await expect(history.getByText('Начало сообщения опущено.')).toBeAttached();
  await history.evaluate((node) => {
    node.scrollTop = 0;
    node.dispatchEvent(new Event('scroll'));
  });
  await expect(page.getByRole('button', { name: 'К последнему сообщению' })).toBeVisible();
  const composer = page.getByRole('textbox', { name: 'Что делает ваш герой?' });
  await composer.fill('Первая строка');
  await composer.press('Shift+Enter');
  await composer.press('a');
  const draft = await composer.inputValue();
  expect(draft).toContain('\n');
  await page.getByRole('button', { name: 'Комната', exact: true }).click();
  await page.keyboard.press('Escape');
  expect(await history.evaluate((node) => node.scrollTop)).toBe(0);
  await expect(composer).toHaveValue(draft);
  await page.getByRole('button', { name: 'К последнему сообщению' }).click();
  expect(await history.evaluate((node) => node.scrollTop)).toBeGreaterThan(0);
  await expect
    .poll(() =>
      page.evaluate(() => localStorage.getItem('dmai-e2e-store:conversation-drafts.json')),
    )
    .toContain(draft.split('\n')[0]);
  await page.reload();
  await page.getByRole('button', { name: 'Совместная игра' }).click();
  await page.getByRole('button', { name: 'Восстановить подключение гостя' }).click();
  await expect(page.getByRole('textbox', { name: 'Что делает ваш герой?' })).toHaveValue(draft);
  await page.screenshot({
    path: '.artifacts/product-uiux/room-conversation-draft.png',
    animations: 'disabled',
  });
});

for (const width of [960, 1280, 1920]) {
  test(`guest context stays public and leaves the composer reachable at ${width}px`, async ({
    page,
  }) => {
    await page.setViewportSize({ width, height: width === 960 ? 640 : 800 });
    const snapshot = structuredClone(wire);
    const campaignId = snapshot.state.room.campaign_id;
    if (!snapshot.state.own_character) throw Error('own hero fixture missing');
    snapshot.state.own_character.backstory = 'Личная история гостя';
    if (snapshot.state.adventure)
      snapshot.state.adventure.public_goals = snapshot.state.adventure.available_actions.filter(
        (action) => action.kind !== 'move' && action.kind !== 'finish',
      );
    snapshot.state.journal = [
      {
        id: 'journal-public',
        campaign_id: campaignId,
        chapter: null,
        entry_html: '<p>Подтверждённая заметка.</p>',
        created_at: '2026-10-13T00:00:00Z',
      },
    ];
    snapshot.state.npcs = [
      {
        id: 'npc-known-one',
        campaign_id: campaignId,
        name: 'Знакомая Мира',
        role: 'Innkeeper',
        disposition: 'friendly',
        trust: 10,
        facts: [{ text: 'Открытая встреча в таверне.', created_at: '2026-10-13T00:00:00Z' }],
        updated_at: '2026-10-13T00:00:00Z',
        hidden: 'НЕПУБЛИЧНЫЙ СЕКРЕТ',
      },
    ];
    await installRoomStub(page, snapshot);
    await page.goto('/');
    await page.getByRole('button', { name: 'Совместная игра' }).click();
    await page.getByRole('button', { name: 'Восстановить подключение гостя' }).click();
    const composer = page.getByRole('textbox', { name: 'Что делает ваш герой?' });
    await composer.fill('Сохранённый ввод рядом с журналом');
    const trigger = page.getByRole('button', { name: 'Приключение', exact: true });
    await trigger.click();
    const pane = page.getByRole('complementary', { name: 'Приключение', exact: true });
    await expect(pane).toBeVisible();
    await page.getByRole('tab', { name: 'Герой', exact: true }).click();
    await expect(pane.getByText('Личная история гостя')).toBeVisible();
    await expect(page.getByRole('textbox', { name: 'Что делает ваш герой?' })).toBeVisible();
    await page.getByRole('tab', { name: 'Журнал', exact: true }).click();
    await expect(pane.getByText('Подтверждённая заметка.')).toBeVisible();
    await expect(composer).toHaveValue('Сохранённый ввод рядом с журналом');
    await page.getByRole('tab', { name: 'NPC', exact: true }).click();
    await expect(pane.getByText('Знакомая Мира')).toBeVisible();
    await expect(pane.locator('[data-asset-id="npc.innkeeper.v2"]')).toHaveCount(1);
    await expect(page.getByText('НЕПУБЛИЧНЫЙ СЕКРЕТ')).toHaveCount(0);
    await page.getByRole('button', { name: 'Развернуть панель' }).click();
    await expect(page.getByRole('dialog', { name: 'NPC', exact: true })).toBeVisible();
    await page.keyboard.press('Escape');
    await expect(page.getByRole('dialog')).toHaveCount(0);
    await expect(trigger).toBeFocused();
    await expect(composer).toHaveValue('Сохранённый ввод рядом с журналом');
    expect(
      await page.evaluate(
        () =>
          document.documentElement.scrollWidth <= innerWidth &&
          document.documentElement.scrollHeight <= innerHeight,
      ),
    ).toBe(true);
    await trigger.click();
    await page.getByRole('tab', { name: 'Журнал', exact: true }).click();
    await page.screenshot({
      path: `.artifacts/product-uiux/context-guest-${width}.png`,
      animations: 'disabled',
    });
  });
}
