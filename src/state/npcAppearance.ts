import { LazyStore } from '@tauri-apps/plugin-store';
import * as v from 'valibot';

const store = new LazyStore('npc-appearance.json');
const schema = v.record(v.pipe(v.string(), v.maxLength(300)), v.pipe(v.string(), v.maxLength(128)));
let queue: Promise<unknown> = Promise.resolve();
/** Additive, device-local cosmetic identity. Catalog growth and changed role
 * descriptions cannot reshuffle already assigned record IDs. */
export function npcAppearance(scope: string, suggestedId: string): Promise<string> {
  const next = queue.then(async () => {
    const previous = await store.get<unknown>('assignments');
    const assignments = previous == null ? {} : v.parse(schema, previous);
    if (assignments[scope]) return assignments[scope];
    const updated = v.parse(schema, { ...assignments, [scope]: suggestedId });
    try {
      await store.set('assignments', updated);
      await store.save();
    } catch (error) {
      await store.set('assignments', assignments);
      throw error;
    }
    return suggestedId;
  });
  queue = next.catch(() => undefined);
  return next;
}
