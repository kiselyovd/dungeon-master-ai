import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { act, renderHook, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { OwnedMediaReference } from '../../api/contracts/mediaAsset';
import { uploadPortrait } from '../../api/mediaAssets';
import { useStore } from '../../state/useStore';
import { useLegacyDraftPortrait } from './useLegacyDraftPortrait';

vi.mock('../../api/mediaAssets', async (original) => ({
  ...(await original<typeof import('../../api/mediaAssets')>()),
  uploadPortrait: vi.fn(),
}));
const bytes = readFileSync('src/assets/living-tabletop/v2/hero-fighter-01-v2.webp');
const source = `data:image/webp;base64,${bytes.toString('base64')}`;
const draftId = 'a2843520-2a22-4d82-b34d-33ae04389f41';
const reference: OwnedMediaReference = {
  source: 'owned',
  asset_id: 'deb3a4e3-5ed9-4f59-9a8b-890c4f87291d',
  category: 'portrait',
  origin: 'migrated',
  descriptor: {
    sha256: createHash('sha256').update(bytes).digest('hex'),
    mime_type: 'image/webp',
    byte_len: bytes.length,
    width: 512,
    height: 512,
  },
};
beforeEach(() => {
  useStore.getState().charCreation.resetDraft();
  vi.resetAllMocks();
});
afterEach(() => vi.restoreAllMocks());
describe('non-destructive legacy portrait migration', () => {
  it('keeps inline data until durable acknowledgment, then replaces only the reference', async () => {
    let finish: (value: OwnedMediaReference) => void = () => {};
    vi.mocked(uploadPortrait).mockImplementation(
      () =>
        new Promise((resolve) => {
          finish = resolve;
        }),
    );
    const draft = useStore.getState().charCreation;
    draft.setDraftField('mediaDraftId', draftId);
    draft.setDraftField('portraitUrl', source);
    draft.setDraftField('name', 'Existing name');
    const hook = renderHook(useLegacyDraftPortrait);
    await waitFor(() => expect(uploadPortrait).toHaveBeenCalledTimes(1));
    expect(useStore.getState().charCreation.portraitUrl).toBe(source);
    await act(async () => finish(reference));
    expect(useStore.getState().charCreation.portraitUrl).toBeNull();
    expect(useStore.getState().charCreation.portraitMediaRef).toEqual(reference);
    expect(useStore.getState().charCreation.name).toBe('Existing name');
    hook.unmount();
  });
  it('retains failed migration and ignores a late result after draft reset', async () => {
    vi.mocked(uploadPortrait).mockRejectedValueOnce(new Error('media_asset_storage_unavailable'));
    const draft = useStore.getState().charCreation;
    draft.setDraftField('mediaDraftId', draftId);
    draft.setDraftField('portraitUrl', source);
    const first = renderHook(useLegacyDraftPortrait);
    await waitFor(() => expect(uploadPortrait).toHaveBeenCalledTimes(1));
    expect(useStore.getState().charCreation.portraitUrl).toBe(source);
    first.unmount();
    let finish: (value: OwnedMediaReference) => void = () => {};
    vi.mocked(uploadPortrait).mockImplementation(
      () =>
        new Promise((resolve) => {
          finish = resolve;
        }),
    );
    const second = renderHook(useLegacyDraftPortrait);
    await waitFor(() => expect(uploadPortrait).toHaveBeenCalledTimes(2));
    act(() => useStore.getState().charCreation.resetDraft());
    await act(async () => finish(reference));
    expect(useStore.getState().charCreation.portraitMediaRef).toBeNull();
    second.unmount();
  });
  it('leaves remote sources recoverable and never fetches them automatically', () => {
    useStore
      .getState()
      .charCreation.setDraftField('portraitUrl', 'https://example.org/private-portrait.png');
    const hook = renderHook(useLegacyDraftPortrait);
    expect(uploadPortrait).not.toHaveBeenCalled();
    expect(useStore.getState().charCreation.portraitUrl).toBe(
      'https://example.org/private-portrait.png',
    );
    hook.unmount();
  });
});
