import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import '../../i18n';
import { npcAppearance } from '../../state/npcAppearance';
import { NpcMemoryGrid } from '../NpcMemoryGrid';

const exampleNpc = {
  id: 'n1',
  campaign_id: 'c1',
  name: 'Mira',
  role: 'Innkeeper',
  disposition: 'friendly' as const,
  trust: 50,
  facts: [{ text: 'She saved the party in session 2', created_at: '2026-05-07T12:00:00Z' }],
  updated_at: '2026-05-07T12:00:00Z',
};
const mockNpcs = [exampleNpc];

describe('NpcMemoryGrid', () => {
  it('renders NPC card', async () => {
    render(<NpcMemoryGrid npcs={mockNpcs} onClose={vi.fn()} />);
    expect(screen.getByText('Mira')).toBeInTheDocument();
    expect(screen.getByText('Innkeeper')).toBeInTheDocument();
    await waitFor(() =>
      expect(document.querySelector('img[data-archetype="innkeeper"]')).toHaveAttribute(
        'src',
        expect.stringContaining('npc-innkeeper'),
      ),
    );
  });

  it('preserves a known legacy face after the role changes', async () => {
    const npc = { ...exampleNpc, id: crypto.randomUUID() };
    await npcAppearance(`${npc.campaign_id}:${npc.id}`, 'npc.guard.legacy.v1');
    const view = render(<NpcMemoryGrid npcs={[npc]} onClose={vi.fn()} />);
    await waitFor(() =>
      expect(
        document.querySelector('img[data-asset-id="npc.guard.legacy.v1"]'),
      ).toBeInTheDocument(),
    );
    view.rerender(<NpcMemoryGrid npcs={[{ ...npc, role: 'Mage' }]} onClose={vi.fn()} />);
    await waitFor(() =>
      expect(
        document.querySelector('img[data-asset-id="npc.guard.legacy.v1"]'),
      ).toBeInTheDocument(),
    );
  });

  it('does not replace an unavailable assigned face', async () => {
    const npc = { ...exampleNpc, id: crypto.randomUUID() };
    await npcAppearance(`${npc.campaign_id}:${npc.id}`, 'npc.missing.v2');
    render(<NpcMemoryGrid npcs={[npc]} onClose={vi.fn()} />);
    await screen.findByText('Portrait unavailable');
    expect(document.querySelector('img')).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Open portrait of Mira' })).toBeDisabled();
  });

  it('closes only the expanded portrait on Escape and restores focus', async () => {
    const onClose = vi.fn();
    render(<NpcMemoryGrid npcs={[{ ...exampleNpc, id: crypto.randomUUID() }]} onClose={onClose} />);
    const button = screen.getByRole('button', { name: 'Open portrait of Mira' });
    await waitFor(() => expect(button).toBeEnabled());
    await userEvent.click(button);
    expect(screen.getByRole('dialog', { name: 'Mira' })).toBeInTheDocument();
    await userEvent.keyboard('{Escape}');
    await waitFor(() =>
      expect(screen.queryByRole('dialog', { name: 'Mira' })).not.toBeInTheDocument(),
    );
    expect(onClose).not.toHaveBeenCalled();
    expect(button).toHaveFocus();
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
    await waitFor(() => expect(onClose).toHaveBeenCalledOnce(), { timeout: 1000 });
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
