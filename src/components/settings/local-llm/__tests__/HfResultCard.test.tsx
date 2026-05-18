import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import '../../../../i18n';
import type { HfModel } from '../../../../api/hf';
import { HfResultCard } from '../HfResultCard';

function mk(opts: Partial<HfModel> = {}): HfModel {
  return {
    repo_id: 'Qwen/Qwen3-4B-Thinking-2507',
    likes: 100,
    downloads: 5000,
    gated: false,
    tags: ['qwen3', 'text-generation'],
    siblings: [{ filename: 'qwen3-4b-thinking-2507-q4_k_m.gguf', size: 4_000_000_000 }],
    ...opts,
  };
}

describe('HfResultCard', () => {
  it('renders compatible card with Download button', () => {
    render(
      <HfResultCard
        model={mk()}
        onDownload={() => {
          /* noop */
        }}
        onOpenHf={() => {
          /* noop */
        }}
      />,
    );
    expect(screen.getByText(/Qwen\/Qwen3-4B-Thinking-2507/)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /download/i })).toBeInTheDocument();
  });

  it('renders gated card with Open HF button', () => {
    const onOpenHf = vi.fn();
    render(
      <HfResultCard
        model={mk({ gated: true })}
        onDownload={() => {
          /* noop */
        }}
        onOpenHf={onOpenHf}
      />,
    );
    expect(screen.getByText(/gated|requires acceptance/i)).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: /open hf/i }));
    expect(onOpenHf).toHaveBeenCalledWith('Qwen/Qwen3-4B-Thinking-2507');
  });

  it('renders unsupported card with Add anyway button', () => {
    render(
      <HfResultCard
        model={mk({
          tags: ['falcon', 'text-generation'],
          siblings: [{ filename: 'model-q2_k.gguf' }],
        })}
        onDownload={() => {
          /* noop */
        }}
        onOpenHf={() => {
          /* noop */
        }}
      />,
    );
    expect(screen.getByText(/not officially supported/i)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /add anyway/i })).toBeInTheDocument();
  });

  it('fires onDownload with model + selected sibling filename', () => {
    const onDownload = vi.fn();
    render(
      <HfResultCard
        model={mk()}
        onDownload={onDownload}
        onOpenHf={() => {
          /* noop */
        }}
      />,
    );
    fireEvent.click(screen.getByRole('button', { name: /download/i }));
    expect(onDownload).toHaveBeenCalled();
    const call = onDownload.mock.calls[0];
    expect(call).toBeDefined();
    const filename = call?.[1];
    expect(filename).toMatch(/q4_k_m\.gguf$/);
  });
});
