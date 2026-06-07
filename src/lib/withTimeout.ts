/**
 * Race a promise against a deadline. If `promise` settles first its result (or
 * rejection) wins; if the deadline fires first the call resolves with
 * `fallback`.
 *
 * Used to bound the encrypted-secrets read during persist hydration: a slow or
 * hanging Stronghold vault open must never block the whole `getItem` (and thus
 * the app's hydration gate) indefinitely. See `persistStorage.getSecretSafe`.
 */
export function withTimeout<T>(promise: Promise<T>, ms: number, fallback: T): Promise<T> {
  return new Promise<T>((resolve, reject) => {
    const timer = setTimeout(() => resolve(fallback), ms);
    promise.then(
      (value) => {
        clearTimeout(timer);
        resolve(value);
      },
      (err) => {
        clearTimeout(timer);
        reject(err);
      },
    );
  });
}
