import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import '../../i18n';
import { postDiscover } from '../../api/discovery';
import { useLocalLlmStore } from '../../state/localLlm';
import { useStore } from '../../state/useStore';
import { SettingsForm, type SettingsSubmission } from '../SettingsForm';

/**
 * Seed the manifest-driven LocalLlmModelSelector store with the four Qwen
 * variants installed. In jsdom `backendUrl` never resolves (no Tauri runtime),
 * so the on-mount `loadManifest()` fetch rejects; its catch only sets
 * loading/error and does not wipe this seed, so the picker renders from it.
 */
function seedInstalledLocalLlms() {
  const sys = (id: string, display_name: string) => ({
    id,
    hf_repo: `unsloth/${id}-GGUF`,
    hf_filename: `${id}.gguf`,
    arch: 'qwen3',
    quant: 'gguf-q4_k_m',
    size_gb: 2.0,
    license: 'apache-2.0',
    display_name,
  });
  useLocalLlmStore.setState({
    system: [
      sys('qwen3.5-0.8b', 'Qwen3.5-0.8B Q4_K_M'),
      sys('qwen3.5-2b', 'Qwen3.5-2B Q4_K_M'),
      sys('qwen3.5-4b', 'Qwen3.5-4B Q4_K_M'),
      sys('qwen3.5-9b', 'Qwen3.5-9B Q4_K_M'),
    ],
    user: [],
    installedIds: new Set(['qwen3.5-0.8b', 'qwen3.5-2b', 'qwen3.5-4b', 'qwen3.5-9b']),
    downloadStates: new Map(),
  });
}

function setupFetchStub() {
  return vi.fn(
    async () => new Response(JSON.stringify({ llm: { state: 'off' }, image: { state: 'off' } })),
  ) as unknown as typeof fetch;
}

vi.mock('../../api/discovery', () => ({
  postDiscover: vi.fn(),
}));

const postDiscoverMock = vi.mocked(postDiscover);

/**
 * SettingsForm focuses on the local-mistralrs branch added in M4. The
 * Anthropic / OpenAI-compat happy paths are already covered by
 * SettingsModal.test.tsx; here we lock down:
 *
 * - the provider select exposes all three kinds (anthropic, openai-compat,
 *   local-mistralrs) so the user can pick local mode without the dev
 *   shortcut.
 * - the Local LLM tab (D8) shows the model picker + VRAM strategy +
 *   runtime control UI mirrored from LocalModeModal.
 * - changing the model selection updates `localMode.selectedLlm`.
 * - changing the VRAM strategy updates `localMode.vramStrategy`.
 * - Save with local-mistralrs builds a `{ kind: 'local-mistralrs', ... }`
 *   submission whose `modelPath` carries the selected ModelId string.
 *
 * D8 promoted the local-LLM config into its own standalone tab, so these
 * tests now navigate to the Local LLM tab instead of selecting the
 * local-mistralrs provider in the Chat tab.
 */

