import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import '../../i18n';
import { NpcMemoryGrid } from '../NpcMemoryGrid';

const mockNpcs = [
  {
    id: 'n1',
    campaign_id: 'c1',
    name: 'Mira',
    role: 'Innkeeper',
    disposition: 'friendly' as const,
    trust: 50,
    facts: [{ text: 'She saved the party in session 2', created_at: '2026-05-07T12:00:00Z' }],
    updated_at: '2026-05-07T12:00:00Z',
  },
];

describe('NpcMemoryGrid', () => {
  it('renders NPC card', () => {
    render(<NpcMemoryGrid npcs={mockNpcs} onClose={vi.fn()} />);
    expect(screen.getByText('Mira')).toBeInTheDocument();
    expect(screen.getByText('Innkeeper')).toBeInTheDocument();
  });

  it('shows disposition label', () => {
    render(<NpcMemoryGrid npcs={mockNpcs} onClose={vi.fn()} />);
    // The component renders translated `disposition_friendly` ('Friendly').
    // i18n init in tests resolves keys; assert on the translated text.
    expect(screen.getAllByText(/friendly/i).length).toBeGreaterThan(0);
  });

  it('shows empty state for no NPCs', () => {
    render(<NpcMemoryGrid npcs={[]} onClose={vi.fn()} />);
    expect(screen.getByText(/no npcs/i)).toBeInTheDocument();
  });

  it('calls onClose', async () => {
    const onClose = vi.fn();
    render(<NpcMemoryGrid npcs={[]} onClose={onClose} />);
    await userEvent.click(screen.getByRole('button', { name: /close/i }));
    expect(onClose).toHaveBeenCalledOnce();
  });

  it('escape in search clears the field, does not close', async () => {
    const onClose = vi.fn();
    render(<NpcMemoryGrid npcs={mockNpcs} onClose={onClose} />);
    const input = screen.getByRole('searchbox');
    await userEvent.type(input, 'mira');
    await userEvent.click(input); // ensure focus
    await userEvent.keyboard('{Escape}');
    expect(onClose).not.toHaveBeenCalled();
    expect((input as HTMLInputElement).value).toBe('');
  });
});
