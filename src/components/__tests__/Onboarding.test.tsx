import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import i18n from '../../i18n';
import { useStore } from '../../state/useStore';
import { Onboarding } from '../Onboarding';

beforeEach(async () => {
  useStore.setState(useStore.getInitialState());
  await i18n.changeLanguage('en');
});
describe('contextual first entry', () => {
  it('offers adventure and guest entry without a global hero or model wizard', () => {
    render(<Onboarding onJoinRoom={() => {}} />);
    expect(screen.getByRole('button', { name: 'Start adventure' })).toBeVisible();
    expect(screen.getByRole('button', { name: 'Join game' })).toBeVisible();
    expect(screen.queryByText(/Step 1 of|Choose a preset|Create hero/i)).not.toBeInTheDocument();
    expect(useStore.getState().pc.heroClass).toBeNull();
  });
  it('starts campaign navigation and retains a legacy sheet and unfinished draft', async () => {
    useStore.getState().pc.applyPreset('fighter');
    useStore.getState().charCreation.setDraftField('name', 'Unfinished Mira');
    const before = useStore.getState().pc;
    const complete = vi.fn();
    render(<Onboarding onComplete={complete} onJoinRoom={() => {}} />);
    await userEvent.click(screen.getByRole('button', { name: 'Start adventure' }));
    expect(complete).toHaveBeenCalledWith('adventure');
    expect(useStore.getState().onboarding).toMatchObject({
      completed: true,
      version: 2,
      entry: 'adventure',
    });
    expect(useStore.getState().pc).toBe(before);
    expect(useStore.getState().charCreation.name).toBe('Unfinished Mira');
  });
  it('persists guest navigation without configuring a model or creating a solo hero', async () => {
    const join = vi.fn();
    const settings = useStore.getState().settings.providers;
    render(<Onboarding onJoinRoom={join} />);
    await userEvent.click(screen.getByRole('button', { name: 'Join game' }));
    expect(join).toHaveBeenCalledOnce();
    expect(useStore.getState().onboarding).toMatchObject({
      completed: true,
      version: 2,
      entry: 'guest',
    });
    expect(useStore.getState().settings.providers).toBe(settings);
    expect(useStore.getState().pc.heroClass).toBeNull();
  });
  it('keeps language selection keyboard accessible', async () => {
    render(<Onboarding onJoinRoom={() => {}} />);
    await userEvent.click(screen.getByRole('button', { name: '\u0420\u0423' }));
    expect(useStore.getState().settings.uiLanguage).toBe('ru');
  });
});
