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
    const pc = store.getState().pc;
    const tokens: CombatToken[] = entries
      .filter((e): e is Record<string, unknown> => e !== null && typeof e === 'object')
      .map((e) => {
        const name = String(e.name ?? 'Unknown');
        const isPc = pc.name !== null && name === pc.name;
        // The model often calls start_combat with just names. Give the player's
        // own entry their real sheet, and enemies a sane monster default (HP 11,
        // AC 13) instead of the old trivial HP 1 that died in a single hit.
        const hp = Number(e.hp ?? (isPc ? pc.hp : 11));
        return {
          id: String(e.id ?? crypto.randomUUID()),
          name,
          hp,
          maxHp: Number(e.max_hp ?? (isPc ? pc.hpMax : hp)),
          ac: Number(e.ac ?? (isPc ? pc.ac : 13)),
          x: 0,
          y: 0,
          conditions: [],
        };
      });

    // The DM frequently omits the player's hero from start_combat. Always put
    // the PC on the board (front of initiative) with their real HP/AC so the
    // map and the board snapshot the model sees are never missing the player.
    if (pc.name !== null && !tokens.some((t) => t.name === pc.name)) {
      tokens.unshift({
        id: 'pc',
        name: pc.name,
        hp: pc.hp,
        maxHp: pc.hpMax,
        ac: pc.ac,
        x: 0,
        y: 0,
        conditions: [],
      });
    }

    // Lay combatants out left to right (8 per row) and mark the first as the
    // active turn so the initiative tracker + board snapshot agree on round 1.
    tokens.forEach((t, i) => {
      t.x = i % 8;
      t.y = Math.floor(i / 8);
      t.isActive = i === 0;
    });

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
