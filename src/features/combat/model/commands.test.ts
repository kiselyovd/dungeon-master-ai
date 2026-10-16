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

  it('drains the SSE body after decoding the projection', async () => {
    const bytes = new TextEncoder().encode(
      `event: combat_projection\ndata: ${JSON.stringify({ projection })}\n\n`,
    );
    const arrayBuffer = vi.fn(async () => bytes.buffer);
    setBackendPortForTesting(45678);
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => ({
        ok: true,
        status: 200,
        body: {},
        arrayBuffer,
      })),
    );

    const result = await sendCombatCommand({
      encounterId: 'enc',
      revision: 0,
      commandId: 'cmd-1',
      command: { kind: 'cast', combatantId: 'hero' },
    });

    expect(result.revision).toBe(1);
    expect(arrayBuffer).toHaveBeenCalledOnce();
  });
});
