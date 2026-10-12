import { readFile, writeFile } from 'node:fs/promises';
import { spawnSync } from 'node:child_process';

interface Asset {
  id: string;
  path: string;
  backendAssetId?: string;
  [key: string]: unknown;
}

// The reviewed manifest owns identity. This generator never invents an ID or
// changes a selected portrait, and needs no image-processing dependency.
const { assets } = JSON.parse(await readFile('docs/visual/asset-manifest.json', 'utf8')) as {
  assets: Asset[];
};
const imports: string[] = [];
const entries: string[] = [];
for (const [index, asset] of assets.entries()) {
  if (!/^(src\/assets\/|crates\/adapter-media\/assets\/bundled\/)[\w./-]+\.(webp|png)$/.test(asset.path))
    throw new Error(`invalid_catalog_path:${asset.id}`);
  const path = asset.path.startsWith('src/assets/')
    ? `./${asset.path.slice('src/assets/'.length)}`
    : `../../${asset.path}`;
  imports.push(`import asset${index} from '${path}';`);
  entries.push(`{...${JSON.stringify({ ...asset, backendAssetId: asset.backendAssetId ?? null })},uri:asset${index}}`);
}
await writeFile('src/assets/visualCatalog.generated.ts',
  '// Generated from the reviewed asset manifest. Stable IDs never choose a random face.\n'
  + imports.join('\n') + '\nexport const VISUAL_ASSET_ENTRIES = [\n'
  + entries.join(',\n') + '\n] as const;\n');
console.info(`[assets:INFO] catalog generated count=${assets.length}`);
const format = spawnSync('bunx', ['biome', 'check', '--write', 'src/assets/visualCatalog.generated.ts'], { stdio: 'inherit' });
if (format.status !== 0) throw new Error('visual_catalog_format_failed');
