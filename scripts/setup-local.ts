/**
 * `bun run setup:local` - one-time heavy setup for LOCAL mode (the bits that
 * `bun run dev:all` only warns about):
 *   1. Python dev venv at .venv + sidecar/requirements.txt (image sidecar), and
 *   2. mistralrs-server built from source into src-tauri/binaries/ (local LLM).
 *
 * Pass --cuda to build mistralrs with GPU acceleration (needs the CUDA toolkit /
 * nvcc on PATH; the documented path for RTX 3080 users). Without it you get the
 * portable CPU-only build. Steps are best-effort: a failure in one is reported
 * but does not abort the others (so a missing CUDA toolkit still leaves you with
 * a working Python venv).
 */
import { spawnSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');
const isWin = process.platform === 'win32';
const cuda = process.argv.includes('--cuda');

function triple(): string {
  if (isWin) return 'x86_64-pc-windows-msvc';
  if (process.platform === 'darwin')
    return process.arch === 'arm64' ? 'aarch64-apple-darwin' : 'x86_64-apple-darwin';
  return 'x86_64-unknown-linux-gnu';
}

const results: { step: string; ok: boolean }[] = [];

function run(step: string, cmd: string, args: string[]): void {
  console.log(`\n=== ${step} ===\n$ ${cmd} ${args.join(' ')}`);
  const r = spawnSync(cmd, args, { cwd: ROOT, stdio: 'inherit', shell: isWin });
  results.push({ step, ok: r.status === 0 });
}

// 1. Python venv + image-sidecar deps -------------------------------------
const venvPython = isWin
  ? join(ROOT, '.venv', 'Scripts', 'python.exe')
  : join(ROOT, '.venv', 'bin', 'python');
if (!existsSync(venvPython)) {
  run('create .venv', isWin ? 'python' : 'python3', ['-m', 'venv', '.venv']);
} else {
  console.log('\n=== .venv already exists, skipping create ===');
  results.push({ step: 'create .venv', ok: true });
}
run('pip install image-sidecar deps', venvPython, [
  '-m',
  'pip',
  'install',
  '-r',
  join('sidecar', 'requirements.txt'),
]);

// 2. mistralrs-server from source ------------------------------------------
if (isWin) {
  const args = ['-NoProfile', '-File', join('scripts', 'build_mistralrs.ps1'), '-Target', triple()];
  if (cuda) args.push('-Cuda');
  run('build mistralrs-server', 'pwsh', args);
} else {
  const args = [join('scripts', 'build_mistralrs.sh'), triple()];
  if (cuda) args.push('--cuda');
  run('build mistralrs-server', 'bash', args);
}

// --- summary ---------------------------------------------------------------
console.log('\n========== setup:local summary ==========');
for (const { step, ok } of results) console.log(`  ${ok ? 'OK  ' : 'FAIL'}  ${step}`);
const failed = results.filter((r) => !r.ok);
if (failed.length > 0) {
  console.log(`\n${failed.length} step(s) failed. Local mode may be partially unavailable.`);
  if (!cuda) console.log('Tip: re-run with --cuda for GPU local LLM (needs the CUDA toolkit).');
  process.exit(1);
}
console.log('\nLocal mode ready. Launch with: bun run dev:all');
