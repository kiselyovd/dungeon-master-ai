import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, expect, it, vi } from 'vitest';
import i18n from '../../../i18n';
import type { CombatToken } from '../model/types';
import { WeaponAttackDialog } from '../WeaponAttackDialog';

function actor(arrows: number): CombatToken {
  return {
    id: 'hero',
    characterId: 'character',
    name: 'Mira',
    hp: 12,
    maxHp: 12,
    ac: 16,
    x: 8,
    y: 8,
    conditions: [],
    loadout: {
      default_weapon: 'shortbow',
      arrows,
      shield: false,
      quantities: { shortbow: 1 },
      attacks: [
        {
          weapon_id: 'shortbow',
          attack_bonus: 5,
          damage: { count: 1, die: 'D6', modifier: 3 },
          damage_type: 'piercing',
          reach_ft: 5,
          normal_range_ft: 80,
          long_range_ft: 320,
          ranged: true,
          thrown: false,
          ammunition: true,
          two_handed: true,
          versatile_damage: null,
          heavy_disadvantage: false,
        },
      ],
    },
  };
}
const enemy = { id: 'enemy', name: 'Foe', hp: 10, maxHp: 10, ac: 10, x: 10, y: 8, conditions: [] };
beforeEach(async () => {
  await i18n.changeLanguage('en');
});
it('confirms the selected target and weapon without client damage fields', async () => {
  const onAttack = vi.fn(async () => true);
  const onClose = vi.fn();
  render(
    <WeaponAttackDialog
      actor={actor(20)}
      targets={[enemy]}
      onAttack={onAttack}
      onClose={onClose}
    />,
  );
  fireEvent.click(screen.getByRole('button', { name: 'Confirm attack' }));
  await waitFor(() => expect(onAttack).toHaveBeenCalledWith('enemy', 'shortbow', 'default', false));
  await waitFor(() => expect(onClose).toHaveBeenCalledOnce());
});
it('disables an attack with exhausted ammunition', () => {
  render(
    <WeaponAttackDialog actor={actor(0)} targets={[enemy]} onAttack={vi.fn()} onClose={vi.fn()} />,
  );
  expect(screen.getByRole('button', { name: 'Confirm attack' })).toBeDisabled();
});
