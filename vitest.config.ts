import react from '@vitejs/plugin-react';
import { defineConfig } from 'vitest/config';

export default defineConfig({
  plugins: [react()],

  define: {
    // @ts-expect-error process is a nodejs global
    __APP_VERSION__: JSON.stringify(process.env.npm_package_version ?? '0.0.0'),
  },

  test: {
    globals: true,
    environment: 'jsdom',
    setupFiles: ['./src/setupTests.ts'],
    css: true,
    include: ['src/**/*.{test,spec}.{ts,tsx}'],
    exclude: ['e2e/**', 'node_modules/**', 'dist/**', 'src-tauri/**', 'target/**'],
  },
});
