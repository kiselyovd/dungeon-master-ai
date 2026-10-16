import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { expect, it, vi } from 'vitest';
import '../../../i18n';
import { RoomCombatPanel } from '../RoomCombatPanel';

const vtt = vi.hoisted(() => ({ render: vi.fn() }));
vi.mock('../../../components/VttCanvas', () => ({
  VttCanvas: (props: unknown) => {
    vtt.render(props);
    return <div data-testid="room-map" />;
  },
}));
vi.mock('../WeaponAttackDialog', () => ({
  WeaponAttackDialog: ({ disabled }: { disabled: boolean }) => (
    <div role="dialog">
      <input aria-label="Retained weapon" defaultValue="sword" />
      <button type="button" disabled={disabled}>
        Confirm attack
      </button>
    </div>
  ),
}));
const ownId = '11111111-1111-4111-8111-111111111111';
it('non-tactical public room scenes use the same painting presentation', () => {
  render(
    <RoomCombatPanel
      raw={null}
      map={null}
      player={player}
      disabled
      onAction={vi.fn()}
      sceneArtwork={{ assetId: 'scene.bell.town.v2', location: 'town' }}
    />,
  );
  expect(vtt.render).toHaveBeenLastCalledWith(
    expect.objectContaining({
      presentation: expect.objectContaining({
        active: false,
        map: null,
        sceneArtwork: { assetId: 'scene.bell.town.v2', location: 'town' },
      }),
    }),
  );
});
const otherId = '22222222-2222-4222-8222-222222222222';
const ownCharacter = '33333333-3333-4333-8333-333333333333';
function actor(id: string, character_id: string | null) {
  return {
    id,
    character_id,
    name: id === ownId ? 'Tara' : 'Mira',
    max_hp: 12,
    current_hp: 12,
    temp_hp: 0,
    ac: 14,
    speed_ft: 30,
    initiative_roll: 12,
    dex_mod: 2,
    conditions: [],
    budget: { action: true, bonus_action: true, reaction: true, movement_ft: 30 },
    is_dead: false,
    position: { x: 1, y: 1 },
  };
}
function projection(current = ownId) {
  return {
    schema_version: 1,
    encounter_id: '44444444-4444-4444-8444-444444444444',
    revision: 5,
    snapshot: {
      active: true,
      round: 1,
      current_combatant: current,
      initiative: [
        { id: ownId, roll: 12, dex_tiebreak: 2 },
        { id: otherId, roll: 10, dex_tiebreak: 1 },
      ],
      combatants: [
        actor(ownId, ownCharacter),
        actor(otherId, '55555555-5555-4555-8555-555555555555'),
      ],
    },
    events: [],
  };
}
const player = { characterId: ownCharacter, heroClass: 'fighter', portraitUrl: null };
it('enables only the initiating human turn and emits a server intention', async () => {
  const action = vi.fn(async () => true);
  render(
    <RoomCombatPanel
      raw={projection()}
      map={null}
      player={player}
      disabled={false}
      onAction={action}
    />,
  );
  await userEvent.setup().click(screen.getByRole('button', { name: 'End Turn' }));
  expect(action).toHaveBeenCalledWith('44444444-4444-4444-8444-444444444444', 5, {
    kind: 'end_turn',
  });
  expect(vtt.render.mock.calls.at(-1)?.[0].presentation.movableTokenId).toBe(ownId);
});
it('disables other player turns and all actions during disconnected/pending state', () => {
  const action = vi.fn(async () => true);
  const { rerender } = render(
    <RoomCombatPanel
      raw={projection(otherId)}
      map={null}
      player={player}
      disabled={false}
      onAction={action}
    />,
  );
  expect(screen.getByRole('button', { name: 'End Turn' })).toBeDisabled();
  expect(vtt.render.mock.calls.at(-1)?.[0].presentation.movableTokenId).toBeNull();
  rerender(
    <RoomCombatPanel raw={projection()} map={null} player={player} disabled onAction={action} />,
  );
  expect(screen.getByRole('button', { name: 'End Turn' })).toBeDisabled();
  expect(action).not.toHaveBeenCalled();
});

it('retains the attack dialog across temporary pending and closes on authoritative scope change', async () => {
  const action = vi.fn(async () => false);
  const { rerender } = render(
    <RoomCombatPanel
      raw={projection()}
      map={null}
      player={player}
      disabled={false}
      onAction={action}
    />,
  );
  await userEvent.setup().click(screen.getByRole('button', { name: 'Attack' }));
  const input = screen.getByLabelText('Retained weapon');
  await userEvent.setup().clear(input);
  await userEvent.setup().type(input, 'chosen weapon');
  rerender(
    <RoomCombatPanel raw={projection()} map={null} player={player} disabled onAction={action} />,
  );
  expect(screen.getByLabelText('Retained weapon')).toBe(input);
  expect(input).toHaveValue('chosen weapon');
  expect(screen.getByRole('button', { name: 'Confirm attack' })).toBeDisabled();
  rerender(
    <RoomCombatPanel
      raw={projection()}
      map={null}
      player={player}
      disabled={false}
      onAction={action}
    />,
  );
  expect(screen.getByLabelText('Retained weapon')).toBe(input);
  const changed = projection();
  changed.revision++;
  rerender(
    <RoomCombatPanel raw={changed} map={null} player={player} disabled={false} onAction={action} />,
  );
  expect(screen.queryByRole('dialog')).toBeNull();
});
