import { readdirSync, readFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { describe, expect, it } from 'vitest';

// Resolves to the `src/` directory (this file lives in `src/api/`).
const SRC_ROOT = resolve(__dirname, '..');

// Matches fetch( or new EventSource( whose first argument is a bare relative
// path starting with /local - either a literal quote (`'/local`, `"/local`,
// `` `/local ``) or a template literal whose leading segment is an
// interpolated expression (`` `${apiBase()}/local ``). Any of these resolve
// against the Tauri webview origin and 404 - they must go through backendUrl().
const BARE_LOCAL_REQUEST = /(?:fetch|new EventSource)\(\s*[`'"](?:\$\{[^}]*\})?\/local/;

function collectSourceFiles(dir: string, acc: string[]): string[] {
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const full = join(dir, entry.name);
    if (entry.isDirectory()) {
      collectSourceFiles(full, acc);
    } else if (
      (entry.name.endsWith('.ts') || entry.name.endsWith('.tsx')) &&
      !entry.name.endsWith('.test.ts') &&
      !entry.name.endsWith('.test.tsx')
    ) {
      acc.push(full);
    }
  }
  return acc;
}

describe('no bare-path /local fetches or EventSource calls', () => {
  it('every /local* fetch and EventSource routes through backendUrl()', () => {
    const offenders = collectSourceFiles(SRC_ROOT, []).filter((file) =>
      BARE_LOCAL_REQUEST.test(readFileSync(file, 'utf8')),
    );
    expect(
      offenders,
      `bare /local request found - route through backendUrl():\n${offenders.join('\n')}`,
    ).toEqual([]);
  });
});
