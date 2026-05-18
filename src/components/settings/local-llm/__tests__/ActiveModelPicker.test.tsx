import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import '../../../../i18n';
import type { MergedEntry } from '../../../../state/local_llm/manifest';
import { ActiveModelPicker } from '../ActiveModelPicker';

function makeEntry(id: string, overrides: Partial<MergedEntry> = {}): MergedEntry {
  return {
    id,
    hf_repo: `org/${id}`,
    hf_filename: `${id}.gguf`,
    arch: 'llama',
    quant: 'Q4_K_M',
    size_gb: 4.2,
    license: 'apache-2.0',
    display_name: id,
    source: 'system',
    installed: true,
    ...overrides,
  };
}

describe('ActiveModelPicker', () => {
  it('renders empty-state copy when no installed models', () => {
    render(
      <ActiveModelPicker
        installedModels={[]}
        activeId={null}
        onChange={() => {}}
        disabled={false}
      />,
    );
    expect(screen.getByText(/no models installed/i)).toBeInTheDocument();
  });

  it('renders one radio per installed model', () => {
    const models = [makeEntry('alpha'), makeEntry('beta'), makeEntry('gamma')];
    render(
      <ActiveModelPicker
        installedModels={models}
        activeId={null}
        onChange={() => {}}
        disabled={false}
      />,
    );
    const radios = screen.getAllByRole('radio');
    expect(radios).toHaveLength(3);
  });

  it('marks the active model radio as checked', () => {
    const models = [makeEntry('alpha'), makeEntry('beta')];
    render(
      <ActiveModelPicker
        installedModels={models}
        activeId="beta"
        onChange={() => {}}
        disabled={false}
      />,
    );
    const radios = screen.getAllByRole('radio') as HTMLInputElement[];
    const alpha = radios.find((r) => r.value === 'alpha');
    const beta = radios.find((r) => r.value === 'beta');
    expect(alpha?.checked).toBe(false);
    expect(beta?.checked).toBe(true);
  });

  it('fires onChange with the model id when a radio is clicked', () => {
    const onChange = vi.fn();
    const models = [makeEntry('alpha'), makeEntry('beta')];
    render(
      <ActiveModelPicker
        installedModels={models}
        activeId="alpha"
        onChange={onChange}
        disabled={false}
      />,
    );
    const radios = screen.getAllByRole('radio') as HTMLInputElement[];
    const beta = radios.find((r) => r.value === 'beta');
    if (!beta) throw new Error('expected beta radio');
    fireEvent.click(beta);
    expect(onChange).toHaveBeenCalledWith('beta');
  });

  it('disables every radio when disabled=true', () => {
    const models = [makeEntry('alpha'), makeEntry('beta')];
    render(
      <ActiveModelPicker
        installedModels={models}
        activeId="alpha"
        onChange={() => {}}
        disabled={true}
      />,
    );
    const radios = screen.getAllByRole('radio') as HTMLInputElement[];
    expect(radios).toHaveLength(2);
    for (const r of radios) {
      expect(r.disabled).toBe(true);
    }
  });
});
