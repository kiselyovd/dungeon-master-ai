import { afterEach, describe, expect, it, vi } from 'vitest';
import type { OwnedMediaReference } from './contracts/mediaAsset';
import { decodeLegacyMedia, verifyMediaBytes } from './mediaAssets';

afterEach(() => vi.restoreAllMocks());
describe('media migration and immutable lookup', () => {
  it('only decodes bounded inline raster data and never fetches remote or local paths', () => {
    const fetch = vi.spyOn(globalThis, 'fetch');
    for (const input of [
      'https://example.org/private.png',
      'file:///D:/private.png',
      '../private.png',
      'data:image/svg+xml;base64,PHN2Zy8+',
      'data:image/png;base64,%%',
      `data:image/png;base64,${'A'.repeat(8 * 1024 * 1024)}`,
    ])
      expect(decodeLegacyMedia(input)).toBeNull();
    expect(decodeLegacyMedia('data:image/png;base64,YWJj')?.bytes).toEqual(
      new Uint8Array([97, 98, 99]),
    );
    expect(fetch).not.toHaveBeenCalled();
  });
  it('detects changed bytes and keeps exact mime/hash metadata', async () => {
    const bytes = new Uint8Array([97, 98, 99]);
    const ref: OwnedMediaReference = {
      source: 'owned',
      asset_id: 'a2843520-2a22-4d82-b34d-33ae04389f41',
      category: 'portrait',
      origin: 'uploaded',
      descriptor: {
        sha256: 'ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad',
        mime_type: 'image/png',
        width: 1,
        height: 1,
        byte_len: 3,
      },
    };
    const blob = await verifyMediaBytes(ref, bytes);
    expect(blob.type).toBe('image/png');
    expect(blob.size).toBe(3);
    await expect(verifyMediaBytes(ref, new Uint8Array([97, 98, 100]))).rejects.toThrow(
      'media_asset_corrupt',
    );
    await expect(verifyMediaBytes(ref, new Uint8Array([97, 98]))).rejects.toThrow(
      'media_asset_corrupt',
    );
  });
});
