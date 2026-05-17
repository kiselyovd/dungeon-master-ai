import { fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import '../../i18n';
import type { ResolvedModelEntry } from '../../state/discoveredCatalogs';
import { ModelSelector } from '../ModelSelector';

const curated: ResolvedModelEntry = {
  model_id: 'claude-opus-4-7',
  display_name: 'Claude Opus 4.7',
  capabilities: { vision_input: true, reasoning: true, tool_calls: true, streaming: true },
  source: 'curated',
  context_length: 1_000_000,
};
const discovered: ResolvedModelEntry = {
  model_id: 'gpt-4o',
  display_name: 'GPT-4o',
  capabilities: { vision_input: true, reasoning: false, tool_calls: true, streaming: true },
  source: 'discovered-api',
  context_length: 128_000,
};

afterEach(() => {
  vi.restoreAllMocks();
});

describe('ModelSelector', () => {
  it('renders the empty state with a Discover button when no models and idle', () => {
    const onDiscover = vi.fn();
    render(
      <ModelSelector
        value=""
        onChange={() => {}}
        models={[]}
        status="idle"
        error={null}
        onDiscover={onDiscover}
        lastCachedAt={null}
      />,
    );
    const button = screen.getByRole('button', { name: /discover models/i });
    expect(button).toBeInTheDocument();
    fireEvent.click(button);
    expect(onDiscover).toHaveBeenCalled();
  });

  it('shows a loading indicator and disables the Discover button when loading', () => {
    render(
      <ModelSelector
        value=""
        onChange={() => {}}
        models={[]}
        status="loading"
        error={null}
        onDiscover={() => {}}
        lastCachedAt={null}
      />,
    );
    expect(screen.getByRole('button', { name: /discovering/i })).toBeDisabled();
    expect(screen.getByText(/discovering/i)).toBeInTheDocument();
  });

  it('shows an error banner with retry that re-fires onDiscover', () => {
    const onDiscover = vi.fn();
    render(
      <ModelSelector
        value=""
        onChange={() => {}}
        models={[]}
        status="error"
        error="unauthorized (401)"
        onDiscover={onDiscover}
        lastCachedAt={null}
      />,
    );
    expect(screen.getByRole('alert')).toHaveTextContent(/unauthorized/i);
    fireEvent.click(screen.getByRole('button', { name: /discover/i }));
    expect(onDiscover).toHaveBeenCalled();
  });

  it('renders curated models under Recommended and discovered-api under Discovered', () => {
    render(
      <ModelSelector
        value=""
        onChange={() => {}}
        models={[curated, discovered]}
        status="idle"
        error={null}
        onDiscover={() => {}}
        lastCachedAt={new Date().toISOString()}
      />,
    );
    expect(screen.getByText(/^Recommended$/i)).toBeInTheDocument();
    expect(screen.getByText(/^Discovered$/i)).toBeInTheDocument();
    expect(screen.getByText('Claude Opus 4.7')).toBeInTheDocument();
    expect(screen.getByText('GPT-4o')).toBeInTheDocument();
  });

  it('renders capability pills (vision and reasoning) when present', () => {
    render(
      <ModelSelector
        value=""
        onChange={() => {}}
        models={[curated]}
        status="idle"
        error={null}
        onDiscover={() => {}}
        lastCachedAt={null}
      />,
    );
    expect(screen.getByText(/^Vision$/i)).toBeInTheDocument();
    expect(screen.getByText(/^Reasoning$/i)).toBeInTheDocument();
  });

  it('filters models by display_name or model_id when the user types in the filter', () => {
    render(
      <ModelSelector
        value=""
        onChange={() => {}}
        models={[curated, discovered]}
        status="idle"
        error={null}
        onDiscover={() => {}}
        lastCachedAt={null}
      />,
    );
    const filter = screen.getByRole('searchbox');
    fireEvent.change(filter, { target: { value: 'gpt' } });
    expect(screen.queryByText('Claude Opus 4.7')).not.toBeInTheDocument();
    expect(screen.getByText('GPT-4o')).toBeInTheDocument();
  });

  it('clicking a model row fires onChange with model_id', () => {
    const onChange = vi.fn();
    render(
      <ModelSelector
        value=""
        onChange={onChange}
        models={[curated]}
        status="idle"
        error={null}
        onDiscover={() => {}}
        lastCachedAt={null}
      />,
    );
    fireEvent.click(screen.getByRole('option', { name: /claude opus 4\.7/i }));
    expect(onChange).toHaveBeenCalledWith('claude-opus-4-7');
  });

  it('typing in the free-text input fires onChange (custom slug support)', () => {
    const onChange = vi.fn();
    render(
      <ModelSelector
        value="qwen3-1.7b"
        onChange={onChange}
        models={[]}
        status="idle"
        error={null}
        onDiscover={() => {}}
        lastCachedAt={null}
      />,
    );
    const textInput = screen.getByRole('textbox');
    fireEvent.change(textInput, { target: { value: 'qwen3-7b-q4_k_m' } });
    expect(onChange).toHaveBeenCalledWith('qwen3-7b-q4_k_m');
  });

  it('renders the Custom HF repo section as a disabled placeholder', () => {
    render(
      <ModelSelector
        value=""
        onChange={() => {}}
        models={[curated]}
        status="idle"
        error={null}
        onDiscover={() => {}}
        lastCachedAt={null}
      />,
    );
    expect(screen.getByText(/Custom HF repo/i)).toBeInTheDocument();
    const placeholders = screen.getAllByText(/coming soon/i);
    expect(placeholders.length).toBeGreaterThanOrEqual(1);
  });

  it('marks the selected row with aria-selected=true', () => {
    render(
      <ModelSelector
        value="claude-opus-4-7"
        onChange={() => {}}
        models={[curated, discovered]}
        status="idle"
        error={null}
        onDiscover={() => {}}
        lastCachedAt={null}
      />,
    );
    const opus = screen.getByRole('option', { name: /claude opus 4\.7/i });
    const gpt = screen.getByRole('option', { name: /gpt-4o/i });
    expect(opus).toHaveAttribute('aria-selected', 'true');
    expect(gpt).toHaveAttribute('aria-selected', 'false');
  });

  it('shows a "cached at" note when lastCachedAt is provided', () => {
    render(
      <ModelSelector
        value=""
        onChange={() => {}}
        models={[curated]}
        status="idle"
        error={null}
        onDiscover={() => {}}
        lastCachedAt={'2026-05-17T12:00:00Z'}
      />,
    );
    expect(screen.getByText(/cached at/i)).toBeInTheDocument();
  });
});
