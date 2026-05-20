import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import '../../i18n';
import { useStore } from '../../state/useStore';
import { SettingsForm } from '../SettingsForm';

/**
 * D8 (Batch D) - Settings tab decomposition.
 *
 * The Settings form went from 4 tabs to 5 by promoting the local-LLM config
 * into a standalone tab and relocating two settings groups:
 *
 *   Chat | Local LLM | Image | Video | Behavior
 *
 * These tests lock down the new tab strip, the default tab, the Local LLM
 * panel, and the two field relocations (languages -> Behavior, Replicate
 * API key -> Image).
 */

// The Local LLM tab mounts the manifest-driven ModelSelector, which loads
// `GET /local-llm/manifest` on mount. Stub the API so the test does not
// fall through to undici.
vi.mock('../../api/localLlm', () => ({
  fetchLocalLlmManifest: vi.fn(async () => ({
    system: [],
    user: [],
    installed_ids: [],
    download_states: {},
  })),
  setActiveLocalModel: vi.fn(async () => {}),
  startModelDownload: vi.fn(async () => {}),
  cancelOrDeleteModel: vi.fn(async () => {}),
  subscribeDownloadEvents: vi.fn(async () => () => {}),
}));

describe('SettingsForm D8 - 5-tab decomposition', () => {
  const originalFetch = globalThis.fetch;

  beforeEach(() => {
    useStore.setState(useStore.getInitialState());
    // The Local LLM panel mounts useLocalRuntimeStatus, which polls
    // /local/runtime/status. Stub fetch so polling does not pollute output.
    globalThis.fetch = vi.fn(
      async () => new Response(JSON.stringify({ llm: { state: 'off' }, image: { state: 'off' } })),
    ) as unknown as typeof fetch;
  });

  afterEach(() => {
    globalThis.fetch = originalFetch;
  });

  it('renders all 5 tabs in the tab strip', () => {
    render(<SettingsForm onSubmit={() => {}} />);
    const tabs = screen.getAllByRole('tab').map((t) => t.textContent);
    expect(tabs).toEqual(['Chat', 'Local LLM', 'Image', 'Video', 'Behavior']);
  });

  it('defaults to the Chat tab being selected', () => {
    render(<SettingsForm onSubmit={() => {}} />);
    expect(screen.getByRole('tab', { name: /^Chat$/i })).toHaveAttribute('aria-selected', 'true');
    // The Chat panel hosts the provider picker.
    expect(screen.getByRole('combobox', { name: /Provider/i })).toBeInTheDocument();
  });

  it('renders the Local LLM panel when the Local LLM tab is selected', async () => {
    const user = userEvent.setup();
    render(<SettingsForm onSubmit={() => {}} />);

    await user.click(screen.getByRole('tab', { name: /Local LLM/i }));

    // The LocalLlmTab content: runtime controls + VRAM strategy select.
    expect(screen.getByRole('button', { name: /start runtimes/i })).toBeInTheDocument();
    expect(screen.getByRole('combobox', { name: /VRAM strategy/i })).toBeInTheDocument();
  });

  it('shows the language selects in the Behavior panel', async () => {
    const user = userEvent.setup();
    render(<SettingsForm onSubmit={() => {}} />);

    await user.click(screen.getByRole('tab', { name: /Behavior/i }));

    expect(screen.getByRole('combobox', { name: /UI language/i })).toBeInTheDocument();
    expect(screen.getByRole('combobox', { name: /Narration language/i })).toBeInTheDocument();
  });

  it('shows the Replicate API key field in the Image panel', async () => {
    const user = userEvent.setup();
    render(<SettingsForm onSubmit={() => {}} />);

    await user.click(screen.getByRole('tab', { name: /Image/i }));

    expect(screen.getByLabelText(/Replicate API key/i)).toBeInTheDocument();
  });

  it('respects the initialTab prop', () => {
    render(<SettingsForm onSubmit={() => {}} initialTab="behavior" />);
    expect(screen.getByRole('tab', { name: /Behavior/i })).toHaveAttribute('aria-selected', 'true');
    // The Behavior panel hosts the system prompt field.
    expect(screen.getByLabelText(/System prompt/i)).toBeInTheDocument();
  });
});
