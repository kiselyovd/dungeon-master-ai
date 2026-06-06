import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import '../../i18n';
import { useStore } from '../../state/useStore';
import { SettingsModal } from '../SettingsModal';

vi.mock('../../api/settings', () => ({
  postSettingsV2: vi.fn().mockResolvedValue(undefined),
}));

vi.mock('../../api/providers', () => ({
  getProviders: vi.fn().mockResolvedValue({
    available: ['openai-compat', 'local-mistralrs'],
    active: { kind: 'openai-compat', default_model: 'anthropic/claude-3.5-sonnet' },
  }),
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

  it('renders the provider picker and OpenAI-compatible fields by default', () => {
    render(<SettingsModal open={true} onClose={() => {}} />);
    expect(screen.getByRole('dialog')).toBeInTheDocument();
    expect(screen.getByRole('combobox', { name: /Provider/i })).toBeInTheDocument();
    expect(screen.getByLabelText(/Base URL/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/API key/i)).toBeInTheDocument();
  });

  it('saves an OpenAI-compatible config to the store, posts to backend, and closes', async () => {
    const user = userEvent.setup();
    const { postSettingsV2 } = await import('../../api/settings');
    const onClose = vi.fn();

    render(<SettingsModal open={true} onClose={onClose} />);

    // openai-compat is the default provider: fill base URL, key, and model.
    await user.clear(screen.getByLabelText(/Base URL/i));
    await user.type(screen.getByLabelText(/Base URL/i), 'http://localhost:1234/v1');
    await user.type(screen.getByLabelText(/API key/i), 'sk-test');
    await user.type(screen.getByLabelText(/Model/i), 'qwen3-1.7b');

    fireEvent.click(screen.getByRole('button', { name: /Save/i }));

    await waitFor(() => {
      expect(postSettingsV2).toHaveBeenCalled();
      expect(onClose).toHaveBeenCalled();
    });
    const stored = useStore.getState().settings.providers['openai-compat'];
    expect(stored).not.toBeNull();
    expect(stored?.apiKey).toBe('sk-test');
    expect(stored?.baseUrl).toBe('http://localhost:1234/v1');
    expect(stored?.model).toBe('qwen3-1.7b');
    expect(useStore.getState().settings.activeProvider).toBe('openai-compat');
  });

  it('stores the openai-compat config entered in the Chat tab', async () => {
    const user = userEvent.setup();
    const { postSettingsV2 } = await import('../../api/settings');
    render(<SettingsModal open={true} onClose={() => {}} />);

    await user.clear(screen.getByLabelText(/Base URL/i));
    await user.type(screen.getByLabelText(/Base URL/i), 'http://localhost:1234/v1');
    await user.type(screen.getByLabelText(/API key/i), 'sk-test');
    await user.type(screen.getByLabelText(/Model/i), 'qwen3-1.7b');

    fireEvent.click(screen.getByRole('button', { name: /Save/i }));

    await waitFor(() => {
      expect(postSettingsV2).toHaveBeenCalled();
    });
    const stored = useStore.getState().settings.providers['openai-compat'];
    expect(stored).not.toBeNull();
    expect(stored?.baseUrl).toBe('http://localhost:1234/v1');
    expect(stored?.model).toBe('qwen3-1.7b');
    expect(useStore.getState().settings.activeProvider).toBe('openai-compat');
  });

  it('keeps the modal open and shows an inline banner when postSettingsV2 fails', async () => {
    const user = userEvent.setup();
    const { postSettingsV2 } = await import('../../api/settings');
    vi.mocked(postSettingsV2).mockRejectedValueOnce(new Error('network down'));
    const onClose = vi.fn();
    render(<SettingsModal open={true} onClose={onClose} />);

    // Base URL is prefilled to OpenRouter; fill key + model so validation passes
    // and the save actually reaches postSettingsV2 (which is mocked to reject).
    await user.type(screen.getByLabelText(/API key/i), 'sk-test');
    await user.type(screen.getByLabelText(/Model/i), 'qwen3-1.7b');
    fireEvent.click(screen.getByRole('button', { name: /Save/i }));

    const banner = await screen.findByTestId('settings-save-error');
    expect(banner).toHaveTextContent(/network down/i);
    expect(onClose).not.toHaveBeenCalled();
  });

  it('blocks save and surfaces validation when api key is missing', async () => {
    const user = userEvent.setup();
    const { postSettingsV2 } = await import('../../api/settings');
    render(<SettingsModal open={true} onClose={() => {}} />);

    // Click save with empty key (Base URL prefilled, but key + model empty).
    fireEvent.click(screen.getByRole('button', { name: /Save/i }));

    // Validation messages appear (API key + Model both required), backend POST is NOT called.
    expect((await screen.findAllByRole('alert')).length).toBeGreaterThan(0);
    expect(postSettingsV2).not.toHaveBeenCalled();
    expect(useStore.getState().settings.providers['openai-compat']).toBeNull();

    // Now fill the key + model and save succeeds.
    await user.type(screen.getByLabelText(/API key/i), 'sk-test-real');
    await user.type(screen.getByLabelText(/Model/i), 'qwen3-1.7b');
    fireEvent.click(screen.getByRole('button', { name: /Save/i }));
    await waitFor(() => {
      expect(postSettingsV2).toHaveBeenCalled();
    });
  });

  it('modal content is removed from the DOM after Cancel is clicked and animation completes', async () => {
    // Use fake timers so we can fast-forward the 280ms closing animation.
    vi.useFakeTimers();
    try {
      // Simulate a controlled parent: open=true, then open=false after onClose fires.
      let openState = true;
      const onClose = vi.fn().mockImplementation(() => {
        openState = false;
      });

      const { rerender } = render(<SettingsModal open={openState} onClose={onClose} />);
      expect(screen.getByRole('dialog')).toBeInTheDocument();

      // Click Cancel - this calls triggerClose inside the hook.
      fireEvent.click(screen.getByRole('button', { name: /Cancel/i }));

      // Advance past the 280ms animation window.
      await act(async () => {
        vi.advanceTimersByTime(300);
      });

      // onClose should have fired; re-render with open=false to match real App.tsx behaviour.
      expect(onClose).toHaveBeenCalledOnce();
      rerender(<SettingsModal open={false} onClose={onClose} />);

      expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    } finally {
      vi.useRealTimers();
    }
  });
});
