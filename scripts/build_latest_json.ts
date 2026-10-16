/** Generate the signed-bundle manifest consumed by tauri-plugin-updater. */
import { constants, copyFileSync, mkdirSync, readdirSync, readFileSync, statSync } from 'node:fs';
import { basename, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

interface PlatformEntry {
  signature: string;
  url: string;
}

interface ReleaseVersions {
  package: string;
  tauri: string;
  native: string;
}

interface ManifestOptions {
  tag: string;
  versions: ReleaseVersions;
  notes: string;
  artifactsDirectory: string;
  repository: string;
  publishedAt?: string;
}

const platformBundles = [
  { label: 'windows-x86_64', suffixes: ['.exe', '.msi'] },
  { label: 'darwin-aarch64', suffixes: ['.app.tar.gz'] },
  { label: 'linux-x86_64', suffixes: ['.AppImage'] },
] as const;

export function normaliseReleaseVersion(tag: string): string {
  const version = tag.trim().replace(/^v(?=\d)/, '');
  if (!/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/.test(version)) {
    throw new Error(`Release tag is not SemVer-compatible: ${tag}`);
  }
  return version;
}

export function validateReleaseVersions(tag: string, versions: ReleaseVersions): string {
  const version = normaliseReleaseVersion(tag);
  for (const [source, actual] of Object.entries(versions)) {
    if (actual !== version) {
      throw new Error(`Release tag ${tag} does not match ${source} version ${actual}`);
    }
  }
  return version;
}

export function extractReleaseNotes(changelog: string, version: string): string {
  const lines = changelog.split(/\r?\n/);
  const start = lines.findIndex((line) => {
    const heading =
      /^##\s+\[?v?(\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?)\]?(?:\s|$)/.exec(line);
    return heading?.[1] === version;
  });
  if (start === -1) throw new Error(`CHANGELOG.md has no release notes for ${version}`);
  const next = lines.findIndex((line, index) => index > start && /^##\s/.test(line));
  const notes = lines
    .slice(start + 1, next === -1 ? undefined : next)
    .join('\n')
    .trim();
  if (!notes) throw new Error(`Release notes for ${version} are empty`);
  return notes;
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

function releaseAssetName(file: string): string {
  return basename(file).replace(/[^A-Za-z0-9._-]/g, '.');
}

export function stageReleaseAssets(artifactsDirectory: string, outputDirectory: string): void {
  mkdirSync(outputDirectory);
  for (const { label } of platformBundles) {
    for (const file of walk(join(artifactsDirectory, `release-${label}`))) {
      if (/\.(?:dmg|msi|exe|AppImage|deb|rpm|app\.tar\.gz)(?:\.sig)?$/.test(file)) {
        copyFileSync(file, join(outputDirectory, releaseAssetName(file)), constants.COPYFILE_EXCL);
      }
    }
  }
}

export function buildReleaseManifest(options: ManifestOptions) {
  const version = validateReleaseVersions(options.tag, options.versions);
  const baseUrl = `https://github.com/${options.repository}/releases/download/${encodeURIComponent(options.tag)}`;
  const platforms: Record<string, PlatformEntry> = {};

  for (const { label, suffixes } of platformBundles) {
    const directory = join(options.artifactsDirectory, `release-${label}`);
    const files = walk(directory).sort();
    const bundleFile = suffixes
      .flatMap((suffix) => files.filter((file) => file.endsWith(suffix)))
      .find((file) => files.includes(`${file}.sig`));
    if (!bundleFile) throw new Error(`No signed updater bundle found in ${directory}`);
    const signature = readFileSync(`${bundleFile}.sig`, 'utf8').trim();
    if (!signature) throw new Error(`Updater signature is empty for ${bundleFile}`);
    platforms[label] = {
      signature,
      url: `${baseUrl}/${releaseAssetName(bundleFile)}`,
    };
  }

  return {
    version,
    notes: options.notes,
    pub_date: options.publishedAt ?? new Date().toISOString(),
    platforms,
  };
}

function main() {
  const tag = process.env.GITHUB_REF_NAME;
  if (!tag) throw new Error('GITHUB_REF_NAME is required');
  const nativePackage = /\[package\]([\s\S]*?)(?=\n\[|$)/.exec(
    readFileSync('src-tauri/Cargo.toml', 'utf8'),
  )?.[1];
  const nativeVersion = nativePackage && /^version\s*=\s*"([^"]+)"/m.exec(nativePackage)?.[1];
  if (!nativeVersion) throw new Error('src-tauri/Cargo.toml package version is missing');
  const versions: ReleaseVersions = {
    package: JSON.parse(readFileSync('package.json', 'utf8')).version,
    tauri: JSON.parse(readFileSync('src-tauri/tauri.conf.json', 'utf8')).version,
    native: nativeVersion,
  };
  const version = validateReleaseVersions(tag, versions);
  const notes = extractReleaseNotes(readFileSync('CHANGELOG.md', 'utf8'), version);
  if (process.argv[2] === '--notes') {
    console.log(notes);
    return;
  }
  const manifest = buildReleaseManifest({
    tag,
    versions,
    notes,
    artifactsDirectory: 'artifacts',
    repository: process.env.GITHUB_REPOSITORY ?? 'kiselyovd/dungeon-master-ai',
  });
  if (process.argv[2] === '--stage-assets') {
    stageReleaseAssets('artifacts', 'release-assets');
    return;
  }
  console.log(JSON.stringify(manifest, null, 2));
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) main();
