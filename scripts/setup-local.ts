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

function run(step: string, cmd: string, args: string[], env?: Record<string, string>): void {
  console.log(`\n=== ${step} ===\n$ ${cmd} ${args.join(' ')}`);
  const r = spawnSync(cmd, args, {
    cwd: ROOT,
    stdio: 'inherit',
    shell: isWin,
    env: env ? { ...process.env, ...env } : process.env,
  });
  results.push({ step, ok: r.status === 0 });
}

// 1. Python 3.12 + image-sidecar deps via uv ------------------------------
// uv owns Python provisioning and dependency install from sidecar/pyproject.toml
// + uv.lock (reproducible). UV_PROJECT_ENVIRONMENT pins the venv at the repo
// root (<root>/.venv) - the location dev-all.ts and the Rust dev-sidecar
// detection already expect - even though the pyproject lives in sidecar/.
const rootVenv = join(ROOT, '.venv');
run('uv: provision Python 3.12', 'uv', ['python', 'install', '3.12']);
run('uv sync: image-sidecar deps', 'uv', ['sync', '--project', join(ROOT, 'sidecar')], {
  UV_PROJECT_ENVIRONMENT: rootVenv,
});

// 2. mistralrs-server from source ------------------------------------------
if (isWin) {
  const args = ['-NoProfile', '-File', join('scripts', 'build_mistralrs.ps1'), '-Target', triple()];
  if (cuda) args.push('-Cuda');
  // Prefer PowerShell 7 (`pwsh`) but fall back to Windows PowerShell, which is
  // always present - `pwsh` is NOT installed by default, so calling it blindly
  // made `setup:local` fail before it ever built mistralrs. (Audit blocker 3.)
  const probe = spawnSync('pwsh', ['-NoProfile', '-Command', '$host.Version.Major'], {
    stdio: 'ignore',
    shell: true,
  });
  const psExe = probe.status === 0 ? 'pwsh' : 'powershell';
  run('build mistralrs-server', psExe, args);
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
