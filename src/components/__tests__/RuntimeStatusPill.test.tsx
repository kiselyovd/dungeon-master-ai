import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { RuntimeStatusPill } from '../RuntimeStatusPill';

describe('RuntimeStatusPill', () => {
  it('shows the off state', () => {
    render(<RuntimeStatusPill label="LLM" state={{ state: 'off' }} />);
    expect(screen.getByText(/off/i)).toBeInTheDocument();
  });

  it('shows ready and port', () => {
    render(<RuntimeStatusPill label="LLM" state={{ state: 'ready', port: 37000 }} />);
    expect(screen.getByText(/37000/)).toBeInTheDocument();
  });

  it('shows failure reason', () => {
    render(<RuntimeStatusPill label="LLM" state={{ state: 'failed', reason: 'crashed' }} />);
    expect(screen.getByText(/crashed/)).toBeInTheDocument();
  });
});
