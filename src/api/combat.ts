import { backendUrl } from './client';

export interface InitiativeEntryPayload {
  id: string;
  name: string;
  roll: number;
  dex_mod: number;
  hp: number;
  max_hp: number;
  ac: number;
}

export interface StartCombatPayload {
  campaign_id: string;
  session_id: string;
  initiative_entries: InitiativeEntryPayload[];
}

export interface CombatActionPayload {
  encounter_id: string;
  action_type: string;
  args: unknown;
}

export interface EndCombatPayload {
  encounter_id: string;
}

async function postCombat(path: string, body: unknown): Promise<Response> {
  const url = await backendUrl(path);
  return fetch(url, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(body),
  });
}

export function startCombat(payload: StartCombatPayload): Promise<Response> {
  return postCombat('/combat/start', payload);
}

export function postCombatAction(payload: CombatActionPayload): Promise<Response> {
  return postCombat('/combat/action', payload);
}

export function endCombat(payload: EndCombatPayload): Promise<Response> {
  return postCombat('/combat/end', payload);
}
