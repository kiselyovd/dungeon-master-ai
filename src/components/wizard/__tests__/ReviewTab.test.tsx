import { act, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import '../../../i18n';
import type { Compendium } from '../../../api/srd';
import { useStore } from '../../../state/useStore';
import { ReviewTab } from '../ReviewTab';

const compendium: Compendium = {
  races: [],
  classes: [],
  backgrounds: [
    {
      id: 'acolyte',
      name_en: 'Acolyte',
      name_ru: '',
      skill_proficiencies: [],
      tool_proficiencies: [],
      language_proficiencies: {},
      starting_equipment: ['a holy symbol', 'vestments'],
      starting_gold: 15,
      feature: { name_en: '', name_ru: '', description: '' },
      suggested_characteristics: {},
    },
  ],
  spells: [],
  equipment: {
    weapons: [
      {
        id: 'longsword',
        name_en: 'Longsword',
        name_ru: '',
        category: 'martial_melee',
        cost: { gp: 15 },
        damage: { dice: '1d8', type: 'slashing' },
        weight_lb: 3,
        properties: [],
        range_ft: {},
        source_url: '',
        srd_section: '',
      },
    ],
    armor: [],
    adventuring_gear: [],
  },
  feats: [],
  weapon_properties: [],
} as unknown as Compendium;

const commitForView = async (character: Record<string, unknown>) => {
  useStore.getState().pc.replaceFromDraft(character as Partial<import('../../../state/pc').PcData>);
};

beforeEach(() => {
  useStore.getState().charCreation.resetDraft();
  useStore.getState().pc.setHeroClass(null);
});

describe('ReviewTab', () => {
  it('keeps an oversized portrait in the draft and explains how to recover', async () => {
    const draft = useStore.getState().charCreation;
    draft.setDraftField('classId', 'fighter');
    draft.setDraftField('raceId', 'human');
    draft.setDraftField('backgroundId', 'acolyte');
    draft.setDraftField('abilityMethod', 'standard_array');
    const portrait = `data:image/png;base64,${'A'.repeat(8 * 1024 * 1024)}`;
    draft.setDraftField('portraitUrl', portrait);
    const commit = vi.fn();
    render(<ReviewTab compendium={compendium} mode="initial" onCommitCharacter={commit} />);
    await userEvent.click(screen.getByRole('button', { name: /begin/i }));
    expect(screen.getByRole('alert')).toHaveTextContent(/storage limit/i);
    expect(commit).not.toHaveBeenCalled();
    expect(useStore.getState().charCreation.portraitUrl).toBe(portrait);
  });
  it('preserves the complete draft when server confirmation fails', async () => {
    const draft = useStore.getState().charCreation;
    draft.setDraftField('classId', 'fighter');
    draft.setDraftField('raceId', 'human');
    draft.setDraftField('backgroundId', 'acolyte');
    draft.setDraftField('abilityMethod', 'standard_array');
    draft.setDraftField('name', 'Mira');
    draft.setDraftField('backstory', 'The ferry debt');
    render(
      <ReviewTab
        compendium={compendium}
        mode="initial"
        onCommitCharacter={vi.fn(async () => {
          throw new Error('offline');
        })}
      />,
    );
    await userEvent.click(screen.getByRole('button', { name: /begin/i }));
    expect(screen.getByRole('alert')).toHaveTextContent(/could not be saved/i);
    expect(useStore.getState().charCreation.backstory).toBe('The ferry debt');
    expect(useStore.getState().pc.heroClass).toBeNull();
  });
  it('keeps the draft and old sheet until the server confirms the character', async () => {
    const draft = useStore.getState().charCreation;
    draft.setDraftField('classId', 'fighter');
    draft.setDraftField('raceId', 'human');
    draft.setDraftField('backgroundId', 'acolyte');
    draft.setDraftField('abilityMethod', 'standard_array');
    draft.setDraftField('name', 'Mira');
    draft.setDraftField('backstory', 'The ferry debt');
    let finish: () => void = () => {};
    const pending = new Promise<void>((resolve) => {
      finish = resolve;
    });
    const commit = vi.fn(async () => pending);
    render(<ReviewTab compendium={compendium} mode="initial" onCommitCharacter={commit} />);
    await userEvent.click(screen.getByRole('button', { name: /begin/i }));
    expect(useStore.getState().charCreation.name).toBe('Mira');
    expect(useStore.getState().pc.heroClass).toBeNull();
    expect(commit).toHaveBeenCalledWith(expect.objectContaining({ backstory: 'The ferry debt' }));
    await act(async () => {
      finish();
      await pending;
    });
    expect(useStore.getState().charCreation.name).toBe('');
  });
  it('shows block warnings and disables Begin Adventure when draft incomplete', () => {
    render(<ReviewTab compendium={compendium} mode="initial" onCommitCharacter={commitForView} />);
    expect(screen.getByText(/pick a class first/i)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /begin/i })).toBeDisabled();
  });

  it('enables Begin Adventure when all blocking fields filled', () => {
    const s = useStore.getState().charCreation;
    s.setDraftField('classId', 'fighter');
    s.setDraftField('raceId', 'human');
    s.setDraftField('backgroundId', 'acolyte');
    s.setDraftField('abilityMethod', 'point_buy');
    render(<ReviewTab compendium={compendium} mode="initial" onCommitCharacter={commitForView} />);
    expect(screen.getByRole('button', { name: /begin/i })).toBeEnabled();
  });

  it('Begin Adventure writes draft into pc slice and resets', async () => {
    const s = useStore.getState().charCreation;
    s.setDraftField('classId', 'wizard');
    s.setDraftField('raceId', 'human');
    s.setDraftField('backgroundId', 'acolyte');
    s.setDraftField('abilityMethod', 'point_buy');
    s.setDraftField('name', 'Gale');
    render(<ReviewTab compendium={compendium} mode="initial" onCommitCharacter={commitForView} />);
    await userEvent.click(screen.getByRole('button', { name: /begin/i }));
    const pc = useStore.getState().pc;
    expect(pc.heroClass).toBe('wizard');
    expect(pc.name).toBe('Gale');
    expect(useStore.getState().charCreation.classId).toBeNull();
  });

  it('Package mode end-to-end: concrete + wildcard-resolved slot + bg items + gold row', async () => {
    const s = useStore.getState().charCreation;
    s.setDraftField('classId', 'fighter');
    s.setDraftField('raceId', 'human');
    s.setDraftField('backgroundId', 'acolyte');
    s.setDraftField('abilityMethod', 'point_buy');
    s.setDraftField('equipmentMode', 'package');
    s.setDraftField('equipmentSlots', [
      {
        slotId: 'class-0',
        category: 'gear',
        itemId: 'a',
        customName: 'longsword',
        fromBackground: false,
        resolvedItemIds: [],
      },
      {
        slotId: 'class-1',
        category: 'gear',
        itemId: 'b',
        customName: 'any martial melee weapon',
        fromBackground: false,
        resolvedItemIds: ['longsword'],
      },
    ]);
    render(<ReviewTab compendium={compendium} mode="initial" onCommitCharacter={commitForView} />);
    await userEvent.click(screen.getByRole('button', { name: /begin/i }));
    const pc = useStore.getState().pc;
    const longsword = pc.inventory.find((it) => it.id === 'longsword');
    expect(longsword?.count).toBe(2);
    const gold = pc.inventory.find((it) => it.id === 'gold');
    expect(gold?.count).toBe(15);
    const vestments = pc.inventory.find((it) => /vestments/i.test(it.name));
    expect(vestments).toBeDefined();
  });

  it('Gold mode end-to-end: equipmentInventory + residual gold row', async () => {
    const s = useStore.getState().charCreation;
    s.setDraftField('classId', 'fighter');
    s.setDraftField('raceId', 'human');
    s.setDraftField('backgroundId', 'acolyte');
    s.setDraftField('abilityMethod', 'point_buy');
    s.setDraftField('equipmentMode', 'gold');
    s.setDraftField('equipmentInventory', [
      { id: 'longsword', name: 'Longsword', count: 1, icon: 'weapon' },
    ]);
    s.setDraftField('goldRemaining', 12.5);
    render(<ReviewTab compendium={compendium} mode="initial" onCommitCharacter={commitForView} />);
    await userEvent.click(screen.getByRole('button', { name: /begin/i }));
    const pc = useStore.getState().pc;
    const longsword = pc.inventory.find((it) => it.id === 'longsword');
    expect(longsword?.count).toBe(1);
    expect(longsword?.icon).toBe('sword');
    const gold = pc.inventory.find((it) => it.id === 'gold');
    expect(gold?.count).toBe(12);
  });

  it('edit mode: clicking Replace opens DmConfirmModal instead of window.confirm', async () => {
    const s = useStore.getState().charCreation;
    s.setDraftField('classId', 'fighter');
    s.setDraftField('raceId', 'human');
    s.setDraftField('backgroundId', 'acolyte');
    s.setDraftField('abilityMethod', 'point_buy');
    const confirmSpy = vi.spyOn(window, 'confirm');
    render(<ReviewTab compendium={compendium} mode="edit" onCommitCharacter={commitForView} />);
    await userEvent.click(screen.getByRole('button', { name: /replace/i }));
    // The modal should be open
    expect(screen.getByRole('dialog')).toBeInTheDocument();
    // window.confirm must NOT have been called
    expect(confirmSpy).not.toHaveBeenCalled();
    confirmSpy.mockRestore();
  });

  it('edit mode: cancelling the DmConfirmModal does NOT run replaceFromDraft and closes the modal', async () => {
    const s = useStore.getState().charCreation;
    s.setDraftField('classId', 'fighter');
    s.setDraftField('raceId', 'human');
    s.setDraftField('backgroundId', 'acolyte');
    s.setDraftField('abilityMethod', 'point_buy');
    const replaceFromDraftSpy = vi.spyOn(useStore.getState().pc, 'replaceFromDraft');
    render(<ReviewTab compendium={compendium} mode="edit" onCommitCharacter={commitForView} />);
    await userEvent.click(screen.getByRole('button', { name: /replace/i }));
    // DmConfirmModal should be open
    expect(screen.getByRole('dialog')).toBeInTheDocument();
    // Click Cancel
    await userEvent.click(screen.getByRole('button', { name: /^cancel$/i }));
    // replaceFromDraft must NOT have been called
    expect(replaceFromDraftSpy).not.toHaveBeenCalled();
    // The DmConfirmModal (rendered via Modal) must be gone
    expect(screen.queryByRole('dialog')).toBeNull();
    replaceFromDraftSpy.mockRestore();
  });

  it('edit mode: confirming the modal runs replaceFromDraft', async () => {
    const s = useStore.getState().charCreation;
    s.setDraftField('classId', 'paladin');
    s.setDraftField('raceId', 'human');
    s.setDraftField('backgroundId', 'acolyte');
    s.setDraftField('abilityMethod', 'point_buy');
    s.setDraftField('name', 'Arthas');
    render(<ReviewTab compendium={compendium} mode="edit" onCommitCharacter={commitForView} />);
    await userEvent.click(screen.getByRole('button', { name: /replace/i }));
    await userEvent.click(screen.getByRole('button', { name: /^confirm$/i }));
    expect(useStore.getState().pc.heroClass).toBe('paladin');
    expect(useStore.getState().pc.name).toBe('Arthas');
  });

  it('shows unresolved wildcard warning when Package mode has unresolved chunks', () => {
    const s = useStore.getState().charCreation;
    s.setDraftField('classId', 'fighter');
    s.setDraftField('raceId', 'human');
    s.setDraftField('backgroundId', 'acolyte');
    s.setDraftField('abilityMethod', 'point_buy');
    s.setDraftField('equipmentMode', 'package');
    s.setDraftField('equipmentSlots', [
      {
        slotId: 'class-0',
        category: 'gear',
        itemId: 'b',
        customName: 'any martial melee weapon',
        fromBackground: false,
        resolvedItemIds: [],
      },
    ]);
    render(<ReviewTab compendium={compendium} mode="initial" onCommitCharacter={commitForView} />);
    expect(screen.getByText(/some equipment choices are unresolved/i)).toBeInTheDocument();
  });
});

it('confirms a room hero without completing local model onboarding or changing providers', async () => {
  useStore.setState((state) => ({ onboarding: { ...state.onboarding, completed: false } }));
  const providers = structuredClone(useStore.getState().settings.providers);
  const draft = useStore.getState().charCreation;
  draft.setDraftField('classId', 'fighter');
  draft.setDraftField('raceId', 'human');
  draft.setDraftField('backgroundId', 'acolyte');
  draft.setDraftField('abilityMethod', 'standard_array');
  draft.setDraftField('name', 'Guest hero');
  const submit = vi.fn(async () => {});
  const close = vi.fn();
  render(
    <ReviewTab
      compendium={compendium}
      mode="initial"
      completeSoloOnboarding={false}
      onCommitCharacter={submit}
      onClose={close}
    />,
  );
  await userEvent.click(screen.getByRole('button', { name: /begin/i }));
  expect(submit).toHaveBeenCalledOnce();
  expect(close).toHaveBeenCalledOnce();
  expect(useStore.getState().onboarding.completed).toBe(false);
  expect(useStore.getState().settings.providers).toEqual(providers);
});
