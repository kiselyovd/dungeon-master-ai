import {createRequire} from 'node:module';import {readFile,writeFile} from 'node:fs/promises';import {createHash} from 'node:crypto';
const require=createRequire(import.meta.url),sharp=require('C:/Users/razer/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/sharp');
const manifest=JSON.parse(await readFile('docs/visual/asset-manifest.json','utf8')),locale=JSON.parse(await readFile('src/locales/ru/adventure.json','utf8')),assets=[];
for(const name of ['town','quay','shrine','warehouse','archive','belfry']){
const record=JSON.parse(await readFile(`.artifacts/visual-production/bell-scene-${name}.json`,'utf8')),master=`.artifacts/visual-production/masters/bell-scene-${name}.png`,path=`src/assets/living-tabletop/v2/bell-scene-${name}-v2.webp`,id=`scene.bell.${name}.v2`;
let quality=86,bytes;
for(;quality>=66;quality-=4){bytes=await sharp(master).resize({width:1440}).webp({quality}).toBuffer();if(bytes.length<=450*1024)break;}
if(!bytes||bytes.length>450*1024)throw Error('scene_budget:'+name);await writeFile(path,bytes);const info=await sharp(bytes).metadata();
manifest.assets=manifest.assets.filter(a=>a.id!==id);manifest.assets.push({id,version:2,category:'scene_art',path,width:info.width,height:info.height,alpha:false,bytes:bytes.length,sha256:createHash('sha256').update(bytes).digest('hex'),heroClass:null,pairedAssetId:null,source:'builtin_imagegen'});
assets.push({assetId:id,generationId:record.generationId,prompt:record.prompt,master,masterSha256:createHash('sha256').update(await readFile(master)).digest('hex'),publicSource:record.publicSource,publicDescription:locale.scenes[name],consumers:['VttCanvas.public-current-scene','SceneArtwork.preview'],codec:{quality,width:info.width,height:info.height,bytes:bytes.length},selection:'candidate: full source viewed; pending production transition/crop QA'});console.info(`[art:INFO] ${id} bytes=${bytes.length} q=${quality}`);
}
await writeFile('docs/visual/asset-manifest.json',JSON.stringify(manifest,null,2)+'\n');await writeFile('docs/visual/bell-scene-provenance.json',JSON.stringify({schemaVersion:1,generator:'imagegen_builtin',assets},null,2)+'\n');
