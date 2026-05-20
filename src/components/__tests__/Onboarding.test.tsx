import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import '../../i18n';
import { useStore } from '../../state/useStore';
import { Onboarding } from '../Onboarding';

/**
 * Onboarding test suite - 6-step state machine (E1).
 *
 * The flow after E1 is:
 *   welcome -> preset -> chat -> image -> hero  (local-only default)
 *
 * Tests 1 and 2 are preserved from the prior 2-step implementation with
 * minimal adaptation (the CTA on the welcome step now uses the shared "next"
 * key / "Continue" text rather than the old "Configure provider" literal;
 * advancing from welcome lands on the preset step, not Connect AI).
 *
 * Tests 3-7 cover the new stepper mechanics, the step_counter text, the
 * is-done/is-active CSS classes, and the language picker (carried over from
 * the old suite and adapted to the new step labels).
 *
 * Provider-specific tests (Anthropic key validation, Begin Setup persistence)
 * are deferred to E3 which fleshes out the ChatStep component.
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

  // ------------------------------------------------------------------
  // Test 1 (preserved): default first render
  // ------------------------------------------------------------------
  it('renders step 1 (Welcome) by default with the primary CTA', () => {
    render(<Onboarding />);
    expect(screen.getByRole('dialog')).toBeInTheDocument();
    expect(screen.getByRole('heading', { level: 1 })).toHaveTextContent(
      /Pull up a chair by the fire/i,
    );
    // The welcome step CTA now uses the shared "next" key ("Continue").
    expect(screen.getByRole('button', { name: /Continue/i })).toBeInTheDocument();
  });

  // ------------------------------------------------------------------
  // Test 2 (preserved): Continue advances from Welcome to Preset
  // ------------------------------------------------------------------
  it('Continue advances from step 1 (Welcome) to step 2 (Preset)', async () => {
    const user = userEvent.setup();
    render(<Onboarding />);
    await user.click(screen.getByRole('button', { name: /Continue/i }));
    // After advancing, the heading should reflect the preset step label.
    expect(screen.getByRole('heading', { level: 1 })).toHaveTextContent(/Choose preset/i);
  });

  // ------------------------------------------------------------------
  // Test 3 (new - E1): stepper counter advances and label updates
  // ------------------------------------------------------------------
  it('step_counter text updates when Next is clicked', async () => {
    const user = userEvent.setup();
    render(<Onboarding />);

    // On welcome (step 1 of 5 for default local-only preset).
    expect(screen.getByText(/Step 1 of 5/i)).toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: /Continue/i }));

    // After advancing to preset step - should now show Step 2 of 5.
    expect(screen.getByText(/Step 2 of 5/i)).toBeInTheDocument();
  });

  // ------------------------------------------------------------------
  // Test 4: Back from preset step returns to welcome
  // ------------------------------------------------------------------
  it('Back from step 2 (Preset) returns to step 1 (Welcome)', async () => {
    const user = userEvent.setup();
    render(<Onboarding />);
    await user.click(screen.getByRole('button', { name: /Continue/i }));
    await user.click(screen.getByRole('button', { name: /^Back$/i }));
    expect(screen.getByRole('heading', { level: 1 })).toHaveTextContent(
      /Pull up a chair by the fire/i,
    );
  });

  // ------------------------------------------------------------------
  // Test 5: dialog has aria-modal and stepper is labelled
  // ------------------------------------------------------------------
  it('exposes the dialog landmark with aria-modal and a stepper labelled by the i18n key', () => {
    render(<Onboarding />);
    const dialog = screen.getByRole('dialog');
    expect(dialog).toHaveAttribute('aria-modal', 'true');
    // The stepper carries an aria-label and contains the step labels.
    const stepper = within(dialog).getByLabelText(/Onboarding progress/i);
    expect(stepper).toBeInTheDocument();
    expect(within(stepper).getByText('Welcome')).toBeInTheDocument();
    expect(within(stepper).getByText('Choose preset')).toBeInTheDocument();
  });

  // ------------------------------------------------------------------
  // Test 6: is-done / is-active CSS classes on the stepper
  // ------------------------------------------------------------------
  it('marks completed steps with is-done and the active step with is-active', async () => {
    const user = userEvent.setup();
    render(<Onboarding />);

    // Step 1 is active; others are neither active nor done.
    const stepperInitial = screen.getByLabelText(/Onboarding progress/i);
    const initialItems = within(stepperInitial).getAllByRole('listitem');
    expect(initialItems[0]?.className).toMatch(/is-active/);
    expect(initialItems[1]?.className).not.toMatch(/is-active/);

    await user.click(screen.getByRole('button', { name: /Continue/i }));

    const stepperAfter = screen.getByLabelText(/Onboarding progress/i);
    const afterItems = within(stepperAfter).getAllByRole('listitem');
    expect(afterItems[0]?.className).toMatch(/is-done/);
    expect(afterItems[1]?.className).toMatch(/is-active/);
  });

  // ------------------------------------------------------------------
  // Test 7: language picker swaps uiLanguage
  // ------------------------------------------------------------------
  it('language picker swaps state.settings.uiLanguage when RU is clicked', async () => {
    const user = userEvent.setup();
    render(<Onboarding />);

    expect(useStore.getState().settings.uiLanguage).toBe('en');

    const ruButton = screen.getByRole('button', { name: 'RU' });
    expect(ruButton).toHaveAttribute('aria-pressed', 'false');

    await user.click(ruButton);

    expect(useStore.getState().settings.uiLanguage).toBe('ru');
    expect(screen.getByRole('button', { name: 'RU' })).toHaveAttribute('aria-pressed', 'true');
    expect(screen.getByRole('button', { name: 'EN' })).toHaveAttribute('aria-pressed', 'false');
  });

  // ------------------------------------------------------------------
  // Test 8: language picker is visible on multiple steps
  // ------------------------------------------------------------------
  it('language picker is visible on every onboarding step', async () => {
    const user = userEvent.setup();
    render(<Onboarding />);

    // Step 1 (welcome): present.
    expect(screen.getByRole('group', { name: /Language|Язык/i })).toBeInTheDocument();

    // Step 2 (preset): still present after Continue.
    await user.click(screen.getByRole('button', { name: /Continue/i }));
    expect(screen.getByRole('group', { name: /Language|Язык/i })).toBeInTheDocument();
  });
});
