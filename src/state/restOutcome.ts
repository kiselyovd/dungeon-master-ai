import type { RestStatus } from '../api/contracts/rest';
import { useStore } from './useStore';
export function applyRestOutcome(status: RestStatus): boolean {
  const current = useStore.getState();
  const campaign = current.campaigns.active;
  if (
    !campaign ||
    current.combat.active ||
    campaign.id !== status.campaign_id ||
    current.session.activeSessionId !== status.session_id ||
    current.pc.characterId !== status.character_id ||
    status.revision < campaign.revision ||
    status.hp < 0 ||
    status.hp > status.max_hp
  )
    return false;
  useStore.setState((state) => ({
    pc: { ...state.pc, hp: status.hp, hpMax: status.max_hp },
    campaigns: { ...state.campaigns, active: { ...campaign, revision: status.revision } },
  }));
  return true;
}
