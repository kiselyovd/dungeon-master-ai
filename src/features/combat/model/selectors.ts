import type { CombatToken } from './types';

export function activeCombatToken(
  tokens: CombatToken[],
  currentTurnId: string | null,
): CombatToken | undefined {
  return tokens.find((token) => token.id === currentTurnId);
}

export function isPlayerTurn(
  tokens: CombatToken[],
  currentTurnId: string | null,
  playerName: string | null,
): boolean {
  if (!playerName) return false;
  return activeCombatToken(tokens, currentTurnId)?.name === playerName;
}
