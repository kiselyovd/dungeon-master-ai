import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import '../../../../i18n';
import { useHfSearchStore } from '../../../../state/hfSearch';
import { HfSearch } from '../HfSearch';

vi.mock('../../../../api/hf', () => ({
  search: vi.fn(async () => [
    {
      repo_id: 'Qwen/Qwen3-4B',
      likes: 10,
      downloads: 100,
      gated: false,
      tags: ['qwen3'],
      siblings: [{ filename: 'qwen3-4b-q4_k_m.gguf', size: 4_000_000_000 }],
    },
  ]),
  getTokenStatus: vi.fn(async () => ({ connected: false })),
  setToken: vi.fn(),
  clearToken: vi.fn(),
  checkLicense: vi.fn(async () => ({ gated: false, accepted: true })),
  addManifest: vi.fn(async () => {
    /* noop */
  }),
}));

vi.mock('@tauri-apps/plugin-opener', () => ({
  openUrl: vi.fn(async () => {
    /* noop */
  }),
}));

describe('HfSearch', () => {
  beforeEach(() => {
    useHfSearchStore.setState({
      params: { q: '', sort: 'downloads' },
      results: [],
      loading: false,
      error: null,
    });
  });

  it('renders the token row + search bar + filters', async () => {
    render(<HfSearch />);
    expect(await screen.findByLabelText(/architecture filter/i)).toBeInTheDocument();
    expect(screen.getByRole('searchbox')).toBeInTheDocument();
  });

  it('submits search and renders result cards', async () => {
    render(<HfSearch />);
    fireEvent.change(screen.getByRole('searchbox'), { target: { value: 'qwen3' } });
    fireEvent.click(screen.getByRole('button', { name: /^search$/i }));
    await waitFor(() => expect(screen.getByText(/Qwen\/Qwen3-4B/)).toBeInTheDocument());
  });
});
