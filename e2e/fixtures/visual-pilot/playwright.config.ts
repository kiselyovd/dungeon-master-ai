import { defineConfig } from '@playwright/test';
export default defineConfig({
  testDir: '.',
  testMatch: ['pilot.spec.ts', 'materials.spec.ts'],
  workers: 1,
  timeout: 60000,
  use: { baseURL: 'http://127.0.0.1:1470', headless: true },
  webServer: {
    command: 'bun run dev -- --host 127.0.0.1 --port 1470 --strictPort',
    url: 'http://127.0.0.1:1470/e2e/fixtures/visual-pilot/index.html',
    reuseExistingServer: false,
    timeout: 120000,
  },
});
