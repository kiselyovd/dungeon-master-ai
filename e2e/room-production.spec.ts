import { readFileSync } from 'node:fs';
import { expect, type Page, test } from '@playwright/test';
import type { RoomUpdate } from '../src/api/contracts/room';

const wire = JSON.parse(
  readFileSync(new URL('../src/api/__fixtures__/roomUpdate.json', import.meta.url), 'utf8'),
) as RoomUpdate;

import { mockTauri } from './fixtures/tauri-mock';

async function installRoomStub(page: Page, snapshot: RoomUpdate, completed = true) {
  await mockTauri(page, { onboarding_completed: completed, ui_language: 'ru' });
  await page.addInitScript((snapshot) => {
    type Internals = {
      invoke: (command: string, args?: Record<string, unknown>) => Promise<unknown>;
    };
    let current: Internals | undefined;
    const install = (value: Internals) => {
      const original = value.invoke.bind(value);
      const streams = new Map<string, () => void>();
      value.invoke = async (command, args = {}) => {
        if (command === 'recover_host_room')
          return {
            room: snapshot.state.room,
            member_id: snapshot.state.room.members.find((member) => member.role === 'host')
              ?.member_id,
            address: '127.0.0.1:4123',
            invitation: 'fixture',
          };
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
            body: (args.operation as { kind: string }).kind === 'pending_action' ? null : snapshot,
          };
        if (command === 'stream_room')
          return await new Promise<void>((resolve) => streams.set(String(args.streamId), resolve));
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
  }, snapshot);
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
  await expect(page.getByRole('heading', { name: 'Герои игроков' })).toBeVisible();
  await expect(page.getByLabel('Что делает ваш герой?')).toBeEnabled();
  await expect(page.getByText('Tara', { exact: true })).toBeVisible();
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
  await page.getByRole('button', { name: 'Покинуть комнату' }).click();
  await expect(page.getByRole('heading', { name: 'Ваши хроники' })).toBeVisible();
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
});

test('a clean guest enters co-op before model onboarding and returns to setup', async ({
  page,
}) => {
  await installRoomStub(page, wire, false);
  await page.goto('/');
  await page.getByRole('button', { name: 'Подключиться к игре' }).click();
  await expect(page.getByRole('heading', { name: 'Совместная игра' })).toBeVisible();
  await page.getByLabel('Адрес и порт организатора').fill('127.0.0.1:4123');
  await page.getByLabel('Код комнаты').fill(wire.state.room.room_id);
  await page.getByLabel('Ваше имя игрока').fill('Tara');
  await page.getByLabel('Приглашение').fill('fixture-invitation');
  await page.getByRole('button', { name: 'Подключиться' }).click();
  await expect(page.getByRole('heading', { name: 'Мастер игры · ИИ' })).toBeVisible();
  await page.getByRole('button', { name: 'Покинуть комнату' }).click();
  await expect(page.getByRole('button', { name: 'Подключиться к игре' })).toBeVisible();
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
});
