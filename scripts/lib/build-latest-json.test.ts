import { mkdirSync, mkdtempSync, readFileSync, rmSync, unlinkSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';
import {
  buildReleaseManifest,
  extractReleaseNotes,
  stageReleaseAssets,
} from '../build_latest_json';

describe('signed updater release manifest', () => {
  it('publishes three supported bundle types and fails closed on incomplete release inputs', () => {
    const artifactsDirectory = mkdtempSync(join(tmpdir(), 'dmai-release-'));
    const bundles = [
      ['windows-x86_64', 'Dungeon Master AI_0.13.0_x64-setup.exe'],
      ['darwin-aarch64', 'Dungeon Master AI.app.tar.gz'],
      ['linux-x86_64', 'Dungeon Master AI_0.13.0_amd64.AppImage'],
    ] as const;
    const version = '0.13.0';
    const notes = extractReleaseNotes(
      '# Changelog\n\n## Unreleased\nFuture work\n\n## [0.13.0] - 2026-10-16\nCurrent changes\n\n## [0.12.1]\nOlder changes\n',
      version,
    );
    const options = {
      tag: `v${version}`,
      versions: { package: version, tauri: version, native: version },
      notes,
      artifactsDirectory,
      repository: 'kiselyovd/dungeon-master-ai',
      publishedAt: '2026-10-16T12:00:00Z',
    };
    try {
      for (const [label, filename] of bundles) {
        const directory = join(artifactsDirectory, `release-${label}`, 'bundle');
        mkdirSync(directory, { recursive: true });
        writeFileSync(join(directory, filename), 'unit-test bundle');
        writeFileSync(join(directory, `${filename}.sig`), `fixture-${label}\n`);
      }
      const manifest = buildReleaseManifest(options);
      expect(manifest.version).toBe(version);
      expect(manifest.notes).toBe('Current changes');
      expect(manifest.pub_date).toBe(options.publishedAt);
      expect(Object.keys(manifest.platforms)).toEqual(bundles.map(([label]) => label));
      for (const [label, filename] of bundles) {
        expect(manifest.platforms[label]).toEqual({
          signature: `fixture-${label}`,
          url: `https://github.com/kiselyovd/dungeon-master-ai/releases/download/v0.13.0/${filename.replaceAll(' ', '.')}`,
        });
      }
      const uploadDirectory = join(artifactsDirectory, 'upload');
      const rpm = join(artifactsDirectory, 'release-linux-x86_64', 'bundle', 'application.rpm');
      writeFileSync(rpm, 'optional RPM installer');
      stageReleaseAssets(artifactsDirectory, uploadDirectory);
      expect(readFileSync(join(uploadDirectory, 'application.rpm'), 'utf8')).toBe(
        'optional RPM installer',
      );
      for (const [label, filename] of bundles) {
        const uploaded = join(uploadDirectory, filename.replaceAll(' ', '.'));
        expect(readFileSync(uploaded, 'utf8')).toBe('unit-test bundle');
        expect(readFileSync(`${uploaded}.sig`, 'utf8')).toBe(`fixture-${label}\n`);
      }
      expect(() =>
        buildReleaseManifest({ ...options, versions: { ...options.versions, native: '0.12.1' } }),
      ).toThrow('does not match native version');
      expect(() => extractReleaseNotes('## Unreleased\nNo versioned entry', version)).toThrow(
        'has no release notes',
      );
      const macDirectory = join(artifactsDirectory, 'release-darwin-aarch64', 'bundle');
      const macBundle = join(macDirectory, bundles[1][1]);
      unlinkSync(`${macBundle}.sig`);
      expect(() => buildReleaseManifest(options)).toThrow('No signed updater bundle');
      writeFileSync(`${macBundle}.sig`, ' \n');
      expect(() => buildReleaseManifest(options)).toThrow('Updater signature is empty');
      unlinkSync(macBundle);
      writeFileSync(join(macDirectory, 'Dungeon Master AI.dmg'), 'installer fixture');
      writeFileSync(join(macDirectory, 'Dungeon Master AI.dmg.sig'), 'fixture-dmg');
      expect(() => buildReleaseManifest(options)).toThrow('No signed updater bundle');
    } finally {
      rmSync(artifactsDirectory, { recursive: true, force: true });
    }
  });
});
