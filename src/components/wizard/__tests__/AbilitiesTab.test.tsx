import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import '../../../i18n';
import { useStore } from '../../../state/useStore';
import { AbilitiesTab } from '../AbilitiesTab';

beforeEach(() => {
  useStore.getState().charCreation.resetDraft();
});

describe('AbilitiesTab', () => {
  it('renders 3 method radios', () => {
    render(<AbilitiesTab />);
    expect(screen.getByRole('radio', { name: /point.buy/i })).toBeInTheDocument();
    expect(screen.getByRole('radio', { name: /standard.array/i })).toBeInTheDocument();
    expect(screen.getByRole('radio', { name: /4d6/i })).toBeInTheDocument();
  });

  it('selects point_buy and shows remaining points', async () => {
    render(<AbilitiesTab />);
    await userEvent.click(screen.getByRole('radio', { name: /point.buy/i }));
    expect(screen.getByText(/15/)).toBeInTheDocument(); // 27 - 6*2 = 15
  });

  it('disables roll after 3 attempts', async () => {
    render(<AbilitiesTab />);
    await userEvent.click(screen.getByRole('radio', { name: /4d6/i }));
    const rollBtn = await screen.findByRole('button', { name: /roll/i });
    await userEvent.click(rollBtn);
    await userEvent.click(rollBtn);
    await userEvent.click(rollBtn);
    expect(rollBtn).toBeDisabled();
  });
});
