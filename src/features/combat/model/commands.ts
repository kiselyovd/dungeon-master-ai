import { backendUrl } from '../../../api/client';
import { ChatError } from '../../../api/errors';
import { SseStreamDecoder } from '../../../api/sseStream';
import { parseCombatProjection } from './combatProjection';
import type { CombatProjectionDto } from './types';

export type CombatCommand =
  | { kind: 'spell'; combatantId: string; spellId: string; targetIds: string[]; slotLevel: number }
  | {
      kind: 'attack';
      attackerId: string;
      targetId: string;
      attackModifier: number;
      damageDice: string;
      damageType: string;
      weaponId?: string;
      mode?: 'default' | 'melee' | 'thrown' | 'ranged';
      twoHanded?: boolean;
    }
  | { kind: 'cast'; combatantId: string }
  | { kind: 'move'; combatantId: string; x: number; y: number }
  | { kind: 'end_turn' | 'retreat'; combatantId: string };

function commandArgs(command: CombatCommand): Record<string, unknown> {
  switch (command.kind) {
    case 'spell':
      return {
        caster_id: command.combatantId,
        spell_id: command.spellId,
        target_ids: command.targetIds,
        slot_level: command.slotLevel,
      };
    case 'attack':
      return {
        attacker_id: command.attackerId,
        target_id: command.targetId,
        ...(command.weaponId ? { weapon_id: command.weaponId } : {}),
        ...(command.mode ? { mode: command.mode } : {}),
        ...(command.twoHanded !== undefined ? { two_handed: command.twoHanded } : {}),
      };
    case 'cast':
    case 'end_turn':
    case 'retreat':
      return { combatant_id: command.combatantId };
    case 'move':
      return { combatant_id: command.combatantId, x: command.x, y: command.y };
  }
}

export async function sendCombatCommand(input: {
  encounterId: string;
  revision: number;
  commandId: string;
  command: CombatCommand;
  signal?: AbortSignal;
}): Promise<CombatProjectionDto> {
  const response = await fetch(await backendUrl('/combat/action'), {
    method: 'POST',
    headers: { 'content-type': 'application/json', accept: 'text/event-stream' },
    body: JSON.stringify({
      encounter_id: input.encounterId,
      action_type: input.command.kind === 'attack' ? 'weapon_attack' : input.command.kind,
      args: commandArgs(input.command),
      request_id: input.commandId,
      expected_revision: input.revision,
    }),
    ...(input.signal ? { signal: input.signal } : {}),
  }).catch((error: unknown) => {
    throw ChatError.from(error);
  });
  if (!response.ok || !response.body) throw new ChatError('http_error', `HTTP ${response.status}`);

  const decoder = new SseStreamDecoder();
  let projection: CombatProjectionDto | null = null;
  const bytes = new Uint8Array(await response.arrayBuffer());
  for (const event of [...decoder.push(bytes), ...decoder.finish()]) {
    if (event.event !== 'combat_projection' || !isRecord(event.data)) continue;
    projection = parseCombatProjection(event.data.projection);
  }
  if (!projection) throw new ChatError('invalid_response', 'combat projection missing');
  return projection;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}
