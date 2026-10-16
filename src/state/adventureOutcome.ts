import * as v from 'valibot';
import { AdventureToolOutcomeSchema } from '../api/contracts/adventure';
import { useStore } from './useStore';

/** Same-campaign tool projection: never invoke full restoration during narration. */
export function applyAdventureToolOutcome(value: unknown): boolean {
  const parsed = v.safeParse(AdventureToolOutcomeSchema, value);
  if (!parsed.success) return false;
  const outcome = parsed.output;
  const current = useStore.getState();
  const campaign = current.campaigns.active;
  if (
    !campaign ||
    campaign.id !== outcome.campaign_id ||
    current.session.activeSessionId !== outcome.session_id ||
    current.pc.characterId !== outcome.character_id ||
    outcome.revision < campaign.revision
  )
    return false;
  useStore.setState((state) => ({
    campaigns: { ...state.campaigns, active: { ...campaign, revision: outcome.revision } },
    session: { ...state.session, adventure: outcome.adventure },
    pc: {
      ...state.pc,
      inventory: outcome.inventory.map(({ icon, ...item }) =>
        icon === undefined ? item : { ...item, icon },
      ),
    },
  }));
  return true;
}
