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
  characterId?: string | null,
): boolean {
  const token = activeCombatToken(tokens, currentTurnId);
  return token ? isPlayerToken(token, characterId, playerName) : false;
}

export function isPlayerToken(
  token: CombatToken,
  characterId: string | null | undefined,
  playerName: string | null,
): boolean {
  return characterId
    ? token.characterId === characterId
    : playerName !== null && token.name === playerName;
}

/** Modern projections carry authoritative mortality; legacy unbound zero-HP monsters retain their prior display. */
export function isDeadToken(token: CombatToken): boolean {
  return token.isDead ?? (token.hp <= 0 && !token.characterId);
}
export function isIncapacitatedToken(token: CombatToken): boolean {
  return (
    token.hp <= 0 ||
    isDeadToken(token) ||
    token.conditions.includes('unconscious') ||
    token.conditions.includes('stunned')
  );
}
