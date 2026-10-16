import { useEffect, useState } from 'react';
import type { MediaReference, MediaScope } from '../../api/contracts/mediaAsset';
import { readMedia } from '../../api/mediaAssets';
import { visualAsset } from '../../assets/visualCatalog';
import { VISUAL_ASSET_ENTRIES } from '../../assets/visualCatalog.generated';

interface CachedImage {
  users: number;
  promise: Promise<string>;
  url: string | null;
  abort: AbortController;
}
const images = new Map<string, CachedImage>();
function acquire(
  key: string,
  reference: Extract<MediaReference, { source: 'owned' }>,
  scope: MediaScope,
) {
  let entry = images.get(key);
  if (!entry) {
    const abort = new AbortController();
    const created: CachedImage = { users: 0, promise: Promise.resolve(''), url: null, abort };
    created.promise = readMedia(reference, scope, abort.signal).then((blob) => {
      if (abort.signal.aborted) throw new Error('media_asset_scope_changed');
      const url = URL.createObjectURL(blob);
      created.url = url;
      return url;
    });
    entry = created;
    images.set(key, entry);
  }
  entry.users++;
  return entry;
}
function release(key: string, entry: CachedImage) {
  entry.users--;
  if (entry.users !== 0) return;
  entry.abort.abort();
  if (entry.url) URL.revokeObjectURL(entry.url);
  if (images.get(key) === entry) images.delete(key);
}
export function useMediaImage(
  reference: MediaReference | null | undefined,
  scope: MediaScope | null,
  category?: 'portrait' | 'map' | 'illustration',
) {
  const [result, setResult] = useState<{
    key: string;
    url: string | null;
    error: string | null;
  } | null>(null);
  const [attempt, setAttempt] = useState(0);
  const key =
    reference?.source === 'owned' && scope && (!category || reference.category === category)
      ? `${attempt}:${JSON.stringify({ reference, scope })}`
      : '';
  useEffect(() => {
    if (!key) return;
    const request = JSON.parse(key.slice(key.indexOf(':') + 1)) as {
      reference: Extract<MediaReference, { source: 'owned' }>;
      scope: MediaScope;
    };
    let alive = true;
    const image = acquire(key, request.reference, request.scope);
    void image.promise.then(
      (url) => {
        if (alive) setResult({ key, url, error: null });
      },
      (error) => {
        if (alive)
          setResult({
            key,
            url: null,
            error: error instanceof Error ? error.message : 'media_asset_missing',
          });
      },
    );
    return () => {
      alive = false;
      release(key, image);
    };
  }, [key]);
  const owned = reference?.source === 'owned';
  const bundled =
    reference?.source === 'bundled'
      ? (visualAsset(reference.asset_id) ??
        VISUAL_ASSET_ENTRIES.find((asset) => asset.backendAssetId === reference.asset_id))
      : null;
  const bundledAllowed =
    bundled &&
    reference?.source === 'bundled' &&
    bundled.version === reference.version &&
    (category === 'portrait'
      ? bundled.category === 'portrait'
      : category === 'map'
        ? bundled.category === 'map'
        : !category || ['scene', 'illustration', 'portrait'].includes(bundled.category));
  return {
    url: owned
      ? result?.key === key
        ? result.url
        : null
      : reference?.source === 'bundled'
        ? bundledAllowed
          ? bundled.uri
          : null
        : null,
    error: owned ? (result?.key === key ? result.error : null) : null,
    loading: owned && Boolean(scope) && result?.key !== key,
    retry: () => setAttempt((value) => value + 1),
  };
}
