import { useEffect, useState } from 'react';
import type { MediaScope, OwnedMediaReference } from '../api/contracts/mediaAsset';
import { encodeMedia, readMedia } from '../api/mediaAssets';

export interface AuxiliaryPortrait {
  assetId: string;
  dataUrl: string;
}
/** Only the authorized main composition reads media. Detached views receive a
 * small in-memory derivative, never a URL or a filesystem/backend capability. */
export function useAuxiliaryPortrait(
  scopeId: string,
  reference: OwnedMediaReference | null,
  scope: MediaScope | null,
) {
  const key = reference && scope ? JSON.stringify({ scopeId, reference, scope }) : '';
  const [result, setResult] = useState<{ key: string; portrait: AuxiliaryPortrait | null } | null>(
    null,
  );
  useEffect(() => {
    if (!key) return;
    const request = JSON.parse(key) as { reference: OwnedMediaReference; scope: MediaScope };
    const abort = new AbortController();
    let active = true;
    void (async () => {
      const blob = await readMedia(request.reference, request.scope, abort.signal);
      if (!active) return;
      const factor = Math.min(
        1,
        256 / Math.max(request.reference.descriptor.width, request.reference.descriptor.height),
      );
      const width = Math.max(1, Math.round(request.reference.descriptor.width * factor)),
        height = Math.max(1, Math.round(request.reference.descriptor.height * factor));
      const bitmap = await createImageBitmap(blob, {
        resizeWidth: width,
        resizeHeight: height,
        resizeQuality: 'high',
      });
      try {
        if (!active) return;
        const canvas = document.createElement('canvas');
        canvas.width = width;
        canvas.height = height;
        const context = canvas.getContext('2d');
        if (!context) throw Error('media_preview_unavailable');
        context.drawImage(bitmap, 0, 0);
        const preview = await new Promise<Blob>((resolve, reject) =>
          canvas.toBlob(
            (value) => (value ? resolve(value) : reject(Error('media_preview_unavailable'))),
            'image/webp',
            0.9,
          ),
        );
        if (preview.size > 384 * 1024) throw Error('media_preview_too_large');
        const dataUrl = `data:${preview.type};base64,${encodeMedia(new Uint8Array(await preview.arrayBuffer()))}`;
        if (dataUrl.length > 512 * 1024) throw Error('media_preview_too_large');
        if (active) setResult({ key, portrait: { assetId: request.reference.asset_id, dataUrl } });
      } finally {
        bitmap.close();
      }
    })().catch(() => {
      if (active) setResult({ key, portrait: null });
    });
    return () => {
      active = false;
      abort.abort();
    };
  }, [key]);
  return {
    portrait: key && result?.key === key ? result.portrait : null,
    loading: Boolean(key) && result?.key !== key,
  };
}
