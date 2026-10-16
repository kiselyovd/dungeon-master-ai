/** Mounted presentation adapters supply disk barriers, without retaining data. */
const preparations = new Set<() => Promise<void>>();
export function registerClosePreparation(prepare: () => Promise<void>): () => void {
  preparations.add(prepare);
  return () => {
    preparations.delete(prepare);
  };
}
export async function flushClosePreparations(): Promise<void> {
  await Promise.all([...preparations].map((prepare) => prepare()));
}
