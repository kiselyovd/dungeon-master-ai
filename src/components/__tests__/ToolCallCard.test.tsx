import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { useStore } from '../../state/useStore';
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
  handledBy: 'engine',
};

const settledEntry = {
  ...pendingEntry,
  result: { rolls: [15], total: 18 },
};

describe('ToolCallCard', () => {
  it('full image preview closes with Escape and restores keyboard focus', async () => {
    const user = userEvent.setup();
    render(
      <ToolCallCard
        entry={{
          ...settledEntry,
          toolName: 'generate_illustration',
          imageKind: 'chat',
          imageDataUrl: 'data:image/png;base64,iVBORw0KGgo=',
        }}
      />,
    );
    const opener = screen.getByRole('button', { name: 'Open image' });
    await user.click(opener);
    expect(screen.getByRole('dialog', { name: 'Image' })).toBeVisible();
    await user.keyboard('{Escape}');
    expect(screen.queryByRole('dialog', { name: 'Image' })).toBeNull();
    expect(opener).toHaveFocus();
  });
  it('renders a localized action instead of a machine tool name', () => {
    render(<ToolCallCard entry={pendingEntry} />);
    expect(screen.getByText('Dice roll')).toBeInTheDocument();
    expect(screen.queryByText('roll_dice')).not.toBeInTheDocument();
  });

  it('shows pending status when result is null', () => {
    render(<ToolCallCard entry={pendingEntry} />);
    expect(screen.getByText(/pending/i)).toBeInTheDocument();
  });

  it('shows success status when result is present', () => {
    render(<ToolCallCard entry={settledEntry} />);
    expect(screen.getByText('Done')).toBeInTheDocument();
  });

  it('shows error status for is_error entries', () => {
    render(<ToolCallCard entry={{ ...settledEntry, isError: true, result: { error: 'bad' } }} />);
    expect(screen.getByText('Error')).toBeInTheDocument();
    expect(screen.queryByText('bad')).not.toBeInTheDocument();
  });

  it('shows the confirmed mechanical number and hides arbitrary args/result bodies', () => {
    render(<ToolCallCard entry={settledEntry} />);
    expect(screen.getByText('Roll: 18')).toBeInTheDocument();
    expect(screen.queryByText(/1d20/)).not.toBeInTheDocument();
    expect(screen.queryByText(/"rolls"/)).not.toBeInTheDocument();
  });

  it('opens the existing inspector on explicit request', async () => {
    useStore.getState().toolLog.close();
    render(<ToolCallCard entry={settledEntry} />);
    await userEvent.click(screen.getByRole('button', { name: 'Action technical details' }));
    expect(useStore.getState().toolLog.isOpen).toBe(true);
    useStore.getState().toolLog.close();
  });

  it('does not turn an error with no result into a pending action', () => {
    render(<ToolCallCard entry={{ ...pendingEntry, isError: true }} />);
    expect(screen.getByText('Error')).toBeInTheDocument();
    expect(screen.queryByTestId('tool-pending-indicator')).not.toBeInTheDocument();
  });
});
