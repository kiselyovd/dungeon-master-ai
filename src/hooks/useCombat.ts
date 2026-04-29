import { endCombat, type InitiativeEntryPayload, startCombat } from '../api/combat';
import { useStore } from '../state/useStore';

/**
 * High-level combat hook. Calls the backend and synchronises the Zustand
 * combat slice. The 280ms cross-fade entry transition is handled purely by
 * CSS (`.vtt-combat-overlay.active` in `src/styles/combat.css`), not here.
 *
 * M3 will replace the local `crypto.randomUUID()` encounter id with the one
 * returned by the backend's `combat_started` SSE event so the frontend and
 * backend stay in sync from the first turn.
 */
export function useCombat() {
  const combat = useStore((s) => s.combat);

  const begin = async (
    campaignId: string,
    sessionId: string,
    entries: InitiativeEntryPayload[],
  ) => {
    const resp = await startCombat({
      campaign_id: campaignId,
      session_id: sessionId,
      initiative_entries: entries,
    });
    if (!resp.ok) throw new Error(`startCombat failed: ${resp.status}`);
    const tokens = entries.map((e) => ({
      id: e.id,
      name: e.name,
      hp: e.hp,
      maxHp: e.max_hp,
      ac: e.ac,
      x: 0,
      y: 0,
      conditions: [] as string[],
    }));
    const encounterId = crypto.randomUUID();
    combat.startCombat(encounterId, tokens);
  };

  const end = async () => {
    if (!combat.encounterId) return;
    await endCombat({ encounter_id: combat.encounterId });
    combat.endCombat();
  };

  return {
    active: combat.active,
    tokens: combat.tokens,
    initiativeOrder: combat.initiativeOrder,
    currentTurnId: combat.currentTurnId,
    round: combat.round,
    begin,
    end,
  };
}
