import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { Select } from '../Select';

const options = [
  { value: 'openai', label: 'OpenAI compatible' },
  { value: 'local', label: 'Local MistralRS' },
] as const;

describe('Select', () => {
  it('selects an option with the pointer and returns focus to the trigger', async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(<Select value="openai" options={options} onChange={onChange} ariaLabel="Provider" />);

    const trigger = screen.getByRole('combobox', { name: 'Provider' });
    await user.click(trigger);
    expect(screen.getByRole('listbox', { name: 'Provider' })).toBeInTheDocument();
    expect(screen.getByRole('option', { name: 'OpenAI compatible' })).toHaveAttribute(
      'aria-selected',
      'true',
    );

    await user.click(screen.getByRole('option', { name: 'Local MistralRS' }));
    expect(onChange).toHaveBeenCalledWith('local');
    expect(trigger).toHaveFocus();
  });

  it('supports arrow navigation, Enter selection, and Escape dismissal', async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(<Select value="openai" options={options} onChange={onChange} ariaLabel="Provider" />);

    const trigger = screen.getByRole('combobox', { name: 'Provider' });
    await user.click(trigger);
    await user.keyboard('{ArrowDown}{Enter}');
    expect(onChange).toHaveBeenCalledWith('local');

    await user.click(trigger);
    await user.keyboard('{Escape}');
    expect(screen.queryByRole('listbox')).not.toBeInTheDocument();
    expect(trigger).toHaveFocus();
  });

  it('closes on outside pointer input and honors disabled state', async () => {
    const user = userEvent.setup();
    const { rerender } = render(
      <div>
        <Select value="openai" options={options} onChange={vi.fn()} ariaLabel="Provider" />
        <button type="button">Outside</button>
      </div>,
    );

    await user.click(screen.getByRole('combobox', { name: 'Provider' }));
    await user.click(screen.getByRole('button', { name: 'Outside' }));
    expect(screen.queryByRole('listbox')).not.toBeInTheDocument();

    rerender(
      <Select value="openai" options={options} onChange={vi.fn()} ariaLabel="Provider" disabled />,
    );
    expect(screen.getByRole('combobox', { name: 'Provider' })).toBeDisabled();
  });
});
