import react from '@vitejs/plugin-react';
import { defineConfig } from 'vitest/config';

export default defineConfig({
  plugins: [react()],

  resolve: {
    // @pixi/react imports this CommonJS entry without its extension. Node's
    // Windows ESM resolver needs the concrete file during real-root tests.
    alias: { 'react-reconciler/constants': 'react-reconciler/constants.js' },
  },

  define: {
    // @ts-expect-error process is a nodejs global
    __APP_VERSION__: JSON.stringify(process.env.npm_package_version ?? '0.0.0'),
  },

  test: {
    globals: true,
    environment: 'jsdom',
    setupFiles: ['./src/setupTests.ts'],
    css: true,
    include: ['src/**/*.{test,spec}.{ts,tsx}', 'scripts/**/*.test.ts'],
    exclude: ['e2e/**', 'node_modules/**', 'dist/**', 'src-tauri/**', 'target/**'],
    server: {
      deps: {
        inline: ['@pixi/react'],
      },
    },
  },
});
