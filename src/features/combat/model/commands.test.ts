import { afterEach, describe, expect, it, vi } from 'vitest';
import { setBackendPortForTesting } from '../../../api/client';
import { sendCombatCommand } from './commands';

const projection = {
  schema_version: 1,
  encounter_id: 'enc',
  revision: 1,
  events: [],
  snapshot: {
    active: true,
    round: 1,
    current_combatant: 'hero',
    initiative: [{ id: 'hero', roll: 20, dex_tiebreak: 2 }],
    combatants: [],
  },
};

describe('sendCombatCommand transport cleanup', () => {
  afterEach(() => {
    setBackendPortForTesting(null);
    vi.unstubAllGlobals();
  });

  it('cancels the one-event SSE body as soon as the projection is decoded', async () => {
    const cancel = vi.fn(async () => undefined);
    const releaseLock = vi.fn();
    const read = vi.fn().mockResolvedValue({
      done: false,
      value: new TextEncoder().encode(
        `event: combat_projection\ndata: ${JSON.stringify({ projection })}\n\n`,
      ),
    });
    setBackendPortForTesting(45678);
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => ({
        ok: true,
        status: 200,
        body: { getReader: () => ({ read, cancel, releaseLock }) },
      })),
    );

    const result = await sendCombatCommand({
      encounterId: 'enc',
      revision: 0,
      commandId: 'cmd-1',
      command: { kind: 'cast', combatantId: 'hero' },
    });

    expect(result.revision).toBe(1);
    expect(cancel).toHaveBeenCalledOnce();
    expect(releaseLock).toHaveBeenCalledOnce();
    expect(read).toHaveBeenCalledOnce();
  });
});
