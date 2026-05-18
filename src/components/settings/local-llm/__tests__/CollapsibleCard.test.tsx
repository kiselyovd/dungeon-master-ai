import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { CollapsibleCard } from '../CollapsibleCard';

describe('CollapsibleCard', () => {
  it('renders title and chip', () => {
    render(
      <CollapsibleCard title="Manage downloads" chip="3 / 7 installed">
        <p>body</p>
      </CollapsibleCard>,
    );
    expect(screen.getByText('Manage downloads')).toBeInTheDocument();
    expect(screen.getByText('3 / 7 installed')).toBeInTheDocument();
  });

  it('starts collapsed when defaultOpen is false', () => {
    render(
      <CollapsibleCard title="t" chip="c" defaultOpen={false}>
        <p data-testid="card-body">body</p>
      </CollapsibleCard>,
    );
    expect(screen.queryByTestId('card-body')).not.toBeInTheDocument();
  });

  it('starts expanded when defaultOpen is true', () => {
    render(
      <CollapsibleCard title="t" chip="c" defaultOpen={true}>
        <p data-testid="card-body">body</p>
      </CollapsibleCard>,
    );
    expect(screen.getByTestId('card-body')).toBeInTheDocument();
  });

  it('toggles aria-expanded on header click', () => {
    render(
      <CollapsibleCard title="t" chip="c">
        <p>body</p>
      </CollapsibleCard>,
    );
    const header = screen.getByRole('button', { name: /t/i });
    expect(header).toHaveAttribute('aria-expanded', 'false');
    fireEvent.click(header);
    expect(header).toHaveAttribute('aria-expanded', 'true');
    fireEvent.click(header);
    expect(header).toHaveAttribute('aria-expanded', 'false');
  });
});
