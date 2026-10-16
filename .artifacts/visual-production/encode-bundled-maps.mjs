import {createRequire} from 'node:module';import {readFile,writeFile,mkdir,copyFile} from 'node:fs/promises';import {createHash} from 'node:crypto';
const require=createRequire(import.meta.url),sharp=require('C:/Users/razer/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/sharp');
const manifest=JSON.parse(await readFile('docs/visual/asset-manifest.json','utf8')),assets=[];
await mkdir('crates/adapter-media/assets/bundled/maps/legacy',{recursive:true});
for(const name of ['tavern-interior','forest-crossing','ruined-courtyard','cave-chamber','dungeon-hall','temple-floor','village-square','tower-chamber']){
const id=`map-${name}`,entry=manifest.assets.find(asset=>asset.id===id);if(!entry||entry.version!==1)throw Error('map_already_upgraded_or_missing:'+id);
const legacyPath=`crates/adapter-media/assets/bundled/maps/legacy/${id}-v1.webp`;
await copyFile(entry.path,legacyPath);manifest.assets.push({...entry,id:`${id}.legacy.v1`,path:legacyPath,backendAssetId:null});
const record=JSON.parse(await readFile(`.artifacts/visual-production/${id}.json`,'utf8')),master=`.artifacts/visual-production/masters/${id}.png`;
let quality=86,bytes;
for(;quality>=66;quality-=4){bytes=await sharp(master).resize(1280,720,{fit:'fill'}).webp({quality}).toBuffer();if(bytes.length<=512*1024)break;}
if(!bytes||bytes.length>512*1024)throw Error('map_payload_budget:'+id);await writeFile(entry.path,bytes);
Object.assign(entry,{version:2,width:1280,height:720,bytes:bytes.length,sha256:createHash('sha256').update(bytes).digest('hex'),source:'builtin_imagegen'});
assets.push({assetId:id,generationId:record.generationId,prompt:record.prompt,master,masterSha256:createHash('sha256').update(await readFile(master)).digest('hex'),legacyAssetId:`${id}.legacy.v1`,consumers:['BundledImageProvider.map','VttCanvas.decorative-scene-preview','ToolCallCard'],codec:{width:1280,height:720,quality,bytes:bytes.length},selection:'candidate: full source reviewed, actual fallback/UI consumer QA pending'});
console.info(`[map:INFO] upgraded stable_id=${id} bytes=${bytes.length} q=${quality}`);
}
await writeFile('docs/visual/asset-manifest.json',JSON.stringify(manifest,null,2)+'\n');await writeFile('docs/visual/map-provenance.json',JSON.stringify({schemaVersion:1,generator:'imagegen_builtin',assets},null,2)+'\n');
