import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import '../../i18n';
import { useStore } from '../../state/useStore';
import { Onboarding } from '../Onboarding';

/**
 * Onboarding test suite. The flow is:
 *   step 1 (Welcome) -> step 2 (Connect AI) -> finalize.
 *
 * The tests below lock down: the default first-render lands on step 1, the
 * step transitions advance/return correctly, the provider radios reveal the
 * right sub-form, validation gates step 2 -> finalize, and the final
 * "Begin Setup" button persists provider + the onboarding flag through the
 * Zustand slices. pc.heroClass is NOT set here - CharacterWizard owns that.
 *
 * postSettings hits `/settings` over fetch; we stub `globalThis.fetch` so the
 * fire-and-forget POST inside `finalize()` does not fall through to undici.
 */

describe('Onboarding', () => {
  const originalFetch = globalThis.fetch;

  beforeEach(() => {
    useStore.setState(useStore.getInitialState());
    globalThis.fetch = vi.fn(
      async () =>
        new Response(JSON.stringify({ kind: 'anthropic', model: 'claude-haiku-4-5-20251001' }), {
          status: 200,
        }),
    ) as unknown as typeof fetch;
  });

  afterEach(() => {
    globalThis.fetch = originalFetch;
  });

  it('renders step 1 (Welcome) by default with the primary CTA', () => {
    render(<Onboarding />);
    expect(screen.getByRole('dialog')).toBeInTheDocument();
    expect(screen.getByRole('heading', { level: 1 })).toHaveTextContent(
      /Pull up a chair by the fire/i,
    );
    expect(screen.getByRole('button', { name: /Configure provider/i })).toBeInTheDocument();
  });

  it('Continue advances from step 1 to step 2 (Connect AI)', async () => {
    const user = userEvent.setup();
    render(<Onboarding />);
    await user.click(screen.getByRole('button', { name: /Configure provider/i }));
    expect(screen.getByRole('heading', { level: 1 })).toHaveTextContent(/Choose a provider/i);
    expect(screen.getByRole('radio', { name: /Anthropic/i })).toBeInTheDocument();
  });

  it('Back from step 2 returns to step 1', async () => {
    const user = userEvent.setup();
    render(<Onboarding />);
    await user.click(screen.getByRole('button', { name: /Configure provider/i }));
    await user.click(screen.getByRole('button', { name: /^Back$/i }));
    expect(screen.getByRole('heading', { level: 1 })).toHaveTextContent(
      /Pull up a chair by the fire/i,
    );
  });

  it('selecting OpenAI compat reveals base URL + API key + model fields', async () => {
    const user = userEvent.setup();
    render(<Onboarding />);
    await user.click(screen.getByRole('button', { name: /Configure provider/i }));
    await user.click(screen.getByRole('radio', { name: /OpenAI compat/i }));

    expect(screen.getByLabelText(/Base URL/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/^API key/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/^Model/i)).toBeInTheDocument();
  });

  it('selecting Local shows the Settings hint instead of an inline form', async () => {
    const user = userEvent.setup();
    render(<Onboarding />);
    await user.click(screen.getByRole('button', { name: /Configure provider/i }));
    await user.click(screen.getByRole('radio', { name: /Local/i }));

    expect(screen.getByRole('note')).toHaveTextContent(/Local model setup happens in Settings/i);
    expect(screen.queryByLabelText(/API key/i)).toBeNull();
  });

  it('blocks step 2 -> finalize when the Anthropic API key is empty', async () => {
    const user = userEvent.setup();
    render(<Onboarding />);
    await user.click(screen.getByRole('button', { name: /Configure provider/i }));
    // Default choice is Anthropic; the key input is empty.
    await user.click(screen.getByRole('button', { name: /Begin Setup/i }));

    // Still on step 2 (the heading is the Step-2 title).
    expect(screen.getByRole('heading', { level: 1 })).toHaveTextContent(/Choose a provider/i);
    expect(screen.getByRole('alert')).toHaveTextContent(/required/i);
  });

  it('Begin Setup persists the provider and completion flag (heroClass NOT set here)', async () => {
    const user = userEvent.setup();
    const onComplete = vi.fn();
    render(<Onboarding onComplete={onComplete} />);

    await user.click(screen.getByRole('button', { name: /Configure provider/i }));
    // Type a valid API key so step 2 validation passes.
    await user.type(screen.getByLabelText(/^API key/i), 'sk-ant-test-1234567890');
    await user.click(screen.getByRole('button', { name: /Begin Setup/i }));

    const state = useStore.getState();
    expect(state.onboarding.completed).toBe(true);
    expect(state.settings.activeProvider).toBe('anthropic');
    expect(state.settings.providers.anthropic).not.toBeNull();
    expect(state.settings.providers.anthropic?.apiKey).toBe('sk-ant-test-1234567890');
    expect(onComplete).toHaveBeenCalledTimes(1);
    // heroClass is NOT set by onboarding - CharacterWizard owns that step.
    expect(state.pc.heroClass).toBeNull();
  });

  it('exposes the dialog landmark with aria-modal and a stepper labelled by the i18n key', () => {
    render(<Onboarding />);
    const dialog = screen.getByRole('dialog');
    expect(dialog).toHaveAttribute('aria-modal', 'true');
    // The stepper carries an aria-label and contains the two step labels.
    const stepper = within(dialog).getByLabelText(/Onboarding progress/i);
    expect(stepper).toBeInTheDocument();
    expect(within(stepper).getByText('Welcome')).toBeInTheDocument();
    expect(within(stepper).getByText('Connect AI')).toBeInTheDocument();
  });

  it('marks completed steps with is-done and the active step with is-active', async () => {
    const user = userEvent.setup();
    render(<Onboarding />);

    // Step 1 is active; the other is neither active nor done yet.
    const stepperInitial = screen.getByLabelText(/Onboarding progress/i);
    const initialItems = within(stepperInitial).getAllByRole('listitem');
    expect(initialItems[0]?.className).toMatch(/is-active/);
    expect(initialItems[1]?.className).not.toMatch(/is-active/);

    await user.click(screen.getByRole('button', { name: /Configure provider/i }));

    const stepperAfter = screen.getByLabelText(/Onboarding progress/i);
    const afterItems = within(stepperAfter).getAllByRole('listitem');
    expect(afterItems[0]?.className).toMatch(/is-done/);
    expect(afterItems[1]?.className).toMatch(/is-active/);
  });

  it('language picker swaps state.settings.uiLanguage when RU is clicked', async () => {
    const user = userEvent.setup();
    render(<Onboarding />);

    // Default UI language is 'en'; the EN pill is active and RU is not.
    expect(useStore.getState().settings.uiLanguage).toBe('en');

    const ruButton = screen.getByRole('button', { name: 'RU' });
    expect(ruButton).toHaveAttribute('aria-pressed', 'false');

    await user.click(ruButton);

    expect(useStore.getState().settings.uiLanguage).toBe('ru');
    // After swap, the RU pill flips to pressed.
    expect(screen.getByRole('button', { name: 'RU' })).toHaveAttribute('aria-pressed', 'true');
    expect(screen.getByRole('button', { name: 'EN' })).toHaveAttribute('aria-pressed', 'false');
  });

  it('language picker is visible on every onboarding step', async () => {
    const user = userEvent.setup();
    render(<Onboarding />);

    // Step 1: present.
    expect(screen.getByRole('group', { name: /Language|Язык/i })).toBeInTheDocument();

    // Step 2: still present after Continue.
    await user.click(screen.getByRole('button', { name: /Configure provider/i }));
    expect(screen.getByRole('group', { name: /Language|Язык/i })).toBeInTheDocument();
  });
});
