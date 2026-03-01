// Image decoding for the Kitty Graphics Protocol.
//
// Handles PNG, RGB24, RGBA32 decoding, zlib decompression, and security validation
// for file/shared-memory transmission mediums.

use super::kitty_parser::{Compression, ParseError, PixelFormat};

/// Decode pixel data from the given format into RGBA32.
pub fn decode_pixels(
    data: &[u8],
    width: u32,
    height: u32,
    format: PixelFormat,
    compression: Compression,
) -> Result<Vec<u8>, ParseError> {
    // Decompress if needed
    let raw = match compression {
        Compression::Zlib => decompress_zlib(data, width, height, format)?,
        Compression::None => data.to_vec(),
    };

    match format {
        PixelFormat::Png => {
            let (_, _, pixels) = decode_png(&raw)?;
            Ok(pixels)
        }
        PixelFormat::Rgb24 => {
            let expected = width as usize * height as usize * 3;
            if raw.len() < expected {
                return Err(ParseError(format!(
                    "insufficient RGB data: expected {} bytes, got {}",
                    expected,
                    raw.len()
                )));
            }
            // Convert RGB to RGBA
            let pixel_count = width as usize * height as usize;
            let mut rgba = Vec::with_capacity(pixel_count * 4);
            for i in 0..pixel_count {
                let base = i * 3;
                rgba.push(raw[base]);     // R
                rgba.push(raw[base + 1]); // G
                rgba.push(raw[base + 2]); // B
                rgba.push(0xFF);          // A
            }
            Ok(rgba)
        }
        PixelFormat::Rgba32 => {
            let expected = width as usize * height as usize * 4;
            if raw.len() < expected {
                return Err(ParseError(format!(
                    "insufficient RGBA data: expected {} bytes, got {}",
                    expected,
                    raw.len()
                )));
            }
            Ok(raw[..expected].to_vec())
        }
    }
}

/// Decode a PNG image into (width, height, RGBA pixels).
pub fn decode_png(data: &[u8]) -> Result<(u32, u32, Vec<u8>), ParseError> {
    use image::GenericImageView;

    let img = image::load_from_memory(data)
        .map_err(|e| ParseError(format!("PNG decode error: {}", e)))?;

    let (w, h) = img.dimensions();
    let rgba = img.to_rgba8().into_raw();
    Ok((w, h, rgba))
}

/// Decompress zlib-compressed data with size limits.
fn decompress_zlib(
    data: &[u8],
    width: u32,
    height: u32,
    format: PixelFormat,
) -> Result<Vec<u8>, ParseError> {
    use flate2::read::ZlibDecoder;
    use std::io::Read;

    // Calculate maximum allowed decompressed size
    let bpp: u64 = match format {
        PixelFormat::Rgb24 => 3,
        PixelFormat::Rgba32 => 4,
        PixelFormat::Png => {
            // For PNG, we don't know the exact decompressed size ahead of time.
            // Use a generous limit: 100MB
            let max_size = 100 * 1024 * 1024;
            let mut decoder = ZlibDecoder::new(data);
            let mut result = Vec::new();
            decoder
                .take(max_size + 1)
                .read_to_end(&mut result)
                .map_err(|e| ParseError(format!("zlib decompression error: {}", e)))?;
            if result.len() as u64 > max_size {
                return Err(ParseError("zlib decompressed data exceeds size limit".to_string()));
            }
            return Ok(result);
        }
    };

    let max_decompressed = width as u64 * height as u64 * bpp + 1024; // small margin
    // Cap at 100MB absolute limit
    let max_decompressed = max_decompressed.min(100 * 1024 * 1024);

    let mut decoder = ZlibDecoder::new(data);
    let mut result = Vec::new();
    decoder
        .take(max_decompressed + 1)
        .read_to_end(&mut result)
        .map_err(|e| ParseError(format!("zlib decompression error: {}", e)))?;

    if result.len() as u64 > max_decompressed {
        return Err(ParseError(
            "zlib decompressed data exceeds expected size (possible bomb)".to_string(),
        ));
    }

    Ok(result)
}

/// Validate a file path for the file transmission medium (t=f).
/// Only allows paths under /tmp/ for security.
pub fn validate_file_path(path: &str) -> Result<(), ParseError> {
    if path.contains("..") {
        return Err(ParseError(
            "path traversal not allowed in file path".to_string(),
        ));
    }
    if !path.starts_with("/tmp/") && !path.starts_with("/tmp") {
        return Err(ParseError(
            "file path must be under /tmp/ for security".to_string(),
        ));
    }
    Ok(())
}

/// Validate a POSIX shared memory name for the shm transmission medium (t=s).
pub fn validate_shm_name(name: &str) -> Result<(), ParseError> {
    if name.contains("..") || name.contains('/') {
        return Err(ParseError(
            "path traversal not allowed in shared memory name".to_string(),
        ));
    }
    Ok(())
}

