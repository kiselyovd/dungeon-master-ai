import { render, screen, within } from '@testing-library/react';
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

  describe('StandardArrayPanel uniqueness', () => {
    it('disables the option for a value already assigned to another ability', async () => {
      render(<AbilitiesTab />);
      await userEvent.click(screen.getByRole('radio', { name: /standard.array/i }));

      // Assign 15 to STR via the str select
      const strSelect = screen.getByRole('combobox', { name: /STR/i });
      await userEvent.selectOptions(strSelect, '15');

      // The option value="15" in the DEX select should now be disabled
      const dexSelect = screen.getByRole('combobox', { name: /DEX/i });
      const takenOption = within(dexSelect).getByRole('option', { name: /^15/ });
      expect(takenOption).toBeDisabled();
    });

    it('does NOT disable a value option in its own ability select', async () => {
      render(<AbilitiesTab />);
      await userEvent.click(screen.getByRole('radio', { name: /standard.array/i }));

      // Assign 15 to STR
      const strSelect = screen.getByRole('combobox', { name: /STR/i });
      await userEvent.selectOptions(strSelect, '15');

      // The option value="15" in STR's own select must NOT be disabled
      const ownOption = within(strSelect).getByRole('option', { name: /^15/ });
      expect(ownOption).not.toBeDisabled();
    });

    it('shows the taken suffix in the label of a disabled option', async () => {
      render(<AbilitiesTab />);
      await userEvent.click(screen.getByRole('radio', { name: /standard.array/i }));

      // Assign 15 to STR
      const strSelect = screen.getByRole('combobox', { name: /STR/i });
      await userEvent.selectOptions(strSelect, '15');

      // The disabled option in DEX select should include the "taken" label
      const dexSelect = screen.getByRole('combobox', { name: /DEX/i });
      const takenOption = within(dexSelect).getByRole('option', { name: /15.*taken/i });
      expect(takenOption).toBeInTheDocument();
    });
  });

  describe('PointBuyPanel aria-label localization', () => {
    it('uses the localized ability name in the increment button aria-label', async () => {
      render(<AbilitiesTab />);
      await userEvent.click(screen.getByRole('radio', { name: /point.buy/i }));

      // Should find "Increase STR" (localized), not "Increase str" (raw key)
      expect(screen.getByRole('button', { name: 'Increase STR' })).toBeInTheDocument();
      expect(screen.queryByRole('button', { name: 'Increase str' })).not.toBeInTheDocument();
    });

    it('uses the localized ability name in the decrement button aria-label', async () => {
      render(<AbilitiesTab />);
      await userEvent.click(screen.getByRole('radio', { name: /point.buy/i }));

      // Should find "Decrease STR" (localized), not "Decrease str" (raw key)
      expect(screen.getByRole('button', { name: 'Decrease STR' })).toBeInTheDocument();
      expect(screen.queryByRole('button', { name: 'Decrease str' })).not.toBeInTheDocument();
    });
  });
});
