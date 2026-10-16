import { expect, it, vi } from 'vitest';
import { exitRoomToLocalCampaign } from '../roomExit';

it('waits for authoritative local state before returning and never uses a remote room ID', async () => {
  let resolve!: (value: unknown) => void;
  const activate = vi.fn(
    () =>
      new Promise<unknown>((done) => {
        resolve = done;
      }),
  );
  const exit = vi.fn();
  const task = exitRoomToLocalCampaign('local-campaign', activate, exit);
  expect(activate).toHaveBeenCalledWith('local-campaign');
  expect(exit).not.toHaveBeenCalled();
  resolve({ revision: 20, hp: 8 });
  await task;
  expect(exit).toHaveBeenCalledOnce();
});
it('keeps room UI reachable if local state cannot be refreshed', async () => {
  const exit = vi.fn();
  await expect(exitRoomToLocalCampaign('local-campaign', async () => null, exit)).rejects.toThrow(
    'local_campaign_recovery_failed',
  );
  expect(exit).not.toHaveBeenCalled();
});
