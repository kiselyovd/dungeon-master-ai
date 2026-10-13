import { LazyStore } from '@tauri-apps/plugin-store';
import * as v from 'valibot';

export const MAX_CONVERSATION_DRAFT_LENGTH = 32_000;
const schema = v.object({
  version: v.literal(1),
  entries: v.pipe(
    v.array(
      v.object({
        scope: v.pipe(v.string(), v.minLength(1), v.maxLength(400)),
        text: v.pipe(v.string(), v.maxLength(MAX_CONVERSATION_DRAFT_LENGTH)),
      }),
    ),
    v.maxLength(32),
  ),
});
const store = new LazyStore('conversation-drafts.json');
let queue: Promise<unknown> = Promise.resolve();
function serial<T>(work: () => Promise<T>) {
  const next = queue.then(work, work);
  queue = next.catch(() => undefined);
  return next;
}
async function read() {
  const raw = await store.get<unknown>('drafts');
  if (raw == null) return { version: 1 as const, entries: [] };
  return v.parse(schema, raw);
}
export function readConversationDraft(scope: string) {
  return serial(
    async () => (await read()).entries.find((entry) => entry.scope === scope)?.text ?? '',
  );
}
/** User-authored text only. No settings, credentials, model output or image bytes. */
export function saveConversationDraft(scope: string, text: string) {
  return serial(async () => {
    const previous = await read();
    const entries = previous.entries.filter((entry) => entry.scope !== scope);
    if (text) entries.push({ scope, text });
    const next = v.parse(schema, { version: 1, entries: entries.slice(-32) });
    try {
      await store.set('drafts', next);
      await store.save();
    } catch (error) {
      await store.set('drafts', previous);
      throw error;
    }
  });
}
export async function flushConversationDrafts() {
  await queue;
}
