//! Portable imports require full decoding, separately from container inspection.
use crate::asset_codec::RasterContentInspector;
use app_application::{models::media_asset::*, ports::media_assets::MediaContentInspector};
use image::{ImageDecoder, ImageFormat, ImageReader, Limits};
use std::io::Cursor;

/// Bounds the output buffer; decoder-internal allocations additionally use the
/// image crate's best-effort max_alloc limit, not a claimed process memory cap.
const MAX_DECODED_BYTES: u64 = 96 * 1024 * 1024;
pub struct DecodedRasterInspector;
impl MediaContentInspector for DecodedRasterInspector {
    fn verify_bundled_asset(
        &self,
        asset_id: &str,
        version: Option<u16>,
    ) -> Result<(), MediaAssetError> {
        static CATALOG: std::sync::OnceLock<serde_json::Value> = std::sync::OnceLock::new();
        let catalog = CATALOG.get_or_init(|| {
            serde_json::from_str(include_str!("../../../docs/visual/asset-manifest.json"))
                .expect("validated bundled asset manifest")
        });
        if catalog
            .get("assets")
            .and_then(serde_json::Value::as_array)
            .is_some_and(|assets| {
                assets.iter().any(|asset| {
                    (asset["id"].as_str() == Some(asset_id)
                        || asset["backendAssetId"].as_str() == Some(asset_id))
                        && version.is_none_or(|version| {
                            asset["version"].as_u64() == Some(u64::from(version))
                        })
                })
            })
        {
            Ok(())
        } else {
            Err(MediaAssetError::Missing)
        }
    }
    fn inspect(&self, bytes: &[u8], mime: &str) -> Result<MediaDescriptor, MediaAssetError> {
        RasterContentInspector.inspect(bytes, mime)
    }
    fn verify_decoded(
        &self,
        bytes: &[u8],
        descriptor: &MediaDescriptor,
    ) -> Result<(), MediaAssetError> {
        if !descriptor.valid() || self.inspect(bytes, &descriptor.mime_type)? != *descriptor {
            return Err(MediaAssetError::Invalid);
        }
        let format = match descriptor.mime_type.as_str() {
            "image/png" => ImageFormat::Png,
            "image/jpeg" => ImageFormat::Jpeg,
            "image/webp" => ImageFormat::WebP,
            _ => return Err(MediaAssetError::Invalid),
        };
        let mut limits = Limits::default();
        limits.max_image_width = Some(MAX_MEDIA_DIMENSION);
        limits.max_image_height = Some(MAX_MEDIA_DIMENSION);
        limits.max_alloc = Some(MAX_DECODED_BYTES);
        let mut reader = ImageReader::with_format(Cursor::new(bytes), format);
        reader.limits(limits);
        let decoder = reader
            .into_decoder()
            .map_err(|_| MediaAssetError::Invalid)?;
        if decoder.dimensions() != (descriptor.width, descriptor.height) {
            return Err(MediaAssetError::Invalid);
        }
        let total = decoder.total_bytes();
        if total == 0 || total > MAX_DECODED_BYTES {
            return Err(MediaAssetError::TooLarge);
        }
        let mut pixels = Vec::new();
        pixels
            .try_reserve_exact(total as usize)
            .map_err(|_| MediaAssetError::TooLarge)?;
        pixels.resize(total as usize, 0);
        decoder
            .read_image(&mut pixels)
            .map_err(|_| MediaAssetError::Invalid)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn encoded(format: ImageFormat) -> Vec<u8> {
        let mut output = Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(2, 1, image::Rgb([1, 2, 3])))
            .write_to(&mut output, format)
            .unwrap();
        output.into_inner()
    }
    #[test]
    fn real_supported_formats_decode_and_a_forged_descriptor_fails() {
        let inspector = DecodedRasterInspector;
        for (format, mime) in [
            (ImageFormat::Png, "image/png"),
            (ImageFormat::Jpeg, "image/jpeg"),
            (ImageFormat::WebP, "image/webp"),
        ] {
            let bytes = encoded(format);
            let descriptor = inspector.inspect(&bytes, mime).unwrap();
            inspector.verify_decoded(&bytes, &descriptor).unwrap();
            let mut forged = descriptor.clone();
            forged.width = 3;
            assert!(inspector.verify_decoded(&bytes, &forged).is_err());
            forged = descriptor;
            forged.sha256 = "a".repeat(64);
            assert!(inspector.verify_decoded(&bytes, &forged).is_err());
        }
    }
    fn repair_chunk_crc(bytes: &mut [u8], tag: usize) {
        let length = u32::from_be_bytes(bytes[tag - 4..tag].try_into().unwrap()) as usize;
        let mut crc = !0u32;
        for byte in &bytes[tag..tag + 4 + length] {
            crc ^= u32::from(*byte);
            for _ in 0..8 {
                crc = (crc >> 1) ^ (0xedb88320 & 0u32.wrapping_sub(crc & 1));
            }
        }
        bytes[tag + 4 + length..tag + 8 + length].copy_from_slice(&(!crc).to_be_bytes());
    }
    #[test]
    fn valid_container_crc_does_not_make_corrupt_compressed_pixels_valid() {
        let mut bytes = encoded(ImageFormat::Png);
        let tag = bytes.windows(4).position(|chunk| chunk == b"IDAT").unwrap();
        bytes[tag + 4] ^= 0xff;
        // Rebuild a correct chunk CRC, preserving the container's integrity while
        // deliberately corrupting its compressed pixel stream.
        repair_chunk_crc(&mut bytes, tag);
        let inspector = DecodedRasterInspector;
        let descriptor = RasterContentInspector.inspect(&bytes, "image/png").unwrap();
        assert!(inspector.verify_decoded(&bytes, &descriptor).is_err());
    }
    #[test]
    fn declared_large_sixteen_bit_pixels_fail_without_allocating_the_output() {
        let mut bytes = encoded(ImageFormat::Png);
        bytes[16..20].copy_from_slice(&8192u32.to_be_bytes());
        bytes[20..24].copy_from_slice(&4096u32.to_be_bytes());
        bytes[24] = 16;
        repair_chunk_crc(&mut bytes, 12);
        let descriptor = RasterContentInspector.inspect(&bytes, "image/png").unwrap();
        assert!(descriptor.valid());
        assert!(DecodedRasterInspector
            .verify_decoded(&bytes, &descriptor)
            .is_err());
    }
}