/// Compress data with zlib (used in tests).
#[cfg(test)]
fn zlib_compress(data: &[u8]) -> Vec<u8> {
    use flate2::write::ZlibEncoder;
    use flate2::Compression as FlateCompression;
    use std::io::Write;

    let mut encoder = ZlibEncoder::new(Vec::new(), FlateCompression::default());
    encoder.write_all(data).unwrap();
    encoder.finish().unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_rgba_passthrough() {
        let pixels = vec![0xFF, 0x00, 0x00, 0xFF, 0x00, 0xFF, 0x00, 0xFF]; // 2 pixels
        let result = decode_pixels(&pixels, 2, 1, PixelFormat::Rgba32, Compression::None).unwrap();
        assert_eq!(result.len(), 8);
        assert_eq!(&result[..4], &[0xFF, 0x00, 0x00, 0xFF]);
    }

    #[test]
    fn decode_rgb_to_rgba() {
        let pixels = vec![0xFF, 0x00, 0x00, 0x00, 0xFF, 0x00]; // 2 RGB pixels
        let result = decode_pixels(&pixels, 2, 1, PixelFormat::Rgb24, Compression::None).unwrap();
        assert_eq!(result.len(), 8);
        assert_eq!(&result[..4], &[0xFF, 0x00, 0x00, 0xFF]); // alpha added
        assert_eq!(&result[4..8], &[0x00, 0xFF, 0x00, 0xFF]);
    }

    #[test]
    fn decode_png_valid() {
        // Minimal 1x1 PNG (white pixel with alpha)
        let png_b64 = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/iZk9HQAAAABJRU5ErkJggg==";
        let png_bytes =
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, png_b64).unwrap();
        let result =
            decode_pixels(&png_bytes, 0, 0, PixelFormat::Png, Compression::None).unwrap();
        assert_eq!(result.len(), 4); // 1x1 RGBA
    }

    #[test]
    fn decode_png_extracts_dimensions() {
        let png_b64 = "iVBORw0KGgoAAAANSUhEUgAAAAIAAAACCAYAAABytg0kAAAAEklEQVR4nGP4z8DwHwyBNBgAAEnICff5q7YNAAAAAElFTkSuQmCC";
        let png_bytes =
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, png_b64).unwrap();
        let (w, h, _) = decode_png(&png_bytes).unwrap();
        assert_eq!(w, 2);
        assert_eq!(h, 2);
    }

    #[test]
    fn decode_invalid_png_returns_error() {
        let garbage = b"not a png at all";
        let result = decode_png(garbage);
        assert!(result.is_err());
    }

    #[test]
    fn decode_zlib_compressed_rgb() {
        let raw = vec![0xFF, 0x00, 0x00];
        let compressed = zlib_compress(&raw);
        let result =
            decode_pixels(&compressed, 1, 1, PixelFormat::Rgb24, Compression::Zlib).unwrap();
        assert_eq!(result.len(), 4);
        assert_eq!(&result[..3], &[0xFF, 0x00, 0x00]);
    }

    #[test]
    fn decode_insufficient_data_returns_error() {
        let pixels = vec![0u8; 4];
        let result = decode_pixels(&pixels, 10, 10, PixelFormat::Rgb24, Compression::None);
        assert!(result.is_err());
        assert!(result.unwrap_err().0.contains("insufficient"));
    }

    #[test]
    fn decode_zlib_bomb_rejected() {
        // Create zlib payload that decompresses to a huge size
        let large_data = vec![0u8; 10 * 1024 * 1024]; // 10MB
        let compressed = zlib_compress(&large_data);
        // Claiming 1x1 pixel but payload decompresses to 10MB — must reject
        let result = decode_pixels(&compressed, 1, 1, PixelFormat::Rgb24, Compression::Zlib);
        assert!(result.is_err(), "zlib bomb should be rejected");
    }

    #[test]
    fn decode_zlib_max_decompressed_size() {
        let raw = vec![0xFFu8; 3 * 100 * 100]; // 100x100 RGB = 30KB
        let compressed = zlib_compress(&raw);
        let result =
            decode_pixels(&compressed, 100, 100, PixelFormat::Rgb24, Compression::Zlib);
        assert!(
            result.is_ok(),
            "legitimate compressed data should decompress"
        );
    }

    // ── File medium security ────────────────────────────────────────

    #[test]
    fn file_medium_rejects_path_traversal() {
        let result = validate_file_path("../../etc/passwd");
        assert!(result.is_err(), "path traversal should be rejected");
    }

    #[test]
    fn file_medium_rejects_absolute_paths_outside_tmp() {
        let result = validate_file_path("/etc/passwd");
        assert!(result.is_err(), "non-tmp absolute path should be rejected");
    }

    #[test]
    fn file_medium_accepts_tmp_path() {
        let result = validate_file_path("/tmp/kitty-image-12345.png");
        assert!(result.is_ok(), "tmp path should be accepted");
    }

    #[test]
    fn shared_memory_medium_rejects_invalid_name() {
        let result = validate_shm_name("../../../etc/shadow");
        assert!(
            result.is_err(),
            "path traversal in shm name should be rejected"
        );
    }

    #[test]
    fn shared_memory_rejects_slashes() {
        let result = validate_shm_name("foo/bar");
        assert!(result.is_err(), "slashes in shm name should be rejected");
    }

    #[test]
    fn shared_memory_accepts_valid_name() {
        let result = validate_shm_name("kitty-img-12345");
        assert!(result.is_ok());
    }
}
