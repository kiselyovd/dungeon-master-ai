/**
 * W1.5 - SRD condition effects engine.
 *
 * Maps the 14 standard SRD conditions to their mechanical effects on the
 * active combatant's turn. Only the subset that gates action economy and
 * movement is modelled here; advantage/disadvantage on rolls is DM-side.
 */

export interface ConditionEffect {
  /** Multiplier applied to base speed. 0 = cannot move. */
  movementMultiplier: number;
  /** True for incapacitated, stunned, paralyzed, unconscious, petrified. */
  preventsActions: boolean;
  /** True for incapacitated, stunned, paralyzed, unconscious, petrified. */
  preventsReactions: boolean;
}

/**
 * Per-condition mechanical effect table.
 * Keys are lowercase SRD condition names.
 * Conditions not in this table default to no mechanical restriction.
 */
const CONDITION_TABLE: Record<string, ConditionEffect> = {
  // Incapacitated is the base condition that many others subsume.
  incapacitated: { movementMultiplier: 0, preventsActions: true, preventsReactions: true },
  stunned: { movementMultiplier: 0, preventsActions: true, preventsReactions: true },
  paralyzed: { movementMultiplier: 0, preventsActions: true, preventsReactions: true },
  unconscious: { movementMultiplier: 0, preventsActions: true, preventsReactions: true },
  petrified: { movementMultiplier: 0, preventsActions: true, preventsReactions: true },
  // Movement locked but actions still available.
  grappled: { movementMultiplier: 0, preventsActions: false, preventsReactions: false },
  restrained: { movementMultiplier: 0, preventsActions: false, preventsReactions: false },
  // No hard mechanical lock on actions/reactions or movement in SRD 5.1;
  // advantages/disadvantages are narrative (handled by DM model).
  prone: { movementMultiplier: 1, preventsActions: false, preventsReactions: false },
  frightened: { movementMultiplier: 1, preventsActions: false, preventsReactions: false },
  poisoned: { movementMultiplier: 1, preventsActions: false, preventsReactions: false },
  blinded: { movementMultiplier: 1, preventsActions: false, preventsReactions: false },
  invisible: { movementMultiplier: 1, preventsActions: false, preventsReactions: false },
  dodging: { movementMultiplier: 1, preventsActions: false, preventsReactions: false },
  charmed: { movementMultiplier: 1, preventsActions: false, preventsReactions: false },
  deafened: { movementMultiplier: 1, preventsActions: false, preventsReactions: false },
  exhaustion: { movementMultiplier: 1, preventsActions: false, preventsReactions: false },
};

/** Defaults returned for an unknown condition (no restriction). */
const NO_EFFECT: ConditionEffect = {
  movementMultiplier: 1,
  preventsActions: false,
  preventsReactions: false,
};

/**
 * Combines a list of condition names into a single aggregate effect.
 * Combination rules:
 * - movementMultiplier: minimum across all conditions (most restrictive wins).
 * - preventsActions: OR of all (any one condition is enough to block).
 * - preventsReactions: OR of all.
 * Unknown condition names are silently ignored.
 */
export function aggregateConditionEffects(conditions: string[]): ConditionEffect {
  let movementMultiplier = 1;
  let preventsActions = false;
  let preventsReactions = false;

  for (const raw of conditions) {
    const entry = CONDITION_TABLE[raw.toLowerCase()];
    if (entry === undefined) continue;
    if (entry.movementMultiplier < movementMultiplier) {
      movementMultiplier = entry.movementMultiplier;
    }
    if (entry.preventsActions) preventsActions = true;
    if (entry.preventsReactions) preventsReactions = true;
  }

  return { movementMultiplier, preventsActions, preventsReactions };
}

/**
 * Convenience: look up a single condition. Returns NO_EFFECT for unknowns.
 */
export function getConditionEffect(condition: string): ConditionEffect {
  return CONDITION_TABLE[condition.toLowerCase()] ?? NO_EFFECT;
}
