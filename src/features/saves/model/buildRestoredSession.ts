import type { RestoreSaveResponse, SessionMessageWire } from '../../../api/saves';
import type { ChatMessage, ChatRole, MessagePart } from '../../../state/chat';
import type { CombatProjectionDto } from '../../../state/combat';
import type { JournalEntry } from '../../../state/journal';
import { DISPOSITIONS, type NpcRecord } from '../../../state/npc';
import type { PcData } from '../../../state/pc';
import type { CurrentScene } from '../../../state/session';

export const RESTORE_MESSAGE_LIMIT = 20;
const RENDERABLE_ROLES = new Set<string>(['user', 'assistant', 'system']);

export interface RestoredSessionProjection {
  campaignId: string;
  sessionId: string;
  revision: number;
  messages: ChatMessage[];
  pc: Partial<PcData>;
  combat: CombatProjectionDto | null;
  scene: CurrentScene | null;
  journal: JournalEntry[];
  npcs: NpcRecord[];
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
  sessionId: string;
  expectedRevision: number;
  restored: RestoreSaveResponse;
}): RestoredSessionProjection {
  if (input.restored.schema_version !== 1)
    throw new RestoreValidationError('restore_version_unsupported');
  if (input.restored.ruleset_id !== 'srd-5.2.1')
    throw new RestoreValidationError('restore_ruleset_unsupported');
  if (
    input.restored.campaign_id !== input.campaignId ||
    input.restored.session_id !== input.sessionId ||
    input.restored.revision > input.expectedRevision
  ) {
    throw new RestoreValidationError('restore_identity_mismatch');
  }
  const projection = input.restored.projection;
  const renderable = projection.messages
    .slice(-RESTORE_MESSAGE_LIMIT)
    .filter((message) => RENDERABLE_ROLES.has(message.role));
  return {
    campaignId: input.restored.campaign_id,
    sessionId: input.restored.session_id,
    revision: input.restored.revision,
    messages: renderable.map((message, index) =>
      restoredMessage(message, `rehydrated-${input.saveId}-${index}`, index),
    ),
    pc: parsePcPatch(projection.character),
    combat: projection.combat === null ? null : parseCombat(projection.combat),
    scene: projection.scene === null ? null : parseScene(projection.scene),
    journal: projection.journal.map(parseJournal),
    npcs: projection.npcs.map(parseNpc),
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

function parseCombat(value: unknown): CombatProjectionDto {
  const combat = asRecord(value, 'restore_combat_invalid');
  const snapshot = asRecord(combat.snapshot, 'restore_combat_snapshot_invalid');
  if (!Array.isArray(snapshot.initiative) || !Array.isArray(snapshot.combatants)) {
    throw new RestoreValidationError('restore_combat_invalid');
  }
  if (!Array.isArray(combat.events)) throw new RestoreValidationError('restore_combat_invalid');
  return {
    schema_version: asNumber(combat.schema_version, 'restore_combat_invalid'),
    encounter_id: asString(combat.encounter_id, 'restore_combat_invalid'),
    revision: asNumber(combat.revision, 'restore_combat_invalid'),
    snapshot: {
      active: asBoolean(snapshot.active, 'restore_combat_invalid'),
      round: asNumber(snapshot.round, 'restore_combat_invalid'),
      current_combatant:
        snapshot.current_combatant === null
          ? null
          : asString(snapshot.current_combatant, 'restore_combat_invalid'),
      initiative: snapshot.initiative.map((value) => {
        const entry = asRecord(value, 'restore_combat_invalid');
        return {
          id: asString(entry.id, 'restore_combat_invalid'),
          roll: asNumber(entry.roll, 'restore_combat_invalid'),
          dex_tiebreak: asNumber(entry.dex_tiebreak, 'restore_combat_invalid'),
        };
      }),
      combatants: snapshot.combatants.map((value) => {
        const combatant = asRecord(value, 'restore_combat_invalid');
        const budget = asRecord(combatant.budget, 'restore_combat_invalid');
        const position = asRecord(combatant.position, 'restore_combat_invalid');
        return {
          id: asString(combatant.id, 'restore_combat_invalid'),
          name: asString(combatant.name, 'restore_combat_invalid'),
          max_hp: asNumber(combatant.max_hp, 'restore_combat_invalid'),
          current_hp: asNumber(combatant.current_hp, 'restore_combat_invalid'),
          temp_hp: asNumber(combatant.temp_hp, 'restore_combat_invalid'),
          ac: asNumber(combatant.ac, 'restore_combat_invalid'),
          speed_ft: asNumber(combatant.speed_ft, 'restore_combat_invalid'),
          initiative_roll: asNumber(combatant.initiative_roll, 'restore_combat_invalid'),
          dex_mod: asNumber(combatant.dex_mod, 'restore_combat_invalid'),
          conditions: stringArray(combatant.conditions, 'restore_combat_invalid'),
          budget: {
            action: asBoolean(budget.action, 'restore_combat_invalid'),
            bonus_action: asBoolean(budget.bonus_action, 'restore_combat_invalid'),
            reaction: asBoolean(budget.reaction, 'restore_combat_invalid'),
            movement_ft: asNumber(budget.movement_ft, 'restore_combat_invalid'),
          },
          is_dead: asBoolean(combatant.is_dead, 'restore_combat_invalid'),
          position: {
            x: asNumber(position.x, 'restore_combat_invalid'),
            y: asNumber(position.y, 'restore_combat_invalid'),
          },
        };
      }),
    },
    events: combat.events,
  };
}

function parseJournal(value: unknown): JournalEntry {
  const entry = asRecord(value, 'restore_journal_invalid');
  return {
    id: asString(entry.id, 'restore_journal_invalid'),
    campaign_id: asString(entry.campaign_id, 'restore_journal_invalid'),
    chapter: entry.chapter === null ? null : asString(entry.chapter, 'restore_journal_invalid'),
    entry_html: asString(entry.entry_html, 'restore_journal_invalid'),
    created_at: asString(entry.created_at, 'restore_journal_invalid'),
  };
}

function parseNpc(value: unknown): NpcRecord {
  const npc = asRecord(value, 'restore_npc_invalid');
  const disposition = asString(npc.disposition, 'restore_npc_invalid');
  if (!DISPOSITIONS.includes(disposition as NpcRecord['disposition'])) {
    throw new RestoreValidationError('restore_npc_invalid');
  }
  if (!Array.isArray(npc.facts)) throw new RestoreValidationError('restore_npc_invalid');
  return {
    id: asString(npc.id, 'restore_npc_invalid'),
    campaign_id: asString(npc.campaign_id, 'restore_npc_invalid'),
    name: asString(npc.name, 'restore_npc_invalid'),
    role: asString(npc.role, 'restore_npc_invalid'),
    disposition: disposition as NpcRecord['disposition'],
    trust: asNumber(npc.trust, 'restore_npc_invalid'),
    facts: npc.facts.map((value) => {
      const fact = asRecord(value, 'restore_npc_invalid');
      return {
        text: asString(fact.text, 'restore_npc_invalid'),
        created_at: asString(fact.created_at, 'restore_npc_invalid'),
      };
    }),
    updated_at: asString(npc.updated_at, 'restore_npc_invalid'),
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

function stringArray(value: unknown, code: string): string[] {
  if (!Array.isArray(value)) throw new RestoreValidationError(code);
  return value.map((entry) => asString(entry, code));
}
