import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { expect, it, vi } from 'vitest';
import i18n from '../../../i18n';
import { AdventurePanel } from '../AdventurePanel';

it('renders only discovered facts and the server supplied actions', async () => {
  await i18n.changeLanguage('en');
  const action = vi.fn();
  render(
    <AdventurePanel
      open
      onClose={vi.fn()}
      pending={false}
      error={false}
      onAction={action}
      onRetry={vi.fn()}
      retryRequired={false}
      projection={{
        state: {
          package_id: 'bell-beneath-water',
          location: 'town',
          facts: ['ferry_route'],
          ending: null,
          closed: false,
          history: [],
        },
        available_actions: [{ kind: 'move', location: 'quay' }],
      }}
    />,
  );
  expect(screen.queryByText(/land ledger proves/)).not.toBeInTheDocument();
  await userEvent.setup().click(screen.getByRole('button', { name: 'Go to Old quay' }));
  expect(action).toHaveBeenCalledWith({ kind: 'move', location: 'quay' });
});

it('keeps ambiguous failure recoverable without issuing a different intent', async () => {
  await i18n.changeLanguage('en');
  const retry = vi.fn();
  render(
    <AdventurePanel
      open
      onClose={vi.fn()}
      pending={false}
      error
      onAction={vi.fn()}
      onRetry={retry}
      retryRequired
      projection={{
        state: {
          package_id: 'bell-beneath-water',
          location: 'town',
          facts: ['ferry_route'],
          ending: null,
          closed: false,
          history: [],
        },
        available_actions: [{ kind: 'move', location: 'quay' }],
      }}
    />,
  );
  expect(screen.getByRole('button', { name: 'Go to Old quay' })).toBeDisabled();
  await userEvent.setup().click(screen.getByRole('button', { name: 'Retry last action' }));
  expect(retry).toHaveBeenCalledOnce();
});
