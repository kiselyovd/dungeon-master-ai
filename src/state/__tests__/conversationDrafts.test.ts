import { LazyStore } from '@tauri-apps/plugin-store';
import { expect, it, vi } from 'vitest';
import { readConversationDraft, saveConversationDraft } from '../conversationDrafts';

it('stores scoped text only and retains the previous acknowledged draft after a failed flush', async () => {
  const scope = crypto.randomUUID();
  await saveConversationDraft(scope, 'Before send');
  const save = vi
    .spyOn(LazyStore.prototype, 'save')
    .mockRejectedValueOnce(new Error('disk_fixture'));
  await expect(saveConversationDraft(scope, '')).rejects.toThrow('disk_fixture');
  expect(await readConversationDraft(scope)).toBe('Before send');
  expect(await readConversationDraft(crypto.randomUUID())).toBe('');
  save.mockRestore();
  await saveConversationDraft(scope, '');
  expect(await readConversationDraft(scope)).toBe('');
});

it('refuses malformed or oversized drafts without replacing their stored document', async () => {
  const store = new LazyStore('conversation-drafts.json');
  await expect(saveConversationDraft('size-check', 'x'.repeat(32_001))).rejects.toThrow();
  const before = await store.get('drafts');
  const corrupt = { version: 99, entries: [{ scope: 'bad', text: 'retain' }] };
  await store.set('drafts', corrupt);
  await expect(saveConversationDraft('bad', 'replacement')).rejects.toThrow();
  expect(await store.get('drafts')).toEqual(corrupt);
  await store.set('drafts', before);
});
