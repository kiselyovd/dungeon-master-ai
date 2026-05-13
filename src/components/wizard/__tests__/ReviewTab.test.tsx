import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import '../../../i18n';
import { useStore } from '../../../state/useStore';
import { ReviewTab } from '../ReviewTab';

const compendium = {} as never;

beforeEach(() => {
  useStore.getState().charCreation.resetDraft();
  useStore.getState().pc.setHeroClass(null);
});

describe('ReviewTab', () => {
  it('shows block warnings and disables Begin Adventure when draft incomplete', () => {
    render(<ReviewTab compendium={compendium} mode="initial" />);
    expect(screen.getByText(/pick a class first/i)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /begin/i })).toBeDisabled();
  });

  it('enables Begin Adventure when all blocking fields filled', () => {
    const s = useStore.getState().charCreation;
    s.setDraftField('classId', 'fighter');
    s.setDraftField('raceId', 'human');
    s.setDraftField('backgroundId', 'acolyte');
    s.setDraftField('abilityMethod', 'point_buy');
    render(<ReviewTab compendium={compendium} mode="initial" />);
    expect(screen.getByRole('button', { name: /begin/i })).toBeEnabled();
  });

  it('Begin Adventure writes draft into pc slice and resets', async () => {
    const s = useStore.getState().charCreation;
    s.setDraftField('classId', 'wizard');
    s.setDraftField('raceId', 'human');
    s.setDraftField('backgroundId', 'acolyte');
    s.setDraftField('abilityMethod', 'point_buy');
    s.setDraftField('name', 'Gale');
    render(<ReviewTab compendium={compendium} mode="initial" />);
    await userEvent.click(screen.getByRole('button', { name: /begin/i }));
    const pc = useStore.getState().pc;
    expect(pc.heroClass).toBe('wizard');
    expect(pc.name).toBe('Gale');
    expect(useStore.getState().charCreation.classId).toBeNull(); // reset
  });
});
