/** Generate latest.json consumed by tauri-plugin-updater. */
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { basename, join } from 'node:path';

interface PlatformEntry {
  signature: string;
  url: string;
}

const labelToTauriKey: Record<string, string> = {
  'windows-x86_64': 'windows-x86_64',
  'darwin-aarch64': 'darwin-aarch64',
  'linux-x86_64': 'linux-x86_64',
};

export function normaliseReleaseVersion(tag: string): string {
  const version = tag.trim().replace(/^v(?=\d)/, '');
  if (!/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/.test(version)) {
    throw new Error(`Release tag is not SemVer-compatible: ${tag}`);
  }
  return version;
}

function walk(directory: string): string[] {
  const output: string[] = [];
  for (const entry of readdirSync(directory)) {
    const fullPath = join(directory, entry);
    if (statSync(fullPath).isDirectory()) output.push(...walk(fullPath));
    else output.push(fullPath);
  }
  return output;
}

function buildManifest() {
  const tag = process.env.GITHUB_REF_NAME;
  if (!tag) throw new Error('GITHUB_REF_NAME is required');
  const version = normaliseReleaseVersion(tag);
  const baseUrl = `https://github.com/kiselyovd/dungeon-master-ai/releases/download/${encodeURIComponent(tag)}`;
  const platforms: Record<string, PlatformEntry> = {};

  for (const directory of readdirSync('artifacts')) {
    if (!directory.startsWith('release-')) continue;
    const label = directory.slice('release-'.length);
    const tauriKey = labelToTauriKey[label];
    if (!tauriKey) continue;

    const files = walk(join('artifacts', directory));
    const signatureFile = files.find((file) => file.endsWith('.sig'));
    if (!signatureFile) {
      throw new Error(`No updater signature found in artifacts/${directory}`);
    }

    const bundleFile = signatureFile.slice(0, -'.sig'.length);
    if (!statSync(bundleFile).isFile()) {
      throw new Error(`Signed bundle is missing for ${signatureFile}`);
    }

    platforms[tauriKey] = {
      signature: readFileSync(signatureFile, 'utf8').trim(),
      url: `${baseUrl}/${encodeURIComponent(basename(bundleFile))}`,
    };
  }

  const missing = Object.values(labelToTauriKey).filter((key) => !platforms[key]);
  if (missing.length > 0) {
    throw new Error(`latest.json would be incomplete; missing platforms: ${missing.join(', ')}`);
  }

  return {
    version,
    notes: 'See CHANGELOG.md',
    pub_date: new Date().toISOString(),
    platforms,
  };
}

console.log(JSON.stringify(buildManifest(), null, 2));
