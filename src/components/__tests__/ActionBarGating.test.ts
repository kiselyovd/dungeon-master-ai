/**
 * Tests for the W1.6 ActionBar gating logic.
 *
 * The logic lives in App.tsx but is too heavy to mount in a unit test.
 * Instead we test the derivation directly: given a token list, pc.name and
 * currentTurnId, compute showActionBar the same way App.tsx does.
 */
import { describe, expect, it } from 'vitest';
import type { CombatToken } from '../../state/combat';

/**
 * Pure helper that mirrors the App.tsx showActionBar derivation:
 *   pcTokenId = tokens.find(t => pcName !== null && t.name === pcName)?.id ?? null
 *   showActionBar = combatActive && pcTokenId !== null && currentTurnId === pcTokenId
 */
function computeShowActionBar(
  combatActive: boolean,
  tokens: CombatToken[],
  pcName: string | null,
  currentTurnId: string | null,
): boolean {
  const pcTokenId = tokens.find((t) => pcName !== null && t.name === pcName)?.id ?? null;
  return combatActive && pcTokenId !== null && currentTurnId === pcTokenId;
}

const HERO: CombatToken = {
  id: 'pc-1',
  name: 'Aria',
  hp: 20,
  maxHp: 20,
  ac: 16,
  x: 0,
  y: 0,
  conditions: [],
};
const GOBLIN: CombatToken = {
  id: 'npc-1',
  name: 'Goblin',
  hp: 7,
  maxHp: 7,
  ac: 13,
  x: 3,
  y: 2,
  conditions: [],
};

describe('ActionBar gating (W1.6)', () => {
  it('shows when combat is active and it is the PC turn', () => {
    expect(computeShowActionBar(true, [HERO, GOBLIN], 'Aria', 'pc-1')).toBe(true);
  });

  it('hides when it is an enemy turn', () => {
    expect(computeShowActionBar(true, [HERO, GOBLIN], 'Aria', 'npc-1')).toBe(false);
  });

  it('hides when combat is not active even if it would be the PC turn', () => {
    expect(computeShowActionBar(false, [HERO, GOBLIN], 'Aria', 'pc-1')).toBe(false);
  });

  it('hides when currentTurnId is null (no active turn yet)', () => {
    expect(computeShowActionBar(true, [HERO, GOBLIN], 'Aria', null)).toBe(false);
  });

  it('hides when pc.name is null (no PC created yet)', () => {
    expect(computeShowActionBar(true, [HERO, GOBLIN], null, 'pc-1')).toBe(false);
  });

  it('hides when no token name matches pc.name (lookup miss)', () => {
    expect(computeShowActionBar(true, [HERO, GOBLIN], 'Unknown Hero', 'pc-1')).toBe(false);
  });

  it('hides when token list is empty', () => {
    expect(computeShowActionBar(true, [], 'Aria', 'pc-1')).toBe(false);
  });
});
