import type { RestoreSaveResponse, SaveRow, SessionMessageWire } from '../../../api/saves';
import type { ChatMessage, ChatRole, MessagePart } from '../../../state/chat';
import type { SnapshotCombat } from '../../../state/combat';
import type { PcData } from '../../../state/pc';
import type { CurrentScene } from '../../../state/session';

export const RESTORE_MESSAGE_LIMIT = 20;
const RENDERABLE_ROLES = new Set<string>(['user', 'assistant', 'system']);

export interface RestoredSessionProjection {
  campaignId: string;
  sessionId: string;
  messages: ChatMessage[];
  pc: Partial<PcData> | null;
  combat: SnapshotCombat | null;
  scene: CurrentScene | null;
}

export class RestoreValidationError extends Error {
  constructor(readonly code: string) {
    super(code);
    this.name = 'RestoreValidationError';
  }
}

export function buildRestoredSession(input: {
  saveId: string;
  campaignId: string;
  row: SaveRow;
  restored: RestoreSaveResponse;
}): RestoredSessionProjection {
  const gameState = asRecord(input.restored.game_state, 'restore_game_state_invalid');
  if (gameState.schema_version !== 2)
    throw new RestoreValidationError('restore_version_unsupported');

  const renderable = input.restored.messages
    .slice(-RESTORE_MESSAGE_LIMIT)
    .filter((message) => RENDERABLE_ROLES.has(message.role));
  const messages = renderable.map((message, index) =>
    restoredMessage(message, `rehydrated-${input.saveId}-${index}`, index),
  );

  const rowState = optionalRecord(input.row.game_state, 'save_game_state_invalid');
  return {
    campaignId: input.campaignId,
    sessionId: input.row.session_id,
    messages,
    pc: rowState?.pc_snapshot === undefined ? null : parsePcPatch(rowState.pc_snapshot),
    combat: gameState.combat == null ? null : parseCombat(gameState.combat),
    scene: gameState.scene == null ? null : parseScene(gameState.scene),
  };
}

function restoredMessage(
  message: SessionMessageWire,
  id: string,
  sequenceIndex: number,
): ChatMessage {
  const role = message.role as ChatRole;
  const content =
    message.content ??
    message.parts
      ?.filter((part) => part.type === 'text')
      .map((part) => part.text ?? '')
      .join('') ??
    '';
  const parts = message.parts?.map(parsePart).filter((part): part is MessagePart => part !== null);
  return {
    id,
    role,
    content,
    sequenceIndex,
    ...(parts && parts.length > 0 ? { parts } : {}),
  };
}

function parsePart(part: NonNullable<SessionMessageWire['parts']>[number]): MessagePart | null {
  if (part.type === 'text' && part.text !== undefined) return { type: 'text', text: part.text };
  if (part.type !== 'image' || part.mime === undefined || part.data_b64 === undefined) return null;
  return {
    type: 'image',
    mime: part.mime,
    data_b64: part.data_b64,
    ...(part.name != null ? { name: part.name } : {}),
  };
}

function parseScene(value: unknown): CurrentScene {
  const scene = asRecord(value, 'restore_scene_invalid');
  return { name: asString(scene.title, 'restore_scene_title_invalid'), stepCounter: 0 };
}

