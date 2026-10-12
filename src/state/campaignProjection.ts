import * as v from 'valibot';
import type { Campaign } from '../api/contracts/campaign';
import type { CampaignStateResponse } from '../api/contracts/campaignState';
import { DefenseChoiceSchema } from '../api/contracts/defense';
import { MediaReferenceSchema } from '../api/contracts/mediaAsset';
import type { RestoreSaveResponse, SessionMessageWire } from '../api/saves';
import { parseMessage } from '../api/saves';
import { parseCombatProjection } from '../features/combat/model/combatProjection';
import type { ChatMessage, ChatRole, MessagePart } from './chat';
import type { CombatProjectionDto } from './combat';
import type { JournalEntry } from './journal';
import { DISPOSITIONS, type NpcRecord } from './npc';
import type { PcData } from './pc';
import type { CurrentScene } from './session';

export const RESTORE_MESSAGE_LIMIT = 20;
const RENDERABLE_ROLES = new Set<string>(['user', 'assistant', 'system']);

export interface RestoredSessionProjection {
  mapMediaRef?: import('../api/contracts/mediaAsset').MediaReference | null;
  adventure?: import('../api/contracts/adventure').AdventureProjection | null;
  preserveDraft?: boolean;
  campaign?: Campaign;
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

export function buildCampaignState(state: CampaignStateResponse): RestoredSessionProjection {
  if (state.session_id !== state.campaign.last_session_id)
    throw new RestoreValidationError('restore_identity_mismatch');
  const document = state.projection.character;
  if (
    state.character_id !== null &&
    (typeof document.heroClass !== 'string' ||
      !document.heroClass ||
      typeof document.name !== 'string' ||
      !document.name ||
      typeof document.hp !== 'number' ||
      !Number.isSafeInteger(document.hp) ||
      document.hp < 0 ||
      typeof document.hpMax !== 'number' ||
      document.hpMax <= 0 ||
      document.hp > document.hpMax ||
      !Array.isArray(document.inventory))
  )
    throw new RestoreValidationError('restore_pc_invalid');
  if (state.character_id === null && Object.keys(document).length !== 0)
    throw new RestoreValidationError('restore_identity_mismatch');
  const projection = buildRestoredSession({
    saveId: `campaign-${state.campaign.id}`,
    campaignId: state.campaign.id,
    sessionId: state.session_id,
    expectedRevision: state.campaign.revision,
    restored: {
      id: state.campaign.id,
      campaign_id: state.campaign.id,
      session_id: state.session_id,
      revision: state.campaign.revision,
      schema_version: 1,
      ruleset_id: 'srd-5.2.1',
      projection: {
        ...state.projection,
        map_media_ref: state.projection.map_media_ref ?? null,
        adventure: state.projection.adventure ?? null,
        messages: state.projection.messages.map(parseMessage),
      },
    },
  });
  projection.pc.characterId = state.character_id;
  return { ...projection, campaign: state.campaign };
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
  if (
    input.restored.current_revision !== undefined &&
    (!Number.isSafeInteger(input.restored.current_revision) ||
      input.restored.current_revision < input.expectedRevision)
  ) {
    throw new RestoreValidationError('restore_identity_mismatch');
  }
  if (input.restored.current_state) {
    const current = input.restored.current_state;
    if (
      current.campaign.id !== input.campaignId ||
      current.session_id !== input.sessionId ||
      current.campaign.revision < input.expectedRevision
    ) {
      throw new RestoreValidationError('restore_identity_mismatch');
    }
    return buildCampaignState(current);
  }
  const renderable = projection.messages
    .slice(-RESTORE_MESSAGE_LIMIT)
    .filter((message) => RENDERABLE_ROLES.has(message.role));
  return {
    campaignId: input.restored.campaign_id,
    adventure: projection.adventure ?? null,
    sessionId: input.restored.session_id,
    revision: input.restored.current_revision ?? input.expectedRevision,
    messages: renderable.map((message, index) =>
      restoredMessage(message, `rehydrated-${input.saveId}-${index}`, index),
    ),
    pc: parsePcPatch(projection.character),
    mapMediaRef: projection.map_media_ref ?? null,
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
  const combat = parseCombatProjection(value);
  if (!combat) throw new RestoreValidationError('restore_combat_invalid');
  return combat;
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
  if (input.defense !== undefined) {
    const parsed = v.safeParse(DefenseChoiceSchema, input.defense);
    if (!parsed.success) throw new RestoreValidationError('restore_pc_invalid');
    output.defense = parsed.output;
  }
  if (input.baseSpeedFt !== undefined) {
    if (
      typeof input.baseSpeedFt !== 'number' ||
      !Number.isSafeInteger(input.baseSpeedFt) ||
      input.baseSpeedFt < 1 ||
      input.baseSpeedFt > 120
    )
      throw new RestoreValidationError('restore_pc_invalid');
    output.baseSpeedFt = input.baseSpeedFt;
  }
  for (const key of ['backstory', 'ideals', 'bonds', 'flaws'] as const) {
    if (input[key] !== undefined) output[key] = asString(input[key], 'restore_pc_invalid');
  }
  if (input.characterId !== undefined)
    output.characterId =
      input.characterId === null ? null : asString(input.characterId, 'restore_pc_invalid');
  if (input.spells !== undefined) {
    const spells = asRecord(input.spells, 'restore_pc_invalid');
    output.spells = {
      cantrips: stringArray(spells.cantrips, 'restore_pc_invalid'),
      level1: stringArray(spells.level1, 'restore_pc_invalid'),
    };
  }
  if (input.personalityFlags !== undefined) {
    if (!Array.isArray(input.personalityFlags))
      throw new RestoreValidationError('restore_pc_invalid');
    output.personalityFlags = input.personalityFlags.map((value) => {
      const flag = asRecord(value, 'restore_pc_invalid');
      const source = asString(flag.source, 'restore_pc_invalid');
      const slotId = asString(flag.slotId, 'restore_pc_invalid');
      if (
        !['background', 'alignment', 'race'].includes(source) ||
        !['bg-trait', 'bg-bond', 'align-trait', 'align-quirk', 'race-trait', 'race-quirk'].includes(
          slotId,
        )
      )
        throw new RestoreValidationError('restore_pc_invalid');
      return {
        slotId,
        source,
        flag: asString(flag.flag, 'restore_pc_invalid'),
      } as import('./charCreation').PersonalityFlag;
    });
  }
  const nullableStrings = [
    'heroClass',
    'name',
    'race',
    'subclass',
    'background',
    'alignment',
    'portraitUrl',
    'portraitAssetId',
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
  if (input.portraitMediaRef !== undefined) {
    if (input.portraitMediaRef === null) output.portraitMediaRef = null;
    else {
      const parsed = v.safeParse(MediaReferenceSchema, input.portraitMediaRef);
      if (
        !parsed.success ||
        (parsed.output.source === 'owned' && parsed.output.category !== 'portrait')
      )
        throw new RestoreValidationError('restore_pc_invalid');
      output.portraitMediaRef = parsed.output;
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

function stringArray(value: unknown, code: string): string[] {
  if (!Array.isArray(value)) throw new RestoreValidationError(code);
  return value.map((entry) => asString(entry, code));
}
