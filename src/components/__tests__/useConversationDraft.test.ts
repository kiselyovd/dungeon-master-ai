import { act, renderHook, waitFor } from '@testing-library/react';
import { expect, it, vi } from 'vitest';
import { readConversationDraft, saveConversationDraft } from '../../state/conversationDrafts';
import { useConversationDraft } from '../useConversationDraft';

it('loads cold text and isolates unsent text/attachments across campaign or room participant scopes', async () => {
  const first = `solo:${crypto.randomUUID()}`;
  const second = `room:${crypto.randomUUID()}`;
  await saveConversationDraft(first, 'Cold draft');
  const { result, rerender, unmount } = renderHook(({ scope }) => useConversationDraft(scope), {
    initialProps: { scope: first },
  });
  await waitFor(() => expect(result.current.text).toBe('Cold draft'));
  act(() => {
    result.current.setText('Unsent first');
    result.current.setAttachments([
      {
        mime: 'image/png',
        dataUrl: 'data:image/png;base64,fixture',
        name: 'fixture.png',
        sizeBytes: 1,
      },
    ]);
  });
  rerender({ scope: second });
  expect(result.current.text).toBe('');
  expect(result.current.attachments).toEqual([]);
  act(() => result.current.setText('Other participant draft'));
  rerender({ scope: first });
  expect(result.current.text).toBe('Unsent first');
  expect(result.current.attachments).toHaveLength(1);
  await act(() => result.current.flush());
  expect(await readConversationDraft(first)).toBe('Unsent first');
  unmount();
});

it('a late disk read cannot overwrite text typed while hydration is pending', async () => {
  let resolve: (text: string) => void = () => {};
  const module = await import('../../state/conversationDrafts');
  const spy = vi.spyOn(module, 'readConversationDraft').mockImplementationOnce(
    () =>
      new Promise((done) => {
        resolve = done;
      }),
  );
  const scope = `late:${crypto.randomUUID()}`;
  const { result, unmount } = renderHook(() => useConversationDraft(scope));
  act(() => result.current.setText('Current input'));
  await act(async () => {
    resolve('Old disk value');
  });
  expect(result.current.text).toBe('Current input');
  unmount();
  spy.mockRestore();
});
