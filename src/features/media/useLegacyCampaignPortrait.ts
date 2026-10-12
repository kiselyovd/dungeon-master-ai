import { useEffect, useRef } from 'react';
import { parseMediaReference } from '../../api/contracts/mediaAsset';
import { migrateCampaignPortrait } from '../../api/mediaAssets';
import { useStore } from '../../state/useStore';

/** Appearance-only migration waits until canonical gameplay recovery is settled. */
export function useLegacyCampaignPortrait(enabled: boolean) {
  const ownMutation = useRef<string | null>(null);
  const locked = useStore(
    (s) => s.session.mutationId !== null && s.session.mutationId !== ownMutation.current,
  );
  const campaign = useStore((s) => s.session.activeCampaignId);
  const session = useStore((s) => s.session.activeSessionId);
  const actor = useStore((s) => s.pc.characterId);
  const source = useStore((s) => s.pc.portraitUrl);
  const selected = useStore((s) => Boolean(s.pc.portraitMediaRef || s.pc.portraitAssetId));
  const streaming = useStore((s) => s.chat.isStreaming);
  useEffect(() => {
    const store = useStore.getState();
    if (
      !enabled ||
      locked ||
      streaming ||
      selected ||
      !source ||
      !campaign ||
      !session ||
      !actor ||
      store.session.mutationId ||
      !store.campaigns.active ||
      store.campaigns.active.id !== campaign
    )
      return;
    const id = crypto.randomUUID();
    ownMutation.current = id;
    if (!store.session.beginMutation(id)) {
      ownMutation.current = null;
      return;
    }
    let alive = true;
    void migrateCampaignPortrait(campaign, actor, store.campaigns.active.revision, id)
      .then((result) => {
        const current = useStore.getState();
        if (
          !alive ||
          current.session.activeCampaignId !== campaign ||
          current.session.activeSessionId !== session ||
          current.pc.characterId !== actor ||
          current.pc.portraitUrl !== source
        )
          return;
        const raw = result.state.projection.character.portraitMediaRef;
        if (raw) {
          current.pc.replaceFromDraft({
            portraitMediaRef: parseMediaReference(raw),
            portraitUrl: null,
          });
        }
        current.campaigns.setActive(result.state.campaign);
      })
      .catch(() => {})
      .finally(() => {
        useStore.getState().session.endMutation(id);
        if (ownMutation.current === id) ownMutation.current = null;
      });
    return () => {
      alive = false;
    };
  }, [enabled, streaming, selected, source, campaign, session, actor, locked]);
}
