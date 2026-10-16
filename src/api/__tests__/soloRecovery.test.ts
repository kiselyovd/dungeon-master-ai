import * as v from 'valibot';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import wire from '../__fixtures__/soloRecovery.json';
import { SoloRecoveryResultSchema, SoloRecoveryStatusSchema } from '../contracts/soloRecovery';
import { performSoloRecovery } from '../soloRecovery';

vi.mock('../client', () => ({
  backendUrl: async (path: string) => `http://127.0.0.1:31415${path}`,
}));
beforeEach(() => vi.stubGlobal('fetch', vi.fn()));
afterEach(() => vi.unstubAllGlobals());
it('decodes the actual Rust HTTP snapshots before and after recovery and rest', () => {
  expect(v.parse(SoloRecoveryStatusSchema, wire.before.status).hp).toBe(0);
  expect(v.parse(SoloRecoveryResultSchema, wire.after.result).status.hp).toBe(1);
  expect(v.parse(SoloRecoveryStatusSchema, wire.afterRest.recovery).hp).toBe(12);
  expect(
    v.safeParse(SoloRecoveryResultSchema, {
      ...wire.after.result,
      receipt: { ...wire.after.result.receipt, target_character_id: wire.before.status.session_id },
    }).success,
  ).toBe(false);
});
it.each([
  408, 429,
])('keeps HTTP %s uncertain so the original nonce is retryable', async (status) => {
  vi.mocked(fetch).mockResolvedValue(
    new Response(JSON.stringify({ error: { code: 'temporarily_unavailable' } }), {
      status,
      headers: { 'content-type': 'application/json' },
    }),
  );
  await expect(
    performSoloRecovery(wire.before.status.campaign_id, {
      session_id: wire.before.status.session_id,
      character_id: wire.before.status.character_id,
      expected_revision: wire.before.status.revision,
      causation_id: '44444444-4444-4444-8444-444444444444',
      intent: { kind: 'wait', target_character_id: wire.before.status.character_id },
    }),
  ).rejects.toMatchObject({ code: 'network' });
});
