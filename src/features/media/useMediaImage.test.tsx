import { act, renderHook, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { OwnedMediaReference } from '../../api/contracts/mediaAsset';
import { readMedia } from '../../api/mediaAssets';
import { useMediaImage } from './useMediaImage';

vi.mock('../../api/mediaAssets', () => ({ readMedia: vi.fn() }));
const reference: OwnedMediaReference = {
  source: 'owned',
  asset_id: 'a2843520-2a22-4d82-b34d-33ae04389f41',
  category: 'portrait',
  origin: 'uploaded',
  descriptor: { sha256: 'a'.repeat(64), mime_type: 'image/png', width: 1, height: 1, byte_len: 3 },
};
afterEach(() => {
  vi.restoreAllMocks();
  vi.resetAllMocks();
});
describe('scoped media image', () => {
  it('shares one resource within a scope and releases it after the last consumer', async () => {
    const create = vi.spyOn(URL, 'createObjectURL').mockReturnValue('blob:one');
    const revoke = vi.spyOn(URL, 'revokeObjectURL').mockImplementation(() => {});
    vi.mocked(readMedia).mockResolvedValue(new Blob(['abc'], { type: 'image/png' }));
    const a = renderHook(() => useMediaImage(reference, { kind: 'room', room_id: 'room-a' }));
    const b = renderHook(() => useMediaImage(reference, { kind: 'room', room_id: 'room-a' }));
    await waitFor(() => expect(a.result.current.url).toBe('blob:one'));
    await waitFor(() => expect(b.result.current.url).toBe('blob:one'));
    expect(readMedia).toHaveBeenCalledTimes(1);
    expect(create).toHaveBeenCalledTimes(1);
    a.unmount();
    expect(revoke).not.toHaveBeenCalled();
    b.unmount();
    expect(revoke).toHaveBeenCalledWith('blob:one');
  });
  it('never delivers a late old-scope result and retries a rejected read', async () => {
    const create = vi.spyOn(URL, 'createObjectURL').mockReturnValue('blob:current');
    vi.spyOn(URL, 'revokeObjectURL').mockImplementation(() => {});
    let finish: (value: Blob) => void = () => {};
    vi.mocked(readMedia)
      .mockImplementationOnce(
        () =>
          new Promise((resolve) => {
            finish = resolve;
          }),
      )
      .mockRejectedValueOnce(new Error('media_asset_denied'))
      .mockResolvedValueOnce(new Blob(['abc']));
    const hook = renderHook(
      ({ room }) => useMediaImage(reference, { kind: 'room', room_id: room }),
      { initialProps: { room: 'old' } },
    );
    hook.rerender({ room: 'new' });
    await waitFor(() => expect(hook.result.current.error).toBe('media_asset_denied'));
    await act(async () => {
      finish(new Blob(['abc']));
    });
    expect(create).not.toHaveBeenCalled();
    expect(hook.result.current.url).toBeNull();
    act(() => hook.result.current.retry());
    await waitFor(() => expect(hook.result.current.url).toBe('blob:current'));
    expect(readMedia).toHaveBeenCalledTimes(3);
    hook.unmount();
  });
});
