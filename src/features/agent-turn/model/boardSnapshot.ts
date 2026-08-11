import type { BoardCombatState } from './ports';

export function buildBoardSnapshot(
  combat: BoardCombatState,
  sceneName?: string,
): string | undefined {
  if (!combat.active || combat.tokens.length === 0) return undefined;

  const byId = new Map(combat.tokens.map((token) => [token.id, token]));
  const orderNames = combat.initiativeOrder
    .map((id) => byId.get(id))
    .filter((token) => token !== undefined)
    .map((token) => (token.isActive ? `${token.name} (current turn)` : token.name));
  const lines = combat.tokens.map((token) => {
    const status = token.hp <= 0 ? ' - DOWN' : '';
    const conditions =
      token.conditions.length > 0 ? `, conditions: ${token.conditions.join(', ')}` : '';
    return `- ${token.name}: HP ${token.hp}/${token.maxHp}, AC ${token.ac}, grid (${token.x},${token.y})${conditions}${status}`;
  });
  const header = [
    sceneName ? `Scene: ${sceneName}.` : null,
    `Round ${combat.round}.`,
    orderNames.length > 0 ? `Initiative order: ${orderNames.join(' -> ')}.` : null,
  ]
    .filter((line): line is string => line !== null)
    .join(' ');
  return `${header}\nGrid squares are 5 ft. Combatants:\n${lines.join('\n')}`;
}
