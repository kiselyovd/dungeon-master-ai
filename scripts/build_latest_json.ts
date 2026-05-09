/**
 * Generate `latest.json` consumed by tauri-plugin-updater. Run from the
 * release.yml `publish` job after all matrix jobs have uploaded their
 * release-<label> artifacts.
 *
 * Reads each release-<label>/ directory under `artifacts/`, picks the .sig
 * file produced by the Tauri signing plugin, and emits the platform map
 * the updater client expects (windows-x86_64 / darwin-aarch64 / etc.).
 */
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { join } from 'node:path';

interface PlatformEntry {
  signature: string;
  url: string;
}

const tag = process.env.GITHUB_REF_NAME ?? 'unknown';
const baseUrl = `https://github.com/kiselyovd/dungeon-master-ai/releases/download/${tag}`;
const platforms: Record<string, PlatformEntry> = {};

const labelToTauriKey: Record<string, string> = {
  'windows-x86_64': 'windows-x86_64',
  'darwin-aarch64': 'darwin-aarch64',
  // darwin-x86_64 dropped from first-GA matrix; revisit if Intel Mac demand emerges.
  'linux-x86_64': 'linux-x86_64',
};

const walk = (dir: string): string[] => {
  const out: string[] = [];
  for (const entry of readdirSync(dir)) {
    const full = join(dir, entry);
    if (statSync(full).isDirectory()) out.push(...walk(full));
    else out.push(full);
  }
  return out;
};

for (const dir of readdirSync('artifacts')) {
  if (!dir.startsWith('release-')) continue;
  const label = dir.slice('release-'.length);
  const tauriKey = labelToTauriKey[label];
  if (!tauriKey) continue;

  const files = walk(join('artifacts', dir));
  const sigFile = files.find((f) => f.endsWith('.sig'));
  if (!sigFile) continue;

  const signature = readFileSync(sigFile, 'utf-8').trim();
  const bundleFile = sigFile.slice(0, -'.sig'.length);
  const baseName = bundleFile.split(/[\\/]/).pop() ?? bundleFile;
  platforms[tauriKey] = {
    signature,
    url: `${baseUrl}/${baseName}`,
  };
}

const out = {
  version: tag,
  notes: 'See CHANGELOG.md',
  pub_date: new Date().toISOString(),
  platforms,
};
console.log(JSON.stringify(out, null, 2));
