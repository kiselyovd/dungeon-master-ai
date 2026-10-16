import { mkdtempSync, readFileSync, rmSync, unlinkSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { basename, join } from 'node:path';
import { expect, it } from 'vitest';
import { verifyReleaseSignatures } from '../verify-release-signatures';

it('rejects a damaged additional installer even when all three manifest bundles pass', () => {
  const directory = mkdtempSync(join(tmpdir(), 'dmai-signature-inputs-'));
  const signature = Buffer.from('fixture-signature').toString('base64');
  const assets = [
    ['windows-x86_64', 'setup.exe'],
    ['darwin-aarch64', 'application.app.tar.gz'],
    ['linux-x86_64', 'application.AppImage'],
  ] as const;
  const options = {
    tag: 'v0.13.0',
    repository: 'kiselyovd/dungeon-master-ai',
    publicKey: Buffer.from('fixture-public-key').toString('base64'),
    assetsDirectory: directory,
    commit: null,
    manifest: {
      version: '0.13.0',
      platforms: Object.fromEntries(
        assets.map(([platform, asset]) => [
          platform,
          {
            url: `https://github.com/kiselyovd/dungeon-master-ai/releases/download/v0.13.0/${asset}`,
            signature,
          },
        ]),
      ),
    },
  };
  const delegated: string[] = [];
  const verifier = (_publicKey: string, bundle: string) => {
    delegated.push(basename(bundle));
    return readFileSync(bundle, 'utf8') === 'test-only signed bytes';
  };
  try {
    for (const [, asset] of assets) {
      writeFileSync(join(directory, asset), 'test-only signed bytes');
      writeFileSync(join(directory, `${asset}.sig`), signature);
    }
    writeFileSync(join(directory, 'additional.msi'), 'test-only signed bytes');
    writeFileSync(join(directory, 'additional.msi.sig'), signature);
    expect(verifyReleaseSignatures(options, verifier).signed_assets).toHaveLength(4);
    expect(delegated).toContain('additional.msi');
    writeFileSync(join(directory, 'additional.msi'), 'damaged bytes');
    expect(() => verifyReleaseSignatures(options, verifier)).toThrow(
      'verification failed for additional.msi',
    );
    unlinkSync(join(directory, 'additional.msi'));
    expect(() => verifyReleaseSignatures(options, verifier)).toThrow();
  } finally {
    rmSync(directory, { recursive: true });
  }
});
