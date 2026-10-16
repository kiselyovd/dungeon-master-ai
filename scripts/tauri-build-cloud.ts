/** Build a cloud-only Tauri bundle without empty local-model sidecars. */
import { spawn } from 'node:child_process';

const child = spawn(
  process.execPath,
  ['x', 'tauri', 'build', '--config', 'src-tauri/tauri.cloud.conf.json'],
  {
    cwd: process.cwd(),
    env: { ...process.env, DMAI_CLOUD_ONLY: '1' },
    stdio: 'inherit',
  },
);

child.on('error', (error) => {
  console.error(`Failed to launch the Tauri cloud build: ${error.message}`);
  process.exit(1);
});
child.on('exit', (code, signal) => {
  if (signal) {
    console.error(`Tauri cloud build terminated by ${signal}`);
    process.exit(1);
  }
  process.exit(code ?? 1);
});
