import { parseJournal, parsePcPatch } from '../state/campaignProjection';
import type { JournalEntry } from '../state/journal';
import { emptyPcData, PC_DATA_KEYS, type PcData } from '../state/pc';
import type { AuxiliaryPortrait } from './useAuxiliaryPortrait';

export interface AuxiliaryProjection {
  schemaVersion: 1;
  sequence: number;
  scopeId: string;
  campaignId: string | null;
  actorId: string | null;
  revision: number;
  language: 'ru' | 'en';
  ready: boolean;
  hero: PcData | null;
  journal: JournalEntry[];
  portrait: AuxiliaryPortrait | null;
  portraitLoading: boolean;
}
export function auxiliaryHero(pc: Partial<PcData> | null): PcData | null {
  if (!pc?.characterId || !pc.name) return null;
  const clean = Object.fromEntries(
    Object.entries(pc).filter(([key]) => PC_DATA_KEYS.has(key as keyof PcData)),
  );
  return { ...emptyPcData(), ...parsePcPatch({ ...clean, portraitUrl: null }) };
}
export function parseAuxiliaryProjection(input: unknown): AuxiliaryProjection | null {
  if (input === null) return null;
  if (!input || typeof input !== 'object') throw Error('desktop_aux_invalid');
  const value = input as Record<string, unknown>;
  if (
    value.schemaVersion !== 1 ||
    !Number.isSafeInteger(value.sequence) ||
    !Number.isSafeInteger(value.revision) ||
    typeof value.scopeId !== 'string' ||
    !['en', 'ru'].includes(String(value.language)) ||
    typeof value.ready !== 'boolean' ||
    !Array.isArray(value.journal) ||
    value.journal.length > 2000
  )
    throw Error('desktop_aux_invalid');
  const campaignId = typeof value.campaignId === 'string' ? value.campaignId : null;
  const actorId = typeof value.actorId === 'string' ? value.actorId : null;
  const hero = value.hero === null ? null : auxiliaryHero(value.hero as Partial<PcData>);
  const journal = value.journal.map(parseJournal);
  let portrait: AuxiliaryPortrait | null = null;
  if (value.portrait != null) {
    const candidate = value.portrait as AuxiliaryPortrait;
    if (
      hero?.portraitMediaRef?.source !== 'owned' ||
      candidate.assetId !== hero.portraitMediaRef.asset_id ||
      typeof candidate.dataUrl !== 'string' ||
      candidate.dataUrl.length > 512 * 1024 ||
      !/^data:image\/(?:png|jpeg|webp);base64,[A-Za-z0-9+/]+={0,2}$/.test(candidate.dataUrl)
    )
      throw Error('desktop_aux_invalid');
    portrait = { assetId: candidate.assetId, dataUrl: candidate.dataUrl };
  }
  if (
    (hero?.characterId !== actorId && hero) ||
    journal.some((entry) => entry.campaign_id !== campaignId) ||
    (value.ready && !campaignId) ||
    (!value.ready && (hero || journal.length || portrait))
  )
    throw Error('desktop_aux_invalid');
  return {
    schemaVersion: 1,
    sequence: value.sequence as number,
    scopeId: value.scopeId,
    campaignId,
    actorId,
    revision: value.revision as number,
    language: value.language as 'en' | 'ru',
    ready: value.ready,
    hero,
    journal,
    portrait,
    portraitLoading: value.portraitLoading === true,
  };
}
