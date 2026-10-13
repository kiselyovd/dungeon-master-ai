import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { expect, it, vi } from 'vitest';
import type { TablePresentation, TablePresentationPorts } from './model/types';
import { TableContext, useTablePresentation } from './TableContext';

function Consumer() {
  const { model, ports } = useTablePresentation();
  return (
    <button type="button" onClick={() => void ports.send('Player intention')}>
      {model.title}
    </button>
  );
}
it('routes the same presentation action to its bound role controller without making a game command', async () => {
  const model: TablePresentation = {
    scope: {
      mode: 'guest',
      campaignId: 'campaign',
      sessionId: 'session',
      revision: 1,
      timeline: 0,
      roomId: 'room',
      roomEpoch: 1,
      encounterId: null,
    },
    title: 'Campaign',
    scene: { kind: 'exploration', title: null, art: null },
    actors: [],
    ownActorId: null,
    messages: [],
    outcomes: [],
    connection: 'connected',
    readiness: 'unknown',
    recovery: 'clear',
    combat: 'inactive',
    availability: {
      send: { enabled: true },
      createHero: { enabled: true },
      configure: { enabled: false, reason: 'host_only' },
    },
  };
  const first: TablePresentationPorts = {
    send: vi.fn(),
    retry: vi.fn(),
    cancel: vi.fn(),
    openHero: vi.fn(),
  };
  const next: TablePresentationPorts = {
    send: vi.fn(),
    retry: vi.fn(),
    cancel: vi.fn(),
    openHero: vi.fn(),
  };
  const view = render(
    <TableContext model={model} ports={first}>
      <Consumer />
    </TableContext>,
  );
  await userEvent.click(screen.getByRole('button', { name: 'Campaign' }));
  expect(first.send).toHaveBeenCalledWith('Player intention');
  view.rerender(
    <TableContext
      model={{ ...model, scope: { ...model.scope, mode: 'solo', roomId: null, roomEpoch: null } }}
      ports={next}
    >
      <Consumer />
    </TableContext>,
  );
  await userEvent.click(screen.getByRole('button', { name: 'Campaign' }));
  expect(first.send).toHaveBeenCalledTimes(1);
  expect(next.send).toHaveBeenCalledTimes(1);
});
