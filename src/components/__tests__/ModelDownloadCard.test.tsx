import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import '../../i18n';
import { ModelDownloadCard } from '../ModelDownloadCard';

describe('ModelDownloadCard', () => {
  it('renders Download button when state is idle', () => {
    render(
      <ModelDownloadCard
        modelId="qwen3_5_4b"
        displayName="Qwen3.5-4B"
        sizeBytes={3e9}
        state={{ state: 'idle' }}
        active={false}
        onSelect={vi.fn()}
        onDownload={vi.fn()}
        onDelete={vi.fn()}
      />,
    );
    expect(screen.getByRole('button', { name: /download/i })).toBeInTheDocument();
  });

  it('renders progressbar with correct value when downloading', () => {
    render(
      <ModelDownloadCard
        modelId="qwen3_5_4b"
        displayName="Qwen3.5-4B"
        sizeBytes={3e9}
        state={{ state: 'downloading', bytesDone: 1e9, totalBytes: 3e9 }}
        active={false}
        onSelect={vi.fn()}
        onDownload={vi.fn()}
        onDelete={vi.fn()}
      />,
    );
    const bar = screen.getByRole('progressbar');
    expect(bar).toHaveAttribute('aria-valuenow', '33');
  });

  it('shows In use vs Use depending on active flag when completed', () => {
    const onSelect = vi.fn();
    const { rerender } = render(
      <ModelDownloadCard
        modelId="qwen3_5_4b"
        displayName="Qwen3.5-4B"
        sizeBytes={3e9}
        state={{ state: 'completed', bytesTotal: 3e9 }}
        active={false}
        onSelect={onSelect}
        onDownload={vi.fn()}
        onDelete={vi.fn()}
      />,
    );
    fireEvent.click(screen.getByRole('button', { name: /^use$/i }));
    expect(onSelect).toHaveBeenCalled();

    rerender(
      <ModelDownloadCard
        modelId="qwen3_5_4b"
        displayName="Qwen3.5-4B"
        sizeBytes={3e9}
        state={{ state: 'completed', bytesTotal: 3e9 }}
        active
        onSelect={onSelect}
        onDownload={vi.fn()}
        onDelete={vi.fn()}
      />,
    );
    expect(screen.getByRole('button', { name: /in use/i })).toBeInTheDocument();
  });

  it('shows error message when failed', () => {
    render(
      <ModelDownloadCard
        modelId="qwen3_5_4b"
        displayName="Qwen3.5-4B"
        sizeBytes={3e9}
        state={{ state: 'failed', reason: 'network down' }}
        active={false}
        onSelect={vi.fn()}
        onDownload={vi.fn()}
        onDelete={vi.fn()}
      />,
    );
    expect(screen.getByRole('alert')).toHaveTextContent(/network down/);
  });
});
