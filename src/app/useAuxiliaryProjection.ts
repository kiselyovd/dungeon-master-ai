import { useLayoutEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { heartbeatAuxiliaryProjection, publishAuxiliaryProjection } from '../api/auxiliaryWindows';
import type { JournalEntry } from '../state/journal';
import type { PcData } from '../state/pc';
import { auxiliaryHero } from './auxiliaryProjection';

interface Source {
  enabled?: boolean;
  scopeId: string;
  campaignId: string | null;
  memberId?: string | null;
  revision: number;
  ready: boolean;
  hero: Partial<PcData> | null;
  journal: JournalEntry[];
}
let sequence = Date.now();
export function useAuxiliaryProjection(source: Source): void {
  const { i18n } = useTranslation();
  const ready = source.ready && Boolean(source.campaignId);
  let hero: PcData | null = null;
  try {
    hero = ready ? auxiliaryHero(source.hero) : null;
  } catch {
    hero = null;
  }
  const signature = JSON.stringify({
    schemaVersion: 1,
    scopeId: source.scopeId,
    campaignId: source.campaignId,
    memberId: source.memberId ?? null,
    actorId: hero?.characterId ?? null,
    revision: source.revision,
    language: i18n.language.startsWith('ru') ? 'ru' : 'en',
    ready,
    hero,
    journal: ready ? source.journal.filter((entry) => entry.campaign_id === source.campaignId) : [],
  });
  useLayoutEffect(() => {
    if (source.enabled === false) return;
    const value = JSON.parse(signature) as Record<string, unknown>;
    const publish = (available: boolean) => {
      const frame = available ? value : { ...value, ready: false, hero: null, journal: [] };
      sequence = Math.max(sequence + 1, Date.now());
      const sentSequence = sequence;
      void publishAuxiliaryProjection({ ...frame, sequence: sentSequence }).catch(() => {});
      return sentSequence;
    };
    const publishedSequence = publish(true);
    const heartbeat = () => void heartbeatAuxiliaryProjection(publishedSequence).catch(() => {});
    const timer = setInterval(heartbeat, 5000);
    const visibility = heartbeat;
    document.addEventListener('visibilitychange', visibility);
    return () => {
      clearInterval(timer);
      document.removeEventListener('visibilitychange', visibility);
      publish(false);
    };
  }, [signature, source.enabled]);
}
