//! Bounded raster container inspection. Does not execute a pixel decoder or fetch URLs.
use app_application::{models::media_asset::*, ports::media_assets::MediaContentInspector};
use sha2::{Digest, Sha256};

pub struct RasterContentInspector;
impl MediaContentInspector for RasterContentInspector {
    fn decode_inline(&self, url: &str) -> Result<InlineMediaData, MediaAssetError> {
        use base64::{engine::general_purpose::STANDARD, Engine};
        if url.len() > MAX_LEGACY_MEDIA_URL_BYTES {
            return Err(MediaAssetError::TooLarge);
        };
        let (prefix, encoded) = url.split_once(";base64,").ok_or(MediaAssetError::Invalid)?;
        let mime = prefix
            .strip_prefix("data:")
            .filter(|mime| matches!(*mime, "image/png" | "image/jpeg" | "image/webp"))
            .ok_or(MediaAssetError::Invalid)?;
        let bytes = STANDARD
            .decode(encoded)
            .map_err(|_| MediaAssetError::Invalid)?;
        self.inspect(&bytes, mime)?;
        Ok(InlineMediaData {
            mime_type: mime.to_owned(),
            bytes,
        })
    }
    fn inspect(&self, bytes: &[u8], mime: &str) -> Result<MediaDescriptor, MediaAssetError> {
        if bytes.len() > MAX_MEDIA_ASSET_BYTES {
            return Err(MediaAssetError::TooLarge);
        }
        let (width, height) = match mime {
            "image/png" => png_size(bytes),
            "image/webp" => webp_size(bytes),
            "image/jpeg" => jpeg_size(bytes),
            _ => None,
        }
        .ok_or(MediaAssetError::Invalid)?;
        let descriptor = MediaDescriptor {
            sha256: Sha256::digest(bytes)
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect(),
            mime_type: mime.into(),
            byte_len: bytes.len() as u64,
            width,
            height,
        };
        if !descriptor.valid() {
            return Err(MediaAssetError::Invalid);
        }
        Ok(descriptor)
    }
}
fn be(bytes: &[u8]) -> Option<u32> {
    Some(u32::from_be_bytes(bytes.try_into().ok()?))
}
fn le(bytes: &[u8]) -> Option<u32> {
    Some(u32::from_le_bytes(bytes.try_into().ok()?))
}
fn crc(bytes: &[u8]) -> u32 {
    const TABLE: [u32; 256] = {
        let mut table = [0; 256];
        let mut i = 0;
        while i < 256 {
            let mut n = i as u32;
            let mut bit = 0;
            while bit < 8 {
                n = if n & 1 == 1 {
                    0xedb8_8320 ^ (n >> 1)
                } else {
                    n >> 1
                };
                bit += 1;
            }
            table[i] = n;
            i += 1;
        }
        table
    };
    !bytes.iter().fold(u32::MAX, |n, b| {
        TABLE[((n ^ u32::from(*b)) & 255) as usize] ^ (n >> 8)
    })
}
fn png_size(bytes: &[u8]) -> Option<(u32, u32)> {
    if !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return None;
    }
    let mut offset: usize = 8;
    let mut size = None;
    let mut image = false;
    while offset < bytes.len() {
        let length = be(bytes.get(offset..offset + 4)?)? as usize;
        let end = offset.checked_add(12)?.checked_add(length)?;
        let chunk = bytes.get(offset + 4..end - 4)?;
        if crc(chunk) != be(bytes.get(end - 4..end)?)? {
            return None;
        }
        let kind = chunk.get(..4)?;
        let payload = chunk.get(4..)?;
        match kind {
            b"IHDR" if offset == 8 && length == 13 => {
                if !matches!(payload[8], 1 | 2 | 4 | 8 | 16)
                    || !matches!(payload[9], 0 | 2 | 3 | 4 | 6)
                    || payload[10] != 0
                    || payload[11] != 0
                    || payload[12] > 1
                {
                    return None;
                }
                size = Some((be(&payload[..4])?, be(&payload[4..8])?));
            }
            b"IHDR" | b"acTL" => return None,
            b"IDAT" => image = true,
            b"IEND" => {
                return (length == 0 && image && end == bytes.len())
                    .then_some(size)
                    .flatten()
            }
            _ => {}
        }
        offset = end;
    }
    None
}
fn webp_size(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.get(..4)? != b"RIFF"
        || bytes.get(8..12)? != b"WEBP"
        || le(bytes.get(4..8)?)? as usize + 8 != bytes.len()
    {
        return None;
    }
    let mut offset: usize = 12;
    let mut canvas = None;
    let mut frame = None;
    while offset < bytes.len() {
        let kind = bytes.get(offset..offset + 4)?;
        let length = le(bytes.get(offset + 4..offset + 8)?)? as usize;
        let start = offset.checked_add(8)?;
        let end = start.checked_add(length)?;
        let payload = bytes.get(start..end)?;
        let found = match kind {
            b"VP8X" if offset == 12 && length == 10 && payload[0] & 2 == 0 => {
                let u24 = |v: &[u8]| {
                    1 + u32::from(v[0]) + (u32::from(v[1]) << 8) + (u32::from(v[2]) << 16)
                };
                canvas = Some((u24(&payload[4..7]), u24(&payload[7..10])));
                None
            }
            b"VP8 " if length >= 10 && payload.get(3..6)? == [0x9d, 0x01, 0x2a] => Some((
                u32::from(u16::from_le_bytes(payload[6..8].try_into().ok()?) & 0x3fff),
                u32::from(u16::from_le_bytes(payload[8..10].try_into().ok()?) & 0x3fff),
            )),
            b"VP8L" if length >= 5 && payload[0] == 0x2f => {
                let bits = le(&payload[1..5])?;
                Some((1 + (bits & 0x3fff), 1 + ((bits >> 14) & 0x3fff)))
            }
            b"VP8X" | b"VP8 " | b"VP8L" | b"ANIM" | b"ANMF" => return None,
            _ => None,
        };
        if let Some(found) = found {
            if frame.replace(found).is_some() {
                return None;
            }
        }
        offset = end.checked_add(length & 1)?;
    }
    if offset != bytes.len() || (canvas.is_some() && canvas != frame) {
        return None;
    }
    frame
}
fn jpeg_size(bytes: &[u8]) -> Option<(u32, u32)> {
    if !bytes.starts_with(&[0xff, 0xd8]) || !bytes.ends_with(&[0xff, 0xd9]) {
        return None;
    }
    let mut offset: usize = 2;
    let mut size = None;
    while offset < bytes.len() - 2 {
        if bytes[offset] != 0xff {
            return None;
        }
        while bytes.get(offset) == Some(&0xff) {
            offset += 1;
        }
        let kind = *bytes.get(offset)?;
        offset += 1;
        let length = u16::from_be_bytes(bytes.get(offset..offset + 2)?.try_into().ok()?) as usize;
        if length < 2 {
            return None;
        }
        let end = offset.checked_add(length)?;
        let payload = bytes.get(offset + 2..end)?;
        if matches!(kind, 0xc0..=0xc2) {
            if payload.len() < 6
                || payload[0] != 8
                || !matches!(payload[5], 1 | 3 | 4)
                || size.is_some()
            {
                return None;
            }
            size = Some((
                u32::from(u16::from_be_bytes(payload[3..5].try_into().ok()?)),
                u32::from(u16::from_be_bytes(payload[1..3].try_into().ok()?)),
            ));
        }
        if kind == 0xda {
            return size;
        }
        offset = end;
    }
    None
}
