import { createHash } from 'node:crypto';
import { readFile, readdir } from 'node:fs/promises';
import { resolve,sep } from 'node:path';
import { rasterHeader } from './lib/raster-header';

interface Asset {id:string;version:number;category:string;path:string;width:number;height:number;alpha:boolean;bytes:number;sha256:string;pairedAssetId?:string|null;backendPath?:string;backendAssetId?:string;source:string;spriteBounds?:{left:number;top:number;right:number;bottom:number}}
const root=process.cwd();
const manifest=JSON.parse(await readFile('docs/visual/asset-manifest.json','utf8')) as {schemaVersion:number;assets:Asset[]};
if(manifest.schemaVersion!==1)throw new Error('visual_manifest_version');
const ids=new Set<string>();
async function cssSources(directory:string):Promise<string[]> {
  const contents:string[]=[];
  for(const entry of await readdir(directory,{withFileTypes:true})) {
    const path=`${directory}/${entry.name}`;
    if(entry.isDirectory())contents.push(...await cssSources(path));
    else if(entry.name.endsWith('.css'))contents.push(await readFile(path,'utf8'));
  }
  return contents;
}
const css=(await cssSources('src')).join('\n');
for(const asset of manifest.assets) {
  if(ids.has(asset.id))throw new Error(`duplicate_visual_identity:${asset.id}`);
  ids.add(asset.id);
  if(!Number.isInteger(asset.version)||asset.version<1||!Number.isInteger(asset.width)||asset.width<1||!Number.isInteger(asset.height)||asset.height<1)throw new Error(`invalid_visual_metadata:${asset.id}`);
  const path=resolve(root,asset.path);
  if(!path.startsWith(root+sep))throw new Error('visual_path_outside_repository');
  const bytes=await readFile(path);
  const header=rasterHeader(bytes);
  if(header.width!==asset.width||header.height!==asset.height||header.alpha!==asset.alpha)throw new Error(`visual_header_mismatch:${asset.id}`);
  if(bytes.length!==asset.bytes||createHash('sha256').update(bytes).digest('hex')!==asset.sha256)throw new Error(`visual_asset_changed:${asset.id}`);
  if(asset.version>=2 && bytes.length>({material:220*1024,portrait:180*1024,miniature:350*1024,map:900*1024}[asset.category]??450*1024))throw new Error(`visual_budget_exceeded:${asset.id}`);
  if(asset.id.startsWith('opponent.')) {
    const bounds=asset.spriteBounds;
    if(!asset.alpha||asset.category!=='miniature'||!bounds||!Object.values(bounds).every(Number.isInteger)||bounds.left<1||bounds.top<1||bounds.right>=asset.width-1||bounds.bottom>=asset.height-1||bounds.right<=bounds.left||bounds.bottom<=bounds.top)throw new Error('invalid_opponent_bounds:'+asset.id);
  }
  if(asset.backendPath && createHash('sha256').update(await readFile(asset.backendPath)).digest('hex')!==asset.sha256)throw new Error(`backend_visual_mismatch:${asset.id}`);
  if(asset.version>=2 && (asset.category==='material'||asset.category==='frame')) {
    const stem=asset.path.split('/').at(-1)?.replace(/\.webp$/,'').replace(/-v2$/,'') ?? '';
    const token=stem.replace(/^material-/,'--dm-texture-').replace(/^frame-/,'--dm-frame-');
    if(!css.includes(`var(${token})`))throw new Error(`unused_decorative_master:${asset.id}`);
  }
}
for(const asset of manifest.assets) {
  if(asset.pairedAssetId) {
    const pair=manifest.assets.find(other=>other.id===asset.pairedAssetId);
    if(asset.category!=='portrait'||pair?.category!=='miniature'||!pair.alpha)throw new Error(`invalid_portrait_pair:${asset.id}`);
  }
}
const provenance=JSON.parse(await readFile('docs/visual/pilot-provenance.json','utf8')) as {assets:Array<{assetId:string;generationId:string;referenceAssetId?:string}>};
const materials=JSON.parse(await readFile('docs/visual/material-provenance.json','utf8')) as {assets:Array<{assetId:string;generationId:string;consumers:string[]}>};
const heroes=JSON.parse(await readFile('docs/visual/hero-provenance.json','utf8')) as {assets:Array<{assetId:string;generationId:string;consumers:string[]}>};
const npcs=JSON.parse(await readFile('docs/visual/npc-provenance.json','utf8')) as {assets:Array<{assetId:string;generationId:string;consumers:string[]}>};
const opponents=JSON.parse(await readFile('docs/visual/opponent-provenance.json','utf8')) as {assets:Array<{assetId:string;generationId:string;consumers:string[]}>};
const keySave=JSON.parse(await readFile('docs/visual/key-save-provenance.json','utf8')) as {assets:Array<{assetId:string;generationId:string;consumers:string[]}>};
provenance.assets.push(...materials.assets,...heroes.assets,...npcs.assets,...opponents.assets,...keySave.assets);
for(const asset of manifest.assets.filter(asset=>asset.source==='builtin_imagegen')) {
  if(!provenance.assets.some(entry=>entry.assetId===asset.id&&entry.generationId))throw new Error(`missing_visual_provenance:${asset.id}`);
  if(asset.category==='frame' && !asset.alpha)throw new Error(`frame_alpha_missing:${asset.id}`);
}
console.log(`[assets:INFO] verified count=${manifest.assets.length} bytes=${manifest.assets.reduce((sum,asset)=>sum+asset.bytes,0)}`);
