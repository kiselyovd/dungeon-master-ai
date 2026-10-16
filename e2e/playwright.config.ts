import { defineConfig } from '@playwright/test';

const e2ePort = process.env.DMAI_E2E_PORT ?? '1420';
const baseURL = `http://127.0.0.1:${e2ePort}`;
const usePreview = process.env.DMAI_E2E_PREVIEW === '1';
const webServerCommand = usePreview
  ? `bun run preview -- --host 127.0.0.1 --port ${e2ePort} --strictPort`
  : `bun run dev -- --host 127.0.0.1 --port ${e2ePort} --strictPort`;

export default defineConfig({
  testDir: './',
  testMatch: '**/*.spec.ts',
  // Force vite's cold pre-bundle to finish once, before any spec navigates.
  ...(usePreview ? {} : { globalSetup: './global-setup.ts' }),
  fullyParallel: false,
  // Single worker: parallel spec files would each race vite's cold pre-bundle
  // simultaneously. Serialised, the first navigation absorbs the bundle and the
  // rest run warm.
  workers: 1,
  // Vite's first cold start has to pre-bundle every dep before the page
  // can render (PixiJS, Zustand, i18next, Tauri shims, Testing Library
  // accidentally pulled in via lazy imports etc.), which on a clean
  // node_modules cache routinely takes 30-60 seconds. Bump the per-test
  // timeout so the very first page.goto does not race the optimizer.
  // Vite's cold pre-bundle (pixi.js + React + i18next + Tauri shims) has been
  // measured at ~115s on a clean optimizer cache, so the first navigation must
  // be allowed to outlast it; warm navigations are sub-second.
  timeout: 300_000,
  use: {
    baseURL,
    headless: true,
    navigationTimeout: 240_000,
    screenshot: 'only-on-failure',
    trace: 'retain-on-failure',
    video: 'retain-on-failure',
  },
  webServer: {
    command: webServerCommand,
    url: baseURL,
    // An explicit test port means this run owns the Vite process. Do not reuse
    // a stale listener left behind by an interrupted browser run.
    reuseExistingServer: !process.env.CI && process.env.DMAI_E2E_PORT === undefined,
    timeout: 200_000,
    stdout: 'pipe',
    stderr: 'pipe',
  },
});
