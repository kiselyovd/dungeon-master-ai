import type { StoreApi } from 'zustand';
import type { CombatToken } from '../state/combat';
import type { AppState } from '../state/useStore';

type ToolArgs = Record<string, unknown>;
type ToolResult = Record<string, unknown>;

export type CombatToolHandler = (
  args: ToolArgs,
  result: ToolResult,
  store: StoreApi<AppState>,
) => void;

/**
 * Handler map for combat SSE tool results. Each key MUST match a tool
 * name emitted by the backend agent loop (crates/app-server/src/agent/
 * tools.rs). useAgentTurn delegates here so the agent-turn hook stays
 * free of combat-specific logic.
 *
 * Contract: this map's keys are exactly the backend combat tools that
 * mutate the VTT. src/hooks/__tests__/useCombatToolHandlers.test.ts
 * enforces it.
 */
export const combatToolHandlers: Record<string, CombatToolHandler | undefined> = {
  start_combat: (args, result, store) => {
    const entries = Array.isArray(args.initiative_entries) ? args.initiative_entries : [];
    const tokens: CombatToken[] = entries
      .filter((e): e is Record<string, unknown> => e !== null && typeof e === 'object')
      .map((e, i) => ({
        id: String(e.id ?? crypto.randomUUID()),
        name: String(e.name ?? 'Unknown'),
        hp: Number(e.hp ?? 1),
        maxHp: Number(e.max_hp ?? Number(e.hp ?? 1)),
        ac: Number(e.ac ?? 10),
        // The backend initiative entry carries no grid position; lay
        // combatants out left to right, 8 per row, as a sane default.
        x: i % 8,
        y: Math.floor(i / 8),
        conditions: [],
      }));
    const encounterId = String(result.encounter_id ?? crypto.randomUUID());
    store.getState().combat.startCombat(encounterId, tokens);
  },

  end_combat: (_args, _result, store) => {
    store.getState().combat.endCombat();
  },

  apply_damage: (args, _result, store) => {
    const tokenId = String(args.token_id ?? '');
    const amount = Number(args.amount ?? 0);
    if (!tokenId || amount <= 0) return;
    store.getState().combat.applyDamage(tokenId, amount);
  },

  apply_healing: (args, _result, store) => {
    const tokenId = String(args.token_id ?? '');
    const amount = Number(args.amount ?? 0);
    if (!tokenId || amount <= 0) return;
    store.getState().combat.applyHealing(tokenId, amount);
  },

  add_token: (args, _result, store) => {
    const id = String(args.id ?? '');
    if (!id) return;
    store.getState().combat.addToken({
      id,
      name: String(args.name ?? 'Unknown'),
      hp: Number(args.hp ?? 1),
      maxHp: Number(args.max_hp ?? Number(args.hp ?? 1)),
      ac: Number(args.ac ?? 10),
      x: Number(args.x ?? 0),
      y: Number(args.y ?? 0),
      conditions: [],
    });
  },

  update_token: (args, _result, store) => {
    const id = String(args.id ?? '');
    if (!id) return;
    const patch: Partial<CombatToken> = {};
    if (args.x !== undefined) patch.x = Number(args.x);
    if (args.y !== undefined) patch.y = Number(args.y);
    if (args.hp !== undefined) patch.hp = Number(args.hp);
    if (Array.isArray(args.conditions)) {
      patch.conditions = (args.conditions as unknown[]).map(String);
    }
    store.getState().combat.updateToken(id, patch);
  },

  remove_token: (args, _result, store) => {
    const id = String(args.id ?? '');
    if (!id) return;
    store.getState().combat.removeToken(id);
  },
};
