import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import '../../i18n';
import { useStore } from '../../state/useStore';
import { SettingsModal } from '../SettingsModal';

vi.mock('../../api/settingsStore', () => ({
  saveProviders: vi.fn().mockResolvedValue(undefined),
  saveActiveProvider: vi.fn().mockResolvedValue(undefined),
  saveUiLanguage: vi.fn().mockResolvedValue(undefined),
  saveNarrationLanguage: vi.fn().mockResolvedValue(undefined),
  loadAll: vi.fn().mockResolvedValue({}),
}));

describe('SettingsModal', () => {
  beforeEach(() => {
    useStore.setState(useStore.getInitialState());
    vi.clearAllMocks();
  });

  it('does not render when closed', () => {
    render(<SettingsModal open={false} onClose={() => {}} />);
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  it('renders the provider picker and Anthropic fields by default', () => {
    render(<SettingsModal open={true} onClose={() => {}} />);
    expect(screen.getByRole('dialog')).toBeInTheDocument();
    expect(screen.getByLabelText(/Provider/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/API key/i)).toBeInTheDocument();
  });

  it('saves an Anthropic config to the store and persistence', async () => {
    const user = userEvent.setup();
    const { saveProviders, saveActiveProvider } = await import('../../api/settingsStore');
    const onClose = vi.fn();

    render(<SettingsModal open={true} onClose={onClose} />);

    const apiKeyInput = screen.getByLabelText(/API key/i);
    await user.type(apiKeyInput, 'sk-ant-xyz');

    fireEvent.click(screen.getByRole('button', { name: /Save/i }));

    await waitFor(() => {
      expect(saveProviders).toHaveBeenCalled();
      expect(saveActiveProvider).toHaveBeenCalledWith('anthropic');
    });
    const stored = useStore.getState().settings.providers.anthropic;
    expect(stored).not.toBeNull();
    expect(stored?.apiKey).toBe('sk-ant-xyz');
    expect(onClose).toHaveBeenCalled();
  });

  it('switches provider to openai-compat and saves config', async () => {
    const user = userEvent.setup();
    const { saveProviders, saveActiveProvider } = await import('../../api/settingsStore');
    render(<SettingsModal open={true} onClose={() => {}} />);

    await user.selectOptions(screen.getByLabelText(/Provider/i), 'openai-compat');
    await user.type(screen.getByLabelText(/Base URL/i), 'http://localhost:1234/v1');
    await user.type(screen.getByLabelText(/API key/i), 'sk-test');
    await user.type(screen.getByLabelText(/Model/i), 'qwen3-1.7b');

    fireEvent.click(screen.getByRole('button', { name: /Save/i }));

    await waitFor(() => {
      expect(saveActiveProvider).toHaveBeenCalledWith('openai-compat');
      expect(saveProviders).toHaveBeenCalled();
    });
    const stored = useStore.getState().settings.providers['openai-compat'];
    expect(stored).not.toBeNull();
    expect(stored?.baseUrl).toBe('http://localhost:1234/v1');
    expect(stored?.model).toBe('qwen3-1.7b');
  });

  it('blocks save and surfaces validation when api key is missing', async () => {
    const user = userEvent.setup();
    const { saveProviders } = await import('../../api/settingsStore');
    render(<SettingsModal open={true} onClose={() => {}} />);

    // Click save with empty key
    fireEvent.click(screen.getByRole('button', { name: /Save/i }));

    // Validation message appears, persistence is NOT called.
    expect(await screen.findByRole('alert')).toBeInTheDocument();
    expect(saveProviders).not.toHaveBeenCalled();

    // Now fix the key and save succeeds.
    await user.type(screen.getByLabelText(/API key/i), 'sk-ant-real');
    fireEvent.click(screen.getByRole('button', { name: /Save/i }));
    await waitFor(() => {
      expect(saveProviders).toHaveBeenCalled();
    });
  });
});