function parseCombat(value: unknown): SnapshotCombat {
  const combat = asRecord(value, 'restore_combat_invalid');
  if (!Array.isArray(combat.initiative) || !Array.isArray(combat.tokens)) {
    throw new RestoreValidationError('restore_combat_invalid');
  }
  return {
    active: asBoolean(combat.active, 'restore_combat_active_invalid'),
    encounter_id: asString(combat.encounter_id, 'restore_combat_encounter_invalid'),
    round: asNumber(combat.round, 'restore_combat_round_invalid'),
    current_turn_id:
      combat.current_turn_id === null
        ? null
        : asString(combat.current_turn_id, 'restore_combat_turn_invalid'),
    initiative: combat.initiative.map((id) => asString(id, 'restore_combat_initiative_invalid')),
    tokens: combat.tokens.map((value) => {
      const token = asRecord(value, 'restore_combat_token_invalid');
      if (!Array.isArray(token.conditions)) {
        throw new RestoreValidationError('restore_combat_conditions_invalid');
      }
      return {
        id: asString(token.id, 'restore_combat_token_id_invalid'),
        name: asString(token.name, 'restore_combat_token_name_invalid'),
        hp: asNumber(token.hp, 'restore_combat_token_hp_invalid'),
        max_hp: asNumber(token.max_hp, 'restore_combat_token_max_hp_invalid'),
        ac: asNumber(token.ac, 'restore_combat_token_ac_invalid'),
        x: asNumber(token.x, 'restore_combat_token_x_invalid'),
        y: asNumber(token.y, 'restore_combat_token_y_invalid'),
        conditions: token.conditions.map((item) =>
          asString(item, 'restore_combat_condition_invalid'),
        ),
        resistances: stringArray(token.resistances, 'restore_combat_resistances_invalid'),
        immunities: stringArray(token.immunities, 'restore_combat_immunities_invalid'),
        vulnerabilities: stringArray(
          token.vulnerabilities,
          'restore_combat_vulnerabilities_invalid',
        ),
      };
    }),
  };
}

function parsePcPatch(value: unknown): Partial<PcData> {
  const input = asRecord(value, 'restore_pc_invalid');
  const output: Partial<PcData> = {};
  const nullableStrings = [
    'heroClass',
    'name',
    'race',
    'subclass',
    'background',
    'alignment',
    'portraitUrl',
  ] as const;
  for (const key of nullableStrings) {
    const candidate = input[key];
    if (candidate !== undefined) {
      if (candidate !== null && typeof candidate !== 'string') {
        throw new RestoreValidationError('restore_pc_invalid');
      }
      output[key] = candidate;
    }
  }
  const numbers = [
    'level',
    'experience',
    'experienceNext',
    'hp',
    'hpMax',
    'ac',
    'initiative',
    'speedFt',
    'proficiencyBonus',
  ] as const;
  for (const key of numbers) {
    if (input[key] !== undefined) output[key] = asNumber(input[key], 'restore_pc_invalid');
  }
  if (input.inventory !== undefined) {
    if (!Array.isArray(input.inventory)) throw new RestoreValidationError('restore_pc_invalid');
    output.inventory = input.inventory.map((itemValue) => {
      const item = asRecord(itemValue, 'restore_pc_invalid');
      return {
        id: asString(item.id, 'restore_pc_invalid'),
        name: asString(item.name, 'restore_pc_invalid'),
        count: asNumber(item.count, 'restore_pc_invalid'),
        ...(item.icon === undefined ? {} : { icon: asString(item.icon, 'restore_pc_invalid') }),
      };
    });
  }
  for (const key of ['abilities', 'savingThrowProfs', 'skillProfs'] as const) {
    if (input[key] !== undefined) output[key] = asRecord(input[key], 'restore_pc_invalid') as never;
  }
  return output;
}

function stringArray(value: unknown, code: string): string[] {
  if (value === undefined) return [];
  if (!Array.isArray(value)) throw new RestoreValidationError(code);
  return value.map((item) => asString(item, code));
}

function optionalRecord(value: unknown, code: string): Record<string, unknown> | null {
  if (value == null) return null;
  return asRecord(value, code);
}

function asRecord(value: unknown, code: string): Record<string, unknown> {
  if (value === null || typeof value !== 'object' || Array.isArray(value)) {
    throw new RestoreValidationError(code);
  }
  return value as Record<string, unknown>;
}

function asString(value: unknown, code: string): string {
  if (typeof value !== 'string') throw new RestoreValidationError(code);
  return value;
}

function asNumber(value: unknown, code: string): number {
  if (typeof value !== 'number' || !Number.isFinite(value)) throw new RestoreValidationError(code);
  return value;
}

function asBoolean(value: unknown, code: string): boolean {
  if (typeof value !== 'boolean') throw new RestoreValidationError(code);
  return value;
}
