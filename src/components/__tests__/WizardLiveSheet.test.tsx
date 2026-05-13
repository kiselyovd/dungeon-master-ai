import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import '../../i18n';
import { WizardLiveSheet } from '../WizardLiveSheet';
import type { LiveSheet } from '../wizard/computeLiveSheet';

const emptySheet: LiveSheet = {
  name: null,
  className: null,
  raceName: null,
  subraceName: null,
  backgroundName: null,
  subclassId: null,
  level: 1,
  hp: null,
  hpMax: null,
  ac: null,
  initiative: 0,
  speedFt: null,
  proficiencyBonus: 2,
  abilities: {
    str: { score: 10, mod: 0 },
    dex: { score: 10, mod: 0 },
    con: { score: 10, mod: 0 },
    int: { score: 10, mod: 0 },
    wis: { score: 10, mod: 0 },
    cha: { score: 10, mod: 0 },
  },
  savingThrows: {
    str: { mod: 0, proficient: false },
    dex: { mod: 0, proficient: false },
    con: { mod: 0, proficient: false },
    int: { mod: 0, proficient: false },
    wis: { mod: 0, proficient: false },
    cha: { mod: 0, proficient: false },
  },
  skills: {},
  inventoryPreview: [],
  inventoryOverflow: 0,
  spellsPreview: null,
  placeholder: 'pick_class_to_begin',
};

describe('WizardLiveSheet', () => {
  it('renders placeholder when no class chosen', () => {
    render(<WizardLiveSheet sheet={emptySheet} />);
    expect(screen.getByText(/pick a class/i)).toBeInTheDocument();
  });

  it('renders HP/AC/init when class is set', () => {
    const populated: LiveSheet = {
      ...emptySheet,
      className: 'Fighter',
      hp: 12,
      hpMax: 12,
      ac: 16,
      initiative: 1,
      speedFt: 30,
      placeholder: null,
    };
    render(<WizardLiveSheet sheet={populated} />);
    expect(screen.getByText(/12/)).toBeInTheDocument();
    expect(screen.getByText(/16/)).toBeInTheDocument();
  });

  it('shows overflow counter when inventory > 3', () => {
    const populated: LiveSheet = {
      ...emptySheet,
      className: 'Fighter',
      placeholder: null,
      inventoryPreview: [
        { id: 'a', name: 'Sword', count: 1, icon: null },
        { id: 'b', name: 'Shield', count: 1, icon: null },
        { id: 'c', name: 'Bow', count: 1, icon: null },
      ],
      inventoryOverflow: 4,
    };
    render(<WizardLiveSheet sheet={populated} />);
    expect(screen.getByText(/\+4/)).toBeInTheDocument();
  });
});
