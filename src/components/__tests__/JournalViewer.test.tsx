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

  it('calls onClose on Escape key', async () => {
    const onClose = vi.fn();
    render(<JournalViewer entries={[]} onClose={onClose} />);
    await userEvent.keyboard('{Escape}');
    expect(onClose).toHaveBeenCalledOnce();
  });

  it('strips <script> tags from entry HTML before rendering (DOMPurify defense in depth)', () => {
    const malicious = [
      {
        id: 'x',
        entry_html:
          '<p>safe text</p><script>window.__pwned = true;</script><img src=x onerror="window.__pwned=true">',
        chapter: null,
        created_at: '2026-05-09T00:00:00Z',
        campaign_id: 'c1',
      },
    ];
    const { container } = render(<JournalViewer entries={malicious} onClose={vi.fn()} />);
    // Original safe text must still render.
    expect(screen.getByText('safe text')).toBeInTheDocument();
    // Script tags must not survive sanitisation.
    expect(container.querySelector('script')).toBeNull();
    // onerror handler must be stripped from the surviving <img>.
    const img = container.querySelector('img');
    if (img !== null) {
      expect(img.getAttribute('onerror')).toBeNull();
    }
  });
});
