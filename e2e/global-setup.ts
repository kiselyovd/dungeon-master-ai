import { request } from '@playwright/test';

/**
 * Warm the vite dev server before any test runs.
 *
 * Vite's first request triggers an on-demand pre-bundle of the whole dep graph
 * (pixi.js + React + i18next + Tauri shims), measured at ~115-165s on a cold
 * optimizer cache. Playwright's `webServer` readiness check releases as soon as
 * the port answers, so without this warmup the first few specs race the cold
 * bundle and fail their `page.goto` on a navigation timeout. A single blocking
 * GET here forces the bundle to finish once, up front, so every spec then
 * navigates against a warm (sub-second) server.
 */
export default async function globalSetup(): Promise<void> {
  const baseURL = 'http://127.0.0.1:1420';
  const ctx = await request.newContext({ baseURL });
  try {
    const res = await ctx.get('/', { timeout: 300_000 });
    if (!res.ok()) {
      throw new Error(`vite warmup GET / returned HTTP ${res.status()}`);
    }
  } finally {
    await ctx.dispose();
  }
}
