import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import '../../i18n';
import { useStore } from '../../state/useStore';
import { SettingsModal } from '../SettingsModal';

vi.mock('../../api/providers', () => ({
  postSettings: vi.fn().mockResolvedValue({ kind: 'anthropic', default_model: 'claude-haiku' }),
  getProviders: vi.fn().mockResolvedValue({
    available: ['anthropic', 'openai-compat'],
    active: { kind: 'anthropic', default_model: 'claude-haiku' },
  }),
}));

vi.mock('../../api/agentSettings', () => ({
  postAgentSettings: vi.fn().mockResolvedValue(undefined),
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
    expect(screen.getByRole('combobox', { name: /Provider/i })).toBeInTheDocument();
    expect(screen.getByLabelText(/API key/i)).toBeInTheDocument();
  });

  it('saves an Anthropic config to the store, posts to backend, and closes', async () => {
    const user = userEvent.setup();
    const { postSettings } = await import('../../api/providers');
    const onClose = vi.fn();

    render(<SettingsModal open={true} onClose={onClose} />);

    const apiKeyInput = screen.getByLabelText(/API key/i);
    await user.type(apiKeyInput, 'sk-ant-xyz');

    fireEvent.click(screen.getByRole('button', { name: /Save/i }));

    await waitFor(() => {
      expect(postSettings).toHaveBeenCalledWith(
        expect.objectContaining({ kind: 'anthropic', apiKey: 'sk-ant-xyz' }),
      );
      expect(onClose).toHaveBeenCalled();
    });
    const stored = useStore.getState().settings.providers.anthropic;
    expect(stored).not.toBeNull();
    expect(stored?.apiKey).toBe('sk-ant-xyz');
    expect(useStore.getState().settings.activeProvider).toBe('anthropic');
  });

  it('switches provider to openai-compat and stores the config', async () => {
    const user = userEvent.setup();
    const { postSettings } = await import('../../api/providers');
    render(<SettingsModal open={true} onClose={() => {}} />);

    await user.selectOptions(screen.getByRole('combobox', { name: /Provider/i }), 'openai-compat');
    await user.type(screen.getByLabelText(/Base URL/i), 'http://localhost:1234/v1');
    await user.type(screen.getByLabelText(/API key/i), 'sk-test');
    await user.type(screen.getByLabelText(/Model/i), 'qwen3-1.7b');

    fireEvent.click(screen.getByRole('button', { name: /Save/i }));

    await waitFor(() => {
      expect(postSettings).toHaveBeenCalledWith(expect.objectContaining({ kind: 'openai-compat' }));
    });
    const stored = useStore.getState().settings.providers['openai-compat'];
    expect(stored).not.toBeNull();
    expect(stored?.baseUrl).toBe('http://localhost:1234/v1');
    expect(stored?.model).toBe('qwen3-1.7b');
    expect(useStore.getState().settings.activeProvider).toBe('openai-compat');
  });

  it('keeps the modal open and shows an inline banner when postSettings fails', async () => {
    const user = userEvent.setup();
    const { postSettings } = await import('../../api/providers');
    vi.mocked(postSettings).mockRejectedValueOnce(new Error('network down'));
    const onClose = vi.fn();
    render(<SettingsModal open={true} onClose={onClose} />);

    await user.type(screen.getByLabelText(/API key/i), 'sk-ant-xyz');
    fireEvent.click(screen.getByRole('button', { name: /Save/i }));

    const banner = await screen.findByTestId('settings-save-error');
    expect(banner).toHaveTextContent(/network down/i);
    expect(onClose).not.toHaveBeenCalled();
  });

  it('blocks save and surfaces validation when api key is missing', async () => {
    const user = userEvent.setup();
    const { postSettings } = await import('../../api/providers');
    render(<SettingsModal open={true} onClose={() => {}} />);

    // Click save with empty key
    fireEvent.click(screen.getByRole('button', { name: /Save/i }));

    // Validation message appears, backend POST is NOT called.
    expect(await screen.findByRole('alert')).toBeInTheDocument();
    expect(postSettings).not.toHaveBeenCalled();
    expect(useStore.getState().settings.providers.anthropic).toBeNull();

    // Now fix the key and save succeeds.
    await user.type(screen.getByLabelText(/API key/i), 'sk-ant-real');
    fireEvent.click(screen.getByRole('button', { name: /Save/i }));
    await waitFor(() => {
      expect(postSettings).toHaveBeenCalled();
    });
  });
});
