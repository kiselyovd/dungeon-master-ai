/** Verify every staged updater bundle against the application's existing public key. */
import { spawnSync } from 'node:child_process';
import { mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { normaliseReleaseVersion } from './build_latest_json';

interface VerificationOptions {
  tag: string;
  repository: string;
  manifest: { version: string; platforms: Record<string, { url: string; signature: string }> };
  publicKey: string;
  assetsDirectory: string;
  commit: string | null;
}

type SignatureVerifier = (publicKey: string, bundle: string, signature: string) => boolean;

const platforms = ['windows-x86_64', 'darwin-aarch64', 'linux-x86_64'];

function decodeSigningText(value: unknown): Buffer {
  if (
    typeof value !== 'string' ||
    !value ||
    Buffer.from(value, 'base64').toString('base64') !== value
  ) {
    throw new Error('Updater signing material is not canonical base64');
  }
  return Buffer.from(value, 'base64');
}

function runMinisign(publicKey: string, bundle: string, signature: string): boolean {
  const result = spawnSync('minisign', ['-V', '-p', publicKey, '-m', bundle, '-x', signature], {
    encoding: 'utf8',
  });
  return !result.error && result.status === 0;
}

export function verifyReleaseSignatures(
  options: VerificationOptions,
  verify: SignatureVerifier = runMinisign,
) {
  const { tag, repository, manifest, assetsDirectory } = options;
  if (manifest.version !== normaliseReleaseVersion(tag)) {
    throw new Error('Updater manifest version does not match the release tag');
  }
  if (Object.keys(manifest.platforms).sort().join() !== [...platforms].sort().join()) {
    throw new Error('Updater manifest must contain exactly the three supported platforms');
  }
  const directory = mkdtempSync(join(tmpdir(), 'dmai-updater-verify-'));
  const verified: Record<string, { asset: string; verified: boolean }> = {};
  try {
    const publicKey = join(directory, 'updater.pub');
    writeFileSync(publicKey, decodeSigningText(options.publicKey));
    const prefix = `https://github.com/${repository}/releases/download/${encodeURIComponent(tag)}/`;
    for (const platform of platforms) {
      const entry = manifest.platforms[platform];
      if (typeof entry.url !== 'string' || !entry.url.startsWith(prefix)) {
        throw new Error(`Unexpected updater download origin for ${platform}`);
      }
      const asset = decodeURIComponent(entry.url.slice(prefix.length));
      if (!/^[A-Za-z0-9][A-Za-z0-9._-]*$/.test(asset)) {
        throw new Error(`Unsafe updater asset name for ${platform}`);
      }
      const bundle = join(assetsDirectory, asset);
      if (readFileSync(`${bundle}.sig`, 'utf8').trim() !== entry.signature) {
        throw new Error(`Manifest signature does not match the staged signature for ${platform}`);
      }
      verified[platform] = { asset, verified: true };
    }
    const signedAssets: string[] = [];
    for (const filename of readdirSync(assetsDirectory)
      .filter((name) => name.endsWith('.sig'))
      .sort()) {
      const asset = filename.slice(0, -4);
      if (!/^[A-Za-z0-9][A-Za-z0-9._-]*$/.test(asset)) throw new Error('Unsafe signed asset name');
      const bundle = join(assetsDirectory, asset);
      // Reading the bundle also rejects orphan signatures before publication.
      readFileSync(bundle);
      const signature = join(directory, 'package.minisig');
      writeFileSync(signature, decodeSigningText(readFileSync(`${bundle}.sig`, 'utf8').trim()));
      if (!verify(publicKey, bundle, signature)) {
        throw new Error(
          `Updater signature verification failed for ${asset}; check signer/public-key compatibility`,
        );
      }
      signedAssets.push(asset);
    }
    return {
      version: manifest.version,
      commit: options.commit,
      platforms: verified,
      signed_assets: signedAssets,
    };
  } finally {
    rmSync(directory, { recursive: true });
  }
}

function main() {
  const tag = process.env.GITHUB_REF_NAME;
  if (!tag) throw new Error('GITHUB_REF_NAME is required');
  const config = JSON.parse(readFileSync('src-tauri/tauri.conf.json', 'utf8'));
  console.log(
    JSON.stringify(
      verifyReleaseSignatures({
        tag,
        repository: process.env.GITHUB_REPOSITORY ?? 'kiselyovd/dungeon-master-ai',
        manifest: JSON.parse(readFileSync('latest.json', 'utf8')),
        publicKey: config.plugins.updater.pubkey,
        assetsDirectory: 'release-assets',
        commit: process.env.GITHUB_SHA ?? null,
      }),
      null,
      2,
    ),
  );
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) main();
