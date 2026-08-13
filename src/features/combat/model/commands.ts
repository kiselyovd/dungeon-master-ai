import { backendUrl } from '../../../api/client';
import { ChatError } from '../../../api/errors';
import { SseStreamDecoder } from '../../../api/sseStream';
import { parseCombatProjection } from './combatProjection';
import type { CombatProjectionDto } from './types';

export type CombatCommand =
  | {
      kind: 'attack';
      attackerId: string;
      targetId: string;
      attackModifier: number;
      damageDice: string;
      damageType: string;
    }
  | { kind: 'cast'; combatantId: string }
  | { kind: 'move'; combatantId: string; x: number; y: number }
  | { kind: 'end_turn'; combatantId: string };

function commandArgs(command: CombatCommand): Record<string, unknown> {
  switch (command.kind) {
    case 'attack':
      return {
        attacker_id: command.attackerId,
        target_id: command.targetId,
        attack_modifier: command.attackModifier,
        damage_dice: command.damageDice,
        damage_type: command.damageType,
      };
    case 'cast':
    case 'end_turn':
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
      action_type: input.command.kind,
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
  const reader = response.body.getReader();
  let projection: CombatProjectionDto | null = null;
  try {
    read: while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      for (const event of decoder.push(value)) {
        if (event.event !== 'combat_projection' || !isRecord(event.data)) continue;
        projection = parseCombatProjection(event.data.projection);
        break read;
      }
    }
    if (!projection) {
      for (const event of decoder.finish()) {
        if (event.event !== 'combat_projection' || !isRecord(event.data)) continue;
        projection = parseCombatProjection(event.data.projection);
      }
    }
  } finally {
    if (projection) await reader.cancel();
    reader.releaseLock();
  }
  if (!projection) throw new ChatError('invalid_response', 'combat projection missing');
  return projection;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}
