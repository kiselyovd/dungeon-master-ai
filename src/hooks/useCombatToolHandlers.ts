import type { StoreApi } from 'zustand';
import type { AppState } from '../state/useStore';

type ToolArgs = Record<string, unknown>;
type ToolResult = Record<string, unknown>;

export type CombatToolHandler = (
  args: ToolArgs,
  result: ToolResult,
  store: StoreApi<AppState>,
) => void;

/**
 * Handler map for combat SSE tool results. Each key matches a tool_name
 * emitted by the backend agent loop. useAgentTurn delegates to this map
 * so the agent-turn hook stays free of combat-specific logic.
 *
 * To add a new combat tool: add a key here. No changes needed in useAgentTurn.
 */
export const combatToolHandlers: Record<string, CombatToolHandler | undefined> = {
  apply_damage: (args, _result, store) => {
    const tokenId = String(args['token_id'] ?? '');
    const amount = Number(args['amount'] ?? 0);
    if (!tokenId || amount <= 0) return;
    store.getState().combat.applyDamage(tokenId, amount);
  },

  apply_healing: (args, _result, store) => {
    const tokenId = String(args['token_id'] ?? '');
    const amount = Number(args['amount'] ?? 0);
    if (!tokenId || amount <= 0) return;
    store.getState().combat.applyHealing(tokenId, amount);
  },

  add_condition: (args, _result, store) => {
    const tokenId = String(args['token_id'] ?? '');
    const condition = String(args['condition'] ?? '');
    if (!tokenId || !condition) return;
    store.getState().combat.addCondition(tokenId, condition);
  },

  remove_condition: (args, _result, store) => {
    const tokenId = String(args['token_id'] ?? '');
    const condition = String(args['condition'] ?? '');
    if (!tokenId || !condition) return;
    store.getState().combat.removeCondition(tokenId, condition);
  },

  set_current_turn: (args, _result, store) => {
    const tokenId = String(args['token_id'] ?? '');
    if (!tokenId) return;
    store.getState().combat.setCurrentTurn(tokenId);
  },

  move_token: (args, _result, store) => {
    const tokenId = String(args['token_id'] ?? '');
    const x = Number(args['x'] ?? 0);
    const y = Number(args['y'] ?? 0);
    if (!tokenId) return;
    store.getState().combat.moveToken(tokenId, x, y);
  },

  start_combat: (args, _result, store) => {
    const encounterId = String(args['encounter_id'] ?? crypto.randomUUID());
    const rawTokens = Array.isArray(args['tokens']) ? args['tokens'] : [];
    const tokens = rawTokens
      .filter((t): t is Record<string, unknown> => t !== null && typeof t === 'object')
      .map((t) => ({
        id: String(t['id'] ?? crypto.randomUUID()),
        name: String(t['name'] ?? 'Unknown'),
        hp: Number(t['hp'] ?? 1),
        maxHp: Number(t['max_hp'] ?? Number(t['hp'] ?? 1)),
        ac: Number(t['ac'] ?? 10),
        x: Number(t['x'] ?? 0),
        y: Number(t['y'] ?? 0),
        conditions: Array.isArray(t['conditions'])
          ? (t['conditions'] as unknown[]).map(String)
          : [],
      }));
    store.getState().combat.startCombat(encounterId, tokens);
  },

  end_combat: (_args, _result, store) => {
    store.getState().combat.endCombat();
  },
};
