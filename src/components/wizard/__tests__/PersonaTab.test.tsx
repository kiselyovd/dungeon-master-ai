import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import '../../../i18n';
import { useStore } from '../../../state/useStore';
import { PersonaTab } from '../PersonaTab';

const compendium = {} as never;

beforeEach(() => {
  useStore.getState().charCreation.resetDraft();
});

describe('PersonaTab', () => {
  it('renders name input + alignment grid + 4 textareas', () => {
    render(<PersonaTab compendium={compendium} />);
    expect(screen.getByLabelText(/name/i)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'LG' })).toBeInTheDocument();
    expect(screen.getByLabelText(/ideals/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/backstory/i)).toBeInTheDocument();
  });

  it('typing in name updates draft', async () => {
    render(<PersonaTab compendium={compendium} />);
    await userEvent.type(screen.getByLabelText(/name/i), 'Astarion');
    expect(useStore.getState().charCreation.name).toBe('Astarion');
  });

  it('clicking alignment sets draft.alignment', async () => {
    render(<PersonaTab compendium={compendium} />);
    await userEvent.click(screen.getByRole('button', { name: 'CG' }));
    expect(useStore.getState().charCreation.alignment).toBe('CG');
  });
});
