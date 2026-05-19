import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: './',
  testMatch: '**/*.spec.ts',
  fullyParallel: false,
  // Vite's first cold start has to pre-bundle every dep before the page
  // can render (PixiJS, Zustand, i18next, Tauri shims, Testing Library
  // accidentally pulled in via lazy imports etc.), which on a clean
  // node_modules cache routinely takes 30-60 seconds. Bump the per-test
  // timeout so the very first page.goto does not race the optimizer.
  timeout: 90_000,
  use: {
    baseURL: 'http://127.0.0.1:1420',
    headless: true,
    navigationTimeout: 60_000,
    screenshot: 'only-on-failure',
    trace: 'retain-on-failure',
    video: 'retain-on-failure',
  },
  webServer: {
    command: 'bun run dev',
    url: 'http://127.0.0.1:1420',
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
    stdout: 'pipe',
    stderr: 'pipe',
  },
});
