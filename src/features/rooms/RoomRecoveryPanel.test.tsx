import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, expect, it, vi } from 'vitest';
import wire from '../../api/__fixtures__/roomUpdate.json';
import type { RoomState } from '../../api/contracts/room';
import i18n from '../../i18n';
import { RoomRecoveryPanel } from './RoomRecoveryPanel';

beforeEach(async () => {
  await i18n.changeLanguage('en');
});
function wounded(status: 'dying' | 'stable'): RoomState {
  const state = structuredClone(wire.state) as RoomState;
  const actor = state.actors[0];
  if (!actor) throw new Error('actor required');
  actor.hp = 0;
  state.recovery = state.recovery.map((r) =>
    r.character_id === actor.character_id
      ? {
          ...r,
          status,
          ready_at_seconds: status === 'stable' ? state.clock.elapsed_seconds + 3600 : null,
          actions: [
            { kind: status === 'stable' ? 'wait' : 'aid', target_character_id: actor.character_id },
          ],
        }
      : r,
  );
  return state;
}
it('uses the server-owned target and current revision for aid', async () => {
  const state = wounded('dying'),
    action = vi.fn(),
    user = userEvent.setup();
  render(<RoomRecoveryPanel state={state} disabled={false} onAction={action} />);
  await user.click(screen.getByRole('button', { name: 'Attempt first aid' }));
  expect(action).toHaveBeenCalledWith(
    expect.objectContaining({
      kind: 'recovery',
      expected_revision: state.revision,
      intent: { kind: 'aid', target_character_id: state.actors[0]?.character_id },
    }),
  );
});
it('wait requires confirmation and a changed revision closes the old choice', async () => {
  const state = wounded('stable'),
    action = vi.fn(),
    user = userEvent.setup();
  const { rerender } = render(
    <RoomRecoveryPanel state={state} disabled={false} onAction={action} />,
  );
  await user.click(screen.getByRole('button', { name: 'Wait for recovery' }));
  expect(action).not.toHaveBeenCalled();
  rerender(
    <RoomRecoveryPanel
      state={{ ...state, revision: state.revision + 1 }}
      disabled={false}
      onAction={action}
    />,
  );
  expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
});
it('temporary blocking preserves wait choice but prevents submission', async () => {
  const state = wounded('stable'),
    action = vi.fn(),
    user = userEvent.setup();
  const { rerender } = render(
    <RoomRecoveryPanel state={state} disabled={false} onAction={action} />,
  );
  await user.click(screen.getByRole('button', { name: 'Wait for recovery' }));
  rerender(<RoomRecoveryPanel state={state} disabled onAction={action} />);
  expect(
    within(screen.getByRole('dialog')).getByRole('button', { name: 'Wait for recovery' }),
  ).toBeDisabled();
  rerender(<RoomRecoveryPanel state={state} disabled={false} onAction={action} />);
  await user.click(
    within(screen.getByRole('dialog')).getByRole('button', { name: 'Wait for recovery' }),
  );
  expect(action).toHaveBeenCalledTimes(1);
});
