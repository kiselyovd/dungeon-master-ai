import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { LanguageSegmentedControl } from '../LanguageSegmentedControl';

describe('LanguageSegmentedControl', () => {
  it('shows Russian first and reports the selected language', async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(<LanguageSegmentedControl value="ru" onChange={onChange} ariaLabel="Язык интерфейса" />);

    const group = screen.getByRole('group', { name: 'Язык интерфейса' });
    const buttons = group.querySelectorAll('button');
    expect(buttons[0]).toHaveTextContent('РУ');
    expect(screen.getByRole('button', { name: 'РУ' })).toHaveAttribute('aria-pressed', 'true');

    await user.click(screen.getByRole('button', { name: 'EN' }));
    expect(onChange).toHaveBeenCalledWith('en');
  });
});
