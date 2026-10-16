import * as v from 'valibot';
import { backendUrl } from './client';
import { CampaignStateSchema } from './contracts/campaignState';
import {
  type LocalMediaScope,
  MAX_MEDIA_BYTES,
  type MediaScope,
  type OwnedMediaReference,
  parseMediaReference,
} from './contracts/mediaAsset';
import { loadRoomMediaAsset, uploadRoomPortrait } from './rooms';

async function requireOk(response: Response): Promise<Response> {
  if (!response.ok) {
    const codes = [
      'media_asset_invalid',
      'media_asset_too_large',
      'media_asset_missing',
      'media_asset_denied',
      'media_asset_corrupt',
      'media_asset_storage_unavailable',
    ];
    const body = await response.json().catch(() => null);
    throw new Error(
      codes.includes(body?.error?.code)
        ? body.error.code
        : response.status === 404
          ? 'media_asset_missing'
          : 'media_asset_storage_unavailable',
    );
  }
  return response;
}
export function encodeMedia(bytes: Uint8Array): string {
  if (bytes.byteLength > MAX_MEDIA_BYTES) throw new Error('media_asset_too_large');
  let text = '';
  for (let start = 0; start < bytes.length; start += 32768)
    text += String.fromCharCode(...bytes.subarray(start, start + 32768));
  return btoa(text);
}
export function decodeLegacyMedia(url: string): { bytes: Uint8Array; mimeType: string } | null {
  // Never fetch arbitrary URLs during restore/migration.
  if (url.length > 8 * 1024 * 1024) return null;
  const match = /^data:(image\/(?:png|jpeg|webp));base64,([A-Za-z0-9+/]*={0,2})$/.exec(url);
  if (!match?.[1] || !match[2]) return null;
  try {
    const bytes = Uint8Array.from(atob(match[2]), (c) => c.charCodeAt(0));
    return bytes.length > 0 && bytes.length <= MAX_MEDIA_BYTES
      ? { bytes, mimeType: match[1] }
      : null;
  } catch {
    return null;
  }
}
export async function uploadPortrait(scope: LocalMediaScope, blob: Blob, migrated = false) {
  if (blob.size === 0 || blob.size > MAX_MEDIA_BYTES)
    throw new Error(blob.size > MAX_MEDIA_BYTES ? 'media_asset_too_large' : 'media_asset_invalid');
  const bytes = new Uint8Array(await blob.arrayBuffer());
  const response = await requireOk(
    await fetch(await backendUrl('/media/assets'), {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({
        scope,
        category: 'portrait',
        mime_type: blob.type,
        data_b64: encodeMedia(bytes),
        migrated,
      }),
    }),
  );
  const reference = parseMediaReference(await response.json());
  if (reference.source !== 'owned' || reference.category !== 'portrait')
    throw new Error('media_asset_invalid');
  if (
    reference.origin !== (migrated ? 'migrated' : 'uploaded') ||
    reference.descriptor.mime_type !== blob.type
  )
    throw new Error('media_asset_corrupt');
  await verifyMediaBytes(reference, bytes);
  return reference;
}
export async function verifyMediaBytes(
  reference: OwnedMediaReference,
  bytes: Uint8Array,
): Promise<Blob> {
  if (bytes.byteLength !== reference.descriptor.byte_len) throw new Error('media_asset_corrupt');
  const hash = Array.from(
    new Uint8Array(await crypto.subtle.digest('SHA-256', new Uint8Array(bytes).buffer)),
    (byte) => byte.toString(16).padStart(2, '0'),
  ).join('');
  if (hash !== reference.descriptor.sha256) throw new Error('media_asset_corrupt');
  const blob = new Blob([new Uint8Array(bytes).buffer], { type: reference.descriptor.mime_type });
  if (typeof createImageBitmap === 'function') {
    let bitmap: ImageBitmap;
    try {
      bitmap = await createImageBitmap(blob);
    } catch {
      throw new Error('media_asset_corrupt');
    }
    const valid =
      bitmap.width === reference.descriptor.width && bitmap.height === reference.descriptor.height;
    bitmap.close();
    if (!valid) throw new Error('media_asset_corrupt');
  }
  return blob;
}
export async function readMedia(
  reference: OwnedMediaReference,
  scope: MediaScope,
  signal?: AbortSignal,
): Promise<Blob> {
  if (scope.kind === 'room') {
    const result = await loadRoomMediaAsset(scope.room_id, reference.asset_id);
    const returned = result.media_ref;
    if (
      returned.source !== 'owned' ||
      returned.asset_id !== reference.asset_id ||
      returned.origin !== reference.origin ||
      returned.category !== reference.category ||
      ['sha256', 'mime_type', 'byte_len', 'width', 'height'].some(
        (key) =>
          returned.descriptor[key as keyof typeof returned.descriptor] !==
          reference.descriptor[key as keyof typeof reference.descriptor],
      )
    )
      throw new Error('media_asset_corrupt');
    let binary: string;
    try {
      binary = atob(result.data_b64);
    } catch {
      throw new Error('media_asset_corrupt');
    }
    return verifyMediaBytes(
      reference,
      Uint8Array.from(binary, (c) => c.charCodeAt(0)),
    );
  }
  const query =
    scope.kind === 'draft'
      ? `draft_id=${encodeURIComponent(scope.draft_id)}`
      : `campaign_id=${encodeURIComponent(scope.campaign_id)}`;
  const response = await requireOk(
    await fetch(
      await backendUrl(`/media/assets/${encodeURIComponent(reference.asset_id)}?${query}`),
      signal ? { signal } : {},
    ),
  );
  if (response.headers.get('content-type') !== reference.descriptor.mime_type)
    throw new Error('media_asset_corrupt');
  if (
    response.headers.has('content-length') &&
    Number(response.headers.get('content-length')) !== reference.descriptor.byte_len
  )
    throw new Error('media_asset_corrupt');
  const reader = response.body?.getReader();
  if (!reader) throw new Error('media_asset_corrupt');
  const parts: Uint8Array[] = [];
  let length = 0;
  try {
    for (;;) {
      const chunk = await reader.read();
      if (chunk.done) break;
      length += chunk.value.byteLength;
      if (length > reference.descriptor.byte_len) throw new Error('media_asset_corrupt');
      parts.push(chunk.value);
    }
  } finally {
    await reader.cancel().catch(() => {});
  }
  const bytes = new Uint8Array(length);
  let offset = 0;
  for (const part of parts) {
    bytes.set(part, offset);
    offset += part.length;
  }
  return verifyMediaBytes(reference, bytes);
}
export async function transferPortrait(
  reference: OwnedMediaReference,
  scope: MediaScope,
  roomId: string,
  assetId: string,
) {
  const blob = await readMedia(reference, scope);
  return uploadRoomPortrait(
    roomId,
    assetId,
    reference.descriptor.mime_type,
    encodeMedia(new Uint8Array(await blob.arrayBuffer())),
  );
}
export async function migrateCampaignPortrait(
  campaignId: string,
  characterId: string,
  revision: number,
  requestId: string,
) {
  const response = await requireOk(
    await fetch(
      await backendUrl(`/campaigns/${encodeURIComponent(campaignId)}/media/portrait/migrate`),
      {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({
          character_id: characterId,
          expected_revision: revision,
          request_id: requestId,
        }),
      },
    ),
  );
  const result = v.safeParse(
    v.strictObject({ state: CampaignStateSchema, changed: v.boolean(), retained: v.boolean() }),
    await response.json(),
  );
  if (
    !result.success ||
    result.output.state.campaign.id !== campaignId ||
    result.output.state.character_id !== characterId
  )
    throw new Error('media_asset_invalid');
  return result.output;
}
