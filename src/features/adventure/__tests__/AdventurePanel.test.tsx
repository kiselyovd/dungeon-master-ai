import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { expect, it, vi } from 'vitest';
import i18n from '../../../i18n';
import { AdventurePanel } from '../AdventurePanel';

it('renders only server-permitted portrait identities and keeps the parent dialog open', async () => {
  await i18n.changeLanguage('en');
  const close = vi.fn();
  render(
    <AdventurePanel
      open
      onClose={close}
      pending={false}
      error={false}
      retryRequired={false}
      onAction={vi.fn()}
      onRetry={vi.fn()}
      projection={{
        state: {
          package_id: 'bell-beneath-water',
          location: 'belfry',
          facts: ['roster_available'],
          ending: null,
          closed: false,
          history: [],
        },
        available_actions: [],
        public_npcs: [{ id: 'mila', portrait_asset_id: 'npc.bell.mila.v2' }],
      }}
    />,
  );
  expect(screen.queryByRole('button', { name: 'Open portrait of Savely' })).not.toBeInTheDocument();
  const button = screen.getByRole('button', { name: 'Open portrait of Mila' });
  await userEvent.click(button);
  expect(screen.getByRole('dialog', { name: 'Mila' })).toBeInTheDocument();
  await userEvent.keyboard('{Escape}');
  expect(screen.queryByRole('dialog', { name: 'Mila' })).not.toBeInTheDocument();
  expect(screen.getByRole('dialog', { name: 'The Bell Beneath the Water' })).toBeInTheDocument();
  expect(close).not.toHaveBeenCalled();
  expect(button).toHaveFocus();
});

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

it('distinguishes personal escort from helping the NPC return alone', async () => {
  await i18n.changeLanguage('ru');
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
          location: 'archive',
          facts: ['ferry_route', 'tikhon_located'],
          ending: null,
          closed: false,
          history: [],
        },
        available_actions: [{ kind: 'rescue_tikhon' }, { kind: 'escort_tikhon_home' }],
      }}
    />,
  );
  expect(
    screen.getByRole('button', { name: 'Помочь Тихону вернуться домой, оставаясь в архиве' }),
  ).toBeVisible();
  await userEvent
    .setup()
    .click(screen.getByRole('button', { name: 'Лично проводить Тихона домой в Вереск' }));
  expect(action).toHaveBeenCalledExactlyOnceWith({ kind: 'escort_tikhon_home' });
});
