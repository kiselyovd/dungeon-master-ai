import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import '../../../../i18n';
import type { MergedEntry } from '../../../../state/local_llm/manifest';
import { ManageDownloads } from '../ManageDownloads';

function entry(id: string, opts: Partial<MergedEntry> = {}): MergedEntry {
  return {
    id,
    hf_repo: 'org/repo',
    hf_filename: `${id}.gguf`,
    arch: 'qwen3',
    quant: 'gguf-q4_k_m',
    size_gb: 4,
    license: 'apache-2.0',
    display_name: id,
    source: 'system',
    installed: false,
    ...opts,
  };
}

describe('ManageDownloads', () => {
  it('renders one row per model', () => {
    const models = [entry('a'), entry('b'), entry('c')];
    render(<ManageDownloads models={models} onDownload={() => {}} onDelete={() => {}} />);
    expect(screen.getAllByTestId('download-row')).toHaveLength(3);
  });

  it('shows Download button when not installed', () => {
    render(<ManageDownloads models={[entry('a')]} onDownload={() => {}} onDelete={() => {}} />);
    expect(screen.getByRole('button', { name: /download/i })).toBeInTheDocument();
  });

  it('shows Delete button when installed', () => {
    render(
      <ManageDownloads
        models={[entry('a', { installed: true })]}
        onDownload={() => {}}
        onDelete={() => {}}
      />,
    );
    expect(screen.getByRole('button', { name: /delete/i })).toBeInTheDocument();
  });

  it('shows progress bar when downloading', () => {
    render(
      <ManageDownloads
        models={[entry('a', { downloadState: 'downloading', downloadProgress: 0.42 })]}
        onDownload={() => {}}
        onDelete={() => {}}
      />,
    );
    const progress = screen.getByRole('progressbar');
    expect(progress).toHaveAttribute('aria-valuenow', '42');
  });

  it('fires onDownload with model id on Download click', () => {
    const onDownload = vi.fn();
    render(<ManageDownloads models={[entry('a')]} onDownload={onDownload} onDelete={() => {}} />);
    fireEvent.click(screen.getByRole('button', { name: /download/i }));
    expect(onDownload).toHaveBeenCalledWith('a');
  });

  it('fires onDelete with model id on Delete click', () => {
    const onDelete = vi.fn();
    render(
      <ManageDownloads
        models={[entry('a', { installed: true })]}
        onDownload={() => {}}
        onDelete={onDelete}
      />,
    );
    fireEvent.click(screen.getByRole('button', { name: /delete/i }));
    expect(onDelete).toHaveBeenCalledWith('a');
  });

  // 5 new cases for C4 wiring

  it('queued status shows a progress bar', () => {
    render(
      <ManageDownloads
        models={[entry('a', { downloadState: 'queued', downloadProgress: 0 })]}
        onDownload={() => {}}
        onDelete={() => {}}
      />,
    );
    expect(screen.getByRole('progressbar')).toBeInTheDocument();
  });

  it('shows Cancel button during an active download', () => {
    render(
      <ManageDownloads
        models={[entry('a', { downloadState: 'downloading', downloadProgress: 0.5 })]}
        onDownload={() => {}}
        onDelete={() => {}}
      />,
    );
    expect(screen.getByRole('button', { name: /cancel/i })).toBeInTheDocument();
  });

  it('clicking Cancel fires onDelete with the model id', () => {
    const onDelete = vi.fn();
    render(
      <ManageDownloads
        models={[entry('a', { downloadState: 'downloading', downloadProgress: 0.3 })]}
        onDownload={() => {}}
        onDelete={onDelete}
      />,
    );
    fireEvent.click(screen.getByRole('button', { name: /cancel/i }));
    expect(onDelete).toHaveBeenCalledWith('a');
  });

  it('renders error text when downloadState is error', () => {
    render(
      <ManageDownloads
        models={[entry('a', { downloadState: 'error', errorMessage: 'network timeout' })]}
        onDownload={() => {}}
        onDelete={() => {}}
      />,
    );
    expect(screen.getByText('network timeout')).toBeInTheDocument();
  });

  it('Download button re-appears after an error', () => {
    render(
      <ManageDownloads
        models={[entry('a', { downloadState: 'error', errorMessage: 'net err' })]}
        onDownload={() => {}}
        onDelete={() => {}}
      />,
    );
    expect(screen.getByRole('button', { name: /download/i })).toBeInTheDocument();
  });
});
