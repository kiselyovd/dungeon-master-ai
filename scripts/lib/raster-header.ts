/** Header facts only; this is not a complete pixel/codec decoder.
 * WebP layout: https://developers.google.com/speed/webp/docs/riff_container */
export interface RasterHeader {width:number;height:number;alpha:boolean}
export function rasterHeader(bytes: Buffer): RasterHeader {
  if(bytes.length>=33 && bytes.subarray(0,8).equals(Buffer.from([137,80,78,71,13,10,26,10]))) {
    let alpha=bytes[25]===4||bytes[25]===6;
    for(let offset=8;offset+12<=bytes.length;) {
      const length=bytes.readUInt32BE(offset);
      if(offset+12+length>bytes.length)throw new Error('png_chunk_out_of_bounds');
      if(bytes.toString('ascii',offset+4,offset+8)==='tRNS')alpha=true;
      offset+=length+12;
    }
    return {width:bytes.readUInt32BE(16),height:bytes.readUInt32BE(20),alpha};
  }
  if(bytes.length<20||bytes.toString('ascii',0,4)!=='RIFF'||bytes.toString('ascii',8,12)!=='WEBP'||bytes.readUInt32LE(4)+8!==bytes.length)throw new Error('unsupported_raster_header');
  for(let offset=12;offset+8<=bytes.length;) {
    const type=bytes.toString('ascii',offset,offset+4),length=bytes.readUInt32LE(offset+4),start=offset+8;
    if(start+length>bytes.length)throw new Error('webp_chunk_out_of_bounds');
    if(type==='VP8X' && length>=10)return {width:bytes.readUIntLE(start+4,3)+1,height:bytes.readUIntLE(start+7,3)+1,alpha:((bytes[start]??0)&16)!==0};
    if(type==='VP8L' && length>=5 && bytes[start]===47) {const bits=bytes.readUInt32LE(start+1);return {width:(bits&16383)+1,height:((bits>>>14)&16383)+1,alpha:(bits&0x10000000)!==0};}
    if(type==='VP8 ' && length>=10 && bytes.subarray(start+3,start+6).equals(Buffer.from([157,1,42])))return {width:bytes.readUInt16LE(start+6)&16383,height:bytes.readUInt16LE(start+8)&16383,alpha:false};
    offset=start+length+(length&1);
  }
  throw new Error('missing_webp_dimensions');
}
