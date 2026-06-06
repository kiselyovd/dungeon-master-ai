/**
 * `bun run dev:all` - dev launch wired for LOCAL mode.
 *
 * `bun run tauri dev` already builds + spawns the Rust backend (dmai-server);
 * what it does NOT do is make the model sidecars usable. This wrapper:
 *   1. exports DMAI_IMAGE_SIDECAR_DEV=<repo root> so dmai-server runs the Python
 *      image sidecar from `.venv` (no PyInstaller bundle needed in dev), and
 *   2. preflights the mistralrs-server binary + the .venv, warning (not failing)
 *      with the exact build command when something is missing,
 * then execs `bun run tauri dev` with that environment.
 *
 * The sidecars still start on demand (when you Start the local runtime in-app);
 * this just removes the manual env juggling so that "Start" actually works.
 * For the one-time heavy builds, run `bun run setup:local` first.
 */
import { spawn } from 'node:child_process';
import { existsSync, statSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');

function targetTriple(): { triple: string; exe: string } {
  const { platform, arch } = process;
  if (platform === 'win32') return { triple: 'x86_64-pc-windows-msvc', exe: '.exe' };
  if (platform === 'darwin')
    return { triple: arch === 'arm64' ? 'aarch64-apple-darwin' : 'x86_64-apple-darwin', exe: '' };
  return { triple: 'x86_64-unknown-linux-gnu', exe: '' };
}

function isRealBinary(p: string): boolean {
  try {
    return statSync(p).size > 0;
  } catch {
    return false;
  }
}

const { triple, exe } = targetTriple();
const venvPython =
  process.platform === 'win32'
    ? join(ROOT, '.venv', 'Scripts', 'python.exe')
    : join(ROOT, '.venv', 'bin', 'python');
const mistralrs = join(ROOT, 'src-tauri', 'binaries', `mistralrs-server-${triple}${exe}`);

// --- preflight (warn, never block) -----------------------------------------
const warnings: string[] = [];
if (!isRealBinary(mistralrs)) {
  warnings.push(
    `Local LLM unavailable: mistralrs-server binary missing/placeholder at\n    ${mistralrs}\n    Build it: bun run setup:local  (or scripts/build_mistralrs.${process.platform === 'win32' ? 'ps1' : 'sh'})`,
  );
}
if (!existsSync(venvPython) || !existsSync(join(ROOT, 'sidecar', 'app.py'))) {
  warnings.push(
    `Local image gen unavailable: Python dev venv missing at\n    ${venvPython}\n    Set it up: bun run setup:local`,
  );
}

console.log('dev:all - launching with local-sidecar dev wiring');
console.log(`  DMAI_IMAGE_SIDECAR_DEV = ${ROOT}`);
if (warnings.length > 0) {
  console.log('\n  Preflight warnings (cloud mode still works; local mode needs these):');
  for (const w of warnings) console.log(`  - ${w}`);
  console.log('');
} else {
  console.log('  Preflight OK: mistralrs binary + Python venv present.\n');
}

// --- exec tauri dev ---------------------------------------------------------
const child = spawn('bun', ['run', 'tauri', 'dev'], {
  cwd: ROOT,
  stdio: 'inherit',
  env: { ...process.env, DMAI_IMAGE_SIDECAR_DEV: ROOT },
  shell: process.platform === 'win32', // resolve `bun` via PATHEXT on Windows
});

const forward = (sig: NodeJS.Signals) => child.kill(sig);
process.on('SIGINT', () => forward('SIGINT'));
process.on('SIGTERM', () => forward('SIGTERM'));
child.on('exit', (code) => process.exit(code ?? 0));
