import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { RuntimeStatusPill } from '../RuntimeStatusPill';
import '../../i18n';

describe('RuntimeStatusPill', () => {
  it('shows the off state', () => {
    render(<RuntimeStatusPill label="LLM" state={{ state: 'off' }} />);
    expect(screen.getByText(/off/i)).toBeInTheDocument();
  });

  it('shows a running process without exposing its port', () => {
    render(<RuntimeStatusPill label="LLM" state={{ state: 'ready', port: 37000 }} />);
    expect(screen.getByText(/running/i)).toBeInTheDocument();
    expect(screen.queryByText(/37000/)).not.toBeInTheDocument();
  });

  it('shows a localized failure without raw runtime details', () => {
    render(<RuntimeStatusPill label="LLM" state={{ state: 'failed', reason: 'crashed' }} />);
    expect(screen.getByText(/could not start/i)).toBeInTheDocument();
    expect(screen.queryByText(/crashed/)).not.toBeInTheDocument();
  });
});
