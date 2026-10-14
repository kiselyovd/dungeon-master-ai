import type { CombatToken } from './types';

export interface CombatMotionFrame {
  scopeKey: string;
  revision: number;
  round: number;
  active: boolean;
  ownActorId: string | null;
  currentTurnId: string | null;
  tokens: readonly CombatToken[];
  events?: readonly unknown[];
}
export interface ConfirmedCombatEffect {
  id: string;
  actorId: string;
  kind: 'damage' | 'heal' | 'resource' | 'own_turn' | 'move';
  amount?: number;
}

function resourceSpent(before: CombatToken, after: CombatToken): boolean {
  if (
    (before.actionAvailable === true && after.actionAvailable === false) ||
    (before.bonusAvailable === true && after.bonusAvailable === false) ||
    (before.reactionAvailable === true && after.reactionAvailable === false) ||
    (before.movementRemaining !== undefined &&
      after.movementRemaining !== undefined &&
      after.movementRemaining < before.movementRemaining)
  )
    return true;
  for (const [id, quantity] of Object.entries(before.loadout?.quantities ?? {})) {
    const remaining = after.loadout?.quantities[id];
    if (remaining !== undefined && remaining < quantity) return true;
  }
  if (before.loadout && after.loadout && after.loadout.arrows < before.loadout.arrows) return true;
  return Object.entries(before.casting?.slots ?? {}).some(([level, slot]) => {
    const remaining = after.casting?.slots[level]?.remaining;
    return remaining !== undefined && remaining < slot.remaining;
  });
}

/** Presentation cues from two observed canonical projections. Hydration, replay,
 * changed ownership/timeline and gaps establish a baseline without celebration. */
export function confirmedCombatEffects(
  before: CombatMotionFrame | null,
  after: CombatMotionFrame,
): ConfirmedCombatEffect[] {
  if (
    !before?.active ||
    !after.active ||
    before.scopeKey !== after.scopeKey ||
    before.ownActorId !== after.ownActorId ||
    !Number.isSafeInteger(before.revision) ||
    !Number.isSafeInteger(after.revision) ||
    before.revision < 0 ||
    !Number.isSafeInteger(before.round) ||
    !Number.isSafeInteger(after.round) ||
    after.revision !== before.revision + 1
  )
    return [];
  const previous = new Map(before.tokens.map((token) => [token.id, token]));
  const effects: ConfirmedCombatEffect[] = [];
  const absorbedDamage = new Map<string, number>();
  for (const group of after.events ?? []) {
    if (!group || typeof group !== 'object' || !('damage' in group) || !Array.isArray(group.damage))
      continue;
    for (const damage of group.damage) {
      if (
        !damage ||
        typeof damage !== 'object' ||
        typeof damage.target !== 'string' ||
        !Number.isSafeInteger(damage.effective_amount) ||
        damage.effective_amount <= 0 ||
        !Number.isSafeInteger(damage.new_hp)
      )
        continue;
      const target = after.tokens.find((token) => token.id === damage.target);
      if (!target || damage.new_hp !== target.hp) continue;
      const amount = (absorbedDamage.get(damage.target) ?? 0) + damage.effective_amount;
      if (Number.isSafeInteger(amount)) absorbedDamage.set(damage.target, amount);
    }
  }
  function add(actorId: string, kind: ConfirmedCombatEffect['kind'], amount?: number) {
    effects.push({
      id: JSON.stringify([after.scopeKey, after.revision, actorId, kind]),
      actorId,
      kind,
      ...(amount === undefined ? {} : { amount }),
    });
  }
  for (const token of after.tokens) {
    const old = previous.get(token.id);
    if (!old || old.characterId !== token.characterId) continue;
    if (Number.isFinite(old.hp) && Number.isFinite(token.hp)) {
      if (token.hp < old.hp) add(token.id, 'damage', old.hp - token.hp);
      else if (token.hp > old.hp) add(token.id, 'heal', token.hp - old.hp);
      else if (absorbedDamage.has(token.id)) add(token.id, 'damage', absorbedDamage.get(token.id));
    }
    if (resourceSpent(old, token)) add(token.id, 'resource');
    if (old.x !== token.x || old.y !== token.y) add(token.id, 'move');
    if (
      token.id === after.ownActorId &&
      token.id === after.currentTurnId &&
      (before.currentTurnId !== after.currentTurnId || after.round > before.round) &&
      !token.isDead &&
      token.hp > 0
    )
      add(token.id, 'own_turn');
  }
  return effects;
}
