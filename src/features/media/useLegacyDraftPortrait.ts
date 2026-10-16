import { useEffect, useRef } from 'react';
import { decodeLegacyMedia, uploadPortrait } from '../../api/mediaAssets';
import { VISUAL_ASSET_ENTRIES } from '../../assets/visualCatalog.generated';
import { useStore } from '../../state/useStore';

/** Migration never fetches a source URL; only a confirmed owned write replaces it. */
export function useLegacyDraftPortrait() {
  const source = useStore((s) => s.charCreation.portraitUrl);
  const selected = useStore((s) =>
    Boolean(s.charCreation.portraitAssetId || s.charCreation.portraitMediaRef),
  );
  const draftId = useStore((s) => s.charCreation.mediaDraftId);
  const sequence = useRef(0);
  useEffect(() => {
    if (!source || selected) return;
    const draft = useStore.getState().charCreation;
    const bundled = VISUAL_ASSET_ENTRIES.find(
      (asset) => asset.category === 'portrait' && asset.uri === source,
    );
    if (bundled) {
      draft.setDraftField('portraitAssetId', bundled.id);
      draft.setDraftField('portraitUrl', null);
      return;
    }
    const decoded = decodeLegacyMedia(source);
    if (!decoded) return;
    if (!draftId) {
      draft.setDraftField('mediaDraftId', crypto.randomUUID());
      return;
    }
    const current = ++sequence.current;
    let alive = true;
    const blob = new Blob([new Uint8Array(decoded.bytes).buffer], { type: decoded.mimeType });
    void uploadPortrait({ kind: 'draft', draft_id: draftId }, blob, true)
      .then((reference) => {
        const latest = useStore.getState().charCreation;
        if (
          !alive ||
          sequence.current !== current ||
          latest.mediaDraftId !== draftId ||
          latest.portraitUrl !== source ||
          latest.portraitAssetId ||
          latest.portraitMediaRef
        )
          return;
        latest.setDraftField('portraitMediaRef', reference);
        latest.setDraftField('portraitUrl', null);
      })
      .catch(() => {
        // Preserve the unresolved source for explicit repair/export, without logging it.
      });
    return () => {
      alive = false;
    };
  }, [source, selected, draftId]);
}
