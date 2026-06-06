import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import '../../i18n';
import { fetchCompendium } from '../../api/srd';
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

describe('CharacterWizard navigation footer', () => {
  it('Back button is disabled when the active tab is the first tab (class)', async () => {
    render(<CharacterWizard mode="initial" />);
    const backBtn = await screen.findByRole('button', { name: /back/i });
    expect(backBtn).toBeDisabled();
  });

  it('Next button is disabled when the current tab is invalid (classId is null)', async () => {
    // Default draft has classId: null, so the class tab is invalid
    render(<CharacterWizard mode="initial" />);
    const nextBtn = await screen.findByRole('button', { name: /next/i });
    expect(nextBtn).toBeDisabled();
  });

  it('Next button advances to the next tab when the current tab is valid', async () => {
    // Set classId so the class tab becomes valid
    useStore.getState().charCreation.setDraftField('classId', 'wizard');
    render(<CharacterWizard mode="initial" />);
    const nextBtn = await screen.findByRole('button', { name: /next/i });
    await userEvent.click(nextBtn);
    expect(useStore.getState().charCreation.activeTab).toBe('race');
  });

  it('Next button is not rendered on the review tab', async () => {
    useStore.getState().charCreation.setActiveTab('review');
    render(<CharacterWizard mode="initial" />);
    // Wait for component to settle
    await screen.findByRole('button', { name: /back/i });
    expect(screen.queryByRole('button', { name: /next/i })).toBeNull();
  });

  it('tab strip renders ordinal numbers for each tab', async () => {
    render(<CharacterWizard mode="initial" />);
    const tabs = await screen.findAllByRole('tab');
    expect(tabs.length).toBe(10);
    // First tab: accessible name starts with "1" (not "10") and includes "Class"
    expect(tabs[0]).toHaveAccessibleName(/^1[^0-9]/);
    expect(tabs[0]).toHaveAccessibleName(/class/i);
    // Last tab: accessible name starts with "10" and includes "Review"
    expect(tabs[9]).toHaveAccessibleName(/^10/);
    expect(tabs[9]).toHaveAccessibleName(/review/i);
  });
});

describe('CharacterWizard compendium load failure (E5)', () => {
  it('shows an error + retry when the compendium fetch fails, then recovers on retry', async () => {
    const okCompendium = {
      races: [],
      classes: [],
      backgrounds: [],
      spells: [],
      equipment: { weapons: [], armor: [], adventuring_gear: [] },
      feats: [],
      weapon_properties: [],
    };
    vi.mocked(fetchCompendium)
      .mockRejectedValueOnce(new Error('SRD fetch /srd/races failed: 500'))
      .mockResolvedValueOnce(okCompendium);

    render(<CharacterWizard mode="initial" />);

    // Error panel + retry button appear instead of a blank wizard.
    const retry = await screen.findByRole('button', { name: /retry/i });
    expect(screen.getByRole('alert')).toBeInTheDocument();

    await userEvent.click(retry);

    // After a successful retry the compendium-gated content renders (the live
    // sheet placeholder only shows once the compendium loaded) and the error
    // alert is gone.
    expect(await screen.findByText(/pick a class/i)).toBeInTheDocument();
    expect(screen.queryByRole('alert')).toBeNull();
  });
});
