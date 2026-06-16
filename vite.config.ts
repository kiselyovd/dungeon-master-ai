import react from '@vitejs/plugin-react';
import { defineConfig } from 'vite';

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [react()],

  define: {
    // @ts-expect-error process is a nodejs global
    __APP_VERSION__: JSON.stringify(process.env.npm_package_version ?? '0.0.0'),
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || '127.0.0.1',
    hmr: host
      ? {
          protocol: 'ws',
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. Ignore non-frontend trees. Watching them does nothing useful, and the
      // Python venv (`<root>/.venv`, thousands of files), the sidecar sources and
      // the multi-GB model cache otherwise trigger constant phantom page reloads.
      ignored: [
        '**/src-tauri/**',
        '**/.venv/**',
        '**/sidecar/**',
        '**/.cache-models/**',
        '**/target/**',
      ],
    },
  },
}));
