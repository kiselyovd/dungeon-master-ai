import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import '../../i18n';
import { useStore } from '../../state/useStore';
import { CharacterWizard } from '../CharacterWizard';

vi.mock('../../api/srd', () => ({
  fetchCompendium: vi.fn().mockResolvedValue({
    races: [],
    classes: [],
    backgrounds: [],
    spells: [],
    equipment: { weapons: [], armor: [], adventuring_gear: [] },
    feats: [],
    weapon_properties: [],
  }),
}));

beforeEach(() => {
  useStore.getState().charCreation.resetDraft();
});

describe('CharacterWizard container', () => {
  it('renders all 10 tab buttons in the strip', async () => {
    render(<CharacterWizard mode="initial" />);
    expect(await screen.findByRole('tab', { name: /class/i })).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: /race/i })).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: /background/i })).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: /abilities/i })).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: /skills/i })).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: /spells/i })).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: /equipment/i })).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: /persona/i })).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: /portrait/i })).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: /review/i })).toBeInTheDocument();
  });

  it('clicking a tab makes it active', async () => {
    render(<CharacterWizard mode="initial" />);
    const raceTab = await screen.findByRole('tab', { name: /race/i });
    await userEvent.click(raceTab);
    expect(raceTab).toHaveClass('is-active');
  });

  it('shows the live sheet placeholder by default', async () => {
    render(<CharacterWizard mode="initial" />);
    expect(await screen.findByText(/pick a class/i)).toBeInTheDocument();
  });
});
