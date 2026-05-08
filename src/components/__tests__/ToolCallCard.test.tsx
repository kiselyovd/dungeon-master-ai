import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import '../../i18n';
import { ToolCallCard } from '../ToolCallCard';

const pendingEntry = {
  id: 'call_1',
  toolName: 'roll_dice',
  args: { dice: '1d20', modifier: 3 },
  result: null,
  isError: false,
  round: 1,
  timestamp: '2026-05-08T11:00:00Z',
};

const settledEntry = {
  ...pendingEntry,
  result: { rolls: [15], total: 18 },
};

describe('ToolCallCard', () => {
  it('renders tool name', () => {
    render(<ToolCallCard entry={pendingEntry} />);
    expect(screen.getByText('roll_dice')).toBeInTheDocument();
  });

  it('shows pending status when result is null', () => {
    render(<ToolCallCard entry={pendingEntry} />);
    expect(screen.getByText(/pending/i)).toBeInTheDocument();
  });

  it('shows success status when result is present', () => {
    render(<ToolCallCard entry={settledEntry} />);
    expect(screen.getByText(/success/i)).toBeInTheDocument();
  });

  it('shows error status for is_error entries', () => {
    render(<ToolCallCard entry={{ ...settledEntry, isError: true, result: { error: 'bad' } }} />);
    // Status badge has exact text "error"; JSON in <pre> renders as "error": "bad"
    // and also matches /error/i, so we look up the badge by exact text.
    expect(screen.getByText('error')).toBeInTheDocument();
  });

  it('renders args and result as text', () => {
    render(<ToolCallCard entry={settledEntry} />);
    expect(screen.getByText(/1d20/)).toBeInTheDocument();
    expect(screen.getByText(/18/)).toBeInTheDocument();
  });
});
