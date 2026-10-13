import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, expect, it, vi } from 'vitest';
import i18n from '../../../i18n';
import type { CombatToken } from '../model/types';
import { SpellCastDialog } from '../SpellCastDialog';

const hero: CombatToken = {
  id: 'hero',
  characterId: 'owner',
  name: 'Mira',
  hp: 2,
  maxHp: 20,
  ac: 16,
  x: 8,
  y: 8,
  conditions: [],
  actionAvailable: true,
  bonusAvailable: true,
  casting: {
    level: 1,
    ability_modifier: 3,
    attack_bonus: 5,
    save_dc: 13,
    prepared: ['cure-wounds', 'bless'],
    slots: { 1: { maximum: 2, remaining: 1 } },
    choices: [
      {
        id: 'cure-wounds',
        level: 1,
        range_ft: 5,
        bonus_action: false,
        healing: true,
        supported: true,
      },
      { id: 'bless', level: 1, range_ft: 0, bonus_action: false, healing: false, supported: false },
    ],
  },
};
beforeEach(async () => {
  await i18n.changeLanguage('en');
});
it('selects the prepared healing spell and self target with a remaining slot', async () => {
  const onCast = vi.fn(async () => true);
  const onClose = vi.fn();
  render(<SpellCastDialog actor={hero} targets={[hero]} onCast={onCast} onClose={onClose} />);
  fireEvent.click(screen.getByRole('button', { name: 'Confirm spell' }));
  await waitFor(() => expect(onCast).toHaveBeenCalledWith('cure-wounds', ['hero'], 1));
  await waitFor(() => expect(onClose).toHaveBeenCalledOnce());
});
it('does not submit an exhausted slot or an unsupported prepared spell', () => {
  if (!hero.casting) throw Error('fixture casting profile missing');
  render(
    <SpellCastDialog
      actor={{ ...hero, casting: { ...hero.casting, slots: { 1: { maximum: 2, remaining: 0 } } } }}
      targets={[hero]}
      onCast={vi.fn()}
      onClose={vi.fn()}
    />,
  );
  expect(screen.getByRole('button', { name: 'Confirm spell' })).toBeDisabled();
});

it('allows aid to a downed human but never offers healing as resurrection of a confirmed dead actor', () => {
  const attack = vi.fn(async () => true);
  const target = {
    ...hero,
    id: 'wounded',
    characterId: 'other',
    hp: 0,
    isDead: true,
    conditions: [],
  };
  const { rerender } = render(
    <SpellCastDialog actor={hero} targets={[target]} onCast={attack} onClose={vi.fn()} />,
  );
  expect(screen.getByRole('button', { name: 'Confirm spell' })).toBeDisabled();
  rerender(
    <SpellCastDialog
      actor={hero}
      targets={[{ ...target, isDead: false }]}
      onCast={attack}
      onClose={vi.fn()}
    />,
  );
  expect(screen.getByRole('button', { name: 'Confirm spell' })).toBeDisabled();
  fireEvent.click(screen.getByRole('combobox', { name: 'Target' }));
  fireEvent.click(screen.getByRole('option', { name: 'Mira (0/20)' }));
  expect(screen.getByRole('button', { name: 'Confirm spell' })).toBeEnabled();
});
