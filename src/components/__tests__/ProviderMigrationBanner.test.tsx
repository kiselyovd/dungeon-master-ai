import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import '../../i18n';
import { useStore } from '../../state/useStore';
import { ProviderMigrationBanner } from '../ProviderMigrationBanner';

/**
 * M11 Batch D.5: the banner is the returning-user recovery path after native
 * Anthropic was removed. It must appear only when the transient
 * `providerMigrationNotice` flag is set, and the dismiss button must clear it.
 */
describe('ProviderMigrationBanner', () => {
  beforeEach(() => {
    useStore.setState(useStore.getInitialState());
  });

  function setNotice(value: boolean) {
    useStore.setState((s) => ({
      settings: { ...s.settings, providerMigrationNotice: value },
    }));
  }

  it('renders nothing when the notice flag is false', () => {
    setNotice(false);
    const { container } = render(<ProviderMigrationBanner />);
    expect(container).toBeEmptyDOMElement();
  });

  it('renders when the flag is set and dismisses on click', async () => {
    const user = userEvent.setup();
    setNotice(true);
    render(<ProviderMigrationBanner />);

    expect(screen.getByText(/no longer supported/i)).toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: /got it/i }));

    expect(useStore.getState().settings.providerMigrationNotice).toBe(false);
  });
});
