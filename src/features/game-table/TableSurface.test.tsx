import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { expect, it } from 'vitest';
import type { TablePresentation } from './model/types';
import { TableSurface } from './TableSurface';

it('retains mounted controls and unsent text across a timeline change while exposing the new scope', async () => {
  const model: TablePresentation = {
    scope: {
      mode: 'guest',
      campaignId: 'campaign',
      sessionId: 'session',
      revision: 1,
      timeline: 0,
      roomEpoch: 1,
      roomId: 'room',
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
      send: { enabled: false, reason: 'hero_required' },
      createHero: { enabled: true },
      configure: { enabled: false, reason: 'host_only' },
    },
  };
  const controls = <input aria-label="Draft" defaultValue="" />;
  const view = render(<TableSurface model={model}>{controls}</TableSurface>);
  await userEvent.type(screen.getByLabelText('Draft'), 'Unsent draft');
  const oldKey = view.container
    .querySelector('[data-game-table]')
    ?.getAttribute('data-table-scope');
  view.rerender(
    <TableSurface model={{ ...model, scope: { ...model.scope, timeline: 3 } }}>
      {controls}
    </TableSurface>,
  );
  expect(screen.getByLabelText('Draft')).toHaveValue('Unsent draft');
  expect(
    view.container.querySelector('[data-game-table]')?.getAttribute('data-table-scope'),
  ).not.toBe(oldKey);
});
