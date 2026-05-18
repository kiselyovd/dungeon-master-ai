import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useHfSearchStore } from '../hfSearch';

vi.mock('../../api/hf', () => ({
  search: vi.fn(async () => [
    {
      repo_id: 'Qwen/Qwen3-4B',
      likes: 10,
      downloads: 100,
      gated: false,
      tags: ['qwen3'],
      siblings: [{ filename: 'q.gguf', size: 1_000_000_000 }],
    },
  ]),
}));

describe('useHfSearchStore', () => {
  beforeEach(() => {
    useHfSearchStore.setState({
      params: { q: '', sort: 'downloads' },
      results: [],
      loading: false,
      error: null,
    });
  });

  it('runs search and stores results', async () => {
    useHfSearchStore.setState({ params: { q: 'qwen3', sort: 'downloads' } });
    await useHfSearchStore.getState().runSearch();
    const state = useHfSearchStore.getState();
    expect(state.loading).toBe(false);
    expect(state.results).toHaveLength(1);
    expect(state.results[0]?.repo_id).toBe('Qwen/Qwen3-4B');
  });

  it('updates filter params via setParam', () => {
    useHfSearchStore.getState().setParam('arch', 'qwen3');
    expect(useHfSearchStore.getState().params.arch).toBe('qwen3');
  });
});
