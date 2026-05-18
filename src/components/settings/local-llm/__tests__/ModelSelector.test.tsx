import { render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import '../../../../i18n';
import { useLocalLlmStore } from '../../../../state/localLlm';
import { ModelSelector } from '../ModelSelector';

vi.mock('../../../../api/localLlm', () => ({
  fetchLocalLlmManifest: vi.fn(async () => ({
    system: [
      {
        id: 's1',
        hf_repo: 'org/repo',
        hf_filename: 's1.gguf',
        arch: 'qwen3',
        quant: 'gguf-q4_k_m',
        size_gb: 4,
        license: 'apache-2.0',
        display_name: 'System One',
      },
    ],
    user: [],
    installed_ids: ['s1'],
    download_states: {},
  })),
  setActiveLocalModel: vi.fn(async () => {}),
}));

describe('ModelSelector', () => {
  beforeEach(() => {
    // Reset the standalone slice between tests so the mocked fetch fires
    // fresh on every render; otherwise the second test would reuse the
    // populated state from the first.
    useLocalLlmStore.setState({
      system: [],
      user: [],
      installedIds: new Set(),
      downloadStates: new Map(),
      loading: false,
      error: null,
    });
  });

  it('loads manifest and renders active picker + cards', async () => {
    render(<ModelSelector activeId="s1" onActiveChange={() => {}} agentTurnInFlight={false} />);
    await waitFor(() => expect(screen.getByText(/Active model/i)).toBeInTheDocument());
    expect(await screen.findByText(/System One \(4 GB\)/)).toBeInTheDocument();
    expect(screen.getByText(/Manage downloads/i)).toBeInTheDocument();
    expect(screen.getByText(/Search Hugging Face/i)).toBeInTheDocument();
  });

  it('passes the agentTurnInFlight flag through to the picker (disables radios)', async () => {
    render(<ModelSelector activeId="s1" onActiveChange={() => {}} agentTurnInFlight={true} />);
    const radios = await screen.findAllByRole('radio');
    const first = radios[0] as HTMLInputElement;
    expect(first.disabled).toBe(true);
  });
});
