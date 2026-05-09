import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import '../../i18n';
import { useStore } from '../../state/useStore';
import { SettingsForm, type SettingsSubmission } from '../SettingsForm';

/**
 * SettingsForm focuses on the local-mistralrs branch added in M4. The
 * Anthropic / OpenAI-compat happy paths are already covered by
 * SettingsModal.test.tsx; here we lock down:
 *
 * - the provider select exposes all three kinds (anthropic, openai-compat,
 *   local-mistralrs) so the user can pick local mode without the dev
 *   shortcut.
 * - selecting local-mistralrs reveals the model picker + VRAM strategy +
 *   runtime control UI mirrored from LocalModeModal.
 * - changing the model selection updates `localMode.selectedLlm`.
 * - changing the VRAM strategy updates `localMode.vramStrategy`.
 * - Save with local-mistralrs builds a `{ kind: 'local-mistralrs', ... }`
 *   submission whose `modelPath` carries the selected ModelId string.
 */

describe('SettingsForm', () => {
  const originalFetch = globalThis.fetch;

  beforeEach(() => {
    useStore.setState(useStore.getInitialState());
    // The local-mistralrs panel mounts useLocalRuntimeStatus, which polls
    // /local/runtime/status on a 5 s interval. Stub fetch so the polling
    // does not fall through to undici and pollute test output.
    globalThis.fetch = vi.fn(
      async () => new Response(JSON.stringify({ llm: { state: 'off' }, image: { state: 'off' } })),
    ) as unknown as typeof fetch;
  });

  afterEach(() => {
    globalThis.fetch = originalFetch;
  });

  it('exposes anthropic, openai-compat, and local-mistralrs in the provider select', () => {
    render(<SettingsForm onSubmit={() => {}} />);
    const select = screen.getByRole('combobox', { name: /Provider/i });
    const options = Array.from(select.querySelectorAll('option')).map((o) => o.value);
    expect(options).toEqual(['anthropic', 'openai-compat', 'local-mistralrs']);
  });

  it('reveals the local-mistralrs fields when that provider is selected', async () => {
    const user = userEvent.setup();
    render(<SettingsForm onSubmit={() => {}} />);

    await user.selectOptions(
      screen.getByRole('combobox', { name: /Provider/i }),
      'local-mistralrs',
    );

    // Model cards from the manifest render with a download button.
    expect(screen.getByText(/Qwen3\.5-4B/i)).toBeInTheDocument();
    expect(screen.getByText(/SDXL-Turbo/i)).toBeInTheDocument();
    // Runtime controls are reachable from this tab without the keyboard
    // shortcut now.
    expect(screen.getByRole('button', { name: /start runtimes/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /stop runtimes/i })).toBeInTheDocument();
    // VRAM strategy select rendered too.
    expect(screen.getByRole('combobox', { name: /VRAM strategy/i })).toBeInTheDocument();
  });

  it('updates localMode.selectedLlm when the user picks a different model', async () => {
    const user = userEvent.setup();
    // Pre-mark all LLMs as completed so the "Use" button is rendered.
    useStore.setState((s) => ({
      localMode: {
        ...s.localMode,
        downloads: {
          qwen3_5_0_8b: { state: 'completed', bytesTotal: 1 },
          qwen3_5_2b: { state: 'completed', bytesTotal: 1 },
          qwen3_5_4b: { state: 'completed', bytesTotal: 1 },
          qwen3_5_9b: { state: 'completed', bytesTotal: 1 },
          sdxl_turbo: { state: 'completed', bytesTotal: 1 },
        },
      },
    }));
    render(<SettingsForm onSubmit={() => {}} />);
    await user.selectOptions(
      screen.getByRole('combobox', { name: /Provider/i }),
      'local-mistralrs',
    );

    // Default selection is qwen3_5_4b - pick 0.8B from its card.
    const cards = screen.getAllByRole('group');
    const card = cards.find((c) => c.textContent?.includes('Qwen3.5-0.8B'));
    if (!card) throw new Error('Qwen3.5-0.8B card not found');
    const useBtn = card.querySelector('button[aria-pressed]') as HTMLButtonElement;
    await user.click(useBtn);

    expect(useStore.getState().localMode.selectedLlm).toBe('qwen3_5_0_8b');
  });

  it('updates localMode.vramStrategy when the user changes the strategy select', async () => {
    const user = userEvent.setup();
    render(<SettingsForm onSubmit={() => {}} />);
    await user.selectOptions(
      screen.getByRole('combobox', { name: /Provider/i }),
      'local-mistralrs',
    );

    await user.selectOptions(
      screen.getByRole('combobox', { name: /VRAM strategy/i }),
      'keep-both-loaded',
    );

    expect(useStore.getState().localMode.vramStrategy).toBe('keep-both-loaded');
  });

  it('builds a local-mistralrs config on save with modelPath set to the ModelId', async () => {
    const user = userEvent.setup();
    const onSubmit = vi.fn<(submission: SettingsSubmission) => void>();

    // Pre-select the 2B variant before mount. The form subscribes to the
    // localMode slice so a later mutation would also propagate, but seeding
    // up front keeps this test focused on the buildConfig output rather
    // than React subscription timing.
    useStore.getState().localMode.selectModel('qwen3_5_2b');

    render(<SettingsForm formId="f" onSubmit={onSubmit} />);
    await user.selectOptions(
      screen.getByRole('combobox', { name: /Provider/i }),
      'local-mistralrs',
    );

    fireEvent.submit(document.getElementById('f') as HTMLFormElement);

    await waitFor(() => {
      expect(onSubmit).toHaveBeenCalledTimes(1);
    });
    const submission = onSubmit.mock.calls[0]?.[0] as SettingsSubmission;
    expect(submission.provider).toEqual({
      kind: 'local-mistralrs',
      modelPath: 'qwen3_5_2b',
      contextWindow: 8192,
    });
  });
});
