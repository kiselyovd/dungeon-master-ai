import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { SettingsModal } from '../SettingsModal';
import { useStore } from '../../state/useStore';
import '../../i18n';

vi.mock('../../api/secrets', () => ({
  getAnthropicApiKey: vi.fn().mockResolvedValue(undefined),
  setAnthropicApiKey: vi.fn().mockResolvedValue(undefined),
  getUiLanguage: vi.fn().mockResolvedValue(undefined),
  setUiLanguage: vi.fn().mockResolvedValue(undefined),
  getNarrationLanguage: vi.fn().mockResolvedValue(undefined),
  setNarrationLanguage: vi.fn().mockResolvedValue(undefined),
}));

describe('SettingsModal', () => {
  beforeEach(() => {
    useStore.setState(useStore.getInitialState());
  });

  it('does not render when closed', () => {
    render(<SettingsModal open={false} onClose={() => {}} />);
    expect(screen.queryByText(/Settings/i)).not.toBeInTheDocument();
  });

  it('renders fields when open', () => {
    render(<SettingsModal open={true} onClose={() => {}} />);
    expect(screen.getByText(/Settings/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/Anthropic API key/i)).toBeInTheDocument();
  });

  it('saves api key to store and calls secrets api', async () => {
    const { setAnthropicApiKey } = await import('../../api/secrets');
    const onClose = vi.fn();
    render(<SettingsModal open={true} onClose={onClose} />);

    const input = screen.getByLabelText(/Anthropic API key/i);
    await userEvent.type(input, 'sk-ant-xyz');
    fireEvent.click(screen.getByRole('button', { name: /Save/i }));

    await new Promise((r) => setTimeout(r, 0));
    expect(setAnthropicApiKey).toHaveBeenCalledWith('sk-ant-xyz');
    expect(useStore.getState().settings.anthropicApiKey).toBe('sk-ant-xyz');
    expect(onClose).toHaveBeenCalled();
  });
});
