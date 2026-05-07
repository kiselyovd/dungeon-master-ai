import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import '../../i18n';
import { JournalViewer } from '../JournalViewer';

const mockEntries = [
  {
    id: '1',
    entry_html: '<p>The party entered the dungeon.</p>',
    chapter: 'Chapter 1',
    created_at: '2026-05-07T12:00:00Z',
    campaign_id: 'c1',
  },
  {
    id: '2',
    entry_html: '<p>They fought goblins.</p>',
    chapter: null,
    created_at: '2026-05-07T12:01:00Z',
    campaign_id: 'c1',
  },
];

describe('JournalViewer', () => {
  it('renders entries', () => {
    render(<JournalViewer entries={mockEntries} onClose={vi.fn()} />);
    expect(screen.getByText('Chapter 1')).toBeInTheDocument();
    expect(screen.getByText(/entered the dungeon/i)).toBeInTheDocument();
  });

  it('calls onClose when close button clicked', async () => {
    const onClose = vi.fn();
    render(<JournalViewer entries={[]} onClose={onClose} />);
    await userEvent.click(screen.getByRole('button', { name: /close/i }));
    expect(onClose).toHaveBeenCalledOnce();
  });

  it('shows empty state when no entries', () => {
    render(<JournalViewer entries={[]} onClose={vi.fn()} />);
    expect(screen.getByText(/no journal entries/i)).toBeInTheDocument();
  });
});
