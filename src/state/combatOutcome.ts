import { parseCombatProjection } from '../features/combat/model/combatProjection';
import { combatProjectionState } from './combat';
import { useStore } from './useStore';

/** Reconcile a confirmed bound actor without performing full restoration. */
export function applyCombatOutcome(value: unknown): boolean {
  const projection = parseCombatProjection(value);
  if (!projection) return false;
  const state = useStore.getState();
  const scope = projection.campaign_scope;
  if (!scope) {
    if (projection.snapshot.combatants.some((actor) => actor.character_id)) return false;
    return state.combat.replaceProjection(projection);
  }
  const campaign = state.campaigns.active;
  const heroes = projection.snapshot.combatants.filter(
    (actor) => actor.character_id === scope.character_id,
  );
  if (
    !campaign ||
    campaign.id !== scope.campaign_id ||
    state.session.activeSessionId !== scope.session_id ||
    state.pc.characterId !== scope.character_id ||
    scope.revision < campaign.revision ||
    heroes.length !== 1
  )
    return false;
  const hero = heroes[0];
  if (!hero || hero.max_hp <= 0 || hero.current_hp < 0 || hero.current_hp > hero.max_hp)
    return false;
  if (
    state.combat.encounterId === projection.encounter_id &&
    projection.revision <= state.combat.revision
  )
    return false;
  useStore.setState((current) => ({
    combat: { ...current.combat, ...combatProjectionState(projection) },
    pc: {
      ...current.pc,
      hp: hero.current_hp,
      hpMax: hero.max_hp,
      ac: hero.ac,
      speedFt: hero.speed_ft,
    },
    campaigns: { ...current.campaigns, active: { ...campaign, revision: scope.revision } },
  }));
  return true;
}
