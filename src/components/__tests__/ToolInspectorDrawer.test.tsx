import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import '../../i18n';
import { ToolInspectorDrawer } from '../ToolInspectorDrawer';

const entries = [
  {
    id: 'c1',
    toolName: 'roll_dice',
    args: { dice: '1d20' },
    result: { rolls: [14], total: 14 },
    isError: false,
    round: 1,
    timestamp: '2026-05-07T12:00:00Z',
  },
];

describe('ToolInspectorDrawer', () => {
  it('renders when open', () => {
    render(<ToolInspectorDrawer entries={entries} isOpen={true} onClose={vi.fn()} />);
    expect(screen.getByText('roll_dice')).toBeInTheDocument();
  });

  it('does not render content when closed', () => {
    render(<ToolInspectorDrawer entries={entries} isOpen={false} onClose={vi.fn()} />);
    expect(screen.queryByText('roll_dice')).not.toBeInTheDocument();
  });

  it('calls onClose', async () => {
    const onClose = vi.fn();
    render(<ToolInspectorDrawer entries={entries} isOpen={true} onClose={onClose} />);
    await userEvent.click(screen.getByRole('button', { name: /close inspector/i }));
    expect(onClose).toHaveBeenCalledOnce();
  });

  it('has copy-as-cURL button per entry', () => {
    render(<ToolInspectorDrawer entries={entries} isOpen={true} onClose={vi.fn()} />);
    expect(screen.getByRole('button', { name: /copy.*curl/i })).toBeInTheDocument();
  });
});