describe('SettingsForm', () => {
  const originalFetch = globalThis.fetch;

  beforeEach(() => {
    useStore.setState(useStore.getInitialState());
    // The manifest-driven picker store is separate from the main store; clear
    // it so a seed in one test does not leak into the next.
    useLocalLlmStore.setState({
      system: [],
      user: [],
      installedIds: new Set(),
      downloadStates: new Map(),
    });
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

  it('shows the local-mistralrs fields on the Local LLM tab', async () => {
    const user = userEvent.setup();
    seedInstalledLocalLlms();
    render(<SettingsForm onSubmit={() => {}} />);

    await user.click(screen.getByRole('tab', { name: /Local LLM/i }));

    // The manifest-driven picker renders a radio per installed model; the
    // image model card (SDXL-Turbo) still renders below it.
    expect(screen.getByRole('radio', { name: /Qwen3\.5-4B/i })).toBeInTheDocument();
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
    // The legacy LOCAL_LLMS cards were removed in M11 Batch D; the active model
    // is now picked from the manifest-driven LocalLlmModelSelector's radios.
    seedInstalledLocalLlms();
    render(<SettingsForm onSubmit={() => {}} />);
    await user.click(screen.getByRole('tab', { name: /Local LLM/i }));

    // Default selection is qwen3_5_4b - pick the 0.8B radio from the picker.
    const radio = screen.getByRole('radio', { name: /Qwen3\.5-0\.8B/i });
    await user.click(radio);

    expect(useStore.getState().localMode.selectedLlm).toBe('qwen3_5_0_8b');
  });

  it('updates localMode.vramStrategy when the user changes the strategy select', async () => {
    const user = userEvent.setup();
    render(<SettingsForm onSubmit={() => {}} />);
    await user.click(screen.getByRole('tab', { name: /Local LLM/i }));

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

describe('ImageTab license-restricted gating', () => {
  const originalFetch = globalThis.fetch;

  beforeEach(() => {
    useStore.setState(useStore.getInitialState());
    globalThis.fetch = setupFetchStub();
  });

  afterEach(() => {
    globalThis.fetch = originalFetch;
  });

  it('disables non-OSS preset radios when licenseRestrictedMode is on', async () => {
    const user = userEvent.setup();
    useStore.setState((s) => ({
      settings: { ...s.settings, licenseRestrictedMode: true },
    }));
    render(<SettingsForm onSubmit={() => {}} initialTab="image" />);

    await user.click(screen.getByRole('tab', { name: /Image/i }));

    // Fast (SAI NC) and Quality (FLUX-dev NC) should be disabled
    const fastRadio = screen.getByRole('radio', { name: /Fast/i });
    const qualityRadio = screen.getByRole('radio', { name: /Quality \(Nunchaku/i });
    expect(fastRadio).toBeDisabled();
    expect(qualityRadio).toBeDisabled();

    // Balanced (Apache 2.0) and Quality-OSS (Apache 2.0) should be enabled
    const balancedRadio = screen.getByRole('radio', { name: /Balanced/i });
    const qualityOssRadio = screen.getByRole('radio', { name: /Quality-OSS/i });
    expect(balancedRadio).not.toBeDisabled();
    expect(qualityOssRadio).not.toBeDisabled();
  });

  it('renders LicenseRestrictedBanner when active preset is non-OSS and restriction is on', async () => {
    useStore.setState((s) => ({
      settings: { ...s.settings, imagePreset: 'quality', licenseRestrictedMode: true },
    }));
    render(<SettingsForm onSubmit={() => {}} initialTab="image" />);

    const alert = screen.getByRole('alert');
    expect(alert).toHaveTextContent(/Nunchaku FLUX/);
  });

  it('does not render banner when active preset is OSS', async () => {
    useStore.setState((s) => ({
      settings: { ...s.settings, imagePreset: 'balanced', licenseRestrictedMode: true },
    }));
    render(<SettingsForm onSubmit={() => {}} initialTab="image" />);

    expect(screen.queryByRole('alert')).toBeNull();
  });
});

describe('VideoTab license-restricted gating', () => {
  const originalFetch = globalThis.fetch;

  beforeEach(() => {
    useStore.setState(useStore.getInitialState());
    globalThis.fetch = setupFetchStub();
  });

  afterEach(() => {
    globalThis.fetch = originalFetch;
  });

  it('disables enable toggle when LTX is non-OSS and restriction is on', async () => {
    useStore.setState((s) => ({
      settings: { ...s.settings, licenseRestrictedMode: true },
    }));
    render(<SettingsForm onSubmit={() => {}} initialTab="video" />);

    const enableCheckbox = screen.getByRole('checkbox', { name: /Enable video generation/i });
    expect(enableCheckbox).toBeDisabled();
  });
});

describe('SettingsForm model discovery integration', () => {
  beforeEach(() => {
    useStore.setState(useStore.getInitialState());
    postDiscoverMock.mockReset();
    globalThis.fetch = vi.fn(
      async () => new Response(JSON.stringify({ llm: { state: 'off' }, image: { state: 'off' } })),
    ) as unknown as typeof fetch;
  });

  it('renders a ModelSelector inside the Anthropic sub-form (Discover button visible)', () => {
    render(<SettingsForm onSubmit={() => {}} />);
    // Default provider is anthropic - Discover button should be visible.
    expect(screen.getByRole('button', { name: /discover models/i })).toBeInTheDocument();
  });

  it('clicking Discover triggers postDiscover for the anthropic provider', async () => {
    postDiscoverMock.mockResolvedValueOnce({
      models: [
        {
          model_id: 'claude-opus-4-7',
          display_name: 'Claude Opus 4.7',
          capabilities: {
            vision_input: true,
            reasoning: true,
            tool_calls: true,
            streaming: true,
          },
          source: 'curated',
          context_length: 1_000_000,
        },
      ],
      cached_at: new Date().toISOString(),
      source: 'curated',
      next_cursor: null,
    });

    render(<SettingsForm onSubmit={() => {}} />);
    fireEvent.click(screen.getByRole('button', { name: /discover models/i }));
    await waitFor(() => expect(postDiscoverMock).toHaveBeenCalled());
    const call = postDiscoverMock.mock.calls[0]?.[0] as { provider_id: string };
    expect(call.provider_id).toBe('anthropic');
  });

  it('clicking a discovered model row updates the model draft', async () => {
    postDiscoverMock.mockResolvedValueOnce({
      models: [
        {
          model_id: 'claude-haiku-4-5-20251001',
          display_name: 'Claude Haiku 4.5',
          capabilities: {
            vision_input: true,
            reasoning: true,
            tool_calls: true,
            streaming: true,
          },
          source: 'curated',
          context_length: 200_000,
        },
      ],
      cached_at: new Date().toISOString(),
      source: 'curated',
      next_cursor: null,
    });

    render(<SettingsForm onSubmit={() => {}} />);
    fireEvent.click(screen.getByRole('button', { name: /discover models/i }));
    const row = await screen.findByRole('option', { name: /claude haiku 4\.5/i });
    fireEvent.click(row);

    // The free-text input is the only textbox in the Anthropic sub-form's
    // model section; its value should now reflect the picked model_id.
    const textInput = screen.getByLabelText(/model id/i);
    expect(textInput).toHaveValue('claude-haiku-4-5-20251001');
  });

  it('switching to openai-compat shows a ModelSelector backed by openai-compat discovery', async () => {
    const user = userEvent.setup();
    render(<SettingsForm onSubmit={() => {}} />);
    await user.selectOptions(screen.getByRole('combobox', { name: /Provider/i }), 'openai-compat');
    // The Discover button is still present in the openai-compat sub-form.
    expect(screen.getByRole('button', { name: /discover models/i })).toBeInTheDocument();

    postDiscoverMock.mockResolvedValueOnce({
      models: [],
      cached_at: new Date().toISOString(),
      source: 'discovered-api',
      next_cursor: null,
    });
    fireEvent.click(screen.getByRole('button', { name: /discover models/i }));
    await waitFor(() => expect(postDiscoverMock).toHaveBeenCalled());
    const call = postDiscoverMock.mock.calls[0]?.[0] as { provider_id: string };
    expect(call.provider_id).toBe('openai-compat');
  });
});
